#![allow(non_snake_case)]
#![feature(type_alias_impl_trait)]

mod Libc;
use Libc::{Clone::clone_args, *};

mod Template;
use Template::{IDMap, Mount};

pub mod UnionFS;
use UnionFS::*;

use libc::*;
use std::{env, ffi::CStr, fs::write, marker, path::Path, ptr};

// REMOUNT_TMP means exactly what it says, whether to remount /tmp
// More specifically, this will occur after the chroot but before
// the user remapping to prevent issues such as setting up the new
// root in `/tmp` of the parent namespace
pub static REMOUNT_TMP: __u64 = 0x01;

#[repr(C)]
#[derive(Debug)]
pub struct unidis_attrs<'a>
{
    pub _phantom: marker::PhantomData<&'a c_char>,
    // ALTROOT is the alternative root to merge into the current root
    // The intended use case is to provide a read/execute only overlay
    // to provide a valid FHS-like structure
    pub altroot: *const c_char,
    // ARGC is the number of command line arguments
    pub argc: uintptr_t,
    // ARGV is the command to run (replacing the process), in accordance
    // to `execvp`, ARGV has to NUL-terminated
    pub argv: *const *const c_char,
    // FLAGS describe some operations that can modify the behavior of
    // unidis
    pub flags: __u64,
    // UNIONFS describes the union filesystem to use
    pub unionfs: SupportedUnionFS,
}

/* pivot_root() switches to the new root. */
fn pivot_root(root: &Path) -> SyscallResult
{
    // Switch current working directory to the new root
    if env::set_current_dir(root).is_err() {
        return Err(EINVAL);
    }

    // Pivot to the new root location
    // It is important that `new_root` is not ".", as while this
    // would not adjust the view of the filesystem, it prevents
    // subsequent `unshares` from correctly viewing the correct
    // root directory in the mount namespace, see the next explanation
    // for more details
    Libc::pivot_root(".", "old_root")?;

    // Typically the `old_root` would be unmounted, for example:
    //   Libc::umount("old_root", MNT_DETACH)?;
    // However this would cause FUSE-based filesystems to fail and
    // the purpose of such an unmount is debateable, as the `old_root`
    // directory is exposed as a result of the union anyway

    // Ensure scope of process
    if env::set_current_dir("/").is_err() {
        return Err(EINVAL);
    }

    new_syscall_result(0, None)
}

/* user_mapping() sets up the user namespace with UID_MAP and GID_MAP. */
fn user_mapping(pid: &str, uid_map: &str, gid_map: &str) -> SyscallResult
{
    let pid_proc_dir = format!("/proc/{}", pid);
    let pid_setgroups = format!("{}/setgroups", pid_proc_dir);
    let pid_uid_map = format!("{}/uid_map", pid_proc_dir);
    let pid_gid_map = format!("{}/gid_map", pid_proc_dir);

    // Apparently some systems don't have /proc/[pid]/setgroups ...
    if Path::new(&pid_setgroups).exists() {
        // Since Linux 3.19 unprivileged writing of /proc/[pid]/gid_map
        // has been disabled unless /proc/[pid]/setgroups is written
        // first to permanently disable the ability to call setgroups
        // in that user namespace
        if let Err(error) = write(&pid_setgroups, "deny") {
            println!("Failed to write to {} got {:?}", &pid_setgroups, error);
            return Err(EINVAL);
        }
    }

    if let Err(error) = write(&pid_uid_map, uid_map) {
        println!("Failed to write to {}, got {:?}", &pid_uid_map, error);
        return Err(EINVAL);
    }

    if let Err(error) = write(&pid_gid_map, gid_map) {
        println!("Failed to write to {}, got {:?}", &pid_gid_map, error);
        return Err(EINVAL);
    }

    new_syscall_result(0, None)
}

/* setup_mounts() sets up the mounts in the namespace defined by MOUNTS. */
fn setup_mounts(mounts: &[Mount::Mount]) -> SyscallResult
{
    for mnt in mounts {
        Libc::mount(mnt.source, mnt.target, mnt.fstype, mnt.mountflags, mnt.data)?;
    }

    new_syscall_result(0, None)
}

