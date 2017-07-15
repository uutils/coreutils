#![crate_name = "uu_sync"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Fomin <xander.fomin@ya.ru>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

 /* Last synced with: sync (GNU coreutils) 8.13 */

extern crate getopts;
extern crate libc;

#[cfg(windows)]
#[macro_use]
extern crate uucore;

#[cfg(not(windows))]
extern crate uucore;

static NAME: &'static str = "sync";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(unix)]
mod platform {
    use super::libc;

    extern {
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
    extern crate kernel32;
    use std::{mem};
    use std::fs::OpenOptions;
    use std::io::{Write};
    use std::os::windows::prelude::*;
    use uucore::wide::{FromWide, ToWide};

    unsafe fn flush_volume(name: &str) {
        let name_wide = name.to_wide_null();
        if kernel32::GetDriveTypeW(name_wide.as_ptr()) == winapi::DRIVE_FIXED {
            let sliced_name = &name[..name.len() - 1]; // eliminate trailing backslash
            match OpenOptions::new().write(true).open(sliced_name) {
                Ok(file) => if kernel32::FlushFileBuffers(file.as_raw_handle()) == 0 {
                    crash!(kernel32::GetLastError() as i32, "failed to flush file buffer");
                },
                Err(e) => crash!(e.raw_os_error().unwrap_or(1), "failed to create volume handle")
            }
        }
    }

    unsafe fn find_first_volume() -> (String, winapi::HANDLE) {
        let mut name: [winapi::WCHAR; winapi::MAX_PATH] = mem::uninitialized();
        let handle = kernel32::FindFirstVolumeW(name.as_mut_ptr(), name.len() as winapi::DWORD);
        if handle == winapi::INVALID_HANDLE_VALUE {
            crash!(kernel32::GetLastError() as i32, "failed to find first volume");
        }
        (String::from_wide_null(&name), handle)
    }

    unsafe fn find_all_volumes() -> Vec<String> {
        let (first_volume, next_volume_handle) = find_first_volume();
        let mut volumes = vec![first_volume];
        loop {
            let mut name: [winapi::WCHAR; winapi::MAX_PATH] = mem::uninitialized();
            if kernel32::FindNextVolumeW(
                next_volume_handle, name.as_mut_ptr(), name.len() as winapi::DWORD
            ) == 0 {
                match kernel32::GetLastError() {
                    winapi::ERROR_NO_MORE_FILES => {
                        kernel32::FindVolumeClose(next_volume_handle);
                        return volumes
                    },
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

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        _ => { help(&opts); return 1 }
    };

    if matches.opt_present("h") {
        help(&opts);
        return 0
    }

    if matches.opt_present("V") {
        version();
        return 0
    }

    sync();
    0
}

fn version() {
    println!("{} (uutils) {}", NAME, VERSION);
    println!("The MIT License");
    println!("");
    println!("Author -- Alexander Fomin.");
}

fn help(opts: &getopts::Options) {
    let msg = format!("{0} {1}

Usage:
  {0} [OPTION]

Force changed blocks to disk, update the super block.", NAME, VERSION);

    print!("{}", opts.usage(&msg));
}

fn sync() -> isize {
    unsafe {
        platform::do_sync()
    }
}
