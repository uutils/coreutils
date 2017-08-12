// Mainly taken from crate `tempdir`

extern crate rand;
use rand::{Rng, thread_rng};

use std::io::Result as IOResult;
use std::io::{Error, ErrorKind};
use std::path::Path;

// How many times should we (re)try finding an unused random name? It should be
// enough that an attacker will run out of luck before we run out of patience.
const NUM_RETRIES: u32 = 1 << 31;

#[cfg(any(unix, target_os = "redox"))]
fn create_dir<P: AsRef<Path>>(path: P) -> IOResult<()> {
    use std::fs::DirBuilder;
    use std::os::unix::fs::DirBuilderExt;

    DirBuilder::new().mode(0o700).create(path)
}

#[cfg(windows)]
fn create_dir<P: AsRef<Path>>(path: P) -> IOResult<()> {
    ::std::fs::create_dir(path)
}

pub fn new_in<P: AsRef<Path>>(tmpdir: P, prefix: &str, rand: usize, suffix: &str) -> IOResult<String> {

    let mut rng = thread_rng();
    for _ in 0..NUM_RETRIES {
        let rand_chars: String = rng.gen_ascii_chars().take(rand).collect();
        let leaf = format!("{}{}{}", prefix, rand_chars, suffix);
        let path = tmpdir.as_ref().join(&leaf);
        match create_dir(&path) {
            Ok(_) => return Ok(path.to_string_lossy().into_owned()),
            Err(ref e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }

    Err(Error::new(ErrorKind::AlreadyExists,
                   "too many temporary directories already exist"))
}
