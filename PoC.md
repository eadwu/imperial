# Proof of Concept

Clearly using the literal definition here. The aim of the document is just to provide an overview of the methodology and provide a way for people to test it out the long way since this project is in a state of it'll be done sometime in the future or never.

`imperial` is just meant to provide some wrappers around the below commands such that it ideally becomes somewhat like `nix-shell` for the end-user.

## Dependencies

- chroot
- mergerfs
- rorbind (source code is provided at `lib/rorbind`)

### Optional Dependencies

- qemu
  - Optional namely due to the ability some distributions provide that bypass the need to install from a virtual machine such as Fedora's `dnf`, but this is recommended for consistency and general ease of use

## Command Line

It is assumed that the reader is on NixOS ( if not, ... ), is an unprivileged user without access to a privileged user, and that unprivileged user namespaces ( the epitome of fake it until you make it ) are enabled as is by default.

The following are a couple of explanations for some weird choices
- Usage of binaries under `/run/current-system/sw/bin` instead of just relying on `$PATH`
  - setuid wrappers are broken under user namespaces in NixOS (or at least the wrapper script NixOS uses)

### One-shot environment variables setup

Nothing special, copy and paste.

```bash
OVMF_FD="$(nix-build "<nixpkgs>" -A OVMF.fd --no-out-link)"

VIRTIOFSD="$(nix-build "<nixpkgs>" --no-out-link -A qemu)/libexec/virtiofsd"
VIRTIOFSD_SOCKET_DIR="/tmp"

OVERLAY_DIR="/persist/overlay"
DISTRO="arch"
```

### Setup Directory Structure

OVMF is used to boot using UEFI, no special reason, BIOS can used as well but this is focused on UEFI.

```bash
OVMF_FD="$(nix-build "<nixpkgs>" -A OVMF.fd --no-out-link)"
OVERLAY_DIR="/persist/overlay"
DISTRO="arch"

mkdir -p "${OVERLAY_DIR}/${DISTRO}/uefi" "${OVERLAY_DIR}/${DISTRO}/root"
cp "${OVMF_FD}/FV/OVMF_CODE.fd" "${OVMF_FD}/FV/OVMF_VARS.fd" "${OVERLAY_DIR}/${DISTRO}/uefi"
```

### Setup VirtIO-FSD Socket

Some intricacies apply here, experiment with VirtIO-FSD options as some can prevent certain binaries (such as `passwd`) from working in the virtual machine.

```bash
unshare --user --mount --map-root-user --propagation slave

> # Needed for virtiofsd to write its PID file into /var/run/virtiofsd
> /run/current-system/sw/bin/mount -t tmpfs none /var

> VIRTIOFSD="$(nix-build "<nixpkgs>" --no-out-link -A qemu)/libexec/virtiofsd"
> VIRTIOFSD_SOCKET_DIR="/tmp"
>
> OVERLAY_DIR="/persist/overlay"
> DISTRO="arch"
>
> mkdir -p "${VIRTIOFSD_SOCKET_DIR}"
> "${VIRTIOFSD}" --socket-path="${VIRTIOFSD_SOCKET_DIR}/${DISTRO}.sock" -o source="$(readlink -f "${OVERLAY_DIR}/${DISTRO}/root")"
```

Or with more options

```bash
# -o posix_acl needs FUSE support (build-time option)
> VIRTIOFSD_OPTS=( "-o" "flock" "-o" "posix_lock" "-o" "xattr" )
> "${VIRTIOFSD}" --socket-path="${VIRTIOFSD_SOCKET_DIR}/${DISTRO}.sock" "${VIRTIOFSD_OPTS[@]}" -o source="${OVERLAY_DIR}/${DISTRO}/root"
```

### Create a Disk Image

Namely for the ESP Partition needed to satisfy `systemd-boot` and UEFI requirements.

```bash
OVERLAY_DIR="/persist/overlay"
DISTRO="arch"
qemu-img create -f qcow2 -o cluster_size=2M "${OVERLAY_DIR}/${DISTRO}/boot.qcow2" 1G
```

### Bootstrap on the Virtual Machine

An example script to startup the virtual machine.

```bash
VIRTIOFSD="$(nix-build "<nixpkgs>" --no-out-link -A qemu)/libexec/virtiofsd"
VIRTIOFSD_SOCKET_DIR="/tmp"

OVERLAY_DIR="/persist/overlay"
DISTRO="arch"

MAC="$(printf '52:54:BE:EF:%02X:%02X' $((RANDOM%256)) $((RANDOM%256)))"
qemu-system-x86_64 \
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

#### Mounting Read-Only Drives

##### With `rorbind`

```bash
ROOT_RO="$(mktemp -d)"
rorbind / "${ROOT_RO}"
```

##### With `bindfs`

```bash
ROOT_RO="$(mktemp -d)"
bindfs -o ro / "${ROOT_RO}"
```

##### With `mount`

This may have some unintuitive behaviors compared to `rorbind` and `bindfs`, namely submounts arenot read-only in particular.

```bash
ROOT_RO="$(mktemp -d)"
/run/current-system/sw/bin/mount -o rbind / "${ROOT_RO}"
/run/current-system/sw/bin/mount -o bind,remount,ro "${ROOT_RO}" "${ROOT_RO}"
```

#### Creating Unioned `/` Filesystem

Only `mergerfs` is recommended here, CoW features are not actually desired here for what is envisioned (by thee author).

Other union filesystem commands are provided as a reference and should not be _blindly_ run unless the consequences don't matter to the reader. For example, in `unionfs`, `-ocow` is _needed_ for read-only branch support, thus `rorbind` is needed as a workaround for read-only branches without copy-on-write.

##### MergerFS

```bash
# MergerFS
CHROOT_ROOT="$(mktemp -d)"
mergerfs \
  -o allow_other,use_ino,cache.files=partial,dropcacheonclose=true,category.create=mfs \
  -o auto_unmount \
  "${OVERLAY_DIR}/${DISTRO}/root"=RW:"${ROOT_RO}"=RO "${CHROOT_ROOT}"
