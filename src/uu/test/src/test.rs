// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) egid euid FiletestOp StrlenOp

mod diagnostics;
pub(crate) mod error;
mod parser;
#[cfg(any(windows, target_os = "wasi"))]
mod platform;

use clap::Command;
use error::{ParseError, ParseErrorKind, ParseResult};
use parser::{Operator, Symbol, UnaryOperator, parse};
#[cfg(windows)]
use platform::fd_is_terminal;
#[cfg(target_os = "wasi")]
use platform::path;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
#[cfg(not(any(windows, target_os = "wasi")))]
use uucore::process::{getegid, geteuid};

use uucore::translate;

// The help_usage method replaces util name (the first word) with {}.
// And, The format_usage method replaces {} with execution_phrase ( e.g. test or [ ).
// However, This test command has two util names.
// So, we use test or [ instead of {} so that the usage string is correct.

// We use after_help so that this comes after the usage string (it would come before if we used about)

pub fn uu_app() -> Command {
    // Disable printing of -h and -v as valid alternatives for --help and --version,
    // since we don't recognize -h and -v as help/version flags.
    // We change the name to test later
    Command::new("[")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("test-about"))
        .override_usage(format_usage(&translate!("test-usage")))
        .after_help(translate!("test-after-help"))
}

#[uucore::main(no_signals)]
pub fn uumain(mut args: impl uucore::Args) -> UResult<()> {
    let program = args.next().unwrap_or_else(|| OsString::from("test"));
    let binary_name = uucore::util_name();
    let mut args: Vec<_> = args.collect();

    if binary_name.ends_with('[') {
        // If invoked as [ we should recognize --help and --version (but not -h or -v)
        if args.len() == 1 && (args[0] == "--help" || args[0] == "--version") {
            uucore::clap_localization::handle_clap_result(
                uu_app(),
                std::iter::once(program).chain(args.into_iter()),
            )?;
            return Ok(());
        }
        // If invoked via name '[', matching ']' must be in the last arg
        let last = args.pop();
        if last.as_deref() != Some(OsStr::new("]")) {
            return Err(USimpleError::new(
                2,
                translate!("test-error-missing-closing-bracket"),
            ));
        }
    } else {
        // Show actual name with error
        let _ = uu_app().name("test");
    }
    // `parse` consumes the arguments, so keep a copy for the diagnostic — but
    // only when one could actually be rendered.
    let expression = uucore::diagnostics::capture(&args);

    match parse(args).and_then(|mut stack| eval(&mut stack)) {
        Ok(true) => Ok(()),
        Ok(false) => Err(1.into()),
        Err(e) => Err(uucore::diagnostics::error_after_report(
            expression.as_deref(),
            e,
            diagnostics::render,
        )),
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
            match op.as_encoded_bytes() {
                b"!=" => Ok(a != b),
                b"<" => Ok(a < b),
                b">" => Ok(a > b),
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
                None => return Ok(true),
                _ => {
                    return Err(ParseError::at_value(
                        ParseErrorKind::MissingArgument(op.quote().to_string()),
                        &op,
                    ));
                }
            };

            Ok((op == "-z") == s.is_empty())
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
                "-h" | "-L" => path(&f, &PathCondition::SymLink),
                "-k" => path(&f, &PathCondition::Sticky),
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
                return Err(ParseError::at_value(
                    ParseErrorKind::UnaryOperatorExpected(op.quote().to_string()),
                    &op,
                ));
            }

            let b = eval(stack)?;
            let a = eval(stack)?;

            Ok(if op == "-a" { a && b } else { a || b })
        }
        _ => Err(ParseErrorKind::ExpectedValue.into()),
    }
}

/// An integer operand of a comparison, split into a sign and its decimal digits.
///
/// Keeping the digits as text instead of converting them to a fixed-width
/// integer is what lets operands of any length be compared, matching GNU, which
/// places no limit on the width of the integers `test` accepts.
#[derive(Debug, PartialEq, Eq)]
struct Integer<'a> {
    negative: bool,
    /// The digits without leading zeros. Empty when the value is zero.
    digits: &'a str,
}

impl<'a> Integer<'a> {
    /// Parse an operand of the form `[+-]?[0-9]+`, surrounded by optional
    /// whitespace, returning [`None`] when it has any other shape.
    fn parse(value: &'a OsStr) -> Option<Self> {
        let value = value.to_str()?.trim();

        // Only ASCII `+`/`-` are sliced off, so this always cuts on a char boundary.
        let (negative, digits) = match value.as_bytes().first()? {
            b'-' => (true, &value[1..]),
            b'+' => (false, &value[1..]),
            _ => (false, value),
        };

        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }

