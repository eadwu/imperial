# Proof of Work

Clearly using the literal definition here. The aim of the document is just to provide an overview of the methodology and provide a way for people to test it out the long way since this project is in a state of it'll be done sometime in the future or never.

`imperial` is just meant to provide some wrappers around the below commands such that it ideally becomes somewhat like `nix-shell` for the end-user.

## Dependencies

- chroot
- mergerfs
- rorbind (source code is provided at `lib/rorbind`)

### Optional Dependencies

- qemu
  - Namely due to the ability some distributions provide that bypass the need to install from a virtual machine such as Fedora's `dnf`, but this is recommended for consistency and general ease of use

## Command Line

### Setup Directory Structure

OVMF is used to boot using UEFI, no special reason, BIOS can used as well but this is focused on UEFI.

```bash
OVMF_FD="$(nix-build "<nixpkgs>" -A OVMF.fd --no-out-link)"
OVERLAY_DIR="/persist/overlay"
DISTRO="arch"

sudo mkdir -p "${OVERLAY_DIR}/${DISTRO}/uefi" "${OVERLAY_DIR}/${DISTRO}/root"
sudo cp "${OVMF_FD}/FV/OVMF_CODE.fd" "${OVMF_FD}/FV/OVMF_VARS.fd" "${OVERLAY_DIR}/${DISTRO}/uefi"
```

### Setup VirtIO-FSD Socket

Some intricacies apply here, experiment with VirtIO-FSD options as some can prevent certain binaries (such as `passwd`) from working in the virtual machine.

```bash
VIRTIOFSD="$(nix-build "<nixpkgs>" --no-out-link -A qemu)/libexec/virtiofsd"
VIRTIOFSD_SOCKET_DIR="/var/run/virtiofsd"

OVERLAY_DIR="/persist/overlay"
DISTRO="arch"

sudo mkdir -p "${VIRTIOFSD_SOCKET_DIR}"
sudo "${VIRTIOFSD}" --socket-path="${VIRTIOFSD_SOCKET_DIR}/${DISTRO}.sock" -o source="${OVERLAY_DIR}/${DISTRO}/root"
```

Or with more options

```bash
# posix_acl needs FUSE support (build-time option)
VIRTIOFSD_OPTS=("-o" "flock" "-o" "posix_lock" "-o" "xattr" )
sudo "${VIRTIOFSD}" --socket-path="${VIRTIOFSD_SOCKET_DIR}/${DISTRO}.sock" "${VIRTIOFSD_OPTS[@]}" -o source="${OVERLAY_DIR}/${DISTRO}/root"
```

### Create a Disk Image

Namely for the ESP Partition needed to satisfy `systemd-boot` and UEFI requirements.

```bash
OVERLAY_DIR="/persist/overlay"
DISTRO="arch"
sudo qemu-img create -f qcow2 -o cluster_size=2M "${OVERLAY_DIR}/${DISTRO}/boot.qcow2" 1G
```

### Bootstrap on the Virtual Machine

An example script to startup the virtual machine.

