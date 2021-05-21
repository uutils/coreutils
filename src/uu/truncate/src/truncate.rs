//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) RFILE refsize rfilename fsize tsize

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs::{metadata, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;

#[derive(Eq, PartialEq)]
enum TruncateMode {
    Absolute,
    Reference,
    Extend,
    Reduce,
    AtMost,
    AtLeast,
    RoundDown,
    RoundUp,
}

static ABOUT: &str = "Shrink or extend the size of each file to the specified size.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod options {
    pub static IO_BLOCKS: &str = "io-blocks";
    pub static NO_CREATE: &str = "no-create";
    pub static REFERENCE: &str = "reference";
    pub static SIZE: &str = "size";
}

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

fn get_long_usage() -> String {
    String::from(
        "
    SIZE is an integer with an optional prefix and optional unit.
    The available units (K, M, G, T, P, E, Z, and Y) use the following format:
        'KB' =>           1000 (kilobytes)
        'K'  =>           1024 (kibibytes)
        'MB' =>      1000*1000 (megabytes)
        'M'  =>      1024*1024 (mebibytes)
        'GB' => 1000*1000*1000 (gigabytes)
        'G'  => 1024*1024*1024 (gibibytes)
    SIZE may also be prefixed by one of the following to adjust the size of each
    file based on its current size:
        '+'  => extend by
        '-'  => reduce by
        '<'  => at most
        '>'  => at least
        '/'  => round down to multiple of
        '%'  => round up to multiple of",
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&long_usage[..])
        .arg(
            Arg::with_name(options::IO_BLOCKS)
            .short("o")
            .long(options::IO_BLOCKS)
            .help("treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)")
        )
        .arg(
            Arg::with_name(options::NO_CREATE)
            .short("c")
            .long(options::NO_CREATE)
            .help("do not create files that do not exist")
        )
        .arg(
            Arg::with_name(options::REFERENCE)
            .short("r")
            .long(options::REFERENCE)
            .help("base the size of each file on the size of RFILE")
            .value_name("RFILE")
        )
        .arg(
            Arg::with_name(options::SIZE)
            .short("s")
            .long("size")
            .help("set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified")
            .value_name("SIZE")
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true).min_values(1))
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        show_error!("Missing an argument");
        return 1;
    } else {
        let io_blocks = matches.is_present(options::IO_BLOCKS);
        let no_create = matches.is_present(options::NO_CREATE);
        let reference = matches.value_of(options::REFERENCE).map(String::from);
        let size = matches.value_of(options::SIZE).map(String::from);
        if reference.is_none() && size.is_none() {
            crash!(1, "you must specify either --reference or --size");
        } else {
            truncate(no_create, io_blocks, reference, size, files);
        }
    }

    0
}

fn truncate(
    no_create: bool,
    _: bool,
    reference: Option<String>,
    size: Option<String>,
    filenames: Vec<String>,
) {
    let (modsize, mode) = match size {
        Some(size_string) => {
            // Trim any whitespace.
            let size_string = size_string.trim();

            // Get the modifier character from the size string, if any. For
            // example, if the argument is "+123", then the modifier is '+'.
            let c = size_string.chars().next().unwrap();

            let mode = match c {
                '+' => TruncateMode::Extend,
                '-' => TruncateMode::Reduce,
                '<' => TruncateMode::AtMost,
                '>' => TruncateMode::AtLeast,
                '/' => TruncateMode::RoundDown,
                '*' => TruncateMode::RoundUp,
                _ => TruncateMode::Absolute, /* assume that the size is just a number */
            };

            // If there was a modifier character, strip it.
            let size_string = match mode {
                TruncateMode::Absolute => size_string,
                _ => &size_string[1..],
            };
            let num_bytes = match parse_size(size_string) {
                Ok(b) => b,
                Err(_) => crash!(1, "Invalid number: ‘{}’", size_string),
            };
            (num_bytes, mode)
        }
        None => (0, TruncateMode::Reference),
    };

    let refsize = match reference {
        Some(ref rfilename) => {
            match mode {
                // Only Some modes work with a reference
                TruncateMode::Reference => (), //No --size was given
                TruncateMode::Extend => (),
                TruncateMode::Reduce => (),
                _ => crash!(1, "you must specify a relative ‘--size’ with ‘--reference’"),
            };
            match metadata(rfilename) {
                Ok(meta) => meta.len(),
                Err(f) => match f.kind() {
                    ErrorKind::NotFound => {
                        crash!(1, "cannot stat '{}': No such file or directory", rfilename)
                    }
                    _ => crash!(1, "{}", f.to_string()),
                },
            }
        }
        None => 0,
    };
    for filename in &filenames {
        let path = Path::new(filename);
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(!no_create)
            .open(path)
        {
            Ok(file) => {
                let fsize = match reference {
                    Some(_) => refsize,
                    None => match metadata(filename) {
                        Ok(meta) => meta.len(),
                        Err(f) => {
                            show_warning!("{}", f.to_string());
                            continue;
                        }
                    },
                };
                let tsize: u64 = match mode {
                    TruncateMode::Absolute => modsize,
                    TruncateMode::Reference => fsize,
                    TruncateMode::Extend => fsize + modsize,
                    TruncateMode::Reduce => fsize - modsize,
                    TruncateMode::AtMost => {
                        if fsize > modsize {
                            modsize
                        } else {
                            fsize
                        }
                    }
                    TruncateMode::AtLeast => {
                        if fsize < modsize {
                            modsize
                        } else {
                            fsize
                        }
                    }
                    TruncateMode::RoundDown => fsize - fsize % modsize,
                    TruncateMode::RoundUp => fsize + fsize % modsize,
                };
                match file.set_len(tsize) {
                    Ok(_) => {}
                    Err(f) => crash!(1, "{}", f.to_string()),
                };
            }
            Err(f) => crash!(1, "{}", f.to_string()),
        }
    }
}