        let digits = digits.trim_start_matches('0');

        // Zero is neither positive nor negative, so `-0` compares equal to `0`.
        Some(Self {
            negative: negative && !digits.is_empty(),
            digits,
        })
    }
}

impl Ord for Integer<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (negative, _) => {
                // Leading zeros are already gone, so the longer run of digits is
                // the larger magnitude and equal-length runs order bytewise.
                let magnitude = self
                    .digits
                    .len()
                    .cmp(&other.digits.len())
                    .then_with(|| self.digits.cmp(other.digits));

                if negative {
                    magnitude.reverse()
                } else {
                    magnitude
                }
            }
        }
    }
}

impl PartialOrd for Integer<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Operations to compare integers
/// `a` is the left hand side
/// `b` is the right hand side
/// `op` the operation (ex: -eq, -lt, etc)
fn integers(a: &OsStr, b: &OsStr, op: &OsStr) -> ParseResult<bool> {
    // Parse the two inputs
    let left = Integer::parse(a).ok_or_else(|| {
        ParseError::at_value(ParseErrorKind::InvalidInteger(a.quote().to_string()), a)
    })?;
    let right = Integer::parse(b).ok_or_else(|| {
        ParseError::at_value(ParseErrorKind::InvalidInteger(b.quote().to_string()), b)
    })?;

    // Do the maths
    let order = left.cmp(&right);

    Ok(match op.to_str() {
        Some("-eq") => order.is_eq(),
        Some("-ne") => order.is_ne(),
        Some("-gt") => order.is_gt(),
        Some("-ge") => order.is_ge(),
        Some("-lt") => order.is_lt(),
        Some("-le") => order.is_le(),
        _ => {
            return Err(ParseError::at_value(
                ParseErrorKind::UnknownOperator(op.quote().to_string()),
                op,
            ));
        }
    })
}

/// Operations to compare files metadata
/// `a` is the left hand side
/// `b` is the right hand side
/// `op` the operation (ex: -ef, -nt, etc)
fn files(a: &OsStr, b: &OsStr, op: &OsStr) -> ParseResult<bool> {
    let f_a = fs::metadata(a);
    let f_b = fs::metadata(b);

    let result = match (op.to_str(), f_a, f_b) {
        #[cfg(unix)]
        (Some("-ef"), Ok(f_a), Ok(f_b)) => f_a.ino() == f_b.ino() && f_a.dev() == f_b.dev(),
        #[cfg(any(windows, target_os = "wasi"))]
        (Some("-ef"), Ok(_), Ok(_)) => platform::same_file(a, b),
        (Some("-nt"), Ok(f_a), Ok(f_b)) => f_a.modified().unwrap() > f_b.modified().unwrap(),
        (Some("-nt"), Ok(_), _) => true,
        (Some("-ot"), Ok(f_a), Ok(f_b)) => f_a.modified().unwrap() < f_b.modified().unwrap(),
        (Some("-ot"), _, Ok(_)) => true,
        (Some("-ef" | "-nt" | "-ot"), _, _) => false,
        (_, _, _) => {
            return Err(ParseError::at_value(
                ParseErrorKind::UnknownOperator(op.quote().to_string()),
                op,
            ));
        }
    };

    Ok(result)
}

fn isatty(fd: &OsStr) -> ParseResult<bool> {
    fd.to_str()
        .map(str::trim)
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| {
            ParseError::at_value(
                ParseErrorKind::InvalidFileDescriptor(fd.quote().to_string()),
                fd,
            )
        })
        .map(fd_is_terminal)
}

#[cfg(not(windows))]
fn fd_is_terminal(fd: i32) -> bool {
    // SAFETY: isatty only inspects the descriptor number it is given.
    unsafe { libc::isatty(fd) == 1 }
}

#[derive(Eq, PartialEq)]
pub(crate) enum PathCondition {
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

/// Whether the file was modified more recently than it was last read, the
/// condition behind `-N`. A timestamp the platform cannot report counts as
/// "not modified since read" rather than aborting.
pub(crate) fn modified_since_read(metadata: &fs::Metadata) -> bool {
    matches!(
        (metadata.accessed(), metadata.modified()),
        (Ok(read), Ok(modified)) if read < modified
    )
}

#[cfg(not(any(windows, target_os = "wasi")))]
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
        PathCondition::ExistsModifiedLastRead => modified_since_read(&metadata),
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
    use crate::platform::{is_executable, is_readable, is_writable, owned_by_current_token};

