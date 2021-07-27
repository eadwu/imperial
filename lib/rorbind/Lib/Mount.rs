#![allow(dead_code)]

use libc::*;

/* include/uapi/linux/mount.h */

/*
 * Mount attributes.
 */
pub static MOUNT_ATTR_RDONLY: __u64 = 0x00000001; /* Mount read-only */
pub static MOUNT_ATTR_NOSUID: __u64 = 0x00000002; /* Ignore suid and sgid bits */
pub static MOUNT_ATTR_NODEV: __u64 = 0x00000004; /* Disallow access to device special files */
pub static MOUNT_ATTR_NOEXEC: __u64 = 0x00000008; /* Disallow program execution */
pub static MOUNT_ATTR__ATIME: __u64 = 0x00000070; /* Setting on how atime should be updated */
pub static MOUNT_ATTR_RELATIME: __u64 = 0x00000000; /* - Update atime relative to mtime/ctime. */
pub static MOUNT_ATTR_NOATIME: __u64 = 0x00000010; /* - Do not update access times. */
pub static MOUNT_ATTR_STRICTATIME: __u64 = 0x00000020; /* - Always perform atime updates */
pub static MOUNT_ATTR_NODIRATIME: __u64 = 0x00000080; /* Do not update directory access times */
pub static MOUNT_ATTR_IDMAP: __u64 = 0x00100000; /* Idmap mount to @userns_fd in struct mount_attr. */

/*
* mount_setattr()
*/
#[repr(C)]
pub struct mount_attr {
    pub attr_set: __u64,
    pub attr_clr: __u64,
    pub propagation: __u64,
    pub userns_fd: __u64,
}

/* List of all mount_attr versions. */
pub static MOUNT_ATTR_SIZE_VER0: __u64 = 32; /* sizeof first published struct */

/* include/uapi/linux/fnctl.h */
pub static AT_RECURSIVE: __u64 = 0x8000; /* Apply to the entire subtree */
