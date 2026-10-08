// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) egid euid faccessat

mod diagnostics;
pub(crate) mod error;
#[cfg(not(any(windows, target_os = "wasi")))]
mod faccessat;
mod parser;

#[cfg(any(windows, target_os = "wasi"))]
mod platform;

#[cfg(not(any(windows, target_os = "wasi")))]
use crate::faccessat::effective_access;
use clap::Command;
use error::{ParseError, ParseErrorKind, ParseResult};
use parser::{BinaryOp, Evaluator, Operand, UnaryOp, evaluate};
#[cfg(windows)]
use platform::fd_is_terminal;
#[cfg(target_os = "wasi")]
use platform::path;
#[cfg(not(any(windows, target_os = "wasi")))]
use rustix::fs::Access;
#[cfg(not(any(windows, target_os = "wasi")))]
use rustix::process::{getegid, geteuid};
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::mem::size_of;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
#[cfg(not(any(windows, target_os = "wasi")))]
use uucore::fs::mode::{S_ISGID, S_ISUID, S_ISVTX};
use uucore::i18n::collator::{init_locale_collation, locale_cmp};
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
    let expression = uucore::diagnostics::capture(&args);

    match evaluate(&args, TestEvaluator) {
        Ok(true) => Ok(()),
        Ok(false) => Err(1.into()),
        Err(e) => Err(uucore::diagnostics::error_after_report(
            expression.as_deref(),
            e,
            diagnostics::render,
        )),
    }
}

struct TestEvaluator;

fn operand_value(operand: Operand<'_>) -> &OsStr {
    match operand {
        Operand::Value(value) => value,
        Operand::Length(_) => unreachable!("length operand passed to non-integer operator"),
    }
}

impl Evaluator for TestEvaluator {
    fn unary(&mut self, op: UnaryOp, arg: &OsStr) -> ParseResult<bool> {
        Ok(match op {
            UnaryOp::BlockSpecial => path(arg, &PathCondition::BlockSpecial),
            UnaryOp::CharacterSpecial => path(arg, &PathCondition::CharacterSpecial),
            UnaryOp::Directory => path(arg, &PathCondition::Directory),
            UnaryOp::Exists => path(arg, &PathCondition::Exists),
            UnaryOp::Regular => path(arg, &PathCondition::Regular),
            UnaryOp::GroupIdFlag => path(arg, &PathCondition::GroupIdFlag),
            UnaryOp::GroupOwns => path(arg, &PathCondition::GroupOwns),
            UnaryOp::SymLink => path(arg, &PathCondition::SymLink),
            UnaryOp::Sticky => path(arg, &PathCondition::Sticky),
            UnaryOp::ModifiedSinceRead => path(arg, &PathCondition::ExistsModifiedLastRead),
            UnaryOp::UserOwns => path(arg, &PathCondition::UserOwns),
            UnaryOp::Fifo => path(arg, &PathCondition::Fifo),
            UnaryOp::Readable => path(arg, &PathCondition::Readable),
            UnaryOp::NonEmpty => path(arg, &PathCondition::NonEmpty),
            UnaryOp::Socket => path(arg, &PathCondition::Socket),
            UnaryOp::Tty => isatty(arg)?,
            UnaryOp::UserIdFlag => path(arg, &PathCondition::UserIdFlag),
            UnaryOp::Writable => path(arg, &PathCondition::Writable),
            UnaryOp::Executable => path(arg, &PathCondition::Executable),
            UnaryOp::StrNonEmpty => !arg.is_empty(),
            UnaryOp::StrEmpty => arg.is_empty(),
        })
    }

