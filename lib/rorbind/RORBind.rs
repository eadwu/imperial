mod Lib;
use Lib::Mount::{self, mount_attr};

use libc::*;
use std::{ffi::CString, mem, os::unix::ffi::OsStrExt, path::PathBuf, ptr};

/* Root error class. */
#[derive(Debug)]
pub enum MountError {
    DecodingError,
    EPERM,
    ENODEV,
    ENOTBLK,
    EBUSY,
    EINVAL,
    EACCES,
    EMFILE,
    Error,
}

/* Fetches ERRNO in the case of a failed syscall. */
fn errno() -> i32 {
    unsafe { (*libc::__errno_location()) as i32 }
}

/* Converts ERRNO to a more readable error. */
fn as_mount_error(errno: i32) -> MountError {
    // Simple, whatever was listed at glibc is just propagated, nothing special.
    // https://www.gnu.org/software/libc/manual/html_mono/libc.html#Mount_002dUnmount_002dRemount
    match errno {
        EPERM => MountError::EPERM,
        ENODEV => MountError::ENODEV,
        ENOTBLK => MountError::ENOTBLK,
        EBUSY => MountError::EBUSY,
        EINVAL => MountError::EINVAL,
        EACCES => MountError::EACCES,
        EMFILE => MountError::EMFILE,
        _ => MountError::Error
    }
}

/* Helper function to convert a `PathBuf` to a C-compatiable string. */
fn as_cstr(path: PathBuf) -> Result<CString, MountError> {
    let path_as_os_str = path.into_os_string();
    CString::new(path_as_os_str.as_bytes()).map_err(|_| MountError::DecodingError)
}

/* Recursively bind mount SOURCE to MOUNTPOINT, while delivering the
expected behavior in the read-only case, propagating to submounts. */
#[cfg(unix)]
pub fn mount(source: PathBuf, mountpoint: PathBuf) -> Result<i32, MountError> {
    // Luckily, `as_bytes` is valid on UNIX
    // https://doc.rust-lang.org/std/os/unix/ffi/trait.OsStrExt.html
    let src = as_cstr(source)?;
    let target = as_cstr(mountpoint)?;
    // There will be a fight if this fails to unwrap ...
    let fstype = CString::new("").unwrap();

    let MOUNT_FLAGS = MS_REC | MS_BIND;

    // Recursive bind mount filesystem
    unsafe {
        let success = libc::mount(
            src.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            MOUNT_FLAGS,
            ptr::null(),
        );

        if success != 0 {
            return Err(as_mount_error(errno()));
        }
    }

    // Remount recursive bind mount as read-only
    unsafe {
        let success = libc::mount(
            src.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            MOUNT_FLAGS | MS_REMOUNT | MS_RDONLY,
            ptr::null(),
        );

        if success != 0 {
            return Err(as_mount_error(errno()));
        }
    }

    // Recursively set bind mount (which means including submounts) as read-only
    let mount_attributes: *const mount_attr = &mount_attr {
        attr_set: Mount::MOUNT_ATTR_RDONLY,
        attr_clr: 0,
        propagation: 0,
        userns_fd: 0,
    };

    unsafe {
        let success = libc::syscall(
            libc::SYS_mount_setattr,
            -1,
            target.as_ptr(),
            Mount::AT_RECURSIVE,
            mount_attributes,
            mem::size_of::<mount_attr>(),
        );

        if success != 0 {
            return Err(as_mount_error(errno()));
        }
    }

    Ok(0)
}
