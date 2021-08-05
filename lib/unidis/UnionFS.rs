mod FuseUnionFS;

use std::{io, path::Path};
use strum_macros::{EnumString, EnumVariantNames};

#[repr(C)]
#[derive(Copy, Clone, Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedUnionFS
{
    // Kernel
    // Userspace (FUSE)
    FuseUnionFS,
}

/* Support Flags. */
pub static PIVOT_ROOT: u64 = 0x01;

pub trait UnionFS
{
    /* support() defines the supported features of the filesystem.
    Otherwise said, which operations it supports. */
    fn support(&self) -> u64;
    /* mountpoint() retrieves the Path at which the unioned filesystem
    resides. */
    fn mountpoint(&self) -> &Path;
    /* union() combines two directories specified by LEFT and RIGHT.
    LEFT will always be read-only while RIGHT is always read-write. */
    fn union(&self, left: &str, right: &str) -> Result<(), io::Error>;
}

/* unionfs_supports() determines if FEATURE is supported given the
set of FLAGS. */
pub fn unionfs_supports(flags: u64, feature: u64) -> bool
{
    flags & feature != 0
}

/* get_union_filesystem() gets the implemented trait based of
SupportedUnionFS. */
pub fn get_union_filesystem(union_fs: SupportedUnionFS) -> Box<dyn UnionFS>
{
    match union_fs {
        SupportedUnionFS::FuseUnionFS => Box::new(FuseUnionFS::FuseUnionFS::default()),
    }
}