    fn binary(&mut self, op: BinaryOp, lhs: Operand<'_>, rhs: Operand<'_>) -> ParseResult<bool> {
        match op {
            BinaryOp::StrEq => Ok(operand_value(lhs) == operand_value(rhs)),
            BinaryOp::StrNe => Ok(operand_value(lhs) != operand_value(rhs)),
            BinaryOp::StrLt => {
                let _ = init_locale_collation();
                Ok(locale_cmp(
                    operand_value(lhs).as_encoded_bytes(),
                    operand_value(rhs).as_encoded_bytes(),
                )
                .is_lt())
            }
            BinaryOp::StrGt => {
                let _ = init_locale_collation();
                Ok(locale_cmp(
                    operand_value(lhs).as_encoded_bytes(),
                    operand_value(rhs).as_encoded_bytes(),
                )
                .is_gt())
            }
            BinaryOp::IntEq
            | BinaryOp::IntNe
            | BinaryOp::IntLt
            | BinaryOp::IntLe
            | BinaryOp::IntGt
            | BinaryOp::IntGe => compare_integer_operands(lhs, rhs, op),
            BinaryOp::FileEf | BinaryOp::FileNt | BinaryOp::FileOt => files(
                operand_value(lhs),
                operand_value(rhs),
                OsStr::new(op.as_str()),
            ),
        }
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
    /// The [POSIX locale convention](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_06_05)
    /// includes U+000B VERTICAL TAB as whitespace.
    fn parse(value: &'a OsStr) -> Option<Self> {
        let value = value
            .to_str()?
            .trim_matches(|c: char| c.is_ascii_whitespace() || c == '\u{000b}');

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

#[derive(Debug)]
enum IntegerOperand<'a> {
    Parsed(Integer<'a>),
    Length(usize),
}

fn integer_operand(operand: Operand<'_>) -> ParseResult<IntegerOperand<'_>> {
    match operand {
        Operand::Length(value) => Ok(IntegerOperand::Length(value.as_encoded_bytes().len())),
        Operand::Value(value) => Integer::parse(value)
            .map(IntegerOperand::Parsed)
            .ok_or_else(|| {
                ParseError::at_value(
                    ParseErrorKind::InvalidInteger(value.quote().to_string()),
                    value,
                )
            }),
    }
}

fn integer_cmp_usize(value: &Integer<'_>, other: usize) -> Ordering {
    if value.negative {
        return Ordering::Less;
    }

    let mut buf = [0_u8; 3 * size_of::<usize>()];
    let mut n = other;
    let mut start = buf.len();
    loop {
        start -= 1;
        buf[start] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }

    let digits = if value.digits.is_empty() {
        b"0".as_slice()
    } else {
        value.digits.as_bytes()
    };
    let other = &buf[start..];
    digits
        .len()
        .cmp(&other.len())
        .then_with(|| digits.cmp(other))
}

fn compare_integer_operands(lhs: Operand<'_>, rhs: Operand<'_>, op: BinaryOp) -> ParseResult<bool> {
    let lhs = integer_operand(lhs)?;
    let rhs = integer_operand(rhs)?;
    let order = match (&lhs, &rhs) {
        (IntegerOperand::Parsed(lhs), IntegerOperand::Parsed(rhs)) => lhs.cmp(rhs),
        (IntegerOperand::Parsed(lhs), IntegerOperand::Length(rhs)) => integer_cmp_usize(lhs, *rhs),
        (IntegerOperand::Length(lhs), IntegerOperand::Parsed(rhs)) => {
            integer_cmp_usize(rhs, *lhs).reverse()
        }
        (IntegerOperand::Length(lhs), IntegerOperand::Length(rhs)) => lhs.cmp(rhs),
    };

    Ok(match op {
        BinaryOp::IntEq => order.is_eq(),
        BinaryOp::IntNe => order.is_ne(),
        BinaryOp::IntLt => order.is_lt(),
        BinaryOp::IntLe => order.is_le(),
        BinaryOp::IntGt => order.is_gt(),
        BinaryOp::IntGe => order.is_ge(),
        _ => unreachable!("non-integer operator passed to integer comparison"),
    })
}

/// Operations to compare files metadata
/// `a` is the left hand side
/// `b` is the right hand side
/// `op` the operation (ex: -ef, -nt, etc.)
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
    let value = Integer::parse(fd).ok_or_else(|| {
        ParseError::at_value(
            ParseErrorKind::InvalidFileDescriptor(fd.quote().to_string()),
            fd,
        )
    })?;

    if value.negative {
        return Ok(false);
    }
    let descriptor = if value.digits.is_empty() {
        0
    } else {
        let Ok(descriptor) = value.digits.parse::<u32>() else {
            return Ok(false);
        };
        if descriptor > i32::MAX as u32 {
            return Ok(false);
        }
        descriptor as i32
    };
    Ok(fd_is_terminal(descriptor))
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
    use std::os::unix::fs::FileTypeExt;

    let metadata = || {
        if matches!(condition, PathCondition::SymLink) {
            fs::symlink_metadata(path)
        } else {
            fs::metadata(path)
        }
    };

    match condition {
        PathCondition::Readable => effective_access(path, Access::READ_OK),
        PathCondition::Writable => effective_access(path, Access::WRITE_OK),
        PathCondition::Executable => effective_access(path, Access::EXEC_OK),

        PathCondition::BlockSpecial => metadata().is_ok_and(|m| m.file_type().is_block_device()),

        PathCondition::CharacterSpecial => metadata().is_ok_and(|m| m.file_type().is_char_device()),

        PathCondition::Directory => metadata().is_ok_and(|m| m.file_type().is_dir()),

        PathCondition::Exists => metadata().is_ok(),

        PathCondition::ExistsModifiedLastRead => metadata().is_ok_and(|m| modified_since_read(&m)),

        PathCondition::Regular => metadata().is_ok_and(|m| m.file_type().is_file()),

        PathCondition::GroupIdFlag => metadata().is_ok_and(|m| m.mode() & S_ISGID != 0),

        PathCondition::GroupOwns => metadata().is_ok_and(|m| m.gid() == getegid().as_raw()),

        PathCondition::SymLink => metadata().is_ok_and(|m| m.file_type().is_symlink()),

        PathCondition::Sticky => metadata().is_ok_and(|m| m.mode() & S_ISVTX != 0),

        PathCondition::UserOwns => metadata().is_ok_and(|m| m.uid() == geteuid().as_raw()),

        PathCondition::Fifo => metadata().is_ok_and(|m| m.file_type().is_fifo()),

        PathCondition::Socket => metadata().is_ok_and(|m| m.file_type().is_socket()),

        PathCondition::NonEmpty => metadata().is_ok_and(|m| m.size() > 0),

        PathCondition::UserIdFlag => metadata().is_ok_and(|m| m.mode() & S_ISUID != 0),
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

    #[cfg(target_os = "linux")]
    #[test]
    fn test_root_access_is_not_owner_mode_bits() {
        use std::os::unix::fs::PermissionsExt;

        if geteuid().as_raw() != 0 {
            return;
        }

        let file = NamedTempFile::new().unwrap();
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o000)).unwrap();
        let path = file.path().as_os_str();

        assert!(effective_access(path, Access::READ_OK));
        assert!(effective_access(path, Access::WRITE_OK));
        assert!(!effective_access(path, Access::EXEC_OK));

        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o001)).unwrap();
        assert!(effective_access(path, Access::EXEC_OK));
    }

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
        assert!(
            !compare_integer_operands(Operand::Value(a), Operand::Value(b), BinaryOp::IntLt)
                .unwrap()
        );
        let a = OsStr::new("18446744073709551616");
        let b = OsStr::new("0");
        assert!(
            compare_integer_operands(Operand::Value(a), Operand::Value(b), BinaryOp::IntGt)
                .unwrap()
        );
        let a = OsStr::new("-1");
        let b = OsStr::new("0");
        assert!(
            compare_integer_operands(Operand::Value(a), Operand::Value(b), BinaryOp::IntLt)
                .unwrap()
        );
        let a = OsStr::new("42");
        let b = OsStr::new("42");
        assert!(
            compare_integer_operands(Operand::Value(a), Operand::Value(b), BinaryOp::IntEq)
                .unwrap()
        );
        let a = OsStr::new("42");
        let b = OsStr::new("42");
        assert!(
            !compare_integer_operands(Operand::Value(a), Operand::Value(b), BinaryOp::IntNe)
                .unwrap()
        );
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

