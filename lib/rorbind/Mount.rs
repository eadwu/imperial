use rorbind;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Arguments {
    /// Source directory
    #[structopt(parse(from_os_str))]
    pub source: PathBuf,
    /// Target directory
    #[structopt(parse(from_os_str))]
    pub target: PathBuf,
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

/* Simple wrapper execution routine to the library. */
pub fn main() {
    let args = Arguments::from_args();

    println!(
        "Requested mount from {:?} to {:?}",
        args.source, args.target
    );

    let source = verify_path(args.source);
    let target = verify_path(args.target);

    println!("Executing mount from {:?} to {:?}", source, target);

    let result = rorbind::mount(source, target);

    // If it failed, exit with a non-zero exit code.
    if result.is_err() {
        println!("Failed to rorbind mount , got {:?}", result.err().unwrap());

        std::process::exit(1);
    }

    // Literally unneeded, but it makes code look nicer on my screen.
    std::process::exit(0);
}
