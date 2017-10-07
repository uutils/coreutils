#![crate_name = "uu_basename"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::io::Write;
use std::path::{is_separator, PathBuf};

static NAME: &'static str = "basename";
static SYNTAX: &'static str = "NAME [SUFFIX]"; 
static SUMMARY: &'static str = "Print NAME with any leading directory components removed
 If specified, also remove a trailing SUFFIX"; 
static LONG_HELP: &'static str = "";

pub fn uumain(args: Vec<String>) -> i32 {
    //
    // Argument parsing
    //
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("a", "multiple", "Support more than one argument. Treat every argument as a name.")
        .optopt("s", "suffix", "Remove a trailing suffix. This option implies the -a option.", "SUFFIX")
        .optflag("z", "zero", "Output a zero byte (ASCII NUL) at the end of each line, rather than a newline.")
        .parse(args);

    // too few arguments
    if matches.free.len() < 1 {
        crash!(
            1,
            "{0}: {1}\nTry '{0} --help' for more information.",
            NAME,
            "missing operand"
        );
    }
    let opt_s = matches.opt_present("s");
    let opt_a = matches.opt_present("a");
    let opt_z = matches.opt_present("z");
    let multiple_paths = opt_s || opt_a;
    // too many arguments
    if !multiple_paths && matches.free.len() > 2 {
        crash!(
            1,
            "{0}: extra operand '{1}'\nTry '{0} --help' for more information.",
            NAME,
            matches.free[2]
        );
    }

    let suffix = if opt_s {
        matches.opt_str("s").unwrap()
    } else if !opt_a && matches.free.len() > 1 {
        matches.free[1].clone()
    } else {
        "".to_owned()
    };

    //
    // Main Program Processing
    //

    let paths = if multiple_paths {
        &matches.free[..]
    } else {
        &matches.free[0..1]
    };

    let line_ending = if opt_z { "\0" } else { "\n" };
    for path in paths {
        print!("{}{}", basename(&path, &suffix), line_ending);
    }

    0
}

fn basename(fullname: &str, suffix: &str) -> String {
    // Remove all platform-specific path separators from the end
    let mut path: String = fullname.chars().rev().skip_while(|&ch| is_separator(ch)).collect();

    // Undo reverse
    path = path.chars().rev().collect();

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(path);
    match pb.components().last() {
        Some(c) => strip_suffix(c.as_os_str().to_str().unwrap(), suffix),
        None => "".to_owned()
    }
}

fn strip_suffix(name: &str, suffix: &str) -> String {
    if name == suffix {
        return name.to_owned();
    }

    if name.ends_with(suffix) {
        return name[..name.len() - suffix.len()].to_owned();
    }

    name.to_owned()
}
