// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#[cfg(unix)]
pub use self::unix::instantiate_current_writer;
#[cfg(unix)]
pub use self::unix::paths_refer_to_same_file;

#[cfg(windows)]
pub use self::windows::instantiate_current_writer;
#[cfg(windows)]
pub use self::windows::paths_refer_to_same_file;

// WASI: no process spawning (filter) or device/inode comparison.
#[cfg(target_os = "wasi")]
pub fn paths_refer_to_same_file(_p1: &std::ffi::OsStr, _p2: &std::ffi::OsStr) -> bool {
    false
}

#[cfg(target_os = "wasi")]
pub fn instantiate_current_writer(
    _filter: Option<&str>,
    filename: &str,
    is_new: bool,
) -> std::io::Result<std::io::BufWriter<Box<dyn std::io::Write>>> {
    let file = if is_new {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(std::path::Path::new(filename))?
    } else {
        std::fs::OpenOptions::new()
            .append(true)
            .open(std::path::Path::new(filename))?
    };
    Ok(std::io::BufWriter::new(
        Box::new(file) as Box<dyn std::io::Write>
    ))
}

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
