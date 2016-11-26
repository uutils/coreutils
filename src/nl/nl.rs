#![crate_name = "uu_nl"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Tobias Bohumir Schottdorf <tobias.schottdorf@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

extern crate aho_corasick;
extern crate getopts;
extern crate memchr;
extern crate regex_syntax;
extern crate regex;

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin, Write};
use std::iter::repeat;
use std::path::Path;

mod helper;

static NAME: &'static str = "nl";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static USAGE: &'static str = "nl [OPTION]... [FILE]...";
// A regular expression matching everything.

// Settings store options used by nl to produce its output.
pub struct Settings {
    // The variables corresponding to the options -h, -b, and -f.
    header_numbering: NumberingStyle,
    body_numbering: NumberingStyle,
    footer_numbering: NumberingStyle,
    // The variable corresponding to -d
    section_delimiter: [char; 2],
    // The variables corresponding to the options -v, -i, -l, -w.
    starting_line_number: u64,
    line_increment: u64,
    join_blank_lines: u64,
    number_width: usize, // Used with String::from_char, hence usize.
    // The format of the number and the (default value for)
    // renumbering each page.
    number_format: NumberFormat,
    renumber: bool,
    // The string appended to each line number output.
    number_separator: String
}

// NumberingStyle stores which lines are to be numbered.
// The possible options are:
// 1. Number all lines
// 2. Number only nonempty lines
// 3. Don't number any lines at all
// 4. Number all lines that match a basic regular expression.
enum NumberingStyle {
    NumberForAll,
    NumberForNonEmpty,
    NumberForNone,
    NumberForRegularExpression(regex::Regex)
}

// NumberFormat specifies how line numbers are output within their allocated
// space. They are justified to the left or right, in the latter case with
// the option of having all unused space to its left turned into leading zeroes.
enum NumberFormat {
    Left,
    Right,
    RightZero,
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("b", "body-numbering", "use STYLE for numbering body lines", "STYLE");
    opts.optopt("d", "section-delimiter", "use CC for separating logical pages", "CC");
    opts.optopt("f", "footer-numbering", "use STYLE for numbering footer lines", "STYLE");
    opts.optopt("h", "header-numbering", "use STYLE for numbering header lines", "STYLE");
    opts.optopt("i", "line-increment", "line number increment at each line", "");
    opts.optopt("l", "join-blank-lines", "group of NUMBER empty lines counted as one", "NUMBER");
    opts.optopt("n", "number-format", "insert line numbers according to FORMAT", "FORMAT");
    opts.optflag("p", "no-renumber", "do not reset line numbers at logical pages");
    opts.optopt("s", "number-separator", "add STRING after (possible) line number", "STRING");
    opts.optopt("v", "starting-line-number", "first line number on each logical page", "NUMBER");
    opts.optopt("w", "number-width", "use NUMBER columns for line numbers", "NUMBER");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("V", "version", "version");

    // A mutable settings object, initialized with the defaults.
    let mut settings = Settings {
        header_numbering: NumberingStyle::NumberForNone,
        body_numbering: NumberingStyle::NumberForAll,
        footer_numbering: NumberingStyle::NumberForNone,
        section_delimiter: ['\\', ':'],
        starting_line_number: 1,
        line_increment: 1,
        join_blank_lines: 1,
        number_width: 6,
        number_format: NumberFormat::Right,
        renumber: true,
        number_separator: String::from("\t"),
    };

    let given_options = match opts.parse(&args[1..]) {
        Ok (m) => { m }
        Err(f) => {
            show_error!("{}", f);
            print_usage(&opts);
            return 1
        }
    };

    if given_options.opt_present("help") {
        print_usage(&opts);
        return 0;
    }
    if given_options.opt_present("version") { version(); return 0; }

    // Update the settings from the command line options, and terminate the
    // program if some options could not successfully be parsed.
    let parse_errors = helper::parse_options(&mut settings, &given_options);
    if !parse_errors.is_empty() {
        show_error!("Invalid arguments supplied.");
        for message in &parse_errors {
            println!("{}", message);
        }
        return 1;
    }

    let files = given_options.free;
    let mut read_stdin = files.is_empty();

    for file in &files {
        if file == "-" {
            // If both file names and '-' are specified, we choose to treat first all
            // regular files, and then read from stdin last.
            read_stdin = true;
            continue
        }
        let path = Path::new(file);
        let reader = File::open(path).unwrap();
        let mut buffer = BufReader::new(reader);
        nl(&mut buffer, &settings);
    }

    if read_stdin {
        let mut buffer = BufReader::new(stdin());
        nl(&mut buffer, &settings);
    }
    0
}

