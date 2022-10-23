//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alexander Fomin <xander.fomin@ya.ru>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* Last synced with: sync (GNU coreutils) 8.13 */

extern crate libc;

use clap::{crate_version, Arg, ArgAction, Command};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::errno::Errno;
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::fcntl::{open, OFlag};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::sys::stat::Mode;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;

static ABOUT: &str = "Synchronize cached writes to persistent storage";
const USAGE: &str = "{} [OPTION]... FILE...";
pub mod options {
    pub static FILE_SYSTEM: &str = "file-system";
    pub static DATA: &str = "data";
}

static ARG_FILES: &str = "files";

#[cfg(unix)]
mod platform {
    use super::libc;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    use std::fs::File;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    use std::os::unix::io::AsRawFd;

    pub unsafe fn do_sync() -> isize {
        // see https://github.com/rust-lang/libc/pull/2161
        #[cfg(target_os = "android")]
        libc::syscall(libc::SYS_sync);
        #[cfg(not(target_os = "android"))]
        libc::sync();
        0
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub unsafe fn do_syncfs(files: Vec<String>) -> isize {
        for path in files {
            let f = File::open(&path).unwrap();
            let fd = f.as_raw_fd();
            libc::syscall(libc::SYS_syncfs, fd);
        }
        0
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub unsafe fn do_fdatasync(files: Vec<String>) -> isize {
        for path in files {
            let f = File::open(&path).unwrap();
            let fd = f.as_raw_fd();
            libc::syscall(libc::SYS_fdatasync, fd);
        }
        0
    }
}

#[cfg(windows)]
mod platform {
    use std::fs::OpenOptions;
    use std::os::windows::prelude::*;
    use std::path::Path;
    use uucore::crash;
    use uucore::wide::{FromWide, ToWide};
    use windows_sys::Win32::Foundation::{
        GetLastError, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE, MAX_PATH,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, FlushFileBuffers, GetDriveTypeW,
    };
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

    unsafe fn flush_volume(name: &str) {
        let name_wide = name.to_wide_null();
        if GetDriveTypeW(name_wide.as_ptr()) == DRIVE_FIXED {
            let sliced_name = &name[..name.len() - 1]; // eliminate trailing backslash
            match OpenOptions::new().write(true).open(sliced_name) {
                Ok(file) => {
                    if FlushFileBuffers(file.as_raw_handle() as HANDLE) == 0 {
                        crash!(GetLastError() as i32, "failed to flush file buffer");
                    }
                }
                Err(e) => crash!(
                    e.raw_os_error().unwrap_or(1),
                    "failed to create volume handle"
                ),
            }
        }
    }

    unsafe fn find_first_volume() -> (String, HANDLE) {
        let mut name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let handle = FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32);
        if handle == INVALID_HANDLE_VALUE {
            crash!(GetLastError() as i32, "failed to find first volume");
        }
        (String::from_wide_null(&name), handle)
    }

    unsafe fn find_all_volumes() -> Vec<String> {
        let (first_volume, next_volume_handle) = find_first_volume();
        let mut volumes = vec![first_volume];
        loop {
            let mut name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
            if FindNextVolumeW(next_volume_handle, name.as_mut_ptr(), name.len() as u32) == 0 {
                match GetLastError() {
                    ERROR_NO_MORE_FILES => {
                        FindVolumeClose(next_volume_handle);
                        return volumes;
                    }
                    err => crash!(err as i32, "failed to find next volume"),
                }
            } else {
                volumes.push(String::from_wide_null(&name));
            }
        }
    }

    pub unsafe fn do_sync() -> isize {
        let volumes = find_all_volumes();
        for vol in &volumes {
            flush_volume(vol);
        }
        0
    }

    pub unsafe fn do_syncfs(files: Vec<String>) -> isize {
        for path in files {
            flush_volume(
                Path::new(&path)
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_str()
                    .unwrap(),
            );
        }
        0
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files: Vec<String> = matches
        .get_many::<String>(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if matches.get_flag(options::DATA) && files.is_empty() {
        return Err(USimpleError::new(1, "--data needs at least one argument"));
    }

    for f in &files {
        // Use the Nix open to be able to set the NONBLOCK flags for fifo files
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            match open(Path::new(&f), OFlag::O_NONBLOCK, Mode::empty()) {
                Ok(_) => {}
                Err(e) => {
                    if e == Errno::ENOENT {
                        return Err(USimpleError::new(
                            1,
                            format!("cannot stat {}: No such file or directory", f.quote()),
                        ));
                    }
                    if e == Errno::EACCES {
                        if Path::new(&f).is_dir() {
                            return Err(USimpleError::new(
                                1,
                                format!("error opening {}: Permission denied", f.quote()),
                            ));
                        } else {
                            // ignore the issue
                            // ./target/debug/coreutils sync --data file
                        }
                    }
                }
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            if !Path::new(&f).exists() {
                return Err(USimpleError::new(
                    1,
                    format!("cannot stat {}: No such file or directory", f.quote()),
                ));
            }
        }
    }

    #[allow(clippy::if_same_then_else)]
    if matches.get_flag(options::FILE_SYSTEM) {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
        syncfs(files);
    } else if matches.get_flag(options::DATA) {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        fdatasync(files);
    } else {
        sync();
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .conflicts_with(options::DATA)
                .help("sync the file systems that contain the files (Linux and Windows only)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DATA)
                .short('d')
                .long(options::DATA)
                .conflicts_with(options::FILE_SYSTEM)
                .help("sync only file data, no unneeded metadata (Linux only)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

fn sync() -> isize {
    unsafe { platform::do_sync() }
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
fn syncfs(files: Vec<String>) -> isize {
    unsafe { platform::do_syncfs(files) }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn fdatasync(files: Vec<String>) -> isize {
    unsafe { platform::do_fdatasync(files) }
}
