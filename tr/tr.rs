#![crate_id(name="tr", vers="1.0.0", author="Michael Gehring")]

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

use collections::hashmap::{HashMap, HashSet};
use getopts::OptGroup;
use std::char::from_u32;
use std::io::print;
use std::io::stdio::{stdin,stdout};
use std::iter::FromIterator;
use std::os;
use std::vec::Vec;

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

fn delete(set: Vec<char>) {
    let mut hset = HashSet::new();
    let mut out = stdout();

    for &c in set.iter() {
        hset.insert(c);
    }

    for c in stdin().chars() {
        match c {
            Ok(c) if !hset.contains(&c) => out.write_char(c).unwrap(),
            Ok(_) => (),
            Err(err) => fail!("{}", err),
        };
    }
}

fn tr(set1: &[char], set2: &[char]) {
    let mut map = HashMap::new();
    let mut out = stdout();

    for i in range(0, set1.len()) {
        if i >= set2.len() {
            map.insert(set1[i], set2[set2.len()-1]);
        } else {
            map.insert(set1[i], set2[i]);
        }
    }

    for c in stdin().chars() {
        match c {
            Ok(inc) => {
                let trc = match map.find(&inc) {
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

pub fn main() {
    let args: Vec<StrBuf> = os::args().iter().map(|x| x.to_strbuf()).collect();
    let opts = [
        getopts::optflag("d", "delete", "delete characters in SET1"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(err) => fail!("{}", err.to_err_msg()),
    };

    if matches.opt_present("help") {
        usage(opts);
        return
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return;
    }

    if matches.free.len() == 0 {
        usage(opts);
        os::set_exit_status(1);
        return
    }

    let dflag = matches.opt_present("d");
    let sets = matches.free;

    if dflag {
        let set1 = expand_set(sets.get(0).as_slice());
        delete(set1);
    } else {
        let set1 = expand_set(sets.get(0).as_slice());
        let set2 = expand_set(sets.get(1).as_slice());
        tr(set1.as_slice(), set2.as_slice());
    }
}