```

##### OverlayFS

Note that ZFS or BCacheFS is only allowed as an lower filesystem, so workarounds are needed for the kernel provided OverlayFS.

```bash
CHROOT_ROOT="$(mktemp -d)"
WORK_DIR="$(mktemp -d)"

# Kernel OverlayFS
/run/current-system/sw/bin/mount -t tmpfs none "${WORK_DIR}"
/run/current-system/sw/bin/mount -t overlay overlay -olowerdir=/,upperdir="${OVERLAY_DIR}/${DISTRO}/root",workdir="${WORK_DIR}" "${CHROOT_ROOT}"

# fuse-overlayfs
fuse-overlayfs -o lowerdir="${OVERLAY_DIR}/${DISTRO}/root",upperdir=/,workdir="${WORK_DIR}" "${CHROOT_ROOT}"
```

##### UnionFS

```bash
# UnionFS-Fuse
CHROOT_ROOT="$(mktemp -d)"
mkdir -p "${CHROOT_PATH}/root" "${CHROOT_PATH}/arch"

rorbind / "${CHROOT_PATH}/root"
/run/current-system/sw/bin/mount -o bind "${OVERLAY_DIR}/${DISTRO}/root" "${CHROOT_PATH}/arch"

UNION_ROOT="$(mktemp -d)"
unionfs -o allow_other,use_ino -o cow,chroot="${CHROOT_PATH}" /root=RO:/arch=RW "${UNION_ROOT}"
```

### Chroot

Mount (or remount) the necessary subsystems.

```bash
cd "${CHROOT_ROOT}"
/run/current-system/sw/bin/mount -t proc /proc proc/
/run/current-system/sw/bin/mount -t sysfs /sys sys/
# Warning: When using --rbind, some subdirectories of dev/ and sys/ will not be unmountable.
# Attempting to unmount with umount -l in this situation will break your session, requiring a reboot.
#   If possible, use -o bind instead.
/run/current-system/sw/bin/mount -o bind /dev dev/
/run/current-system/sw/bin/mount -o bind /run run/
/run/current-system/sw/bin/mount -o bind /sys/firmware/efi/efivars sys/firmware/efi/efivars/

chroot "${CHROOT_ROOT}" /bin/bash
# Graphical Application support (unneeded by default most likely)
## export DISPLAY="unix${DISPLAY}" # nomially probably unix:0
# FHS exposure to Shell
## export PATH="${PATH}:/usr/bin:/usr/local/bin"
```

When done, the environment can be erased using `/run/current-system/sw/bin/umount --recursive "${CHROOT_ROOT}"` or `/run/current-system/sw/bin/fusermount -u [-z] "${CHROOT_ROOT}"`

#### Example

```bash
unshare --user --pid --mount --map-root-user --fork --mount-proc --propagation slave

> # If --fork --mount-proc is omitted
> /run/current-system/sw/bin/mount --make-rprivate /proc
> /run/current-system/sw/bin/umount /proc
> /run/current-system/sw/bin/mount -t proc -o nosuid,nodev,noexec proc /proc
>
> CHROOT_PATH="$(mktemp -d)"
>
> mkdir -p "${CHROOT_PATH}/arch_ro"
> rorbind "${OVERLAY_DIR}/${DISTRO}/root" "${CHROOT_PATH}/arch_ro"
>
> mkdir -p "${CHROOT_PATH}/root"
> /run/current-system/sw/bin/mount -o rbind / "${CHROOT_PATH}/root"
>
> UNION_ROOT="$(mktemp -d)"
> mergerfs -o use_ino,cache.files=off,dropcacheonclose=true,allow_other,category.create=mfs -o auto_unmount /=RW:"${CHROOT_PATH}/arch_ro"=RO "${UNION_ROOT}"
>
> cd "${UNION_ROOT}"
> /run/current-system/sw/bin/mount -t proc none proc/
> # Sadly not permitted in a user namespace without MS_REC
> # Is a problem, but not really, since it resolves itself when the
> #   namespace is discarded
> /run/current-system/sw/bin/mount -o rbind,nosuid /dev dev/
> # Sticky bit fails in user namespace, so remount /tmp
> /run/current-system/sw/bin/mount -t tmpfs none tmp/
>
> chroot "${UNION_ROOT}"
> > ...
> /run/current-system/sw/bin/fusermount -u -z "${CHROOT_ROOT}"
```

This examples has several pitfalls, some of which are outlined below
- It is not currently possible to run `pivot_root` on the unioned root with `mergerfs` (see [trapexit/mergerfs@935](https://github.com/trapexit/mergerfs/issues/935))
  - In the first place, `mergerfs` should be run as `root` so this is a result of the workarounds used for unprivileged access
  - This prevents a subsequent `unshare` to remap `root` to the default `$UID` and `$GID` which will add confusion to certain applications (i.e. `chromium` and `code-insiders`)

## Security Considerations

### VirtIO-FS

See https://vmsplice.net/~stefan/virtio-fs_%20A%20Shared%20File%20System%20for%20Virtual%20Machines%20%28FOSDEM%29.pdf for a full overview but a few will be listed below. And yes, this example doesn't follow most of them.

- Guests have full uid/gid access to shared directory
- Guests have no access outside shared directory
- Use dedicated file system for shared directory to prevent inode exhaustion or other Denial-of-Service attacks
- Parent directory of shared directory should have rwx------ permissions to prevent non-owners from accessing untrusted files
- Mount shared directory nosuid,nodev on host
