#![crate_name = "fmt"]
#![allow(unstable)]

/*
 * This file is part of `fmt` from the uutils coreutils package.
 *
 * (c) kwantam <kwantam@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(box_syntax)]

extern crate core;
extern crate getopts;
extern crate unicode;

use std::cmp;
use std::old_io::{BufferedReader, BufferedWriter, File, IoResult};
use std::old_io::stdio::{stdin_raw, stdout_raw};
use linebreak::break_lines;
use parasplit::ParagraphStream;

macro_rules! silent_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(_) => (),
            Err(_) => unsafe { ::util::libc::exit(1) }
        }
    )
);

#[path = "../common/util.rs"]
#[macro_use]
mod util;
mod linebreak;
mod parasplit;

// program's NAME and VERSION are used for -V and -h
static NAME: &'static str = "fmt";
static VERSION: &'static str = "0.0.3";

struct FmtOptions {
    crown           : bool,
    tagged          : bool,
    mail            : bool,
    split_only      : bool,
    use_prefix      : bool,
    prefix          : String,
    xprefix         : bool,
    use_anti_prefix : bool,
    anti_prefix     : String,
    xanti_prefix    : bool,
    uniform         : bool,
    quick           : bool,
    width           : usize,
    goal            : usize,
    tabwidth        : usize,
}

pub fn uumain(args: Vec<String>) -> isize {

    let opts = [
        getopts::optflag("c", "crown-margin", "First and second line of paragraph may have different indentations, in which case the first line's indentation is preserved, and each subsequent line's indentation matches the second line."),
        getopts::optflag("t", "tagged-paragraph", "Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs."),
        getopts::optflag("m", "preserve-headers", "Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p."),
        getopts::optflag("s", "split-only", "Split lines only, do not reflow."),
        getopts::optflag("u", "uniform-spacing", "Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break."),

        getopts::optopt("p", "prefix", "Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.", "PREFIX"),
        getopts::optopt("P", "skip-prefix", "Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP", "PSKIP"),

        getopts::optflag("x", "exact-prefix", "PREFIX must match at the beginning of the line with no preceding whitespace."),
        getopts::optflag("X", "exact-skip-prefix", "PSKIP must match at the beginning of the line with no preceding whitespace."),

        getopts::optopt("w", "width", "Fill output lines up to a maximum of WIDTH columns, default 79.", "WIDTH"),
        getopts::optopt("g", "goal", "Goal width, default ~0.94*WIDTH. Must be less than WIDTH.", "GOAL"),

        getopts::optflag("q", "quick", "Break lines more quickly at the expense of a potentially more ragged appearance."),

        getopts::optopt("T", "tab-width", "Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.", "TABWIDTH"),

        getopts::optflag("V", "version", "Output version information and exit."),
        getopts::optflag("h", "help", "Display this help message and exit.")
            ];

    let matches = match getopts::getopts(args.tail(), opts.as_slice()) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}\nTry `{} --help' for more information.", f, args[0])
    };

    if matches.opt_present("h") {
        print_usage(args[0].as_slice(), opts.as_slice(), "");
    }

    if matches.opt_present("V") || matches.opt_present("h") {
        println!("uutils {} v{}", NAME, VERSION);
        return 0
    }

    let mut fmt_opts = FmtOptions {
        crown           : false,
        tagged          : false,
        mail            : false,
        uniform         : false,
        quick           : false,
        split_only      : false,
        use_prefix      : false,
        prefix          : String::new(),
        xprefix         : false,
        use_anti_prefix : false,
        anti_prefix     : String::new(),
        xanti_prefix    : false,
        width           : 79,
        goal            : 74,
        tabwidth        : 8,
    };

    if matches.opt_present("t") { fmt_opts.tagged       = true; }
    if matches.opt_present("c") { fmt_opts.crown        = true; fmt_opts.tagged = false; }
    if matches.opt_present("m") { fmt_opts.mail         = true; }
    if matches.opt_present("u") { fmt_opts.uniform      = true; }
    if matches.opt_present("q") { fmt_opts.quick        = true; }
    if matches.opt_present("s") { fmt_opts.split_only   = true; fmt_opts.crown  = false; fmt_opts.tagged = false; }
    if matches.opt_present("x") { fmt_opts.xprefix      = true; }
    if matches.opt_present("X") { fmt_opts.xanti_prefix = true; }

    match matches.opt_str("p") {
        Some(s) => {
            fmt_opts.prefix = s;
            fmt_opts.use_prefix = true;
        }
        None => ()
    };

    match matches.opt_str("P") {
        Some(s) => {
            fmt_opts.anti_prefix = s;
            fmt_opts.use_anti_prefix = true;
        }
        None => ()
    };

    match matches.opt_str("w") {
        Some(s) => {
            fmt_opts.width =
                match s.parse::<usize>() {
                    Ok(t) => t,
                    Err(e) => { crash!(1, "Invalid WIDTH specification: `{}': {}", s, e); }
                };
            fmt_opts.goal = cmp::min(fmt_opts.width * 94 / 100, fmt_opts.width - 3);
        }
        None => ()
    };

    match matches.opt_str("g") {
        Some(s) => {
            fmt_opts.goal =
                match s.parse::<usize>() {
                    Ok(t) => t,
                    Err(e) => { crash!(1, "Invalid GOAL specification: `{}': {}", s, e); }
                };
            if !matches.opt_present("w") {
                fmt_opts.width = cmp::max(fmt_opts.goal * 100 / 94, fmt_opts.goal + 3);
            } else if fmt_opts.goal > fmt_opts.width {
                crash!(1, "GOAL cannot be greater than WIDTH.");
            }
        }
        None => ()
    };

    match matches.opt_str("T") {
        Some(s) => {
            fmt_opts.tabwidth =
                match s.parse::<usize>() {
                    Ok(t) => t,
                    Err(e) => { crash!(1, "Invalid TABWIDTH specification: `{}': {}", s, e); }
                };
        }
        None => ()
    };

    if fmt_opts.tabwidth < 1 {
        fmt_opts.tabwidth = 1;
    }

    // immutable now
    let fmt_opts = fmt_opts;

    let mut files = matches.free;
    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut ostream = box BufferedWriter::new(stdout_raw()) as Box<Writer>;

    for i in files.iter().map(|x| x.as_slice()) {
        let mut fp =
            match open_file(i) {
                Err(e) => {
                    show_warning!("{}: {}", i, e);
                    continue;
                }
                Ok(f) => f
            };
        let mut p_stream = ParagraphStream::new(&fmt_opts, &mut fp);
        for para_result in p_stream {
            match para_result {
                Err(s) => silent_unwrap!(ostream.write_all(s.as_bytes())),
                Ok(para) => break_lines(&para, &fmt_opts, &mut ostream)
            }
        }

        // flush the output after each file
        silent_unwrap!(ostream.flush());
    }

    0
}

fn print_usage(arg0: &str, opts: &[getopts::OptGroup], errmsg: &str) {
    println!("Usage: {} [OPTION]... [FILE]...\n\n{}{}", arg0, getopts::usage("Reformat paragraphs from input files (or stdin) to stdout.", opts), errmsg);
}

// uniform interface for opening files
// since we don't need seeking
type FileOrStdReader = BufferedReader<Box<Reader+'static>>;

fn open_file(filename: &str) -> IoResult<FileOrStdReader> {
    if filename == "-" {
        Ok(BufferedReader::new(box stdin_raw() as Box<Reader>))
    } else {
        match File::open(&Path::new(filename)) {
            Ok(f) => Ok(BufferedReader::new(box f as Box<Reader>)),
            Err(e) => return Err(e)
        }
    }
}
