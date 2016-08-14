#![crate_name = "uu_basename"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate libc;

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
    // too many arguments
    else if matches.free.len() > 2 {
        crash!(
            1,
            "{0}: extra operand '{1}'\nTry '{0} --help' for more information.",
            NAME,
            matches.free[2]
        );
    }

    //
    // Main Program Processing
    //

    let mut name = strip_dir(&matches.free[0]);

    if matches.free.len() > 1 {
        let suffix = matches.free[1].clone();
        name = strip_suffix(name.as_ref(), suffix.as_ref());
    }

    println!("{}", name);

    0
}

fn strip_dir(fullname: &str) -> String {
    // Remove all platform-specific path separators from the end
    let mut path: String = fullname.chars().rev().skip_while(|&ch| is_separator(ch)).collect();

    // Undo reverse
    path = path.chars().rev().collect();

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(path);
    match pb.components().last() {
        Some(c) => c.as_os_str().to_str().unwrap().to_owned(),
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
