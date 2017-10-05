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

trait SymbolTranslator {
    fn translate(&self, c: &char, prev_c: &char) -> Option<char>;
}

struct DeleteOperation {
    bset: BitSet,
    complement: bool,
}

impl DeleteOperation {
    fn new(set: ExpandSet, complement: bool) -> DeleteOperation {
        DeleteOperation {
            bset: set.map(|c| c as usize).collect(),
            complement: complement
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&self, c: &char, _prev_c: &char) -> Option<char> {
        let uc = *c as usize;
        if self.complement == self.bset.contains(uc) {
            Some(*c)
        } else {
            None
        }
    }
}

struct SqueezeOperation {
    squeeze_set: BitSet,
    complement: bool,
}

impl SqueezeOperation {
    fn new(squeeze_set: ExpandSet, complement: bool) -> SqueezeOperation {
        SqueezeOperation {
            squeeze_set: squeeze_set.map(|c| c as usize).collect(),
            complement: complement
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&self, c: &char, prev_c: &char) -> Option<char> {
        if *prev_c == *c && self.complement != self.squeeze_set.contains(*c as usize) {
            None
        } else {
            Some(*c)
        }
    }
}

struct DeleteAndSqueezeOperation {
    delete_set: BitSet,
    squeeze_set: BitSet,
    complement: bool,
}

impl DeleteAndSqueezeOperation {
    fn new(delete_set: ExpandSet, squeeze_set: ExpandSet, complement: bool) -> DeleteAndSqueezeOperation {
        DeleteAndSqueezeOperation {
            delete_set: delete_set.map(|c| c as usize).collect(),
            squeeze_set: squeeze_set.map(|c| c as usize).collect(),
            complement: complement
        }
    }
}

impl SymbolTranslator for DeleteAndSqueezeOperation {
    fn translate(&self, c: &char, prev_c: &char) -> Option<char> {
        if self.complement != self.delete_set.contains(*c as usize) || *prev_c == *c && self.squeeze_set.contains(*c as usize) {
            None
        } else {
            Some(*c)
        }
    }
}

struct TranslateOperation {
    translate_map: FnvHashMap<usize, char>,
}

impl TranslateOperation {
    fn new(set1: ExpandSet, set2: &mut ExpandSet, truncate: bool) -> TranslateOperation {
        let mut map = FnvHashMap::default();
        let mut s2_prev = '_';
        for i in set1 {
            let s2_next = set2.next();

            if s2_next.is_none() && truncate {
                map.insert(i as usize, i);
            } else {
                s2_prev = s2_next.unwrap_or(s2_prev);
                map.insert(i as usize, s2_prev);
            }
        }
        TranslateOperation {
            translate_map: map,
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&self, c: &char, _prev_c: &char) -> Option<char> {
        Some(*self.translate_map.get(&(*c as usize)).unwrap_or(c))
    }
}

fn translate_input<T: SymbolTranslator>(input: &mut BufRead, output: &mut Write, translator: T) {
    let mut buf = String::with_capacity(BUFFER_LEN + 4);
    let mut output_buf = String::with_capacity(BUFFER_LEN + 4);

    while let Ok(length) = input.read_line(&mut buf) {
        let mut prev_c = 0 as char;
        if length == 0 { break }
        { // isolation to make borrow checker happy
            let filtered = buf.chars().filter_map(|c| {
                let res = translator.translate(&c, &prev_c);
                if res.is_some() {
                    prev_c = c;
                }
                res
            });

            output_buf.extend(filtered);
            output.write_all(output_buf.as_bytes()).unwrap();
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
    opts.optflag("s", "squeeze", "replace each sequence of a repeated character that is listed in the last specified SET, with a single occurrence of that character");
    opts.optflag("t", "truncate-set1", "first truncate SET1 to length of SET2");
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
    let sflag = matches.opt_present("s");
    let tflag = matches.opt_present("t");
    let sets = matches.free;

    if cflag && !dflag && !sflag {
        show_error!("-c is only supported with -d or -s");
        return 1;
    }

    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let stdout = stdout();
    let locked_stdout = stdout.lock();
    let mut buffered_stdout = BufWriter::new(locked_stdout);

    let set1 = ExpandSet::new(sets[0].as_ref());
    if dflag {
        if sflag {
            let set2 = ExpandSet::new(sets[1].as_ref());
            let op = DeleteAndSqueezeOperation::new(set1, set2, cflag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let op = DeleteOperation::new(set1, cflag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else if sflag {
        let op = SqueezeOperation::new(set1, cflag);
        translate_input(&mut locked_stdin, &mut buffered_stdout, op);
    } else {
        let mut set2 = ExpandSet::new(sets[1].as_ref());
        let op = TranslateOperation::new(set1, &mut set2, tflag);
        translate_input(&mut locked_stdin, &mut buffered_stdout, op)
    }

    0
}
