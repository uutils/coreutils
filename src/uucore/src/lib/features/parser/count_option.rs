// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Extraction of the `-c`/`-n` values given on the command line.
//!
//! `clap` only keeps the last value of an option that overrides itself, but
//! commands such as `head` and `tail` have to validate every value that was
//! given, even the ones that are overridden later.

use std::ffi::{OsStr, OsString};

/// The count option a value belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountOption {
    Bytes,
    Lines,
}

/// One `-c` or `-n` value as typed on the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountArgument {
    pub option: CountOption,
    pub value: String,
}

/// Collect every literal `-c`/`-n` value, including overridden ones.
///
/// `args` is the raw argument vector, including `argv[0]`. Scanning stops at
/// `--`. Values that cannot be parsed are still returned, so that the caller
/// can report them exactly like `clap` would.
pub fn count_option_arguments(args: &[OsString]) -> Vec<CountArgument> {
    let mut found = Vec::new();
    let mut index = 1;
    while index < args.len() {
        let arg = args[index].to_string_lossy();
        if arg == "--" {
            break;
        }
        if arg == "-c" || arg == "-n" {
            let option = short_option(arg.as_bytes()[1]);
            if let Some(value) = args.get(index + 1) {
                found.push(CountArgument {
                    option,
                    value: value_of(value),
                });
                index += 1;
            }
        } else if let Some(value) = arg.strip_prefix("--bytes=") {
            found.push(CountArgument {
                option: CountOption::Bytes,
                value: value.to_owned(),
            });
        } else if let Some(value) = arg.strip_prefix("--lines=") {
            found.push(CountArgument {
                option: CountOption::Lines,
                value: value.to_owned(),
            });
        } else if arg == "--bytes" || arg == "--lines" {
            let option = if arg == "--bytes" {
                CountOption::Bytes
            } else {
                CountOption::Lines
            };
            if let Some(value) = args.get(index + 1) {
                found.push(CountArgument {
                    option,
                    value: value_of(value),
                });
                index += 1;
            }
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            // A short option cluster such as `-vc1-3`: the value is what
            // follows `c` or `n`. A cluster that starts with a digit is
            // obsolete syntax (`-2cX`) and is handled elsewhere.
            let body = &arg[1..];
            if body.starts_with(|c: char| c.is_ascii_digit()) {
                index += 1;
                continue;
            }
            for (offset, character) in body.char_indices() {
                if character == 'c' || character == 'n' {
                    let option = short_option(character as u8);
                    let rest = &body[offset + character.len_utf8()..];
                    let rest = rest.strip_prefix('=').unwrap_or(rest);
                    if !rest.is_empty() {
                        found.push(CountArgument {
                            option,
                            value: rest.to_owned(),
                        });
                    } else if let Some(value) = args.get(index + 1) {
                        found.push(CountArgument {
                            option,
                            value: value_of(value),
                        });
                        index += 1;
                    }
                    break;
                }
            }
        }
        index += 1;
    }
    found
}

fn value_of(value: &OsStr) -> String {
    value.to_string_lossy().into_owned()
}

fn short_option(byte: u8) -> CountOption {
    if byte == b'c' {
        CountOption::Bytes
    } else {
        CountOption::Lines
    }
}
