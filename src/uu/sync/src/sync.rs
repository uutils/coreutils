//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alexander Fomin <xander.fomin@ya.ru>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* Last synced with: sync (GNU coreutils) 8.13 */

extern crate clap;
extern crate libc;

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::path::Path;

static EXIT_ERR: i32 = 1;

static ABOUT: &str = "Synchronize cached writes to persistent storage";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static OPT_FILE_SYSTEM: &str = "file-system";
static OPT_DATA: &str = "data";

static ARG_FILES: &str = "files";

#[cfg(unix)]
mod platform {
    use super::libc;
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    pub unsafe fn do_sync() -> isize {
        libc::sync();
        0
    }

    #[cfg(target_os = "linux")]
    pub unsafe fn do_syncfs(files: Vec<String>) -> isize {
        for path in files {
            let f = File::open(&path).unwrap();
            let fd = f.as_raw_fd();
            libc::syscall(libc::SYS_syncfs, fd);
        }
        0
    }

    #[cfg(target_os = "linux")]
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
    extern crate winapi;
    use self::winapi::shared::minwindef;
    use self::winapi::shared::winerror;
    use self::winapi::um::handleapi;
    use self::winapi::um::winbase;
    use self::winapi::um::winnt;
    use std::fs::OpenOptions;
    use std::mem;
    use std::os::windows::prelude::*;
    use std::path::Path;
    use uucore::wide::{FromWide, ToWide};

    unsafe fn flush_volume(name: &str) {
        let name_wide = name.to_wide_null();
        if winapi::um::fileapi::GetDriveTypeW(name_wide.as_ptr()) == winbase::DRIVE_FIXED {
            let sliced_name = &name[..name.len() - 1]; // eliminate trailing backslash
            match OpenOptions::new().write(true).open(sliced_name) {
                Ok(file) => {
                    if winapi::um::fileapi::FlushFileBuffers(file.as_raw_handle()) == 0 {
                        crash!(
                            winapi::um::errhandlingapi::GetLastError() as i32,
                            "failed to flush file buffer"
                        );
                    }
                }
                Err(e) => crash!(
                    e.raw_os_error().unwrap_or(1),
                    "failed to create volume handle"
                ),
            }
        }
    }

    unsafe fn find_first_volume() -> (String, winnt::HANDLE) {
        #[allow(deprecated)]
        let mut name: [winnt::WCHAR; minwindef::MAX_PATH] = mem::uninitialized();
        let handle = winapi::um::fileapi::FindFirstVolumeW(
            name.as_mut_ptr(),
            name.len() as minwindef::DWORD,
        );
        if handle == handleapi::INVALID_HANDLE_VALUE {
            crash!(
                winapi::um::errhandlingapi::GetLastError() as i32,
                "failed to find first volume"
            );
        }
        (String::from_wide_null(&name), handle)
    }

    unsafe fn find_all_volumes() -> Vec<String> {
        let (first_volume, next_volume_handle) = find_first_volume();
        let mut volumes = vec![first_volume];
        loop {
            #[allow(deprecated)]
            let mut name: [winnt::WCHAR; minwindef::MAX_PATH] = mem::uninitialized();
            if winapi::um::fileapi::FindNextVolumeW(
                next_volume_handle,
                name.as_mut_ptr(),
                name.len() as minwindef::DWORD,
            ) == 0
            {
                match winapi::um::errhandlingapi::GetLastError() {
                    winerror::ERROR_NO_MORE_FILES => {
                        winapi::um::fileapi::FindVolumeClose(next_volume_handle);
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

fn get_usage() -> String {
    format!("{0} [OPTION]... FILE...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_FILE_SYSTEM)
                .short("f")
                .long(OPT_FILE_SYSTEM)
                .conflicts_with(OPT_DATA)
                .help("sync the file systems that contain the files (Linux and Windows only)"),
        )
        .arg(
            Arg::with_name(OPT_DATA)
                .short("d")
                .long(OPT_DATA)
                .conflicts_with(OPT_FILE_SYSTEM)
                .help("sync only file data, no unneeded metadata (Linux only)"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    for f in &files {
        if !Path::new(&f).exists() {
            crash!(EXIT_ERR, "cannot stat '{}': No such file or directory", f);
        }
    }

    if matches.is_present(OPT_FILE_SYSTEM) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        syncfs(files);
    } else if matches.is_present(OPT_DATA) {
        #[cfg(target_os = "linux")]
        fdatasync(files);
    } else {
        sync();
    }
    0
}

fn sync() -> isize {
    unsafe { platform::do_sync() }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn syncfs(files: Vec<String>) -> isize {
    unsafe { platform::do_syncfs(files) }
}

#[cfg(target_os = "linux")]
fn fdatasync(files: Vec<String>) -> isize {
    unsafe { platform::do_fdatasync(files) }
}
