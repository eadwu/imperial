use cty;
use libc::*;

/* include/uapi/linux/mount.h */

/*
 * Mount attributes.
 */
pub static MOUNT_ATTR_RDONLY: c_ulong = 0x00000001; /* Mount read-only */
pub static MOUNT_ATTR_NOSUID: c_ulong = 0x00000002; /* Ignore suid and sgid bits */
pub static MOUNT_ATTR_NODEV: c_ulong = 0x00000004; /* Disallow access to device special files */
pub static MOUNT_ATTR_NOEXEC: c_ulong = 0x00000008; /* Disallow program execution */
pub static MOUNT_ATTR__ATIME: c_ulong = 0x00000070; /* Setting on how atime should be updated */
pub static MOUNT_ATTR_RELATIME: c_ulong = 0x00000000; /* - Update atime relative to mtime/ctime. */
pub static MOUNT_ATTR_NOATIME: c_ulong = 0x00000010; /* - Do not update access times. */
pub static MOUNT_ATTR_STRICTATIME: c_ulong = 0x00000020; /* - Always perform atime updates */
pub static MOUNT_ATTR_NODIRATIME: c_ulong = 0x00000080; /* Do not update directory access times */
pub static MOUNT_ATTR_IDMAP: c_ulong = 0x00100000; /* Idmap mount to @userns_fd in struct mount_attr. */

/*
* mount_setattr()
*/
#[repr(C)]
pub struct mount_attr {
    pub attr_set: cty::uint64_t,
    pub attr_clr: cty::uint64_t,
    pub propagation: cty::uint64_t,
    pub userns_fd: cty::uint64_t,
}

/* List of all mount_attr versions. */
pub static MOUNT_ATTR_SIZE_VER0: c_ulong = 32; /* sizeof first published struct */

/* include/uapi/linux/fnctl.h */
pub static AT_RECURSIVE: c_ulong = 0x8000; /* Apply to the entire subtree */
