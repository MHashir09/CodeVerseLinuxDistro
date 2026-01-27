#!/bin/bash
# CVH Linux Installer - Finalize Module
# Password setting, cleanup, and reboot

# Set passwords
set_passwords() {
    step_header "Setting Passwords"

    echo -e "  ${BOLD}Set root password:${NC}"
    arch-chroot /mnt passwd root

    echo
    echo -e "  ${BOLD}Set password for $USERNAME:${NC}"
    arch-chroot /mnt passwd "$USERNAME"

    echo -e "\n  ${GREEN}✓${NC} Passwords set"
}

# Finish installation
finish_installation() {
    step_header "Finishing Installation"

    echo -n "  Syncing filesystems... "
    sync
    echo -e "${GREEN}done${NC}"

    unmount_all

    show_overall_progress

    show_completion

    echo -e "  ${BOLD}System Details:${NC}"
    echo -e "    Username:  ${CYAN}$USERNAME${NC}"
    echo -e "    Hostname:  ${CYAN}$HOSTNAME${NC}"
    echo -e "    Timezone:  ${CYAN}$TIMEZONE${NC}"
    echo -e "    Boot Mode: ${CYAN}$BOOT_MODE${NC}"
    echo

    echo -e "  ${BOLD}After Reboot:${NC}"
    echo -e "    1. ${GREEN}Ly display manager${NC} will appear on boot"
    echo -e "    2. Select ${CYAN}$COMPOSITOR${NC} session"
    echo -e "    3. Enter your username and password"
    echo -e "    4. Press ${CYAN}Mod+Return${NC} to open terminal"
    echo -e "    5. Press ${CYAN}Mod+D${NC} to open app launcher (cvh-fuzzy)"
    echo
    echo -e "  ${DIM}CVH Tools: cvh-fuzzy (launcher), cvh-icons (desktop icons)${NC}"
    echo

    read -r -p "  Press Enter to reboot..." _ || true
    reboot
}
