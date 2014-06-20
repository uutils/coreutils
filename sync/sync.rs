#![crate_id(name="uusync", vers="1.0.0", author="Alexander Fomin")]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Fomin <xander.fomin@ya.ru>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

 /* Last synced with: sync (GNU coreutils) 8.13 */

 #![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::os;
use getopts::{optflag, getopts, usage};

#[path = "../common/util.rs"] mod util;

#[cfg(unix)]
mod platform {
    use super::libc;

    extern {
        fn sync() -> libc::c_void;
    }

    pub unsafe fn do_sync() -> int {
        sync();
        0
    }
}

#[cfg(windows)]
mod platform {
    pub use super::libc;
    use std::{mem, str};

    extern "system" {
        fn CreateFileA(lpFileName: *libc::c_char,
                      dwDesiredAccess: libc::uint32_t,
                      dwShareMode: libc::uint32_t,
                      lpSecurityAttributes: *libc::c_void, // *LPSECURITY_ATTRIBUTES
                      dwCreationDisposition: libc::uint32_t,
                      dwFlagsAndAttributes: libc::uint32_t,
                      hTemplateFile: *libc::c_void) -> *libc::c_void;
        fn GetDriveTypeA(lpRootPathName: *libc::c_char) -> libc::c_uint;
        fn GetLastError() -> libc::uint32_t;
        fn FindFirstVolumeA(lpszVolumeName: *libc::c_char,
                            cchBufferLength: libc::uint32_t) -> *libc::c_void;
        fn FindNextVolumeA(hFindVolume: *libc::c_void,
                           lpszVolumeName: *libc::c_char,
                           cchBufferLength: libc::uint32_t) -> libc::c_int;
        fn FindVolumeClose(hFindVolume: *libc::c_void) -> libc::c_int;
        fn FlushFileBuffers(hFile: *libc::c_void) -> libc::c_int;
    }

    #[allow(unused_unsafe)]
    unsafe fn flush_volume(name: &str) {
        name.to_c_str().with_ref(|name_buffer| {
            if 0x00000003 == GetDriveTypeA(name_buffer) { // DRIVE_FIXED
                let sliced_name = name.slice_to(name.len() - 1); // eliminate trailing backslash
                sliced_name.to_c_str().with_ref(|sliced_name_buffer| {
                    match CreateFileA(sliced_name_buffer,
                                      0xC0000000, // GENERIC_WRITE
                                      0x00000003, // FILE_SHARE_WRITE,
                                      0 as *libc::c_void,
                                      0x00000003, // OPEN_EXISTING
                                      0,
                                      0 as *libc::c_void) {
                        _x if _x == -1 as *libc::c_void => { // INVALID_HANDLE_VALUE
                            crash!(GetLastError(), "failed to create volume handle");
                        }
                        handle @ _ => {
                            if FlushFileBuffers(handle) == 0 {
                                crash!(GetLastError(), "failed to flush file buffer");
                            }
                        }
                    }
                });
            }
        });
    }

    #[allow(unused_unsafe)]
    unsafe fn find_first_volume() -> (String, *libc::c_void) {
        let name: [libc::c_char, ..260] = mem::uninitialized(); // MAX_PATH
        match FindFirstVolumeA(name.as_ptr(), name.len() as libc::uint32_t) {
            _x if _x == -1 as *libc::c_void => { // INVALID_HANDLE_VALUE
                crash!(GetLastError(), "failed to find first volume");
            }
            handle @ _ => {
                (str::raw::from_c_str(name.as_ptr()), handle)
            }
        }
    }

    #[allow(unused_unsafe)]
    unsafe fn find_all_volumes() -> Vec<String> {
        match find_first_volume() {
            (first_volume, next_volume_handle) => {
                let mut volumes = Vec::from_elem(1, first_volume);
                loop {
                    let name: [libc::c_char, ..260] = mem::uninitialized(); // MAX_PATH
                    match FindNextVolumeA(next_volume_handle, name.as_ptr(), name.len() as libc::uint32_t) {
                        0 => {
                            match GetLastError() {
                                0x12 => { // ERROR_NO_MORE_FILES
                                    FindVolumeClose(next_volume_handle); // ignore FindVolumeClose() failures
                                    break;
                                }
                                err @ _ => {
                                    crash!(err, "failed to find next volume");
                                }
                            }
                        }
                        _ => {
                            volumes.push(str::raw::from_c_str(name.as_ptr()));
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
            flush_volume(vol.as_slice());
        }
        0
    }
}

static NAME: &'static str = "sync";

#[allow(dead_code)]
fn main () { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0);

    let options = [
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts(args.tail(), options) {
        Ok(m) => { m }
        _ => { help(program.as_slice(), options); return 0 }
    };

    if matches.opt_present("h") {
        help(program.as_slice(), options);
        return 0
    }

    if matches.opt_present("V") {
        version();
        return 0
    }

    uusync();
    0
}

fn version() {
    println!("uusync (uutils) 1.0.0");
    println!("The MIT License");
    println!("");
    println!("Author -- Alexander Fomin.");
}

fn help(program: &str, options: &[getopts::OptGroup]) {
    println!("Usage: {:s} [OPTION]", program);
    print!("{:s}", usage("Force changed blocks to disk, update the super block.", options));
}

fn uusync() -> int {
    unsafe {
        platform::do_sync()
    }
}