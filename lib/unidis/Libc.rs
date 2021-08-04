pub mod Clone;

use libc::*;
use std::{
    ffi::{CStr, CString},
    mem,
    os::unix::ffi::OsStrExt,
    path::Path,
};

/* errno() returns the ERRNO value, typically of a syscall result. */
fn errno() -> c_int
{
    unsafe { *libc::__errno_location() }
}

/* SyscallResult is just a wrapper to the result of an syscall, which
usually return error codes that are integers. */
pub type SyscallResult = Result<impl Into<i64>, c_int>;

/* handle_syscall_result() runs the routine for the result of an syscall
loosely wrapped for compatibility with "idiomatic" Rust. */
pub fn handle_syscall_result(result: SyscallResult) -> SyscallResult
{
    match &result {
        Ok(_) => result,
        Err(errno) => {
            let err = unsafe { CStr::from_ptr(libc::strerror(*errno)) };
            println!("{:?}", err);

            result
        }
    }
}

/* new_syscall_result() is a small wrapper to create a SyscallResult with
a custom result on success, instead of the default ERRNO. */
pub fn new_syscall_result<T>(result: T, state: Option<T>) -> SyscallResult
where
    T: Into<i64>,
{
    let result = result.into();
    match result {
        -1 => Err(errno()),
        _ => Ok(state.map(|x| Into::<i64>::into(x)).unwrap_or(result)),
    }
}

/* pivot_root is a wrapper against the syscall SYS_pivot_root. */
pub fn pivot_root(old_root: &str, new_root: &str) -> SyscallResult
{
    let old_root = CString::new(old_root).unwrap();
    let new_root = CString::new(new_root).unwrap();

    new_syscall_result::<i64>(
        unsafe { libc::syscall(SYS_pivot_root, old_root.as_ptr(), new_root.as_ptr()) },
        None,
    )
}

/* mount() is a wrapper against the syscall SYS_mount. */
pub fn mount(src: &str, target: &str, fstype: &str, flags: u64, data: *const usize)
    -> SyscallResult
{
    let src = CString::new(src).unwrap();
    let target = CString::new(target).unwrap();
    let fstype = CString::new(fstype).unwrap();

    new_syscall_result::<i32>(
        unsafe {
            libc::mount(
                src.as_ptr(),
                target.as_ptr(),
                fstype.as_ptr(),
                flags as __u64,
                data as *const c_void,
            )
        },
        None,
    )
}

/* umount() is a wrapper against the syscall SYS_umount2. */
pub fn umount(target: &str, flags: c_int) -> SyscallResult
{
    let target = CString::new(target).unwrap();

    new_syscall_result::<i64>(
        unsafe { libc::syscall(SYS_umount2, target.as_ptr(), flags) },
        None,
    )
}

/* chroot() is a wrapper against the syscall SYS_chroot. */
#[cfg(unix)]
pub fn chroot(name: &Path) -> SyscallResult
{
    let name_os = name.as_os_str();
    let name = CString::new(name_os.as_bytes()).unwrap();
    new_syscall_result::<i32>(unsafe { libc::chroot(name.as_ptr()) }, None)
}

/* waitpid() waits for a process to terminate and returns the
STATUS of the terminated process. */
pub fn waitpid(pid: pid_t, options: c_int) -> SyscallResult
{
    let mut status: c_int = 0;
    let res = unsafe { libc::waitpid(pid, &mut status, options) };
    let status = status;

    new_syscall_result::<i32>(res, Some(status))
}

/* clone() is a wrapper against the syscall SYS_clone3. */
pub fn clone(clone_args: &Clone::clone_args) -> SyscallResult
{
    // On success, `res` is 0 on the parent and the process's PID on the
    // child
    let res = unsafe { libc::syscall(SYS_clone3, clone_args, mem::size_of::<Clone::clone_args>()) };
    new_syscall_result::<i64>(res, None)
}

/* unshare() is a wrapper against the syscall SYS_unshare. */
pub fn unshare(flags: c_int) -> SyscallResult
{
    new_syscall_result::<i32>(unsafe { libc::unshare(flags) }, None)
}

/* execvp() is a wrapper against the syscall SYS_execvp. */
pub fn execvp(executable: *const c_char, argv: *const *const c_char) -> SyscallResult
{
    new_syscall_result::<i32>(unsafe { libc::execvp(executable, argv) }, None)
}
