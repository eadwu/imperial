use rorbind;

use libc::*;
use structopt::StructOpt;
use std::{ffi::CString, os::unix::ffi::OsStrExt, path::PathBuf};

#[derive(StructOpt)]
pub struct Arguments {
    /// Source directory
    #[structopt(parse(from_os_str))]
    pub source: PathBuf,
    /// Target directory
    #[structopt(parse(from_os_str))]
    pub target: PathBuf,
}

/* Enum which aggregates all errors. */
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

/* A simple check, nothing extensive. */
fn verify_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        panic!("{:?} does not exists", path);
    }

    match path.canonicalize() {
        Err(err) => panic!("{:?}", err),
        Ok(full_path) => return full_path,
    }
}

/* Converts ERRNO to a more readable error in Rust. */
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
    // Luckily, `as_bytes` is valid on UNIX
    // https://doc.rust-lang.org/std/os/unix/ffi/trait.OsStrExt.html
    let path_as_os_str = path.into_os_string();
    CString::new(path_as_os_str.as_bytes()).map_err(|_| MountError::DecodingError)
}

/* Wrapper routine to the library. */
fn mount(src: PathBuf, target: PathBuf) -> Result<i32, MountError> {
    let src_cstr = as_cstr(src)?.as_ptr();
    let target_cstr = as_cstr(target)?.as_ptr();

    let success = rorbind::rormount(src_cstr, target_cstr);
    if success != 0 {
        return Err(as_mount_error(success));
    }

    Ok(success as i32)
}

/* Binary execution routine. */
pub fn main() {
    let args = Arguments::from_args();

    println!(
        "Requested mount from {:?} to {:?}",
        args.source, args.target
    );

    let source = verify_path(args.source);
    let target = verify_path(args.target);

    println!("Executing mount from {:?} to {:?}", source, target);

    let result = mount(source, target);

    // If it failed, exit with a non-zero exit code.
    if result.is_err() {
        println!("Failed to rorbind mount , got {:?}", result.err().unwrap());

        std::process::exit(1);
    }

    // Literally unneeded, but it makes code look nicer on my screen.
    std::process::exit(0);
}
