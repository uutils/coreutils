#![crate_name = "uu_fmt"]

/*
 * This file is part of `fmt` from the uutils coreutils package.
 *
 * (c) kwantam <kwantam@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate unicode_width;

#[macro_use]
extern crate uucore;

use std::cmp;
use std::io::{Read, BufReader, BufWriter};
use std::fs::File;
use std::io::{stdin, stdout, Write};
use linebreak::break_lines;
use parasplit::ParagraphStream;

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
static NAME: &'static str = "fmt";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub type FileOrStdReader = BufReader<Box<Read+'static>>;
pub struct FmtOptions {
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

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("c", "crown-margin", "First and second line of paragraph may have different indentations, in which case the first line's indentation is preserved, and each subsequent line's indentation matches the second line.");
    opts.optflag("t", "tagged-paragraph", "Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.");
    opts.optflag("m", "preserve-headers", "Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.");
    opts.optflag("s", "split-only", "Split lines only, do not reflow.");
    opts.optflag("u", "uniform-spacing", "Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.");

    opts.optopt("p", "prefix", "Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.", "PREFIX");
    opts.optopt("P", "skip-prefix", "Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP", "PSKIP");

    opts.optflag("x", "exact-prefix", "PREFIX must match at the beginning of the line with no preceding whitespace.");
    opts.optflag("X", "exact-skip-prefix", "PSKIP must match at the beginning of the line with no preceding whitespace.");

    opts.optopt("w", "width", "Fill output lines up to a maximum of WIDTH columns, default 79.", "WIDTH");
    opts.optopt("g", "goal", "Goal width, default ~0.94*WIDTH. Must be less than WIDTH.", "GOAL");

    opts.optflag("q", "quick", "Break lines more quickly at the expense of a potentially more ragged appearance.");

    opts.optopt("T", "tab-width", "Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.", "TABWIDTH");

    opts.optflag("V", "version", "Output version information and exit.");
    opts.optflag("h", "help", "Display this help message and exit.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}\nTry `{} --help' for more information.", f, NAME)
    };

    if matches.opt_present("h") {
        println!("Usage: {} [OPTION]... [FILE]...\n\n{}", NAME, opts.usage("Reformat paragraphs from input files (or stdin) to stdout."));
    }

    if matches.opt_present("V") || matches.opt_present("h") {
        println!("{} {}", NAME, VERSION);
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
        files.push("-".to_owned());
    }

    let mut ostream = BufWriter::new(stdout());

    for i in files.iter().map(|x| &x[..]) {
        let mut fp = match i {
            "-" => BufReader::new(Box::new(stdin()) as Box<Read+'static>),
            _ => match File::open(i) {
                Ok(f) => BufReader::new(Box::new(f) as Box<Read+'static>),
                Err(e) => {
                    show_warning!("{}: {}", i, e);
                    continue;
                },
            },
        };
        let p_stream = ParagraphStream::new(&fmt_opts, &mut fp);
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
