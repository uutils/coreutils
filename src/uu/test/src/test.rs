// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) egid euid FiletestOp StrlenOp

pub(crate) mod error;
mod parser;

use clap::{crate_version, Command};
use error::{ParseError, ParseResult};
use parser::{parse, Operator, Symbol, UnaryOperator};
use std::ffi::{OsStr, OsString};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
#[cfg(not(windows))]
use uucore::process::{getegid, geteuid};
use uucore::{format_usage, help_about, help_section};

const ABOUT: &str = help_about!("test.md");

// The help_usage method replaces util name (the first word) with {}.
// And, The format_usage method replaces {} with execution_phrase ( e.g. test or [ ).
// However, This test command has two util names.
// So, we use test or [ instead of {} so that the usage string is correct.
const USAGE: &str = "\
test EXPRESSION
[
[ EXPRESSION ]
[ ]
[ OPTION
]";

// We use after_help so that this comes after the usage string (it would come before if we used about)
const AFTER_HELP: &str = help_section!("after help", "test.md");

pub fn uu_app() -> Command {
    // Disable printing of -h and -v as valid alternatives for --help and --version,
    // since we don't recognize -h and -v as help/version flags.
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
}

#[uucore::main]
pub fn uumain(mut args: impl uucore::Args) -> UResult<()> {
    let program = args.next().unwrap_or_else(|| OsString::from("test"));
    let binary_name = uucore::util_name();
    let mut args: Vec<_> = args.collect();

    if binary_name.ends_with('[') {
        // If invoked as [ we should recognize --help and --version (but not -h or -v)
        if args.len() == 1 && (args[0] == "--help" || args[0] == "--version") {
            uu_app().get_matches_from(std::iter::once(program).chain(args.into_iter()));
            return Ok(());
        }
        // If invoked via name '[', matching ']' must be in the last arg
        let last = args.pop();
        if last.as_deref() != Some(OsStr::new("]")) {
            return Err(USimpleError::new(2, "missing ']'"));
        }
    }

    let result = parse(args).map(|mut stack| eval(&mut stack))??;

    if result {
        Ok(())
    } else {
        Err(1.into())
    }
}

/// Evaluate a stack of Symbols, returning the result of the evaluation or
/// an error message if evaluation failed.
fn eval(stack: &mut Vec<Symbol>) -> ParseResult<bool> {
    macro_rules! pop_literal {
        () => {
            match stack.pop() {
                Some(Symbol::Literal(s)) => s,
                _ => panic!(),
            }
        };
    }

    let s = stack.pop();

    match s {
        Some(Symbol::Bang) => {
            let result = eval(stack)?;

            Ok(!result)
        }
        Some(Symbol::Op(Operator::String(op))) => {
            let b = pop_literal!();
            let a = pop_literal!();
            match op.to_string_lossy().as_ref() {
                "!=" => Ok(a != b),
                "<" => Ok(a < b),
                ">" => Ok(a > b),
                _ => Ok(a == b),
            }
        }
        Some(Symbol::Op(Operator::Int(op))) => {
            let b = pop_literal!();
            let a = pop_literal!();

            Ok(integers(&a, &b, &op)?)
        }
        Some(Symbol::Op(Operator::File(op))) => {
            let b = pop_literal!();
            let a = pop_literal!();
            Ok(files(&a, &b, &op)?)
        }
        Some(Symbol::UnaryOp(UnaryOperator::StrlenOp(op))) => {
            let s = match stack.pop() {
                Some(Symbol::Literal(s)) => s,
                Some(Symbol::None) => OsString::from(""),
                None => {
                    return Ok(true);
                }
                _ => {
                    return Err(ParseError::MissingArgument(op.quote().to_string()));
                }
            };

            Ok(if op == "-z" {
                s.is_empty()
            } else {
                !s.is_empty()
            })
        }
        Some(Symbol::UnaryOp(UnaryOperator::FiletestOp(op))) => {
            let op = op.to_str().unwrap();

            let f = pop_literal!();

            Ok(match op {
                "-b" => path(&f, &PathCondition::BlockSpecial),
                "-c" => path(&f, &PathCondition::CharacterSpecial),
                "-d" => path(&f, &PathCondition::Directory),
                "-e" => path(&f, &PathCondition::Exists),
                "-f" => path(&f, &PathCondition::Regular),
                "-g" => path(&f, &PathCondition::GroupIdFlag),
                "-G" => path(&f, &PathCondition::GroupOwns),
                "-h" => path(&f, &PathCondition::SymLink),
                "-k" => path(&f, &PathCondition::Sticky),
                "-L" => path(&f, &PathCondition::SymLink),
                "-N" => path(&f, &PathCondition::ExistsModifiedLastRead),
                "-O" => path(&f, &PathCondition::UserOwns),
                "-p" => path(&f, &PathCondition::Fifo),
                "-r" => path(&f, &PathCondition::Readable),
                "-S" => path(&f, &PathCondition::Socket),
                "-s" => path(&f, &PathCondition::NonEmpty),
                "-t" => isatty(&f)?,
                "-u" => path(&f, &PathCondition::UserIdFlag),
                "-w" => path(&f, &PathCondition::Writable),
                "-x" => path(&f, &PathCondition::Executable),
                _ => panic!(),
            })
        }
        Some(Symbol::Literal(s)) => Ok(!s.is_empty()),
        Some(Symbol::None) | None => Ok(false),
        Some(Symbol::BoolOp(op)) => {
            if (op == "-a" || op == "-o") && stack.len() < 2 {
                return Err(ParseError::UnaryOperatorExpected(op.quote().to_string()));
            }

            let b = eval(stack)?;
            let a = eval(stack)?;

            Ok(if op == "-a" { a && b } else { a || b })
        }
        _ => Err(ParseError::ExpectedValue),
    }
}

/// Operations to compare integers
/// `a` is the left hand side
/// `b` is the left hand side
/// `op` the operation (ex: -eq, -lt, etc)
fn integers(a: &OsStr, b: &OsStr, op: &OsStr) -> ParseResult<bool> {
    // Parse the two inputs
    let a: i128 = a
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ParseError::InvalidInteger(a.quote().to_string()))?;

    let b: i128 = b
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ParseError::InvalidInteger(b.quote().to_string()))?;

    // Do the maths
    Ok(match op.to_str() {
        Some("-eq") => a == b,
        Some("-ne") => a != b,
        Some("-gt") => a > b,
        Some("-ge") => a >= b,
        Some("-lt") => a < b,
        Some("-le") => a <= b,
        _ => return Err(ParseError::UnknownOperator(op.quote().to_string())),
    })
}

