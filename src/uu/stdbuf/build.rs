// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) dylib libstdbuf deps liblibstdbuf

use std::env;
use std::env::current_exe;
use std::fs;
use std::path::Path;

#[cfg(all(unix, not(target_vendor = "apple")))]
mod platform {
    pub const DYLIB_EXT: &str = ".so";
}

#[cfg(target_vendor = "apple")]
mod platform {
    pub const DYLIB_EXT: &str = ".dylib";
}

#[cfg(unix)]
fn find_and_copy_libstdbuf() {
    let current_exe = current_exe().unwrap();

    let out_dir_string = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_string);

    let deps_dir = current_exe.ancestors().nth(3).unwrap().join("deps");
    dbg!(&deps_dir);

    let libstdbuf = deps_dir
        .read_dir()
        .unwrap()
        .flatten()
        .find(|entry| {
            let n = entry.file_name();
            let name = n.to_string_lossy();

            name.starts_with("liblibstdbuf") && name.ends_with(platform::DYLIB_EXT)
        })
        .expect("unable to find libstdbuf");

    fs::copy(libstdbuf.path(), out_dir.join("libstdbuf.so")).unwrap();
}

fn main() {
    #[cfg(unix)]
    find_and_copy_libstdbuf()
}
