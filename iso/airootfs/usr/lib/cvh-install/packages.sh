#!/bin/bash
# CVH Linux Installer - Packages Module
# Package installation functions

# Install base system
install_base() {
    step_header "Installing Base System"

    # Check network connectivity
    if ! check_network; then
        exit 1
    fi

    # Initialize pacman keyring
    echo -e "  ${BLUE}●${NC} Initializing package keyring..."
    pacman-key --init >/dev/null 2>&1
    pacman-key --populate archlinux >/dev/null 2>&1
    echo -e "  ${GREEN}✓${NC} Keyring initialized"

    # Build package list
    local packages=()
    while IFS= read -r pkg; do
        [[ -n "$pkg" ]] && packages+=($pkg)
    done < <(get_all_packages)

    echo
    echo -e "  ${BLUE}●${NC} Installing packages (this may take a while)..."
    echo -e "  ${DIM}────────────────────────────────────────────────────────${NC}"
    echo

    # Run pacstrap - show output directly
    if pacstrap -K /mnt "${packages[@]}"; then
        echo
        echo -e "  ${DIM}────────────────────────────────────────────────────────${NC}"
        echo -e "  ${GREEN}✓${NC} Base system installed"
    else
        echo
        echo -e "  ${RED}✗${NC} Package installation failed!"
        exit 1
    fi
}

# Copy CVH custom packages from ISO
copy_cvh_packages() {
    echo -e "  ${BLUE}●${NC} Copying CVH custom packages from ISO..."

    mkdir -p /mnt/var/cache/pacman/cvh-packages
    if [[ -d /opt/cvh-repo ]] && ls /opt/cvh-repo/*.pkg.tar.zst >/dev/null 2>&1; then
        cp /opt/cvh-repo/*.pkg.tar.zst /mnt/var/cache/pacman/cvh-packages/ 2>/dev/null || true
        echo -e "  ${GREEN}✓${NC} CVH packages copied ($(ls /opt/cvh-repo/*.pkg.tar.zst 2>/dev/null | wc -l) packages)"
    else
        echo -e "  ${YELLOW}⚠${NC}  CVH packages not found on ISO"
    fi
}

# Create mirrorlist for installed system
create_mirrorlist() {
    echo -e "  ${BLUE}●${NC} Creating package mirrorlist..."
    mkdir -p /mnt/etc/pacman.d
    cat > /mnt/etc/pacman.d/mirrorlist << 'EOF'
# Arch Linux mirrorlist - CVH Linux
# Israeli mirrors
Server = https://mirror.isoc.org.il/pub/archlinux/$repo/os/$arch
Server = https://archlinux.mivzakim.net/$repo/os/$arch
# Global mirrors
Server = https://geo.mirror.pkgbuild.com/$repo/os/$arch
Server = https://mirrors.kernel.org/archlinux/$repo/os/$arch
Server = https://mirror.rackspace.com/archlinux/$repo/os/$arch
EOF
    echo -e "  ${GREEN}✓${NC} Mirrorlist created"
}

# Configure pacman repositories
configure_pacman_repos() {
    echo -e "  ${BLUE}●${NC} Configuring package repositories..."
    if [[ -f /mnt/etc/pacman.conf ]]; then
        # Check if repos are already configured
        if ! grep -q "^\[core\]" /mnt/etc/pacman.conf; then
            cat >> /mnt/etc/pacman.conf << 'EOF'

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist
EOF
            echo -e "  ${GREEN}✓${NC} Repositories configured"
        else
            echo -e "  ${GREEN}✓${NC} Repositories already configured"
        fi
    fi
}