/* init(UNIDIS_ATTRS) sets up the "container" given the configuration
outlined in UNIDIS_ATTRS. */
#[cfg(unix)]
fn init(unidis_attrs: *const unidis_attrs, revuidmap: &str, revgidmap: &str) -> SyscallResult
{
    // Setup mount namespace by fixing the propagation and remounting
    // /proc in case CLONE_NEWPID was given
    Libc::mount("none", "/", "", MS_REC | MS_PRIVATE, ptr::null())?;
    Libc::mount(
        "none",
        "/proc",
        "proc",
        MS_NOSUID | MS_NODEV | MS_NOEXEC,
        ptr::null(),
    )?;

    // Mount unioned filesystem
    let unionfs = get_union_filesystem(unsafe { (*unidis_attrs).unionfs });
    let altroot = unsafe { CStr::from_ptr((*unidis_attrs).altroot) };
    let res = (*unionfs).union(altroot.to_str().unwrap(), "/");
    if res.is_err() {
        println!("{}", res.err().unwrap());
        return Err(EINVAL);
    }

    // Setup mounts in new root
    let mnt = (*unionfs).mountpoint();
    println!("Setting up unioned mountpoint at {:?}", mnt);
    if env::set_current_dir(mnt).is_err() {
        return Err(EINVAL);
    }
    setup_mounts(&Mount::MOUNTPOINTS)?;

    // Change to "new" root directory
    println!("Attempting pivot_root to mountpoint");
    let support = (*unionfs).support();
    let can_pivot_root = unionfs_supports(support, UnionFS::PIVOT_ROOT);
    if can_pivot_root {
        pivot_root(mnt)?;
    }

    // Unnecessary, but seems like chdir/chroot after pivot_root
    // is standard to prevent escapes from the new root directory
    println!("Executing chroot");
    Libc::chroot(&Path::new("."))?;

    println!("Remounting TMPDIR [/tmp]");
    if unsafe { (*unidis_attrs).flags } & REMOUNT_TMP != 0 {
        // Remount tmp directory
        Libc::mount("none", "tmp/", "tmpfs", 0, ptr::null())?;
    }

    // Effectively reverse applied user mapping for "normality" which
    // requires a new user namespace, a requirement of this step is
    // the success of `pivot_root` for the root directory of the mount
    // namespace to match
    if can_pivot_root {
        Libc::unshare(CLONE_NEWUSER)?;
        user_mapping("self", revuidmap, revgidmap)?;
    }

    // Replace running process with EXECUTABLE[ ARGV]
    let executable = unsafe { *((*unidis_attrs).argv) as *const c_char };
    let argv = unsafe { (*unidis_attrs).argv };
    Libc::execvp(executable, argv)
}

/* isolate_namespace() is the unwrapped routine for the library, allowing for
a cleaner `Result` implementation. */
fn isolate_namespace() -> SyscallResult
{
    let gid_map = IDMap::newgidmap();
    let uid_map = IDMap::newuidmap();

    // Map current user to root before creating the other namespaces
    // This shouldn't cause a big disruption in the functionality, though
    // it increase the nested depth of user namespaces ...
    // This is a tradeoff for readability, since the other approach
    // would require the child needed to wait for the parent to write
    // to /proc/[pid]/{setgroups,uid_map,gid_map} for a proper mapping
    Libc::unshare(CLONE_NEWUSER)?;
    user_mapping("self", &uid_map, &gid_map)?;

    let flags = CLONE_NEWNS | CLONE_NEWPID;
    let clone_args = clone_args {
        flags: flags as __u64,                     // Unshared namespaces
        pidfd: 0,                                  // See CLONE_PIDFD
        child_tid: 0,                              // See CLONE_CHILD_SETTID
        parent_tid: 0,                             // See CLONE_PARENT_SETTID
        exit_signal: SIGCHLD as __u64,             // Default signal
        stack: ptr::null::<*const u64>() as __u64, // Use parent's stack
        stack_size: 0,                             // ^
        tls: 0,                                    // See CLONE_SETTLS
        set_tid: [1].as_ptr() as __u64,            // Root "init" process = PID 1
        set_tid_size: 1,                           // ^ Length 1 array
        cgroup: 0,                                 // See CLONE_INTO_CGROUP
    };

    // Spawn a process into a separate user namespace as desired
    Libc::clone(&clone_args)
}

#[no_mangle]
pub extern "C" fn unc(unidis_attrs: *const unidis_attrs) -> i64
{
    let revuidmap = IDMap::revuidmap();
    let revgidmap = IDMap::revgidmap();

    // The child takes over the main execution process now as it is in the
    match handle_syscall_result(isolate_namespace()) {
        Err(errno) => errno.into(),
        Ok(pid) => {
            let pid = pid.into() as i32;
            match pid {
                // Child process routine
                0 => match handle_syscall_result(init(unidis_attrs, &revuidmap, &revgidmap)) {
                    Err(errno) => errno.into(),
                    // execvp should've replaced the running process if it succeeded and
                    // returned the errno() if it did not.
                    Ok(_) => unreachable!(),
                },
                _ => match Libc::waitpid(pid, 0) {
                    Err(errno) => errno.into(),
                    Ok(_) => 0,
                },
            }
        }
    }
}
