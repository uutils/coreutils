#![crate_name = "uu_tr"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 * (c) kwantam <kwantam@gmail.com>
 *     20150428 created `expand` module to eliminate most allocs during setup
 * (c) Sergey "Shnatsel" Davidoff <shnatsel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate bit_set;
extern crate getopts;
extern crate fnv;

#[macro_use]
extern crate uucore;

use bit_set::BitSet;
use getopts::Options;
use std::io::{stdin, stdout, BufRead, BufWriter, Write};
use fnv::FnvHashMap;

use expand::ExpandSet;

mod expand;

static NAME: &'static str = "tr";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");
const BUFFER_LEN: usize = 1024;

fn delete(set: ExpandSet, complement: bool) {
    let mut bset = BitSet::new();
    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let mut buffered_stdout = BufWriter::new(stdout());
    let mut buf = String::with_capacity(BUFFER_LEN + 4);
    let mut char_output_buffer: [u8; 4] = [0;4];

    for c in set {
        bset.insert(c as usize);
    }

    let is_allowed = |c : char| {
        if complement {
            bset.contains(c as usize)
        } else {
            !bset.contains(c as usize)
        }
    };

    while let Ok(length) = locked_stdin.read_line(&mut buf) {
        if length == 0 { break }
        { // isolation to make borrow checker happy
            let filtered = buf.chars().filter(|c| is_allowed(*c));
            for c in filtered {
                let char_as_bytes = c.encode_utf8(&mut char_output_buffer);
                buffered_stdout.write_all(char_as_bytes.as_bytes()).unwrap();
            }
        }
        buf.clear();
    }
}

fn tr<'a>(set1: ExpandSet<'a>, mut set2: ExpandSet<'a>) {
    let mut map = FnvHashMap::default();
    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let mut buffered_stdout = BufWriter::new(stdout());
    let mut buf = String::with_capacity(BUFFER_LEN + 4);
    let mut output_buf = String::with_capacity(BUFFER_LEN + 4);

    let mut s2_prev = '_';
    for i in set1 {
        s2_prev = set2.next().unwrap_or(s2_prev);

        map.insert(i as usize, s2_prev);
    }

    while let Ok(length) = locked_stdin.read_line(&mut buf) {
        if length == 0 { break }

        { // isolation to make borrow checker happy
            let output_stream = buf.chars().map(|c| *map.get(&(c as usize)).unwrap_or(&c));
            output_buf.extend(output_stream);
            buffered_stdout.write_all(output_buf.as_bytes()).unwrap();
        }

        buf.clear();
        output_buf.clear();
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

    if matches.free.is_empty() {
        usage(&opts);
        return 1;
    }

    let dflag = matches.opt_present("d");
    let cflag = matches.opts_present(&["c".to_owned(), "C".to_owned()]);
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
