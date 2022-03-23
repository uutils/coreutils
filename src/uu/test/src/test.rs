// This file is part of the uutils coreutils package.
//
// (c) mahkoh (ju.orth [at] gmail [dot] com)
// (c) Daniel Rocco <drocco@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) egid euid FiletestOp StrlenOp

mod parser;

use clap::{crate_version, Command};
use parser::{parse, Operator, Symbol, UnaryOperator};
use std::ffi::{OsStr, OsString};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;

const USAGE: &str = "\
    {} EXPRESSION
    {}
    [ EXPRESSION ]
    [ ]
    [ OPTION";

// We use after_help so that this comes after the usage string (it would come before if we used about)
const AFTER_HELP: &str = "
Exit with the status determined by EXPRESSION.

An omitted EXPRESSION defaults to false.  Otherwise,
EXPRESSION is true or false and sets exit status.  It is one of:

  ( EXPRESSION )               EXPRESSION is true
  ! EXPRESSION                 EXPRESSION is false
  EXPRESSION1 -a EXPRESSION2   both EXPRESSION1 and EXPRESSION2 are true
  EXPRESSION1 -o EXPRESSION2   either EXPRESSION1 or EXPRESSION2 is true

  -n STRING            the length of STRING is nonzero
  STRING               equivalent to -n STRING
  -z STRING            the length of STRING is zero
  STRING1 = STRING2    the strings are equal
  STRING1 != STRING2   the strings are not equal

  INTEGER1 -eq INTEGER2   INTEGER1 is equal to INTEGER2
  INTEGER1 -ge INTEGER2   INTEGER1 is greater than or equal to INTEGER2
  INTEGER1 -gt INTEGER2   INTEGER1 is greater than INTEGER2
  INTEGER1 -le INTEGER2   INTEGER1 is less than or equal to INTEGER2
  INTEGER1 -lt INTEGER2   INTEGER1 is less than INTEGER2
  INTEGER1 -ne INTEGER2   INTEGER1 is not equal to INTEGER2

  FILE1 -ef FILE2   FILE1 and FILE2 have the same device and inode numbers
  FILE1 -nt FILE2   FILE1 is newer (modification date) than FILE2
  FILE1 -ot FILE2   FILE1 is older than FILE2

  -b FILE     FILE exists and is block special
  -c FILE     FILE exists and is character special
  -d FILE     FILE exists and is a directory
  -e FILE     FILE exists
  -f FILE     FILE exists and is a regular file
  -g FILE     FILE exists and is set-group-ID
  -G FILE     FILE exists and is owned by the effective group ID
  -h FILE     FILE exists and is a symbolic link (same as -L)
  -k FILE     FILE exists and has its sticky bit set
  -L FILE     FILE exists and is a symbolic link (same as -h)
  -N FILE     FILE exists and has been modified since it was last read
  -O FILE     FILE exists and is owned by the effective user ID
  -p FILE     FILE exists and is a named pipe
  -r FILE     FILE exists and read permission is granted
  -s FILE     FILE exists and has a size greater than zero
  -S FILE     FILE exists and is a socket
  -t FD       file descriptor FD is opened on a terminal
  -u FILE     FILE exists and its set-user-ID bit is set
  -w FILE     FILE exists and write permission is granted
  -x FILE     FILE exists and execute (or search) permission is granted

Except for -h and -L, all FILE-related tests dereference symbolic links.
Beware that parentheses need to be escaped (e.g., by backslashes) for shells.
INTEGER may also be -l STRING, which evaluates to the length of STRING.

NOTE: Binary -a and -o are inherently ambiguous.  Use 'test EXPR1 && test
EXPR2' or 'test EXPR1 || test EXPR2' instead.

NOTE: [ honors the --help and --version options, but test does not.
test treats each of those as it treats any other nonempty STRING.

NOTE: your shell may have its own version of test and/or [, which usually supersedes
the version described here.  Please refer to your shell's documentation
for details about the options it supports.";

const ABOUT: &str = "Check file types and compare values.";

pub fn uu_app<'a>() -> Command<'a> {
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
            // Let clap pretty-print help and version
            Command::new(binary_name)
                .version(crate_version!())
                .about(ABOUT)
                .override_usage(format_usage(USAGE))
                .after_help(AFTER_HELP)
                // Disable printing of -h and -v as valid alternatives for --help and --version,
                // since we don't recognize -h and -v as help/version flags.
                .get_matches_from(std::iter::once(program).chain(args.into_iter()));
            return Ok(());
        }
        // If invoked via name '[', matching ']' must be in the last arg
        let last = args.pop();
        if last.as_deref() != Some(OsStr::new("]")) {
            return Err(USimpleError::new(2, "missing ']'"));
        }
    }

    let result = parse(args).and_then(|mut stack| eval(&mut stack));

    match result {
        Ok(result) => {
            if result {
                Ok(())
            } else {
                Err(1.into())
            }
        }
        Err(e) => Err(USimpleError::new(2, e)),
    }
}

/// Evaluate a stack of Symbols, returning the result of the evaluation or
/// an error message if evaluation failed.
fn eval(stack: &mut Vec<Symbol>) -> Result<bool, String> {
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
            let b = stack.pop();
            let a = stack.pop();
            Ok(if op == "!=" { a != b } else { a == b })
        }
        Some(Symbol::Op(Operator::Int(op))) => {
            let b = pop_literal!();
            let a = pop_literal!();

            Ok(integers(&a, &b, &op)?)
        }
        Some(Symbol::Op(Operator::File(_op))) => unimplemented!(),
        Some(Symbol::UnaryOp(UnaryOperator::StrlenOp(op))) => {
            let s = match stack.pop() {
                Some(Symbol::Literal(s)) => s,
                Some(Symbol::None) => OsString::from(""),
                None => {
                    return Ok(true);
                }
                _ => {
                    return Err(format!("missing argument after '{:?}'", op));
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
            let b = eval(stack)?;
            let a = eval(stack)?;

            Ok(if op == "-a" { a && b } else { a || b })
        }
        _ => Err("expected value".to_string()),
    }
}

fn integers(a: &OsStr, b: &OsStr, op: &OsStr) -> Result<bool, String> {
    let format_err = |value: &OsStr| format!("invalid integer {}", value.quote());

    let a: i64 = a
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format_err(a))?;

    let b: i64 = b
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format_err(b))?;

    Ok(match op.to_str() {
        Some("-eq") => a == b,
        Some("-ne") => a != b,
        Some("-gt") => a > b,
        Some("-ge") => a >= b,
        Some("-lt") => a < b,
        Some("-le") => a <= b,
        _ => return Err(format!("unknown operator {}", op.quote())),
    })
}

