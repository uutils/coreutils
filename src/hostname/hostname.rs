#![crate_name = "uu_hostname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;
#[cfg(windows)]
extern crate winapi;

#[macro_use]
extern crate uucore;

use std::collections::hash_set::HashSet;
use std::iter::repeat;
use std::io;
use std::str;
use std::net::ToSocketAddrs;
use getopts::Matches;

#[cfg(windows)]
use winapi::um::winsock2::{GetHostNameW, WSACleanup, WSAStartup};
#[cfg(windows)]
use winapi::um::sysinfoapi::{ComputerNamePhysicalDnsHostname, SetComputerNameExW};
#[cfg(windows)]
use winapi::shared::minwindef::MAKEWORD;
#[cfg(windows)]
use uucore::wide::*;

#[cfg(not(windows))]
use libc::gethostname;
#[cfg(not(windows))]
use libc::sethostname;

const SYNTAX: &str = "[OPTION]... [HOSTNAME]";
const SUMMARY: &str = "Print or set the system's host name.";
const LONG_HELP: &str = "";

pub fn uumain(args: Vec<String>) -> i32 {
    #[cfg(windows)]
    unsafe {
        let mut data = std::mem::uninitialized();
        if WSAStartup(MAKEWORD(2, 2), &mut data as *mut _) != 0 {
            eprintln!("Failed to start Winsock 2.2");
            return 1;
        }
    }
    let result = execute(args);
    #[cfg(windows)]
    unsafe {
        WSACleanup();
    }
    result
}

fn execute(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("d", "domain", "Display the name of the DNS domain if possible")
        .optflag("i", "ip-address", "Display the network address(es) of the host")
        // TODO: support --long
        .optflag("f", "fqdn", "Display the FQDN (Fully Qualified Domain Name) (default)")
        .optflag("s", "short", "Display the short hostname (the portion before the first dot) if \
                                possible")
        .parse(args);

    match matches.free.len() {
        0 => display_hostname(matches),
        1 => {
            if let Err(err) = xsethostname(matches.free.last().unwrap()) {
                show_error!("{}", err);
                1
            } else {
                0
            }
        }
        _ => {
            show_error!("{}", msg_wrong_number_of_arguments!(0, 1));
            1
        }
    }
}

fn display_hostname(matches: Matches) -> i32 {
    let hostname = return_if_err!(1, xgethostname());

    if matches.opt_present("i") {
        // XXX: to_socket_addrs needs hostname:port so append a dummy port and remove it later.
        // This was originally supposed to use std::net::lookup_host, but that seems to be
        // deprecated.  Perhaps we should use the dns-lookup crate?
        let hostname = hostname + ":1";
        match hostname.to_socket_addrs() {
            Ok(addresses) => {
                let mut hashset = HashSet::new();
                let mut output = String::new();
                for addr in addresses {
                    // XXX: not sure why this is necessary...
                    if !hashset.contains(&addr) {
                        let mut ip = format!("{}", addr);
                        if ip.ends_with(":1") {
                            let len = ip.len();
                            ip.truncate(len - 2);
                        }
                        output.push_str(&ip);
                        output.push_str(" ");
                        hashset.insert(addr);
                    }
                }
                let len = output.len();
                if len > 0 {
                    println!("{}", &output[0..len - 1]);
                }

                0
            }
            Err(f) => {
                show_error!("{}", f);

                1
            }
        }
    } else {
        if matches.opt_present("s") || matches.opt_present("d") {
            let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
            if let Some(ci) = it.next() {
                if matches.opt_present("s") {
                    println!("{}", &hostname[0..ci.0]);
                } else {
                    println!("{}", &hostname[ci.0 + 1..]);
                }
                return 0;
            }
        }

        println!("{}", hostname);

        0
    }
}

#[cfg(not(windows))]
fn xgethostname() -> io::Result<String> {
    use std::ffi::CStr;

    let namelen = 256;
    let mut name: Vec<u8> = repeat(0).take(namelen).collect();
    let err = unsafe {
        gethostname(
            name.as_mut_ptr() as *mut libc::c_char,
            namelen as libc::size_t,
        )
    };

    if err == 0 {
        let null_pos = name.iter().position(|byte| *byte == 0).unwrap_or(namelen);
        if null_pos == namelen {
            name.push(0);
        }

        Ok(CStr::from_bytes_with_nul(&name[..null_pos + 1])
            .unwrap()
            .to_string_lossy()
            .into_owned())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn xgethostname() -> io::Result<String> {
    let namelen = 256;
    let mut name: Vec<u16> = repeat(0).take(namelen).collect();
    let err = unsafe { GetHostNameW(name.as_mut_ptr(), namelen as libc::c_int) };

    if err == 0 {
        Ok(String::from_wide_null(&name))
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn xsethostname(name: &str) -> io::Result<()> {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe { sethostname(vec_name.as_ptr(), vec_name.len() as _) };

    if err != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn xsethostname(name: &str) -> io::Result<()> {
    use std::ffi::OsStr;

    let wide_name = OsStr::new(name).to_wide_null();

    let err = unsafe { SetComputerNameExW(ComputerNamePhysicalDnsHostname, wide_name.as_ptr()) };

    if err == 0 {
        // NOTE: the above is correct, failure is when the function returns 0 apparently
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
