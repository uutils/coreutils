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

struct TranslateOperation {
    translate_map: FnvHashMap<usize, char>,
}

impl TranslateOperation {
    fn new(set1: ExpandSet, set2: &mut ExpandSet) -> TranslateOperation {
        let mut map = FnvHashMap::default();
        let mut s2_prev = '_';
        for i in set1 {
            s2_prev = set2.next().unwrap_or(s2_prev);

            map.insert(i as usize, s2_prev);
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
    // let mut char_output_buffer: [u8; 4] = [0;4];

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
    opts.optflag("s", "squeeze", "");
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

    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let stdout = stdout();
    let locked_stdout = stdout.lock();
    let mut buffered_stdout = BufWriter::new(locked_stdout);

    if dflag {
        let set1 = ExpandSet::new(sets[0].as_ref());
        let delete_op = DeleteOperation::new(set1, cflag);
        translate_input(&mut locked_stdin, &mut buffered_stdout, delete_op);
    } else {
        let set1 = ExpandSet::new(sets[0].as_ref());
        let mut set2 = ExpandSet::new(sets[1].as_ref());
        let op = TranslateOperation::new(set1, &mut set2);
        translate_input(&mut locked_stdin, &mut buffered_stdout, op)
    }

    0
}
