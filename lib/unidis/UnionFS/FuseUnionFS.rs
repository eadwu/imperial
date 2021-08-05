use super::{UnionFS, PIVOT_ROOT};
use crate::Libc;

use rorbind::rormount;

use libc::*;
use std::{ffi::CString, fs, io, os::unix::ffi::OsStrExt, path::Path, process::Command, ptr};
use tempdir::TempDir;

pub struct FuseUnionFS
{
    chroot_root: TempDir,
    union_root: TempDir,
    flags: u64,
}

impl Default for FuseUnionFS
{
    fn default() -> Self
    {
        let chroot_root = TempDir::new("").unwrap();
        let union_root = TempDir::new("").unwrap();
        FuseUnionFS {
            chroot_root,
            union_root,
            flags: PIVOT_ROOT,
        }
    }
}

impl UnionFS for FuseUnionFS
{
    fn support(&self) -> u64
    {
        self.flags
    }

    fn mountpoint(&self) -> &Path
    {
        self.union_root.path()
    }

    #[cfg(unix)]
    fn union(&self, left: &str, right: &str) -> Result<(), io::Error>
    {
        // Create a dummy directory for mounting the old root in `pivot_root`
        let chroot_polyfill_dir = self.chroot_root.path().join("polyfill");
        fs::create_dir(&chroot_polyfill_dir)?;
        fs::create_dir(&chroot_polyfill_dir.join("old_root"))?;

        // Create temporary directories for mounting "aliases"
        let chroot_root_left = self.chroot_root.path().join("left");
        let chroot_root_right = self.chroot_root.path().join("right");
        fs::create_dir(&chroot_root_left)?;
        fs::create_dir(&chroot_root_right)?;

        // LEFT is read-only, nested read-only mounts through `rorbind`
        let src_cstr = CString::new(left).unwrap();
        let target_cstr = CString::new(chroot_root_left.as_os_str().as_bytes()).unwrap();
        if rormount(src_cstr.as_ptr(), target_cstr.as_ptr()) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to recursive read-only bind mount {:?} -> {:?}",
                    src_cstr, target_cstr
                ),
            ));
        }

        // RIGHT is read-write, so propagate everything
        if Libc::mount(
            right,
            chroot_root_right.to_str().unwrap(),
            "",
            MS_REC | MS_BIND,
            ptr::null(),
        )
        .is_err()
        {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to recursive bind mount {} -> {:?}",
                    right, chroot_root_right
                ),
            ));
        }

        // Union both directories at UNION_ROOT
        let cmd = Command::new("unionfs")
            .arg("-o")
            .arg("allow_other,use_ino")
            .arg("-o")
            .arg(format!(
                "cow,chroot={}",
                self.chroot_root.path().to_str().unwrap()
            ))
            .arg("/polyfill=RW:/right=RW:/left=RO")
            .arg(self.union_root.path().to_str().unwrap())
            .spawn();

        if cmd.is_err() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to union {:?} | {:?} at {:?}",
                    chroot_root_left,
                    chroot_root_right,
                    self.union_root.path()
                ),
            ));
        }

        cmd.unwrap().wait().expect("Command failed to execute");

        // Ideally, resources would be manually release via drop() and
        // <impl TempDir>::close, but this works fine as well, although
        // it relies on out-of-scope behavior
        Ok(())
    }
}
