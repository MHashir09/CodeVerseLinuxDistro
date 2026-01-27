#!/bin/bash
# CVH Linux Installer - Input Module
# User input and selection functions

# Select keyboard layout
select_keyboard() {
    step_header "Keyboard Layout"

    echo "  Available layouts:"
    echo -e "    ${BOLD}1)${NC} us - US English ${DIM}[default]${NC}"
    echo -e "    ${BOLD}2)${NC} uk - UK English"
    echo -e "    ${BOLD}3)${NC} de - German"
    echo -e "    ${BOLD}4)${NC} fr - French"
    echo -e "    ${BOLD}5)${NC} es - Spanish"
    echo -e "    ${BOLD}6)${NC} il - Hebrew"
    echo -e "    ${BOLD}7)${NC} Other"
    echo

    read -r -p "  Select layout [1]: " kb_choice
    kb_choice=${kb_choice:-1}

    case $kb_choice in
        1) KEYMAP="us" ;;
        2) KEYMAP="uk" ;;
        3) KEYMAP="de" ;;
        4) KEYMAP="fr" ;;
        5) KEYMAP="es" ;;
        6) KEYMAP="il" ;;
        7) read -r -p "  Enter keymap name: " KEYMAP ;;
        *) KEYMAP="us" ;;
    esac

    loadkeys "$KEYMAP" 2>/dev/null || true
    echo -e "\n  ${GREEN}✓${NC} Keyboard: ${BOLD}$KEYMAP${NC}"
}

# Select timezone
select_timezone() {
    step_header "Timezone"

    echo "  Common timezones:"
    echo -e "    ${BOLD}1)${NC} Asia/Jerusalem ${DIM}[default]${NC}"
    echo -e "    ${BOLD}2)${NC} UTC"
    echo -e "    ${BOLD}3)${NC} America/New_York"
    echo -e "    ${BOLD}4)${NC} America/Los_Angeles"
    echo -e "    ${BOLD}5)${NC} Europe/London"
    echo -e "    ${BOLD}6)${NC} Europe/Berlin"
    echo -e "    ${BOLD}7)${NC} Other"
    echo

    read -r -p "  Select timezone [1]: " tz_choice
    tz_choice=${tz_choice:-1}

    case $tz_choice in
        1) TIMEZONE="Asia/Jerusalem" ;;
        2) TIMEZONE="UTC" ;;
        3) TIMEZONE="America/New_York" ;;
        4) TIMEZONE="America/Los_Angeles" ;;
        5) TIMEZONE="Europe/London" ;;
        6) TIMEZONE="Europe/Berlin" ;;
        7) read -r -p "  Enter timezone (Region/City): " TIMEZONE ;;
        *) TIMEZONE="Asia/Jerusalem" ;;
    esac

    echo -e "\n  ${GREEN}✓${NC} Timezone: ${BOLD}$TIMEZONE${NC}"
}

# Select compositor
select_compositor() {
    step_header "Compositor Selection"

    echo "  Available Wayland compositors:"
    echo -e "    ${BOLD}1)${NC} Niri - Scrollable-tiling compositor ${DIM}[default]${NC}"
    echo -e "    ${BOLD}2)${NC} Hyprland - Dynamic tiling compositor"
    echo

    read -r -p "  Select compositor [1]: " comp_choice
    comp_choice=${comp_choice:-1}

    case $comp_choice in
        1) COMPOSITOR="niri" ;;
        2) COMPOSITOR="hyprland" ;;
        *) COMPOSITOR="niri" ;;
    esac

    echo -e "\n  ${GREEN}✓${NC} Compositor: ${BOLD}$COMPOSITOR${NC}"
}

# Select disk for installation
select_disk() {
    step_header "Disk Selection"

    echo "  Available disks:"
    echo

    # Filter and display disks with nice formatting
    local i=1
    while IFS= read -r line; do
        local name=$(echo "$line" | awk '{print $1}')
        local size=$(echo "$line" | awk '{print $2}')
        local model=$(echo "$line" | awk '{$1=$2=""; print $0}' | xargs)
        printf "    ${BOLD}%d)${NC} /dev/%-8s ${CYAN}%8s${NC}  %s\n" "$i" "$name" "$size" "$model"
        ((i++))
    done < <(list_disks)
    echo

    # Get list of disks
    local disks=($(get_disk_names))

    if [[ ${#disks[@]} -eq 0 ]]; then
        log_error "No suitable disks found!"
        exit 1
    fi

    read -r -p "  Enter disk number: " disk_num

    if [[ ! "$disk_num" =~ ^[0-9]+$ ]] || [[ $disk_num -lt 1 ]] || [[ $disk_num -gt ${#disks[@]} ]]; then
        log_error "Invalid selection!"
        exit 1
    fi

    DISK="/dev/${disks[$((disk_num-1))]}"

    echo
    echo -e "  ${YELLOW}⚠${NC}  Selected: ${BOLD}$DISK${NC}"
    echo -e "  ${RED}    ALL DATA WILL BE DESTROYED!${NC}"
    echo
    read -r -p "  Type 'yes' to confirm: " confirm
    if [[ "$confirm" != "yes" ]]; then
        log_error "Installation cancelled"
        exit 1
    fi

    echo -e "\n  ${GREEN}✓${NC} Disk: ${BOLD}$DISK${NC}"
}

# Set hostname
set_hostname() {
    step_header "System Configuration"

    read -r -p "  Enter hostname [cvh-linux]: " input_hostname
    HOSTNAME=${input_hostname:-cvh-linux}

    if [[ ! "$HOSTNAME" =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?$ ]]; then
        log_warn "Invalid hostname, using: cvh-linux"
        HOSTNAME="cvh-linux"
    fi

    echo -e "  ${GREEN}✓${NC} Hostname: ${BOLD}$HOSTNAME${NC}"
}

# Create user account configuration
create_user_config() {
    echo
    read -r -p "  Enter username [cvh]: " input_username
    USERNAME=${input_username:-cvh}

    if [[ ! "$USERNAME" =~ ^[a-z_][a-z0-9_-]*$ ]]; then
        log_warn "Invalid username, using: cvh"
        USERNAME="cvh"
    fi

    echo -e "  ${GREEN}✓${NC} Username: ${BOLD}$USERNAME${NC}"
}