    let metadata = if condition == &PathCondition::SymLink {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    };

    let Ok(metadata) = metadata else {
        return false;
    };

    match condition {
        PathCondition::Directory => metadata.is_dir(),
        PathCondition::Exists => true,
        PathCondition::ExistsModifiedLastRead => modified_since_read(&metadata),
        PathCondition::GroupOwns => owned_by_current_token(path, true),
        PathCondition::UserOwns => owned_by_current_token(path, false),
        PathCondition::Regular => metadata.is_file(),
        PathCondition::SymLink => metadata.file_type().is_symlink(),
        PathCondition::NonEmpty => metadata.len() > 0,
        PathCondition::Readable => is_readable(path),
        PathCondition::Writable => is_writable(path, &metadata),
        PathCondition::Executable => is_executable(path, &metadata),
        PathCondition::BlockSpecial
        | PathCondition::CharacterSpecial
        | PathCondition::Fifo
        | PathCondition::GroupIdFlag
        | PathCondition::Socket
        | PathCondition::Sticky
        | PathCondition::UserIdFlag => false,
    }
}

// Every test here needs a temporary file, and a WASI guest only sees the
// directories it was granted, so there is no temporary directory to use.
#[cfg(all(test, not(target_os = "wasi")))]
mod tests {
    use super::*;
    use std::{ffi::OsStr, time::UNIX_EPOCH};
    use tempfile::NamedTempFile;

    #[test]
    fn test_files_with_unknown_op() {
        let a = NamedTempFile::new().unwrap();
        let b = NamedTempFile::new().unwrap();
        let a = OsStr::new(a.path());
        let b = OsStr::new(b.path());
        let op = OsStr::new("unknown_op");

        assert!(files(a, b, op).is_err());
    }

    #[test]
    fn test_files_with_ef_op() {
        let a = NamedTempFile::new().unwrap();
        let b = NamedTempFile::new().unwrap();
        let a = OsStr::new(a.path());
        let b = OsStr::new(b.path());
        let op = OsStr::new("-ef");

        assert!(files(a, a, op).unwrap());
        assert!(!files(a, b, op).unwrap());
        assert!(!files(b, a, op).unwrap());

        let existing_file = a;
        let non_existing_file = OsStr::new("non_existing_file");

        assert!(!files(existing_file, non_existing_file, op).unwrap());
        assert!(!files(non_existing_file, existing_file, op).unwrap());
        assert!(!files(non_existing_file, non_existing_file, op).unwrap());
    }

    #[test]
    fn test_files_with_nt_op() {
        let older_file = NamedTempFile::new().unwrap();
        older_file.as_file().set_modified(UNIX_EPOCH).unwrap();
        let older_file = OsStr::new(older_file.path());
        let newer_file = NamedTempFile::new().unwrap();
        let newer_file = OsStr::new(newer_file.path());
        let op = OsStr::new("-nt");

        assert!(files(newer_file, older_file, op).unwrap());
        assert!(!files(older_file, newer_file, op).unwrap());

        let existing_file = newer_file;
        let non_existing_file = OsStr::new("non_existing_file");

        assert!(files(existing_file, non_existing_file, op).unwrap());
        assert!(!files(non_existing_file, existing_file, op).unwrap());
        assert!(!files(non_existing_file, non_existing_file, op).unwrap());
    }

    #[test]
    fn test_files_with_ot_op() {
        let older_file = NamedTempFile::new().unwrap();
        older_file.as_file().set_modified(UNIX_EPOCH).unwrap();
        let older_file = OsStr::new(older_file.path());
        let newer_file = NamedTempFile::new().unwrap();
        let newer_file = OsStr::new(newer_file.path());
        let op = OsStr::new("-ot");

        assert!(!files(newer_file, older_file, op).unwrap());
        assert!(files(older_file, newer_file, op).unwrap());

        let existing_file = newer_file;
        let non_existing_file = OsStr::new("non_existing_file");

        assert!(!files(existing_file, non_existing_file, op).unwrap());
        assert!(files(non_existing_file, existing_file, op).unwrap());
        assert!(!files(non_existing_file, non_existing_file, op).unwrap());
    }

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

