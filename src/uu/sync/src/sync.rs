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

use clap::App;
static ABOUT: &str = "Synchronize cached writes to persistent storage";
static VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(unix)]
mod platform {
    use super::libc;

    extern "C" {
        fn sync() -> libc::c_void;
    }

    pub unsafe fn do_sync() -> isize {
        sync();
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
}

fn get_usage() -> String {
    format!("{0} [OPTION]... FILE...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let _matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .get_matches_from(args);

    sync();
    0
}

fn sync() -> isize {
    unsafe { platform::do_sync() }
}
