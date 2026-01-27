#!/bin/bash
# CVH Linux Installer - UI Module
# Colors, logging, progress bars, spinners, and banners

# Colors
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export MAGENTA='\033[0;35m'
export BOLD='\033[1m'
export DIM='\033[2m'
export NC='\033[0m'

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Progress bar function
# Usage: progress_bar <current> <total> <label>
progress_bar() {
    local current=$1
    local total=$2
    local label=${3:-"Progress"}
    local width=40
    local percent=$((current * 100 / total))
    local filled=$((current * width / total))
    local empty=$((width - filled))

    # Build the bar
    local bar=""
    for ((i=0; i<filled; i++)); do bar+="█"; done
    for ((i=0; i<empty; i++)); do bar+="░"; done

    printf "\r${CYAN}%s${NC} [${GREEN}%s${NC}] ${BOLD}%3d%%${NC}" "$label" "$bar" "$percent"
}

# Spinner function for background tasks
# Usage: run_with_spinner "message" command args...
run_with_spinner() {
    local message="$1"
    shift
    local spin_chars='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
    local pid

    # Run command in background
    "$@" &>/dev/null &
    pid=$!

    # Show spinner while command runs
    local i=0
    while kill -0 $pid 2>/dev/null; do
        local char="${spin_chars:i++%${#spin_chars}:1}"
        printf "\r${CYAN}%s${NC} %s" "$char" "$message"
        sleep 0.1
    done

    # Check exit status
    wait $pid
    local status=$?

    if [[ $status -eq 0 ]]; then
        printf "\r${GREEN}✓${NC} %s\n" "$message"
    else
        printf "\r${RED}✗${NC} %s\n" "$message"
        return $status
    fi
}

# Step header with progress
step_header() {
    ((CURRENT_STEP++))
    echo
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  Step ${CURRENT_STEP}/${TOTAL_STEPS}: $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo
}

# Overall progress indicator
show_overall_progress() {
    local width=60
    local filled=$((CURRENT_STEP * width / TOTAL_STEPS))
    local empty=$((width - filled))
    local percent=$((CURRENT_STEP * 100 / TOTAL_STEPS))

    echo
    echo -ne "${DIM}Overall Progress: ${NC}"
    echo -ne "${GREEN}"
    for ((i=0; i<filled; i++)); do echo -n "▓"; done
    echo -ne "${NC}${DIM}"
    for ((i=0; i<empty; i++)); do echo -n "░"; done
    echo -e "${NC} ${BOLD}${percent}%${NC}"
}

# Display welcome banner
show_welcome() {
    clear
    echo
    echo -e "${BOLD}${CYAN}"
    cat << 'EOF'
   ██████╗██╗   ██╗██╗  ██╗    ██╗     ██╗███╗   ██╗██╗   ██╗██╗  ██╗
  ██╔════╝██║   ██║██║  ██║    ██║     ██║████╗  ██║██║   ██║╚██╗██╔╝
  ██║     ██║   ██║███████║    ██║     ██║██╔██╗ ██║██║   ██║ ╚███╔╝
  ██║     ╚██╗ ██╔╝██╔══██║    ██║     ██║██║╚██╗██║██║   ██║ ██╔██╗
  ╚██████╗ ╚████╔╝ ██║  ██║    ███████╗██║██║ ╚████║╚██████╔╝██╔╝ ██╗
   ╚═════╝  ╚═══╝  ╚═╝  ╚═╝    ╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝
EOF
    echo -e "${NC}"
    echo -e "${DIM}                    CodeVerse Hub Linux Distribution${NC}"
    echo
    echo -e "  ${BOLD}Features:${NC}"
    echo -e "    ${GREEN}●${NC} Niri or Hyprland Wayland compositor"
    echo -e "    ${GREEN}●${NC} Ly display manager"
    echo -e "    ${GREEN}●${NC} Zsh + Oh My Zsh"
    echo -e "    ${GREEN}●${NC} Custom fuzzy finder & icon system"
    echo -e "    ${GREEN}●${NC} Minimal & lightweight"
    echo
    echo -e "  ${YELLOW}⚠${NC}  ${BOLD}WARNING:${NC} This will ERASE all data on the selected disk!"
    echo
    read -r -p "  Press Enter to begin installation or Ctrl+C to cancel..." _ || true
}

# Display completion banner
show_completion() {
    echo
    echo -e "${BOLD}${GREEN}"
    cat << 'EOF'
  ╔════════════════════════════════════════════════════════════════╗
  ║                                                                ║
  ║              Installation Complete!                            ║
  ║                                                                ║
  ╚════════════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"
}