    /// The 71-digit operand reported in the GNU compatibility issue, which is
    /// far wider than any fixed-size integer type.
    const BIG: &str = "16267277278126277227728782172782882627278282882172762677623672762783782";
    /// `BIG` with its final digit incremented, so the two only differ in the
    /// least significant digit.
    const BIG_PLUS_ONE: &str =
        "16267277278126277227728782172782882627278282882172762677623672762783783";
    /// One digit shorter than `BIG`, so the two differ in width.
    const SMALLER: &str = "1626727727812627722772878217278288262727828288217276267762367276278378";

    #[test]
    fn test_integer_op_beyond_i128() {
        let big = OsStr::new(BIG);
        let big_plus_one = OsStr::new(BIG_PLUS_ONE);
        let smaller = OsStr::new(SMALLER);
        let one = OsStr::new("1");

        assert!(integers(big, big, OsStr::new("-eq")).unwrap());
        assert!(!integers(big, big, OsStr::new("-ne")).unwrap());
        assert!(integers(big, big, OsStr::new("-ge")).unwrap());
        assert!(integers(big, big, OsStr::new("-le")).unwrap());

        assert!(integers(one, big, OsStr::new("-ne")).unwrap());
        assert!(integers(one, big, OsStr::new("-lt")).unwrap());
        assert!(integers(big, one, OsStr::new("-gt")).unwrap());

        // Same width, differing only in the least significant digit.
        assert!(integers(big_plus_one, big, OsStr::new("-gt")).unwrap());
        assert!(integers(big, big_plus_one, OsStr::new("-lt")).unwrap());
        assert!(!integers(big, big_plus_one, OsStr::new("-eq")).unwrap());

        // Differing widths.
        assert!(integers(big, smaller, OsStr::new("-gt")).unwrap());
        assert!(integers(smaller, big, OsStr::new("-lt")).unwrap());
    }

    #[test]
    fn test_integer_op_beyond_i128_negative() {
        let big = OsStr::new(BIG);
        let neg_big =
            OsStr::new("-16267277278126277227728782172782882627278282882172762677623672762783782");
        let neg_smaller =
            OsStr::new("-1626727727812627722772878217278288262727828288217276267762367276278378");

        assert!(integers(neg_big, neg_big, OsStr::new("-eq")).unwrap());
        assert!(integers(neg_big, OsStr::new("0"), OsStr::new("-lt")).unwrap());
        assert!(integers(neg_big, big, OsStr::new("-lt")).unwrap());
        assert!(integers(big, neg_big, OsStr::new("-gt")).unwrap());

        // A wider negative number is the smaller of the two.
        assert!(integers(neg_big, neg_smaller, OsStr::new("-lt")).unwrap());
        assert!(integers(neg_smaller, neg_big, OsStr::new("-gt")).unwrap());
    }

    #[test]
    fn test_integer_parse_normalizes_sign_and_leading_zeros() {
        // Zero carries no sign, so `-0` and `0` are the same value.
        assert_eq!(
            Integer::parse(OsStr::new("-0")),
            Integer::parse(OsStr::new("0"))
        );
        assert_eq!(
            Integer::parse(OsStr::new("+0")),
            Integer::parse(OsStr::new("-0"))
        );
        assert_eq!(
            Integer::parse(OsStr::new("007")),
            Integer::parse(OsStr::new("7"))
        );
        assert_eq!(
            Integer::parse(OsStr::new("-007")),
            Integer::parse(OsStr::new("-7"))
        );
        // Surrounding whitespace is ignored.
        assert_eq!(
            Integer::parse(OsStr::new(" 42 ")),
            Integer::parse(OsStr::new("42"))
        );
        // Normalization is not limited by width either.
        let padded = OsString::from(format!("+00{BIG}"));
        assert_eq!(Integer::parse(&padded), Integer::parse(OsStr::new(BIG)));
    }

    #[test]
    fn test_integer_op_rejects_malformed_operands() {
        // Widening the accepted range must not make any of these parse.
        // "\u{664}\u{662}" and "\u{ff11}\u{ff12}" are non-ASCII digits, which
        // also exercise operands that are not one byte per character.
        for operand in [
            "",
            "-",
            "+",
            "++5",
            "--5",
            "5-",
            "+-5",
            "1_0",
            "0x10",
            "1e3",
            "123.45",
            "4 2",
            "\u{664}\u{662}",
            "\u{ff11}\u{ff12}",
        ] {
            let operand = OsStr::new(operand);
            assert!(
                integers(operand, OsStr::new("0"), OsStr::new("-eq")).is_err(),
                "{operand:?} should not parse as an integer"
            );
            assert!(
                integers(OsStr::new("0"), operand, OsStr::new("-eq")).is_err(),
                "{operand:?} should not parse as an integer"
            );
        }
    }
}
