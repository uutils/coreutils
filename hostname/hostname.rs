#![crate_id(name="hostname", vers="1.0.0", author="Alan Andrade")]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * Synced with:
 *
 * https://www.opensource.apple.com/source/shell_cmds/shell_cmds-170/hostname/hostname.c?txt
 */

extern crate getopts;
extern crate libc;

use std::str;
use getopts::{optflag, getopts, usage};

extern {
    fn gethostname(name: *libc::c_char, namelen: libc::size_t) -> libc::c_int;
}

#[cfg(target_os = "macos")]
extern {
    fn sethostname(name: *libc::c_char, namelen: libc::c_int) -> libc::c_int;
}

#[cfg(target_os = "linux")]
extern {
    fn sethostname(name: *libc::c_char, namelen: libc::size_t) -> libc::c_int;
}

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0);

    let options = [
        optflag("f", "full", "Default option to show full name"),
        optflag("s", "slice subdomain", "Cuts the subdomain off if any"),
        optflag("h", "help", "Show help"),
        optflag("V", "version", "Show program's version")
    ];

    let matches = match getopts(args.tail(), options) {
        Ok(m) => { m }
        _ => { help_menu(program.as_slice(), options); return 0; }
    };

    if matches.opt_present("h") {
        help_menu(program.as_slice(), options);
        return 0
    }
    if matches.opt_present("V") { version(); return 0 }

    match matches.free.len() {
        0 => {
            let hostname = xgethostname();

            if matches.opt_present("s") {
                let pos = hostname.as_slice().find_str(".");
                if pos.is_some() {
                    println!("{:s}", hostname.as_slice().slice_to(pos.unwrap()));
                    return 0;
                }
            }

            println!("{:s}", hostname.as_slice());
        }
        1 => { xsethostname( matches.free.last().unwrap().as_slice() ) }
        _ => { help_menu(program.as_slice(), options); }
    };

    0
}

fn version() {
    println!("hostname 1.0.0");
}

fn help_menu(program: &str, options: &[getopts::OptGroup]) {
    version();
    println!("");
    println!("Usage:");
    println!("  {:s} [OPTION]... [HOSTNAME]", program);
    println!("");
    print!("{:s}", usage("Print or set the system's host name.", options));
}

fn xgethostname() -> String {
    let namelen = 256u;
    let mut name = Vec::from_elem(namelen, 0u8);

    let err = unsafe {
        gethostname (name.as_mut_ptr() as *libc::c_char,
                                        namelen as libc::size_t)
    };

    if err != 0 {
        fail!("Cannot determine hostname");
    }

    let last_char = name.iter().position(|byte| *byte == 0).unwrap_or(namelen);

    str::from_utf8(name.slice_to(last_char)).unwrap().to_string()
}

#[cfg(target_os = "macos")]
fn xsethostname(name: &str) {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as libc::c_int)
    };

    if err != 0 {
        println!("Cannot set hostname to {:s}", name);
    }
}

#[cfg(target_os = "linux")]
fn xsethostname(name: &str) {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as libc::size_t)
    };

    if err != 0 {
        println!("Cannot set hostname to {:s}", name);
    }
}
