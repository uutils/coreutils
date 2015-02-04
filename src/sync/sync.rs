#![crate_name = "sync"]
#![feature(collections, libc, rustc_private)]

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

use getopts::{optflag, getopts, usage};

#[path = "../common/util.rs"] #[macro_use] mod util;

static NAME: &'static str = "sync";
static VERSION: &'static str = "1.0.0";

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
    pub use super::libc;
    use std::{mem, string};
    use std::ptr::null;

    extern "system" {
        fn CreateFileA(lpFileName: *const libc::c_char,
                      dwDesiredAccess: libc::uint32_t,
                      dwShareMode: libc::uint32_t,
                      lpSecurityAttributes: *const libc::c_void, // *LPSECURITY_ATTRIBUTES
                      dwCreationDisposition: libc::uint32_t,
                      dwFlagsAndAttributes: libc::uint32_t,
                      hTemplateFile: *const libc::c_void) -> *const libc::c_void;
        fn GetDriveTypeA(lpRootPathName: *const libc::c_char) -> libc::c_uint;
        fn GetLastError() -> libc::uint32_t;
        fn FindFirstVolumeA(lpszVolumeName: *mut libc::c_char,
                            cchBufferLength: libc::uint32_t) -> *const libc::c_void;
        fn FindNextVolumeA(hFindVolume: *const libc::c_void,
                           lpszVolumeName: *mut libc::c_char,
                           cchBufferLength: libc::uint32_t) -> libc::c_int;
        fn FindVolumeClose(hFindVolume: *const libc::c_void) -> libc::c_int;
        fn FlushFileBuffers(hFile: *const libc::c_void) -> libc::c_int;
    }

    #[allow(unused_unsafe)]
    unsafe fn flush_volume(name: &str) {
        let name_buffer = name.to_c_str().as_ptr();
        if 0x00000003 == GetDriveTypeA(name_buffer) { // DRIVE_FIXED
            let sliced_name = &name[..name.len() - 1]; // eliminate trailing backslash
            let sliced_name_buffer = sliced_name.to_c_str().as_ptr();
            match CreateFileA(sliced_name_buffer,
                              0xC0000000, // GENERIC_WRITE
                              0x00000003, // FILE_SHARE_WRITE,
                              null(),
                              0x00000003, // OPEN_EXISTING
                              0,
                              null()) {
                -1 => { // INVALID_HANDLE_VALUE
                    crash!(GetLastError(), "failed to create volume handle");
                }
                handle => {
                    if FlushFileBuffers(handle) == 0 {
                        crash!(GetLastError(), "failed to flush file buffer");
                    }
                }
            }
        }
    }

    #[allow(unused_unsafe)]
    unsafe fn find_first_volume() -> (String, *const libc::c_void) {
        let mut name: [libc::c_char; 260] = mem::uninitialized(); // MAX_PATH
        match FindFirstVolumeA(name.as_mut_ptr(),
                               name.len() as libc::uint32_t) {
            -1 => { // INVALID_HANDLE_VALUE
                crash!(GetLastError(), "failed to find first volume");
            }
            handle => {
                (string::raw::from_buf(name.as_ptr() as *const u8), handle)
            }
        }
    }

    #[allow(unused_unsafe)]
    unsafe fn find_all_volumes() -> Vec<String> {
        match find_first_volume() {
            (first_volume, next_volume_handle) => {
                let mut volumes = vec![first_volume];
                loop {
                    let mut name: [libc::c_char; 260] = mem::uninitialized(); // MAX_PATH
                    match FindNextVolumeA(next_volume_handle,
                                          name.as_mut_ptr(),
                                          name.len() as libc::uint32_t) {
                        0 => {
                            match GetLastError() {
                                0x12 => { // ERROR_NO_MORE_FILES
                                    FindVolumeClose(next_volume_handle); // ignore FindVolumeClose() failures
                                    break;
                                }
                                err => {
                                    crash!(err, "failed to find next volume");
                                }
                            }
                        }
                        _ => {
                            volumes.push(string::raw::from_buf(name.as_ptr() as *const u8));
                        }
                    }
                }
                volumes
            }
        }
    }

    pub unsafe fn do_sync() -> int {
        let volumes = find_all_volumes();
        for vol in volumes.iter() {
            flush_volume(&vol);
        }
        0
    }
}

pub fn uumain(args: Vec<String>) -> isize {
    let program = &args[0][];

    let options = [
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts(args.tail(), &options) {
        Ok(m) => { m }
        _ => { help(program, &options); return 1 }
    };

    if matches.opt_present("h") {
        help(program, &options);
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

fn help(program: &str, options: &[getopts::OptGroup]) {
    println!("Usage: {} [OPTION]", program);
    print!("{}", usage("Force changed blocks to disk, update the super block.", options));
}

fn sync() -> isize {
    unsafe {
        platform::do_sync()
    }
}
