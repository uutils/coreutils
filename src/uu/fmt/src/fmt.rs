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

mod linebreak;
mod parasplit;

// program's NAME and VERSION are used for -V and -h
static SYNTAX: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Reformat paragraphs from input files (or stdin) to stdout.";
static LONG_HELP: &str = "";
static MAX_WIDTH: usize = 2500;

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
    let args = args.collect_str();

    let matches = app!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("c", "crown-margin", "First and second line of paragraph may have different indentations, in which case the first line's indentation is preserved, and each subsequent line's indentation matches the second line.")
        .optflag("t", "tagged-paragraph", "Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.")
        .optflag("m", "preserve-headers", "Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.")
        .optflag("s", "split-only", "Split lines only, do not reflow.")
        .optflag("u", "uniform-spacing", "Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.")
        .optopt("p", "prefix", "Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.", "PREFIX")
        .optopt("P", "skip-prefix", "Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP", "PSKIP")
        .optflag("x", "exact-prefix", "PREFIX must match at the beginning of the line with no preceding whitespace.")
        .optflag("X", "exact-skip-prefix", "PSKIP must match at the beginning of the line with no preceding whitespace.")
        .optopt("w", "width", "Fill output lines up to a maximum of WIDTH columns, default 79.", "WIDTH")
        .optopt("g", "goal", "Goal width, default ~0.94*WIDTH. Must be less than WIDTH.", "GOAL")
        .optflag("q", "quick", "Break lines more quickly at the expense of a potentially more ragged appearance.")
        .optopt("T", "tab-width", "Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.", "TABWIDTH")
        .parse(args);

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

    if matches.opt_present("t") {
        fmt_opts.tagged = true;
    }
    if matches.opt_present("c") {
        fmt_opts.crown = true;
        fmt_opts.tagged = false;
    }
    if matches.opt_present("m") {
        fmt_opts.mail = true;
    }
    if matches.opt_present("u") {
        fmt_opts.uniform = true;
    }
    if matches.opt_present("q") {
        fmt_opts.quick = true;
    }
    if matches.opt_present("s") {
        fmt_opts.split_only = true;
        fmt_opts.crown = false;
        fmt_opts.tagged = false;
    }
    if matches.opt_present("x") {
        fmt_opts.xprefix = true;
    }
    if matches.opt_present("X") {
        fmt_opts.xanti_prefix = true;
    }

    if let Some(s) = matches.opt_str("p") {
        fmt_opts.prefix = s;
        fmt_opts.use_prefix = true;
    };

    if let Some(s) = matches.opt_str("P") {
        fmt_opts.anti_prefix = s;
        fmt_opts.use_anti_prefix = true;
    };

    if let Some(s) = matches.opt_str("w") {
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

    if let Some(s) = matches.opt_str("g") {
        fmt_opts.goal = match s.parse::<usize>() {
            Ok(t) => t,
            Err(e) => {
                crash!(1, "Invalid GOAL specification: `{}': {}", s, e);
            }
        };
        if !matches.opt_present("w") {
            fmt_opts.width = cmp::max(fmt_opts.goal * 100 / 94, fmt_opts.goal + 3);
        } else if fmt_opts.goal > fmt_opts.width {
            crash!(1, "GOAL cannot be greater than WIDTH.");
        }
    };

    if let Some(s) = matches.opt_str("T") {
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

    let mut files = matches.free;
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
