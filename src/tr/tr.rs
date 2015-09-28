#![crate_name = "tr"]
#![feature(io)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 * (c) kwantam <kwantam@gmail.com>
 *     20150428 created `expand` module to eliminate most allocs during setup
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate bit_set;
extern crate getopts;
extern crate vec_map;

use bit_set::BitSet;
use getopts::Options;
use std::io::{stdin, stdout, BufReader, Read, Write};
use vec_map::VecMap;

use expand::ExpandSet;

#[path="../common/util.rs"]
#[macro_use]
mod util;

mod expand;

static NAME: &'static str = "tr";
static VERSION: &'static str = "1.0.0";
const BUFFER_LEN: usize = 1024;

fn delete<'a>(set: ExpandSet<'a>, complement: bool) {
    let mut bset = BitSet::new();
    let mut stdout = stdout();
    let mut buf = String::with_capacity(BUFFER_LEN + 4);

    for c in set {
        bset.insert(c as usize);
    }

    let is_allowed = |c : char| {
        if complement {
            bset.contains(&(c as usize))
        } else {
            !bset.contains(&(c as usize))
        }
    };

    for c in BufReader::new(stdin()).chars() {
        match c {
            Ok(c) if is_allowed(c) => buf.push(c),
            Ok(_) => (),
            Err(err) => panic!("{}", err),
        };
        if buf.len() >= BUFFER_LEN {
            safe_unwrap!(stdout.write_all(&buf[..].as_bytes()));
        }
    }
    if buf.len() > 0 {
        safe_unwrap!(stdout.write_all(&buf[..].as_bytes()));
        pipe_flush!();
    }
}

fn tr<'a>(set1: ExpandSet<'a>, mut set2: ExpandSet<'a>) {
    let mut map = VecMap::new();
    let mut stdout = stdout();
    let mut buf = String::with_capacity(BUFFER_LEN + 4);

    let mut s2_prev = '_';
    for i in set1 {
        s2_prev = set2.next().unwrap_or(s2_prev);

        map.insert(i as usize, s2_prev);
    }

    for c in BufReader::new(stdin()).chars() {
        match c {
            Ok(inc) => {
                let trc = match map.get(&(inc as usize)) {
                    Some(t) => *t,
                    None => inc,
                };
                buf.push(trc);
                if buf.len() >= BUFFER_LEN {
                    safe_unwrap!(stdout.write_all(&buf[..].as_bytes()));
                    buf.truncate(0);
                }
            }
            Err(err) => {
                panic!("{}", err);
            }
        }
    }
    if buf.len() > 0 {
        safe_unwrap!(stdout.write_all(&buf[..].as_bytes()));
        pipe_flush!();
    }
}

fn usage(opts: &Options) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage:");
    println!("  {} [OPTIONS] SET1 [SET2]", NAME);
    println!("");
    println!("{}", opts.usage("Translate or delete characters."));
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("c", "complement", "use the complement of SET1");
    opts.optflag("C", "", "same as -c");
    opts.optflag("d", "delete", "delete characters in SET1");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => {
            show_error!("{}", err);
            return 1;
        }
    };

    if matches.opt_present("help") {
        usage(&opts);
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.free.len() == 0 {
        usage(&opts);
        return 1;
    }

    let dflag = matches.opt_present("d");
    let cflag = matches.opts_present(&["c".to_string(), "C".to_string()]);
    let sets = matches.free;

    if cflag && !dflag {
        show_error!("-c is only supported with -d");
        return 1;
    }

    if dflag {
        let set1 = ExpandSet::new(sets[0].as_ref());
        delete(set1, cflag);
    } else {
        let set1 = ExpandSet::new(sets[0].as_ref());
        let set2 = ExpandSet::new(sets[1].as_ref());
        tr(set1, set2);
    }

    0
}