// nl implements the main functionality for an individual buffer.
fn nl<T: Read> (reader: &mut BufReader<T>, settings: &Settings) {
    let regexp: regex::Regex = regex::Regex::new(r".?").unwrap();
    let mut line_no = settings.starting_line_number;
    // The current line number's width as a string. Using to_string is inefficient
    // but since we only do it once, it should not hurt.
    let mut line_no_width = line_no.to_string().len();
    let line_no_width_initial = line_no_width;
    // Stores the smallest integer with one more digit than line_no, so that
    // when line_no >= line_no_threshold, we need to use one more digit.
    let mut line_no_threshold = 10u64.pow(line_no_width as u32);
    let mut empty_line_count: u64 = 0;
    let fill_char = match settings.number_format {
        NumberFormat::RightZero => '0',
        _ => ' '
    };
    // Initially, we use the body's line counting settings
    let mut regex_filter = match settings.body_numbering {
        NumberingStyle::NumberForRegularExpression(ref re) => re,
        _ => &regexp,
    };
    let mut line_filter : fn(&str, &regex::Regex) -> bool = pass_regex;
    for mut l in reader.lines().map(|r| r.unwrap()) {
        // Sanitize the string. We want to print the newline ourselves.
        if !l.is_empty() && l.chars().rev().next().unwrap() == '\n' {
            l.pop();
        }
        // Next we iterate through the individual chars to see if this
        // is one of the special lines starting a new "section" in the
        // document.
        let line = l;
        let mut odd = false;
        // matched_group counts how many copies of section_delimiter
        // this string consists of (0 if there's anything else)
        let mut matched_groups = 0u8;
        for c in line.chars() {
            // If this is a newline character, the loop should end.
            if c == '\n' {
                break;
            }
            // If we have already seen three groups (corresponding to
            // a header) or the current char does not form part of
            // a new group, then this line is not a segment indicator.
            if matched_groups >= 3
                || settings.section_delimiter[if odd { 1 } else { 0 }] != c {
                matched_groups = 0;
                break;
            }
            if odd {
                // We have seen a new group and count it.
                matched_groups += 1;
            }
            odd = !odd;
        }

        // See how many groups we matched. That will tell us if this is
        // a line starting a new segment, and the number of groups
        // indicates what type of segment.
        if matched_groups > 0 {
            // The current line is a section delimiter, so we output
            // a blank line.
            println!("");
            // However the line does not count as a blank line, so we
            // reset the counter used for --join-blank-lines.
            empty_line_count = 0;
            match *match matched_groups {
                3 => {
                    // This is a header, so we may need to reset the
                    // line number and the line width
                    if settings.renumber {
                        line_no = settings.starting_line_number;
                        line_no_width = line_no_width_initial;
                        line_no_threshold = 10u64.pow(line_no_width as u32);
                    }
                    &settings.header_numbering
                },
                1 => {
                    &settings.footer_numbering
                },
                // The only option left is 2, but rust wants
                // a catch-all here.
                _ => {
                    &settings.body_numbering
                }
            } {
                NumberingStyle::NumberForAll => {
                    line_filter = pass_all;
                },
                NumberingStyle::NumberForNonEmpty => {
                    line_filter = pass_nonempty;
                },
                NumberingStyle::NumberForNone => {
                    line_filter = pass_none;
                }
                NumberingStyle::NumberForRegularExpression(ref re) => {
                    line_filter = pass_regex;
                    regex_filter = re;
                }
            }
            continue;
        }
        // From this point on we format and print a "regular" line.
        if line == "" {
            // The line is empty, which means that we have to care
            // about the --join-blank-lines parameter.
            empty_line_count += 1;
        } else {
            // This saves us from having to check for an empty string
            // in the next selector.
            empty_line_count = 0;
        }
        if !line_filter(&line, regex_filter)
            || ( empty_line_count > 0 && empty_line_count < settings.join_blank_lines) {
            // No number is printed for this line. Either we did not
            // want to print one in the first place, or it is a blank
            // line but we are still collecting more blank lines via
            // the option --join-blank-lines.
            println!("{}", line);
            continue;
        }
        // If we make it here, then either we are printing a non-empty
        // line or assigning a line number to an empty line. Either
        // way, start counting empties from zero once more.
        empty_line_count = 0;
        // A line number is to be printed.
        let mut w: usize = 0;
        if settings.number_width > line_no_width {
            w = settings.number_width - line_no_width;
        }
        let fill: String = repeat(fill_char).take(w).collect();
        match settings.number_format {
            NumberFormat::Left => {
                println!("{1}{0}{2}{3}", fill, line_no, settings.number_separator, line)
            },
            _ => {
                println!("{0}{1}{2}{3}", fill, line_no, settings.number_separator, line)
            }
        }
        // Now update the variables for the (potential) next
        // line.
        line_no += settings.line_increment;
        while line_no >= line_no_threshold {
            // The line number just got longer.
            line_no_threshold *= 10;
            line_no_width += 1;
        }

    }
}

fn pass_regex(line: &str, re: &regex::Regex) -> bool {
    re.is_match(line)
}

fn pass_nonempty(line: &str, _: &regex::Regex) -> bool {
    line.len() > 0
}

fn pass_none(_: &str, _: &regex::Regex) -> bool {
    false
}

fn pass_all(_: &str, _: &regex::Regex) -> bool {
    true
}

fn print_usage(opts: &getopts::Options) {
    println!("{}", opts.usage(USAGE));
}

fn version() {
    println!("{} {}", NAME, VERSION);
}
