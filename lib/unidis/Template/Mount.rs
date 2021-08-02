use std::ptr;

use libc::*;

/* Mount is a simple structure adapted for the mount() syscall. */
pub struct Mount<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub fstype: &'a str,
    pub mountflags: u64,
    pub data: *const usize,
}

/* MOUNTPOINTS is the mountings to perform for a functioning system
within the user namespace.  Paths are relative to the new "/". */
pub const MOUNTPOINTS: [Mount; 1] = [
  // Rebind /dev in a user namespace if /dev is not readable (unprivileged)
  Mount {
    source: "/dev", target: "dev/", fstype: "",
    mountflags: MS_REC|MS_BIND|MS_NOSUID, data: ptr::null(),
  },
];
