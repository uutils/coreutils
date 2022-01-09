//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  * (c) kwantam <kwantam@gmail.com>
//  *     * 2015-04-28 ~ created `expand` module to eliminate most allocs during setup
//  * (c) Sergey "Shnatsel" Davidoff <shnatsel@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs bset dflag cflag sflag tflag

mod expand;

use bit_set::BitSet;
use clap::{crate_version, App, Arg};
use fnv::FnvHashMap;
use std::io::{stdin, stdout, BufRead, BufWriter, Write};

use crate::expand::ExpandSet;
use uucore::error::{UResult, UUsageError};
use uucore::{display::Quotable, InvalidEncodingHandling};

static ABOUT: &str = "translate or delete characters";

const BUFFER_LEN: usize = 1024;

mod options {
    pub const COMPLEMENT: &str = "complement";
    pub const DELETE: &str = "delete";
    pub const SQUEEZE: &str = "squeeze-repeats";
    pub const TRUNCATE: &str = "truncate";
    pub const SETS: &str = "sets";
}

trait SymbolTranslator {
    fn translate(&self, c: char, prev_c: char) -> Option<char>;
}

struct DeleteOperation {
    bset: BitSet,
    complement: bool,
}

impl DeleteOperation {
    fn new(set: ExpandSet, complement: bool) -> DeleteOperation {
        DeleteOperation {
            bset: set.map(|c| c as usize).collect(),
            complement,
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&self, c: char, _prev_c: char) -> Option<char> {
        let uc = c as usize;
        if self.complement == self.bset.contains(uc) {
            Some(c)
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
            complement,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&self, c: char, prev_c: char) -> Option<char> {
        if prev_c == c && self.complement != self.squeeze_set.contains(c as usize) {
            None
        } else {
            Some(c)
        }
    }
}

struct DeleteAndSqueezeOperation {
    delete_set: BitSet,
    squeeze_set: BitSet,
    complement: bool,
}

impl DeleteAndSqueezeOperation {
    fn new(
        delete_set: ExpandSet,
        squeeze_set: ExpandSet,
        complement: bool,
    ) -> DeleteAndSqueezeOperation {
        DeleteAndSqueezeOperation {
            delete_set: delete_set.map(|c| c as usize).collect(),
            squeeze_set: squeeze_set.map(|c| c as usize).collect(),
            complement,
        }
    }
}

impl SymbolTranslator for DeleteAndSqueezeOperation {
    fn translate(&self, c: char, prev_c: char) -> Option<char> {
        if self.complement != self.delete_set.contains(c as usize)
            || prev_c == c && self.squeeze_set.contains(c as usize)
        {
            None
        } else {
            Some(c)
        }
    }
}

struct TranslateOperation {
    translate_map: FnvHashMap<usize, char>,
    complement: bool,
    s2_last: char,
}

impl TranslateOperation {
    fn new(
        set1: ExpandSet,
        set2: &mut ExpandSet,
        truncate: bool,
        complement: bool,
    ) -> TranslateOperation {
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
            complement,
            s2_last: set2.last().unwrap_or(s2_prev),
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&self, c: char, _prev_c: char) -> Option<char> {
        if self.complement {
            Some(if self.translate_map.contains_key(&(c as usize)) {
                c
            } else {
                self.s2_last
            })
        } else {
            Some(*self.translate_map.get(&(c as usize)).unwrap_or(&c))
        }
    }
}

struct TranslateAndSqueezeOperation {
    translate: TranslateOperation,
    squeeze: SqueezeOperation,
}

impl TranslateAndSqueezeOperation {
    fn new(sets: Vec<String>, truncate: bool, complement: bool) -> TranslateAndSqueezeOperation {
        let set1 = ExpandSet::new(sets[0].as_ref());
        let set1_ = ExpandSet::new(sets[0].as_ref());
        let mut set2 = ExpandSet::new(sets[1].as_ref());
        let set2_ = ExpandSet::new(sets[1].as_ref());
        TranslateAndSqueezeOperation {
            translate: TranslateOperation::new(set1, &mut set2, truncate, complement),
            squeeze: SqueezeOperation::new(if complement { set1_ } else { set2_ }, complement),
        }
    }
}

impl SymbolTranslator for TranslateAndSqueezeOperation {
    fn translate(&self, c: char, prev_c: char) -> Option<char> {
        // `unwrap()` will never panic because `Translate.translate()`
        // always returns `Some`.
        self.squeeze
            .translate(self.translate.translate(c, 0 as char).unwrap(), prev_c)
    }
}

fn translate_input<T: SymbolTranslator>(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    translator: T,
) {
    let mut buf = String::with_capacity(BUFFER_LEN + 4);
    let mut output_buf = String::with_capacity(BUFFER_LEN + 4);

    while let Ok(length) = input.read_line(&mut buf) {
        let mut prev_c = 0 as char;
        if length == 0 {
            break;
        }
        {
            // isolation to make borrow checker happy
            let filtered = buf.chars().filter_map(|c| {
                let res = translator.translate(c, prev_c);
                // Set `prev_c` to the post-translate character. This
                // allows the squeeze operation to correctly function
                // after the translate operation.
                if let Some(rc) = res {
                    prev_c = rc;
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

fn usage() -> String {
    format!("{} [OPTION]... SET1 [SET2]", uucore::execution_phrase())
}

fn get_long_usage() -> String {
    "Translate, squeeze, and/or delete characters from standard input,
writing to standard output."
        .to_string()
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = usage();
    let after_help = get_long_usage();

    let matches = uu_app()
        .usage(&usage[..])
        .after_help(&after_help[..])
        .get_matches_from(args);

    let delete_flag = matches.is_present(options::DELETE);
    let complement_flag = matches.is_present(options::COMPLEMENT) || matches.is_present("C");
    let squeeze_flag = matches.is_present(options::SQUEEZE);
    let truncate_flag = matches.is_present(options::TRUNCATE);

    let sets = matches
        .values_of(options::SETS)
        .map(|v| v.map(ToString::to_string).collect::<Vec<_>>())
        .unwrap_or_default();

    if sets.is_empty() {
        return Err(UUsageError::new(1, "missing operand"));
    }

    if !(delete_flag || squeeze_flag) && sets.len() < 2 {
        return Err(UUsageError::new(
            1,
            format!("missing operand after {}", sets[0].quote()),
        ));
    }

    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let stdout = stdout();
    let locked_stdout = stdout.lock();
    let mut buffered_stdout = BufWriter::new(locked_stdout);

    let set1 = ExpandSet::new(sets[0].as_ref());
    if delete_flag {
        if squeeze_flag {
            let set2 = ExpandSet::new(sets[1].as_ref());
            let op = DeleteAndSqueezeOperation::new(set1, set2, complement_flag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let op = DeleteOperation::new(set1, complement_flag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else if squeeze_flag {
        if sets.len() < 2 {
            let op = SqueezeOperation::new(set1, complement_flag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let op = TranslateAndSqueezeOperation::new(sets, truncate_flag, complement_flag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else {
        let mut set2 = ExpandSet::new(sets[1].as_ref());
        let op = TranslateOperation::new(set1, &mut set2, truncate_flag, complement_flag);
        translate_input(&mut locked_stdin, &mut buffered_stdout, op);
    }
    Ok(())
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::COMPLEMENT)
                // .visible_short_alias('C')  // TODO: requires clap "3.0.0-beta.2"
                .short("c")
                .long(options::COMPLEMENT)
                .help("use the complement of SET1"),
        )
        .arg(
            Arg::with_name("C") // work around for `Arg::visible_short_alias`
                .short("C")
                .help("same as -c"),
        )
        .arg(
            Arg::with_name(options::DELETE)
                .short("d")
                .long(options::DELETE)
                .help("delete characters in SET1, do not translate"),
        )
        .arg(
            Arg::with_name(options::SQUEEZE)
                .long(options::SQUEEZE)
                .short("s")
                .help(
                    "replace each sequence  of  a  repeated  character  that  is
  listed  in the last specified SET, with a single occurrence
  of that character",
                ),
        )
        .arg(
            Arg::with_name(options::TRUNCATE)
                .long(options::TRUNCATE)
                .short("t")
                .help("first truncate SET1 to length of SET2"),
        )
        .arg(
            Arg::with_name(options::SETS)
                .multiple(true)
                .takes_value(true)
                .min_values(1)
                .max_values(2),
        )
}