/// Operations to compare files metadata
/// `a` is the left hand side
/// `b` is the left hand side
/// `op` the operation (ex: -ef, -nt, etc)
fn files(a: &OsStr, b: &OsStr, op: &OsStr) -> ParseResult<bool> {
    // Don't manage the error. GNU doesn't show error when doing
    // test foo -nt bar
    let (Ok(f_a), Ok(f_b)) = (fs::metadata(a), fs::metadata(b)) else {
        return Ok(false);
    };

    Ok(match op.to_str() {
        #[cfg(unix)]
        Some("-ef") => f_a.ino() == f_b.ino() && f_a.dev() == f_b.dev(),
        #[cfg(not(unix))]
        Some("-ef") => unimplemented!(),
        Some("-nt") => f_a.modified().unwrap() > f_b.modified().unwrap(),
        Some("-ot") => f_a.modified().unwrap() < f_b.modified().unwrap(),
        _ => return Err(ParseError::UnknownOperator(op.quote().to_string())),
    })
}

fn isatty(fd: &OsStr) -> ParseResult<bool> {
    fd.to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ParseError::InvalidInteger(fd.quote().to_string()))
        .map(|i| unsafe { libc::isatty(i) == 1 })
}

#[derive(Eq, PartialEq)]
enum PathCondition {
    BlockSpecial,
    CharacterSpecial,
    Directory,
    Exists,
    ExistsModifiedLastRead,
    Regular,
    GroupIdFlag,
    GroupOwns,
    SymLink,
    Sticky,
    UserOwns,
    Fifo,
    Readable,
    Socket,
    NonEmpty,
    UserIdFlag,
    Writable,
    Executable,
}