fn isatty(fd: &OsStr) -> Result<bool, String> {
    fd.to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("invalid integer {}", fd.quote()))
        .map(|i| {
            #[cfg(not(target_os = "redox"))]
            unsafe {
                libc::isatty(i) == 1
            }
            #[cfg(target_os = "redox")]
            syscall::dup(i, b"termios").map(syscall::close).is_ok()
        })
}

#[derive(Eq, PartialEq)]
enum PathCondition {
    BlockSpecial,
    CharacterSpecial,
    Directory,
    Exists,
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
    use std::fs::{self, Metadata};
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    const S_ISUID: u32 = 0o4000;
    const S_ISGID: u32 = 0o2000;
    const S_ISVTX: u32 = 0o1000;

    enum Permission {
        Read = 0o4,
        Write = 0o2,
        Execute = 0o1,
    }

    let geteuid = || {
        #[cfg(not(target_os = "redox"))]
        let euid = unsafe { libc::geteuid() };
        #[cfg(target_os = "redox")]
        let euid = syscall::geteuid().unwrap() as u32;

        euid
    };

    let getegid = || {
        #[cfg(not(target_os = "redox"))]
        let egid = unsafe { libc::getegid() };
        #[cfg(target_os = "redox")]
        let egid = syscall::getegid().unwrap() as u32;

        egid
    };

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

    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(_) => {
            return false;
        }
    };

    let file_type = metadata.file_type();

    match condition {
        PathCondition::BlockSpecial => file_type.is_block_device(),
        PathCondition::CharacterSpecial => file_type.is_char_device(),
        PathCondition::Directory => file_type.is_dir(),
        PathCondition::Exists => true,
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