/// Parse a size string into a number of bytes.
///
/// A size string comprises an integer and an optional unit. The unit
/// may be K, M, G, T, P, E, Z, or Y (powers of 1024) or KB, MB,
/// etc. (powers of 1000).
///
/// # Errors
///
/// This function returns an error if the string does not begin with a
/// numeral, or if the unit is not one of the supported units described
/// in the preceding section.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_size("123").unwrap(), 123);
/// assert_eq!(parse_size("123K").unwrap(), 123 * 1024);
/// assert_eq!(parse_size("123KB").unwrap(), 123 * 1000);
/// ```
fn parse_size(size: &str) -> Result<u64, ()> {
    // Get the numeric part of the size argument. For example, if the
    // argument is "123K", then the numeric part is "123".
    let numeric_string: String = size.chars().take_while(|c| c.is_digit(10)).collect();
    let number: u64 = match numeric_string.parse() {
        Ok(n) => n,
        Err(_) => return Err(()),
    };

    // Get the alphabetic units part of the size argument and compute
    // the factor it represents. For example, if the argument is "123K",
    // then the unit part is "K" and the factor is 1024. This may be the
    // empty string, in which case, the factor is 1.
    let n = numeric_string.len();
    let (base, exponent): (u64, u32) = match &size[n..] {
        "" => (1, 0),
        "K" | "k" => (1024, 1),
        "M" | "m" => (1024, 2),
        "G" | "g" => (1024, 3),
        "T" | "t" => (1024, 4),
        "P" | "p" => (1024, 5),
        "E" | "e" => (1024, 6),
        "Z" | "z" => (1024, 7),
        "Y" | "y" => (1024, 8),
        "KB" | "kB" => (1000, 1),
        "MB" | "mB" => (1000, 2),
        "GB" | "gB" => (1000, 3),
        "TB" | "tB" => (1000, 4),
        "PB" | "pB" => (1000, 5),
        "EB" | "eB" => (1000, 6),
        "ZB" | "zB" => (1000, 7),
        "YB" | "yB" => (1000, 8),
        _ => return Err(()),
    };
    let factor = base.pow(exponent);
    Ok(number * factor)
}

#[cfg(test)]
mod tests {
    use crate::parse_size;

    #[test]
    fn test_parse_size_zero() {
        assert_eq!(parse_size("0").unwrap(), 0);
        assert_eq!(parse_size("0K").unwrap(), 0);
        assert_eq!(parse_size("0KB").unwrap(), 0);
    }

    #[test]
    fn test_parse_size_without_factor() {
        assert_eq!(parse_size("123").unwrap(), 123);
    }

    #[test]
    fn test_parse_size_kilobytes() {
        assert_eq!(parse_size("123K").unwrap(), 123 * 1024);
        assert_eq!(parse_size("123KB").unwrap(), 123 * 1000);
    }

    #[test]
    fn test_parse_size_megabytes() {
        assert_eq!(parse_size("123").unwrap(), 123);
        assert_eq!(parse_size("123M").unwrap(), 123 * 1024 * 1024);
        assert_eq!(parse_size("123MB").unwrap(), 123 * 1000 * 1000);
    }
}
