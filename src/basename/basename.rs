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
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;

use std::io::{self, Read, Write};
use std::path::{is_separator, PathBuf};
use uucore::{ProgramInfo, UStatus, Util};

static NAME: &'static str = "basename";
static SYNTAX: &'static str = "NAME [SUFFIX]";
static SUMMARY: &'static str = "Print NAME with any leading directory components removed
 If specified, also remove a trailing SUFFIX";
static LONG_HELP: &'static str = "";

pub const UTILITY: Basename = Basename;

pub struct Basename;

impl<'a, I: Read, O: Write, E: Write> Util<'a, I, O, E, Error> for Basename {
    fn uumain(args: Vec<String>, pio: &mut ProgramInfo<I, O, E>) -> Result<i32, Error> {
        //
        // Argument parsing
        //
        let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
            .optflag("a", "multiple", "Support more than one argument. Treat every argument as a name.")
            .optopt("s", "suffix", "Remove a trailing suffix. This option implies the -a option.", "SUFFIX")
            .optflag("z", "zero", "Output a zero byte (ASCII NUL) at the end of each line, rather than a newline.")
            .parse(args, pio)?;

        // too few arguments
        let matches = match matches {
            None => return Ok(0),
            Some(ref matches) if matches.free.len() < 1 => {
                Err(format_err!(
                    "{1}\nTry '{0} --help' for more information.",
                    NAME,
                    "missing operand"
                ))?;
                unreachable!()
            }
            Some(matches) => matches
        };

        let opt_s = matches.opt_present("s");
        let opt_a = matches.opt_present("a");
        let opt_z = matches.opt_present("z");
        let multiple_paths = opt_s || opt_a;
        // too many arguments
        if !multiple_paths && matches.free.len() > 2 {
            Err(format_err!(
                "extra operand '{1}'\nTry '{0} --help' for more information.",
                NAME,
                matches.free[2]
            ))?;
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
            write!(pio, "{}{}", basename(&path, &suffix), line_ending)?;
        }

        Ok(0)
    }
}

fn basename(fullname: &str, suffix: &str) -> String {
    // Remove all platform-specific path separators from the end
    // TODO: remove allocations (should be possible given that we are only making the path smaller)
    let mut path: String = fullname.chars().rev().skip_while(|&ch| is_separator(ch)).collect();

    // Undo reverse
    path = path.chars().rev().collect();

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(path);
    match pb.components().last() {
        Some(c) => strip_suffix(c.as_os_str().to_str().unwrap(), suffix).to_owned(),
        None => "".to_owned()
    }
}

fn strip_suffix<'a>(name: &'a str, suffix: &str) -> &'a str {
    if name == suffix || !name.ends_with(suffix) {
        name
    } else {
        &name[..name.len() - suffix.len()]
    }
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "{}", _0)]
    CoreOpts(#[cause] uucore::coreopts::Error),

    #[fail(display = "{}", _0)]
    General(failure::Error)
}

impl UStatus for Error { }

generate_from_impl!(Error, Io, io::Error);
generate_from_impl!(Error, CoreOpts, uucore::coreopts::Error);
generate_from_impl!(Error, General, failure::Error);