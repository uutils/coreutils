#![crate_name = "tr"]
#![feature(macro_rules)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate collections;
extern crate getopts;

use collections::bitv::BitvSet;
use collections::smallintmap::SmallIntMap;
use getopts::OptGroup;
use std::char::from_u32;
use std::io::print;
use std::io::stdio::{stdin,stdout};
use std::iter::FromIterator;
use std::vec::Vec;

#[path="../common/util.rs"]
mod util;

static NAME : &'static str = "tr";
static VERSION : &'static str = "1.0.0";

fn unescape_char(c: char) -> char {
    match c {
        'a' => 0x07u8 as char,
        'b' => 0x08u8 as char,
        'f' => 0x0cu8 as char,
        'v' => 0x0bu8 as char,
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        _ => c,
    }
}

fn unescape(v: Vec<char>) -> Vec<char> {
    let mut out = Vec::new();
    let mut input = v.as_slice();
    loop {
        input = match input {
            ['\\', e, ..rest] => {
                out.push(unescape_char(e));
                rest
            }
            [c, ..rest] => {
                out.push(c);
                rest
            }
            [] => break
        }
    }
    out
}

fn expand_range(from: char, to: char) -> Vec<char> {
    range(from as u32, to as u32 + 1).map(|c| from_u32(c).unwrap()).collect()
}

fn expand_set(s: &str) -> Vec<char> {
    let mut set = Vec::<char>::new();
    let unesc = unescape(FromIterator::from_iter(s.chars()));
    let mut input = unesc.as_slice();

    loop {
        input = match input {
            [f, '-', t, ..rest] => {
                set.push_all(expand_range(f, t).as_slice());
                rest
            }
            [c, ..rest] => {
                set.push(c);
                rest
            }
            [] => break
        };
    }
    set
}

fn delete(set: Vec<char>, complement: bool) {
    let mut bset = BitvSet::new();
    let mut out = stdout();

    for &c in set.iter() {
        bset.insert(c as uint);
    }

    let is_allowed = if complement {
        |c: char| bset.contains(&(c as uint))
    } else {
        |c: char| !bset.contains(&(c as uint))
    };

    for c in stdin().chars() {
        match c {
            Ok(c) if is_allowed(c) => out.write_char(c).unwrap(),
            Ok(_) => (),
            Err(err) => fail!("{}", err),
        };
    }
}

fn tr(set1: &[char], set2: &[char]) {
    let mut map = SmallIntMap::<char>::new();
    let mut out = stdout();

    for i in range(0, set1.len()) {
        if i >= set2.len() {
            map.insert(set1[i] as uint, set2[set2.len()-1]);
        } else {
            map.insert(set1[i] as uint, set2[i]);
        }
    }

    for c in stdin().chars() {
        match c {
            Ok(inc) => {
                let trc = match map.find(&(inc as uint)) {
                    Some(t) => *t,
                    None => inc,
                };
                out.write_char(trc).unwrap();
            }
            Err(err) => {
                fail!("{}", err);
            }
        }
    }
}

fn usage(opts: &[OptGroup]) {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] SET1 [SET2]", NAME);
        println!("");
        print(getopts::usage("Translate or delete characters.", opts).as_slice());
}

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        getopts::optflag("c", "complement", "use the complement of SET1"),
        getopts::optflag("C", "", "same as -c"),
        getopts::optflag("d", "delete", "delete characters in SET1"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(err) => {
            show_error!("{}", err);
            return 1;
        }
    };

    if matches.opt_present("help") {
        usage(opts);
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.free.len() == 0 {
        usage(opts);
        return 1;
    }

    let dflag = matches.opt_present("d");
    let cflag = matches.opts_present(["c".to_string(), "C".to_string()]);
    let sets = matches.free;

    if cflag && !dflag {
        show_error!("-c is only supported with -d");
        return 1;
    }

    if dflag {
        let set1 = expand_set(sets[0].as_slice());
        delete(set1, cflag);
    } else {
        let set1 = expand_set(sets[0].as_slice());
        let set2 = expand_set(sets[1].as_slice());
        tr(set1.as_slice(), set2.as_slice());
    }

    0
}