        assert!(
            compare_integer_operands(Operand::Value(big), Operand::Value(big), BinaryOp::IntEq)
                .unwrap()
        );
        assert!(
            !compare_integer_operands(Operand::Value(big), Operand::Value(big), BinaryOp::IntNe)
                .unwrap()
        );
        assert!(
            compare_integer_operands(Operand::Value(big), Operand::Value(big), BinaryOp::IntGe)
                .unwrap()
        );
        assert!(
            compare_integer_operands(Operand::Value(big), Operand::Value(big), BinaryOp::IntLe)
                .unwrap()
        );

        assert!(
            compare_integer_operands(Operand::Value(one), Operand::Value(big), BinaryOp::IntNe)
                .unwrap()
        );
        assert!(
            compare_integer_operands(Operand::Value(one), Operand::Value(big), BinaryOp::IntLt)
                .unwrap()
        );
        assert!(
            compare_integer_operands(Operand::Value(big), Operand::Value(one), BinaryOp::IntGt)
                .unwrap()
        );

        // Same width, differing only in the least significant digit.
        assert!(
            compare_integer_operands(
                Operand::Value(big_plus_one),
                Operand::Value(big),
                BinaryOp::IntGt
            )
            .unwrap()
        );
        assert!(
            compare_integer_operands(
                Operand::Value(big),
                Operand::Value(big_plus_one),
                BinaryOp::IntLt
            )
            .unwrap()
        );
        assert!(
            !compare_integer_operands(
                Operand::Value(big),
                Operand::Value(big_plus_one),
                BinaryOp::IntEq
            )
            .unwrap()
        );