```bash
VIRTIOFSD="$(nix-build "<nixpkgs>" --no-out-link -A qemu)/libexec/virtiofsd"
VIRTIOFSD_SOCKET_DIR="/var/run/virtiofsd"

OVERLAY_DIR="/persist/overlay"
DISTRO="arch"

MAC="$(printf '52:54:BE:EF:%02X:%02X' $((RANDOM%256)) $((RANDOM%256)))"
sudo qemu-system-x86_64 \
  # Communication sockets
  -chardev socket,id=guest-root,path="${VIRTIOFSD_SOCKET_DIR}/${DISTRO}.sock" \
  # Instantiate device (cache-size=2G = Enables DAX)
  -device vhost-user-fs-pci,queue-size=1024,chardev=guest-root,tag=root \
  # Force use of memory shareable with virtiofsd
  -m 4G \
  # -object memory-backend-file,id=mem,size=4G,mem-path=/dev/shm,share=on -numa node,memdev=mem
  -object memory-backend-memfd,id=mem,size=4G,share=on -numa node,memdev=mem \
  # Input Devices
  -device pci-bridge,chassis_nr=2,id=bridge.1 \
  -device virtio-keyboard-pci,bus=bridge.1,addr=03.0 \
  -device virtio-mouse-pci,bus=bridge.1,addr=04.0 \
  -device virtio-tablet-pci,bus=bridge.1,addr=05.0 \
  # Network Passthrough
  -netdev user,id=vmnic,hostfwd=tcp::2222-:22 -device virtio-net-pci,netdev=vmnic,mac="${MAC}" \
  # RNG Passthrough
  -object rng-random,id=rng0,filename=/dev/random -device virtio-rng-pci,rng=rng0 \
  # virtio display
  # -vga virtio -display sdl,gl=on \
  -vga virtio -display gtk,gl=on \
  # Resource Allocation
  -enable-kvm -cpu host -smp 4,sockets=1,cores=2,threads=2 \
  # Disks
  -boot order=dc,menu=on \
  ## UEFI Support
  -drive if=pflash,format=raw,readonly=on,file="${OVERLAY_DIR}/${DISTRO}/uefi/OVMF_CODE.fd" \
  -drive if=pflash,format=raw,file="${OVERLAY_DIR}/${DISTRO}/uefi/OVMF_VARS.fd" \
  ## "Hard Disk"
  -drive if=none,id=vda0,format=qcow2,file="${OVERLAY_DIR}/${DISTRO}/boot.qcow2",cache=none,cache.direct=on,aio=native,discard=unmap \
  -object iothread,id=io1 -device virtio-scsi-pci,ioeventfd=on,iothread=io1,num_queues=8 -device scsi-hd,drive=vda0 \
  ## Installation ISO
  -drive file="$(readlink -f "${OVERLAY_DIR}/${DISTRO}/iso/"*.iso)",readonly=on,media=cdrom
```

#### Example (Arch Linux)

Enable `sshd` by ensuring `PermitRootLogin` is `yes` in `/etc/ssh/sshd_config` before running `systemctl start sshd` and setting the root password via `passwd`.

SSH into the virtual machine through `ssh -o "UserKnownHostsFile /dev/null" -p 2222 root@localhost`.

To ensure the virtual machine booted through UEFI, check for the existance of `/sys/firmware/efi/efivars`, such as `ls /sys/firmware/efi/efivars`.

Bootstrap the root filesystem

```bash
timedatectl set-ntp true

fdisk /dev/sda # g -> n -> t (1)
mkfs.fat -F32 /dev/sda1

mount -t virtiofs root /mnt

mkdir /mnt/boot
mount /dev/sda1 /mnt/boot

pacman -Sy pacman-contrib # rankmirrors binary
curl -sSL "https://www.archlinux.org/mirrorlist/?country=US&protocol=http&protocol=https&ip_version=4&use_mirror_status=on" > /etc/pacman.d/mirrorlist.source
sed -i 's/^#\(Server\)/\1/' /etc/pacman.d/mirrorlist.source
rankmirrors -n 6 /etc/pacman.d/mirrorlist.source > /etc/pacman.d/mirrorlist

# base-devel was added (contains build utilities)
# linux-hardened-headers, ... + added
pacstrap /mnt base base-devel linux-hardened linux-firmware linux-hardened-headers man-pages man-db texinfo pacman-contrib

# `noatime` may be added to mount flags, i.e. "rw,noatime"
genfstab -U /mnt >> /mnt/etc/fstab
```

Enter a `chroot` under `/mnt` (`arch-chroot /mnt`) and setup the system.

```bash
arch-chroot /mnt

curl -sSL "https://www.archlinux.org/mirrorlist/?country=US&protocol=http&protocol=https&ip_version=4&use_mirror_status=on" > /etc/pacman.d/mirrorlist.source
sed -i 's/^#\(Server\)/\1/' /etc/pacman.d/mirrorlist.source
rankmirrors -n 6 /etc/pacman.d/mirrorlist.source > /etc/pacman.d/mirrorlist

ln -sf /usr/share/zoneinfo/America/New_York /etc/localtime
hwclock --systohc --utc

sed -i 's/^#\(en_US.UTF-8 UTF-8\)/\1/' /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf
export LANG="en_US.UTF-8"

echo "arch" > /etc/hostname

mkinitcpio -P

# Set root password
## If bugged, try changing in a chroot instead of VM
### Remove `x` from /etc/passwd to root for passwordless root login
passwd
useradd -m -g users -G wheel -s /bin/bash arch
## If groups are bugged try manually adding them
groupadd users
groupadd wheel
passwd arch
echo "%wheel ALL=(ALL) ALL" > /etc/sudoers.d/10-grant-wheel-group

bootctl --path /boot install
# Comment out default at /boot/loader/loader.conf
# /boot/loader/entries/default.conf
## title   Arch Linux
## linux   /vmlinuz-linux-hardened
## initrd  /initramfs-linux-hardened.img
## options rootfstype=virtiofs root=root rw

exit
```

