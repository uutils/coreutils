#[crate_id(name="hostname", vers="1.0.0", author="Alan Andrade")];
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

extern mod extra;
extern mod getopts;

use std::{os,libc,vec,str};
use getopts::{optflag, getopts, usage};

extern {
    fn gethostname(name: *libc::c_char, namelen: libc::size_t) -> libc::c_int;
    fn sethostname(name: *libc::c_char, namelen: libc::c_int) -> libc::c_int;
}

fn main () {
    let args = os::args();
    let program = args[0].to_owned();

    let options = [
        optflag("f", "full", "Default option to show full name"),
        optflag("s", "slice subdomain", "Cuts the subdomain off if any"),
        optflag("h", "help", "Show help"),
        optflag("V", "version", "Show program's version")
    ];

    let matches = match getopts(args.tail(), options) {
        Ok(m) => { m }
        _ => { println!("{:s}", usage(program, options)); return; }
    };

    if matches.opt_present("h") {
        println!("{:s}", usage(program, options));
        return
    }
    if matches.opt_present("V") { version(); return }

    match matches.free.len() {
        0 => {
            let hostname: ~str = xgethostname();

            if matches.opt_present("s") {
                let pos = hostname.find_str(".");
                if pos.is_some() {
                    println!("{:s}", hostname.slice_to(pos.unwrap()));
                    return;
                }
            }

            println!("{:s}", hostname);
        }
        1 => { xsethostname( matches.free.last().unwrap() ) }
        _ => { println!("{:s}", usage(program, options)); }
    };
}

fn version() {
    println!("hostname version 1.0.0");
}

fn xgethostname() -> ~str {
    let namelen = 255u;
    let mut name = vec::from_elem(namelen, 0u8);

    let err = unsafe {
        gethostname (name.as_mut_ptr() as *libc::c_char,
                                        namelen as libc::size_t)
    };

    if err != 0 {
        fail!("Cannot determine hostname");
    }

    let last_char = name.iter().position(|byte| *byte == 0).unwrap_or(namelen);

    str::from_utf8(name.slice_to(last_char)).unwrap().to_owned()
}

fn xsethostname(name: &~str) {
    let vec_name: ~[libc::c_char] = name.bytes().map(|c| c as i8).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as i32)
    };

    if err != 0 {
        println!("Cannot set hostname to {:s}", *name);
    }
}
