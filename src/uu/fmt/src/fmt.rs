//  * This file is part of `fmt` from the uutils coreutils package.
//  *
//  * (c) kwantam <kwantam@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) PSKIP linebreak ostream parasplit tabwidth xanti xprefix

#[macro_use]
extern crate uucore;

use std::cmp;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::io::{BufReader, BufWriter, Read};

use app::*;

use self::linebreak::break_lines;
use self::parasplit::ParagraphStream;

macro_rules! silent_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(_) => (),
            Err(_) => ::std::process::exit(1),
        }
    )
);

mod app;
mod linebreak;
mod parasplit;

const MAX_WIDTH: usize = 2500;

fn get_usage() -> String {
    format!("{} [OPTION]... [FILE]...", executable!())
}

pub type FileOrStdReader = BufReader<Box<dyn Read + 'static>>;
pub struct FmtOptions {
    crown: bool,
    tagged: bool,
    mail: bool,
    split_only: bool,
    use_prefix: bool,
    prefix: String,
    xprefix: bool,
    use_anti_prefix: bool,
    anti_prefix: String,
    xanti_prefix: bool,
    uniform: bool,
    quick: bool,
    width: usize,
    goal: usize,
    tabwidth: usize,
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(usage.as_str())
        .get_matches_from(args);

    let mut files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let mut fmt_opts = FmtOptions {
        crown: false,
        tagged: false,
        mail: false,
        uniform: false,
        quick: false,
        split_only: false,
        use_prefix: false,
        prefix: String::new(),
        xprefix: false,
        use_anti_prefix: false,
        anti_prefix: String::new(),
        xanti_prefix: false,
        width: 79,
        goal: 74,
        tabwidth: 8,
    };

    fmt_opts.tagged = matches.is_present(OPT_TAGGED_PARAGRAPH);
    if matches.is_present(OPT_CROWN_MARGIN) {
        fmt_opts.crown = true;
        fmt_opts.tagged = false;
    }
    fmt_opts.mail = matches.is_present(OPT_PRESERVE_HEADERS);
    fmt_opts.uniform = matches.is_present(OPT_UNIFORM_SPACING);
    fmt_opts.quick = matches.is_present(OPT_QUICK);
    if matches.is_present(OPT_SPLIT_ONLY) {
        fmt_opts.split_only = true;
        fmt_opts.crown = false;
        fmt_opts.tagged = false;
    }
    fmt_opts.xprefix = matches.is_present(OPT_EXACT_PREFIX);
    fmt_opts.xanti_prefix = matches.is_present(OPT_SKIP_PREFIX);

    if let Some(s) = matches.value_of(OPT_PREFIX).map(String::from) {
        fmt_opts.prefix = s;
        fmt_opts.use_prefix = true;
    };

    if let Some(s) = matches.value_of(OPT_SKIP_PREFIX).map(String::from) {
        fmt_opts.anti_prefix = s;
        fmt_opts.use_anti_prefix = true;
    };

    if let Some(s) = matches.value_of(OPT_WIDTH) {
        fmt_opts.width = match s.parse::<usize>() {
            Ok(t) => t,
            Err(e) => {
                crash!(1, "Invalid WIDTH specification: `{}': {}", s, e);
            }
        };
        if fmt_opts.width > MAX_WIDTH {
            crash!(
                1,
                "invalid width: '{}': Numerical result out of range",
                fmt_opts.width
            );
        }
        fmt_opts.goal = cmp::min(fmt_opts.width * 94 / 100, fmt_opts.width - 3);
    };

    if let Some(s) = matches.value_of(OPT_GOAL) {
        fmt_opts.goal = match s.parse::<usize>() {
            Ok(t) => t,
            Err(e) => {
                crash!(1, "Invalid GOAL specification: `{}': {}", s, e);
            }
        };
        if !matches.is_present(OPT_WIDTH) {
            fmt_opts.width = cmp::max(fmt_opts.goal * 100 / 94, fmt_opts.goal + 3);
        } else if fmt_opts.goal > fmt_opts.width {
            crash!(1, "GOAL cannot be greater than WIDTH.");
        }
    };

    if let Some(s) = matches.value_of(OPT_TAB_WIDTH) {
        fmt_opts.tabwidth = match s.parse::<usize>() {
            Ok(t) => t,
            Err(e) => {
                crash!(1, "Invalid TABWIDTH specification: `{}': {}", s, e);
            }
        };
    };

    if fmt_opts.tabwidth < 1 {
        fmt_opts.tabwidth = 1;
    }

    // immutable now
    let fmt_opts = fmt_opts;

    if files.is_empty() {
        files.push("-".to_owned());
    }

    let mut ostream = BufWriter::new(stdout());

    for i in files.iter().map(|x| &x[..]) {
        let mut fp = match i {
            "-" => BufReader::new(Box::new(stdin()) as Box<dyn Read + 'static>),
            _ => match File::open(i) {
                Ok(f) => BufReader::new(Box::new(f) as Box<dyn Read + 'static>),
                Err(e) => {
                    show_warning!("{}: {}", i, e);
                    continue;
                }
            },
        };
        let p_stream = ParagraphStream::new(&fmt_opts, &mut fp);
        for para_result in p_stream {
            match para_result {
                Err(s) => {
                    silent_unwrap!(ostream.write_all(s.as_bytes()));
                    silent_unwrap!(ostream.write_all(b"\n"));
                }
                Ok(para) => break_lines(&para, &fmt_opts, &mut ostream),
            }
        }

        // flush the output after each file
        silent_unwrap!(ostream.flush());
    }

    0
}
