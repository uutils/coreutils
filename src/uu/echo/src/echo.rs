// This file is part of the uutils coreutils package.
//
// (c) Derek Chiang <derekchiang93@gmail.com>
// (c) Christopher Brown <ccbrown112@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::io::{self, Write};
use std::iter::Peekable;
use std::str::Chars;
use clap::{crate_version, App, Arg};

<<<<<<< Updated upstream
static VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "echo";
const SYNTAX: &str = "[OPTIONS]... [STRING]...";
const SUMMARY: &str = "display a line of text";
const HELP: &str = r#"
=======
static NAME: &str = "echo";
static USAGE: &str = "[OPTIONS]... [STRING]...";
static SUMMARY: &str = "display a line of text";
static AFTER_HELP: &str = r#"
>>>>>>> Stashed changes
 Echo the STRING(s) to standard output.
 If -e is in effect, the following sequences are recognized:

 \\      backslash
 \a      alert (BEL)
 \b      backspace
 \c      produce no further output
 \e      escape
 \f      form feed
 \n      new line
 \r      carriage return
 \t      horizontal tab
 \v      vertical tab
 \0NNN   byte with octal value NNN (1 to 3 digits)
 \xHH    byte with hexadecimal value HH (1 to 2 digits)
"#;

mod options {
<<<<<<< Updated upstream
    pub const STRING: &str = "string";
    pub const NEWLINE: &str = "n";
    pub const ENABLE_ESCAPE: &str = "e";
    pub const DISABLE_ESCAPE: &str = "E";
=======
    pub static STRING: &str = "STRING";
    pub static NO_NEWLINE: &str = "no_newline";
    pub static ENABLE_BACKSLASH_ESCAPE: &str = "enable_backslash_escape";
    pub static DISABLE_BACKSLASH_ESCAPE: &str = "disable_backslash_escape";
>>>>>>> Stashed changes
}

fn parse_code(
    input: &mut Peekable<Chars>,
    base: u32,
    max_digits: u32,
    bits_per_digit: u32,
) -> Option<char> {
    let mut ret = 0x8000_0000;
    for _ in 0..max_digits {
        match input.peek().and_then(|c| c.to_digit(base)) {
            Some(n) => ret = (ret << bits_per_digit) | n,
            None => break,
        }
        input.next();
    }
    std::char::from_u32(ret)
}

fn print_escaped(input: &str, mut output: impl Write) -> io::Result<bool> {
    let mut should_stop = false;

    let mut buffer = ['\\'; 2];

    let mut iter = input.chars().peekable();
    while let Some(mut c) = iter.next() {
        let mut start = 1;

        if c == '\\' {
            if let Some(next) = iter.next() {
                c = match next {
                    '\\' => '\\',
                    'a' => '\x07',
                    'b' => '\x08',
                    'c' => {
                        should_stop = true;
                        break;
                    }
                    'e' => '\x1b',
                    'f' => '\x0c',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'v' => '\x0b',
                    'x' => parse_code(&mut iter, 16, 2, 4).unwrap_or_else(|| {
                        start = 0;
                        next
                    }),
                    '0' => parse_code(&mut iter, 8, 3, 3).unwrap_or_else(|| {
                        start = 0;
                        next
                    }),
                    _ => {
                        start = 0;
                        next
                    }
                };
            }
        }

        buffer[1] = c;

        // because printing char slices is apparently not available in the standard library
        for ch in &buffer[start..] {
            write!(output, "{}", ch)?;
        }
    }

    Ok(should_stop)
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let matches = App::new(executable!())
        .name(NAME)
<<<<<<< Updated upstream
        .version(VERSION)
        .usage(SYNTAX)
        .about(SUMMARY)
        .help(HELP)
        .arg(Arg::with_name(options::STRING).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::NEWLINE)
                .short("n")
                .help("do not output the trailing newline"),
        )
        .arg(
            Arg::with_name(options::ENABLE_ESCAPE)
                .short("e")
                .help("enable interpretation of backslash escapes"),
        )
        .arg(
            Arg::with_name(options::DISABLE_ESCAPE)
                .short("E")
                .help("disable interpretation of backslash escapes (default)"),
        )
        .get_matches_from(args);

    let no_newline = matches.is_present("n");
    let escaped = matches.is_present("e");
    let values: Vec<String> = match matches.values_of(options::STRING) {
        Some(v) => v.map(|v| v.to_string()).collect(),
=======
        .version(crate_version!())
        .author("TEST")
        .usage(USAGE)
        .about(SUMMARY)
        .after_help(AFTER_HELP)
        .arg(Arg::with_name("HELP")
            .long("help")
            .long_help("print usage information")
            .takes_value(false)
        )
        .arg(Arg::with_name(options::NO_NEWLINE)
            .short("n")
            .long_help("do not output the trailing newline")
            .takes_value(false)
        )
        .arg(Arg::with_name(options::ENABLE_BACKSLASH_ESCAPE)
            .short("e")
            .long_help("enable interpretation of backslash escapes")
            .takes_value(false)
        )
        .arg(Arg::with_name(options::DISABLE_BACKSLASH_ESCAPE)
            .short("E")
            .long_help("disable interpretation of backslash escapes (default)")
        )
        .arg(Arg::with_name(options::STRING)
        .hidden(true)
        .multiple(true)
        .takes_value(true)
        // .allow_hyphen_values(true)
        )
        .get_matches_from(args);

/*     let matches = app!(SYNTAX, SUMMARY, HELP)
        .optflag("n", "", "do not output the trailing newline")
        .optflag("e", "", "enable interpretation of backslash escapes")
        .optflag(
            "E",
            "",
            "disable interpretation of backslash escapes (default)",
        )
        .parse(args); */

    let no_newline = matches.is_present(options::NO_NEWLINE);
    let escaped = matches.is_present(options::ENABLE_BACKSLASH_ESCAPE);
    let values: Vec<String> =  match matches.values_of(options::STRING) {
        Some(s) => s.map(|s| s.to_string()).collect(),
>>>>>>> Stashed changes
        None => vec!["".to_string()],
    };

    match execute(no_newline, escaped, values) {
        Ok(_) => 0,
        Err(f) => {
            show_error!("{}", f);
            1
        }
    }
}

fn execute(no_newline: bool, escaped: bool, free: Vec<String>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    for (i, input) in free.iter().enumerate() {
        if i > 0 {
            write!(output, " ")?;
        }
        if escaped {
            let should_stop = print_escaped(&input, &mut output)?;
            if should_stop {
                break;
            }
        } else {
            write!(output, "{}", input)?;
        }
    }

    if !no_newline {
        writeln!(output)?;
    }

    Ok(())
}
