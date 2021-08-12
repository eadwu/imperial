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

pub trait UnionFS
{
    /* mountpoint() retrieves the Path at which the unioned filesystem
    resides. */
    fn mountpoint(&self) -> &Path;
    /* union() combines two directories specified by LEFT and RIGHT.
    LEFT will always be read-only while RIGHT is always read-write. */
    fn union(&self, left: &str, right: &str) -> Result<(), io::Error>;
}

/* get_union_filesystem() gets the implemented trait based of
SupportedUnionFS. */
pub fn get_union_filesystem(union_fs: SupportedUnionFS) -> Box<dyn UnionFS>
{
    match union_fs {
        SupportedUnionFS::FuseUnionFS => Box::new(FuseUnionFS::FuseUnionFS::default()),
    }
}
