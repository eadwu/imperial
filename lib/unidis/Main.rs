use unidis::{self, UnionFS};

use std::{ffi::CString, iter, marker, os::unix::ffi::OsStrExt, path, ptr};
use structopt::{clap::AppSettings, StructOpt};
use strum::VariantNames;

#[derive(StructOpt, Debug)]
#[structopt(
    setting = AppSettings::ArgRequiredElseHelp,
    setting = AppSettings::TrailingVarArg,
    setting = AppSettings::UnifiedHelpMessage,
)]
struct Arguments
{
    /// Type of union fileystem to use
    #[structopt(
        short = "t", long = "unionfs", default_value = "fuseunionfs",
        possible_values = UnionFS::SupportedUnionFS::VARIANTS,
        case_insensitive = true,
    )]
    unionfs: UnionFS::SupportedUnionFS,
    /// Whether to remount /tmp
    #[structopt(long = "remount-tmp")]
    remount_tmp: bool,
    /// Override the RIGHT (read-write) side of the union
    #[structopt(long = "right", parse(from_os_str), default_value = "/")]
    right: path::PathBuf,
    /// Support root directory to merge
    #[structopt(parse(from_os_str))]
    left: path::PathBuf,
    /// Command to run
    #[structopt(use_delimiter(false))]
    argv: Vec<String>,
}

/* Wrapper routine to library. */
pub fn main()
{
    let args = Arguments::from_args();
    println!("{:?}", &args);

    // right -> char *
    let right_osstr = args.right.as_os_str();
    let right = CString::new(right_osstr.as_bytes()).unwrap();

    // left -> char *
    let left_osstr = args.left.as_os_str();
    let left = CString::new(left_osstr.as_bytes()).unwrap();

    // argv -> char ** + NUL-terminated
    let argv = args
        .argv
        .iter()
        .cloned()
        .map(|arg| CString::new(arg).unwrap())
        .collect::<Vec<_>>();
    let argv = argv
        .iter()
        .map(|cstr| cstr.as_ptr())
        .chain(iter::once(ptr::null()))
        .collect::<Vec<_>>();

    // flags -> bit flags
    let mut flags: u64 = 0;
    if args.remount_tmp {
        flags = flags | unidis::REMOUNT_TMP;
    }

    let flags = flags;
    let unidis_attrs = &unidis::unidis_attrs {
        _phantom: marker::PhantomData,
        left: left.as_ptr(),
        right: right.as_ptr(),
        argc: argv.len(),
        argv: argv.as_ptr(),
        flags,
        unionfs: args.unionfs,
    };

    unidis::unidis(unidis_attrs);
}