Finalize installation using `umount -R /mnt` and `shutdown now`.

### Union Filesystems

#### MergerFS (with RORBind)

```bash
ROOT_RO="$(mktemp -d)"
sudo rorbind / "${ROOT_RO}"

ARCH_RO="$(mktemp -d)"
sudo rorbind /persist/overlay/arch/root "${ARCH_RO}"

CHROOT_ROOT="$(mktemp -d)"
sudo mergerfs \
  -o allow_other,use_ino,cache.files=partial,dropcacheonclose=true,category.create=mfs \
  -o posix_acl=true \
  "${ROOT_RO}"=RO:/persist/overlay/arch/root=RW "${CHROOT_ROOT}"
```

#### OverlayFS (Untested)

```bash
# OverlayFS (not supported for upper layer for BCacheFS and ZFS)
sudo mount -t overlay overlay -olowerdir=/,upperdir=/persist/overlay/arch/root,workdir=/ "$(readlink -f ./root)"
```

#### MergerFS

```bash
# MergerFS
ROOT_RO="$(mktemp -d)"
sudo mount -o rbind / "${ROOT_RO}"
sudo mount -o bind,remount,ro "${ROOT_RO}" "${ROOT_RO}"

ARCH_RO="$(mktemp -d)"
sudo mount -o bind,ro /persist/overlay/arch/root "${ARCH_RO}"

CHROOT_ROOT="$(mktemp -d)"
sudo mergerfs \
  -o allow_other,use_ino,cache.files=partial,dropcacheonclose=true,category.create=mfs \
  -o posix_acl=true \
  "${ROOT_RO}"=RO:/persist/overlay/arch/root=RW "${CHROOT_ROOT}"
```

#### UnionFS-FUSE (Untested)

```bash
# UnionFS-Fuse
CHROOT_ROOT="$(mktemp -d)"
mount -t unionfs-fuse none /dest -o dirs=/source1=RW,/source2=RO
```

### Chroot

```bash
cd "${CHROOT_ROOT}"
sudo mount -t proc /proc proc/
sudo mount -t sysfs /sys sys/
# Warning: When using --rbind, some subdirectories of dev/ and sys/ will not be unmountable.
# Attempting to unmount with umount -l in this situation will break your session, requiring a reboot.
#   If possible, use -o bind instead.
sudo mount -o bind /dev dev/
sudo mount -o bind /run run/
sudo mount -o bind /sys/firmware/efi/efivars sys/firmware/efi/efivars/

sudo chroot "${CHROOT_ROOT}" /bin/bash
# Graphical Application support (unneeded by default most likely)
## export DISPLAY="unix${DISPLAY}" # nomially probably unix:0
# FHS exposure to Shell
## export PATH="${PATH}:/usr/bin:/usr/local/bin"
```

When done, the environment can be erased using `sudo umount --recursive "${CHROOT_ROOT}"` or `sudo fusermount -u [-z] "${CHROOT_ROOT}"`

## Security Considerations

### VirtIO-FS

See https://vmsplice.net/~stefan/virtio-fs_%20A%20Shared%20File%20System%20for%20Virtual%20Machines%20%28FOSDEM%29.pdf for a full overview but a few will be listed below. And yes, this example doesn't follow most of them.

- Guests have full uid/gid access to shared directory
- Guests have no access outside shared directory
- Use dedicated file system for shared directory to prevent inode exhaustion or other Denial-of-Service attacks
- Parent directory of shared directory should have rwx------ permissions to prevent non-owners from accessing untrusted files
- Mount shared directory nosuid,nodev on host