        // Differing widths.
        assert!(
            compare_integer_operands(
                Operand::Value(big),
                Operand::Value(smaller),
                BinaryOp::IntGt
            )
            .unwrap()
        );
        assert!(
            compare_integer_operands(
                Operand::Value(smaller),
                Operand::Value(big),
                BinaryOp::IntLt
            )
            .unwrap()
        );
    }

    #[test]
    fn test_integer_op_beyond_i128_negative() {
        let big = OsStr::new(BIG);
        let neg_big =
            OsStr::new("-16267277278126277227728782172782882627278282882172762677623672762783782");
        let neg_smaller =
            OsStr::new("-1626727727812627722772878217278288262727828288217276267762367276278378");

        assert!(
            compare_integer_operands(
                Operand::Value(neg_big),
                Operand::Value(neg_big),
                BinaryOp::IntEq
            )
            .unwrap()
        );
        assert!(
            compare_integer_operands(
                Operand::Value(neg_big),
                Operand::Value(OsStr::new("0")),
                BinaryOp::IntLt
            )
            .unwrap()
        );
        assert!(
            compare_integer_operands(
                Operand::Value(neg_big),
                Operand::Value(big),
                BinaryOp::IntLt
            )
            .unwrap()
        );
        assert!(
            compare_integer_operands(
                Operand::Value(big),
                Operand::Value(neg_big),
                BinaryOp::IntGt
            )
            .unwrap()
        );

        // A wider negative number is the smaller of the two.
        assert!(
            compare_integer_operands(
                Operand::Value(neg_big),
                Operand::Value(neg_smaller),
                BinaryOp::IntLt
            )
            .unwrap()
        );
        assert!(
            compare_integer_operands(
                Operand::Value(neg_smaller),
                Operand::Value(neg_big),
                BinaryOp::IntGt
            )
            .unwrap()
        );
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
                compare_integer_operands(
                    Operand::Value(operand),
                    Operand::Value(OsStr::new("0")),
                    BinaryOp::IntEq,
                )
                .is_err(),
                "{operand:?} should not parse as an integer"
            );
            assert!(
                compare_integer_operands(
                    Operand::Value(OsStr::new("0")),
                    Operand::Value(operand),
                    BinaryOp::IntEq,
                )
                .is_err(),
                "{operand:?} should not parse as an integer"
            );
        }
    }
}