#[cfg(not(windows))]
fn path(path: &OsStr, condition: &PathCondition) -> bool {
    use std::fs::Metadata;
    use std::os::unix::fs::FileTypeExt;

    const S_ISUID: u32 = 0o4000;
    const S_ISGID: u32 = 0o2000;
    const S_ISVTX: u32 = 0o1000;

    enum Permission {
        Read = 0o4,
        Write = 0o2,
        Execute = 0o1,
    }

    let perm = |metadata: Metadata, p: Permission| {
        if geteuid() == metadata.uid() {
            metadata.mode() & ((p as u32) << 6) != 0
        } else if getegid() == metadata.gid() {
            metadata.mode() & ((p as u32) << 3) != 0
        } else {
            metadata.mode() & (p as u32) != 0
        }
    };

    let metadata = if condition == &PathCondition::SymLink {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    };

    let Ok(metadata) = metadata else {
        return false;
    };

    let file_type = metadata.file_type();

    match condition {
        PathCondition::BlockSpecial => file_type.is_block_device(),
        PathCondition::CharacterSpecial => file_type.is_char_device(),
        PathCondition::Directory => file_type.is_dir(),
        PathCondition::Exists => true,
        PathCondition::ExistsModifiedLastRead => {
            metadata.accessed().unwrap() < metadata.modified().unwrap()
        }
        PathCondition::Regular => file_type.is_file(),
        PathCondition::GroupIdFlag => metadata.mode() & S_ISGID != 0,
        PathCondition::GroupOwns => metadata.gid() == getegid(),
        PathCondition::SymLink => metadata.file_type().is_symlink(),
        PathCondition::Sticky => metadata.mode() & S_ISVTX != 0,
        PathCondition::UserOwns => metadata.uid() == geteuid(),
        PathCondition::Fifo => file_type.is_fifo(),
        PathCondition::Readable => perm(metadata, Permission::Read),
        PathCondition::Socket => file_type.is_socket(),
        PathCondition::NonEmpty => metadata.size() > 0,
        PathCondition::UserIdFlag => metadata.mode() & S_ISUID != 0,
        PathCondition::Writable => perm(metadata, Permission::Write),
        PathCondition::Executable => perm(metadata, Permission::Execute),
    }
}

#[cfg(windows)]
fn path(path: &OsStr, condition: &PathCondition) -> bool {
    use std::fs::metadata;

    let stat = match metadata(path) {
        Ok(s) => s,
        _ => return false,
    };

    match condition {
        PathCondition::BlockSpecial => false,
        PathCondition::CharacterSpecial => false,
        PathCondition::Directory => stat.is_dir(),
        PathCondition::Exists => true,
        PathCondition::ExistsModifiedLastRead => unimplemented!(),
        PathCondition::Regular => stat.is_file(),
        PathCondition::GroupIdFlag => false,
        PathCondition::GroupOwns => unimplemented!(),
        PathCondition::SymLink => false,
        PathCondition::Sticky => false,
        PathCondition::UserOwns => unimplemented!(),
        PathCondition::Fifo => false,
        PathCondition::Readable => false, // TODO
        PathCondition::Socket => false,
        PathCondition::NonEmpty => stat.len() > 0,
        PathCondition::UserIdFlag => false,
        PathCondition::Writable => false,   // TODO
        PathCondition::Executable => false, // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::integers;
    use std::ffi::OsStr;

    #[test]
    fn test_integer_op() {
        let a = OsStr::new("18446744073709551616");
        let b = OsStr::new("0");
        assert!(!integers(a, b, OsStr::new("-lt")).unwrap());
        let a = OsStr::new("18446744073709551616");
        let b = OsStr::new("0");
        assert!(integers(a, b, OsStr::new("-gt")).unwrap());
        let a = OsStr::new("-1");
        let b = OsStr::new("0");
        assert!(integers(a, b, OsStr::new("-lt")).unwrap());
        let a = OsStr::new("42");
        let b = OsStr::new("42");
        assert!(integers(a, b, OsStr::new("-eq")).unwrap());
        let a = OsStr::new("42");
        let b = OsStr::new("42");
        assert!(!integers(a, b, OsStr::new("-ne")).unwrap());
    }
}
