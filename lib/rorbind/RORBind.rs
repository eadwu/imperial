mod Lib;
use Lib::Mount::{self, mount_attr};

use libc::*;
use std::{ffi::CString, mem, ptr};

/* Fetches ERRNO in the case of a failed syscall. */
fn errno() -> libc::c_int {
    unsafe { *libc::__errno_location() }
}

/* Recursively bind mount SRC to TARGET, while delivering the
expected behavior in the read-only case, propagating to submounts. */
#[cfg(unix)] #[no_mangle]
pub extern "C" fn rormount(src: *const libc::c_char, target: *const libc::c_char) -> libc::c_int {
    // There will be a fight if this fails to unwrap ...
    let fstype = CString::new("").unwrap();

    // Recursive bind mount filesystem
    // Propagation type is set to MS_SLAVE, as a read-only mount will never
    // have changes anyway
    unsafe {
        let success = libc::mount(
            src,
            target,
            fstype.as_ptr(),
            MS_REC | MS_BIND | MS_SLAVE,
            ptr::null(),
        );

        if success != 0 {
            return errno();
        }
    }

    // Remount bind mount as read-only
    // This probably isn't necessary due to the next syscall but for old times'
    // consistency
    unsafe {
        let success = libc::mount(
            src,
            target,
            fstype.as_ptr(),
            MS_REMOUNT | MS_BIND | MS_RDONLY,
            ptr::null(),
        );

        if success != 0 {
            return errno();
        }
    }

    // Recursively set all mounts' attributes, including submounts
    // Same rationale as the above, read-only (as this tool would suggest), and
    // one way propagation from the "master"
    let mount_attributes: *const mount_attr = &mount_attr {
        attr_set: Mount::MOUNT_ATTR_RDONLY,
        attr_clr: 0,
        propagation: MS_SLAVE,
        userns_fd: 0,
    };

    unsafe {
        let success = libc::syscall(
            libc::SYS_mount_setattr,
            -1,
            target,
            Mount::AT_RECURSIVE,
            mount_attributes,
            mem::size_of::<mount_attr>(),
        );

        if success != 0 {
            return errno();
        }
    }

    0 as c_int
}
