#!/bin/bash
# CVH Linux Installer - Disk Module
# Disk partitioning and formatting functions

# Partition the disk
partition_disk() {
    step_header "Partitioning Disk"

    echo "  Preparing disk..."

    # Wipe existing partition table
    wipefs -af "$DISK" >/dev/null 2>&1 || true
    sgdisk -Z "$DISK" >/dev/null 2>&1 || true

    if [[ "$BOOT_MODE" == "uefi" ]]; then
        partition_disk_uefi
    else
        partition_disk_bios
    fi

    echo -e "\n\n  ${GREEN}✓${NC} Disk prepared successfully"
}

# Partition disk for UEFI
partition_disk_uefi() {
    echo -e "  ${BLUE}●${NC} Creating GPT partition table (UEFI)"

    parted -s "$DISK" mklabel gpt
    progress_bar 1 5 "  Partitioning"

    parted -s "$DISK" mkpart primary fat32 1MiB 513MiB
    parted -s "$DISK" set 1 esp on
    progress_bar 2 5 "  Partitioning"

    parted -s "$DISK" mkpart primary ext4 513MiB 100%
    progress_bar 3 5 "  Partitioning"

    EFI_PART=$(get_partition_name "$DISK" 1)
    ROOT_PART=$(get_partition_name "$DISK" 2)

    sleep 1  # Wait for kernel to recognize partitions

    echo -e "\n  ${BLUE}●${NC} Formatting EFI partition (FAT32)"
    mkfs.fat -F32 "$EFI_PART" >/dev/null 2>&1
    progress_bar 4 5 "  Formatting "

    echo -e "\n  ${BLUE}●${NC} Formatting root partition (ext4)"
    mkfs.ext4 -F "$ROOT_PART" >/dev/null 2>&1
    progress_bar 5 5 "  Formatting "

    echo -e "\n\n  ${BLUE}●${NC} Mounting partitions"
    mount "$ROOT_PART" /mnt
    mkdir -p /mnt/boot/efi
    mount "$EFI_PART" /mnt/boot/efi
}

# Partition disk for BIOS
partition_disk_bios() {
    echo -e "  ${BLUE}●${NC} Creating MBR partition table (BIOS)"

    parted -s "$DISK" mklabel msdos
    progress_bar 1 4 "  Partitioning"

    parted -s "$DISK" mkpart primary ext4 1MiB 100%
    parted -s "$DISK" set 1 boot on
    progress_bar 2 4 "  Partitioning"

    ROOT_PART=$(get_partition_name "$DISK" 1)

    sleep 1

    echo -e "\n  ${BLUE}●${NC} Formatting root partition (ext4)"
    mkfs.ext4 -F "$ROOT_PART" >/dev/null 2>&1
    progress_bar 3 4 "  Formatting "

    echo -e "\n\n  ${BLUE}●${NC} Mounting partition"
    mount "$ROOT_PART" /mnt
    progress_bar 4 4 "  Mounting   "
}

# Generate fstab
generate_fstab() {
    step_header "Generating Filesystem Table"

    echo -n "  Creating /etc/fstab... "
    genfstab -U /mnt >> /mnt/etc/fstab
    echo -e "${GREEN}done${NC}"

    echo -e "\n  ${GREEN}✓${NC} Filesystem table generated"
}

# Unmount all partitions
unmount_all() {
    echo -n "  Unmounting partitions... "
    umount -R /mnt
    echo -e "${GREEN}done${NC}"
}
