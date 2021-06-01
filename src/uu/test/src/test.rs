// This file is part of the uutils coreutils package.
//
// (c) mahkoh (ju.orth [at] gmail [dot] com)
// (c) Daniel Rocco <drocco@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) FiletestOp StrlenOp

mod parser;

use parser::{parse, Symbol};
use std::ffi::{OsStr, OsString};

pub fn uumain(args: impl uucore::Args) -> i32 {
    // TODO: handle being called as `[`
    let args: Vec<_> = args.skip(1).collect();

    let result = parse(args).and_then(|mut stack| eval(&mut stack));

    match result {
        Ok(result) => {
            if result {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("test: {}", e);
            2
        }
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
        Some(Symbol::StringOp(op)) => {
            let b = stack.pop();
            let a = stack.pop();
            Ok(if op == "!=" { a != b } else { a == b })
        }
        Some(Symbol::IntOp(op)) => {
            let b = pop_literal!();
            let a = pop_literal!();

            Ok(integers(&a, &b, &op)?)
        }
        Some(Symbol::FileOp(_op)) => unimplemented!(),
        Some(Symbol::StrlenOp(op)) => {
            let s = match stack.pop() {
                Some(Symbol::Literal(s)) => s,
                Some(Symbol::None) => OsString::from(""),
                None => {
                    return Ok(true);
                }
                _ => {
                    return Err(format!("missing argument after ‘{:?}’", op));
                }
            };

            Ok(if op == "-z" {
                s.is_empty()
            } else {
                !s.is_empty()
            })
        }
        Some(Symbol::FiletestOp(op)) => {
            let op = op.to_string_lossy();

            let f = pop_literal!();

            Ok(match op.as_ref() {
                "-b" => path(&f, PathCondition::BlockSpecial),
                "-c" => path(&f, PathCondition::CharacterSpecial),
                "-d" => path(&f, PathCondition::Directory),
                "-e" => path(&f, PathCondition::Exists),
                "-f" => path(&f, PathCondition::Regular),
                "-g" => path(&f, PathCondition::GroupIdFlag),
                "-h" => path(&f, PathCondition::SymLink),
                "-L" => path(&f, PathCondition::SymLink),
                "-p" => path(&f, PathCondition::Fifo),
                "-r" => path(&f, PathCondition::Readable),
                "-S" => path(&f, PathCondition::Socket),
                "-s" => path(&f, PathCondition::NonEmpty),
                "-t" => isatty(&f)?,
                "-u" => path(&f, PathCondition::UserIdFlag),
                "-w" => path(&f, PathCondition::Writable),
                "-x" => path(&f, PathCondition::Executable),
                _ => panic!(),
            })
        }
        Some(Symbol::Literal(s)) => Ok(!s.is_empty()),
        Some(Symbol::None) => Ok(false),
        Some(Symbol::BoolOp(op)) => {
            let b = eval(stack)?;
            let a = eval(stack)?;

            Ok(if op == "-a" { a && b } else { a || b })
        }
        None => Ok(false),
        _ => Err("expected value".to_string()),
    }
}

fn integers(a: &OsStr, b: &OsStr, op: &OsStr) -> Result<bool, String> {
    let format_err = |value| format!("invalid integer ‘{}’", value);

    let a = a.to_string_lossy();
    let a: i64 = a.parse().map_err(|_| format_err(a))?;

    let b = b.to_string_lossy();
    let b: i64 = b.parse().map_err(|_| format_err(b))?;

    let operator = op.to_string_lossy();
    Ok(match operator.as_ref() {
        "-eq" => a == b,
        "-ne" => a != b,
        "-gt" => a > b,
        "-ge" => a >= b,
        "-lt" => a < b,
        "-le" => a <= b,
        _ => return Err(format!("unknown operator ‘{}’", operator)),
    })
}

fn isatty(fd: &OsStr) -> Result<bool, String> {
    let fd = fd.to_string_lossy();

    fd.parse()
        .map_err(|_| format!("invalid integer ‘{}’", fd))
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
    SymLink,
    Fifo,
    Readable,
    Socket,
    NonEmpty,
    UserIdFlag,
    Writable,
    Executable,
}

#[cfg(not(windows))]
fn path(path: &OsStr, condition: PathCondition) -> bool {
    use std::fs::{self, Metadata};
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    const S_ISUID: u32 = 0o4000;
    const S_ISGID: u32 = 0o2000;

    enum Permission {
        Read = 0o4,
        Write = 0o2,
        Execute = 0o1,
    }

    let perm = |metadata: Metadata, p: Permission| {
        #[cfg(not(target_os = "redox"))]
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        #[cfg(target_os = "redox")]
        let (uid, gid) = (
            syscall::getuid().unwrap() as u32,
            syscall::getgid().unwrap() as u32,
        );

        if uid == metadata.uid() {
            metadata.mode() & ((p as u32) << 6) != 0
        } else if gid == metadata.gid() {
            metadata.mode() & ((p as u32) << 3) != 0
        } else {
            metadata.mode() & (p as u32) != 0
        }
    };

    let metadata = if condition == PathCondition::SymLink {
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
        PathCondition::SymLink => metadata.file_type().is_symlink(),
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
fn path(path: &OsStr, condition: PathCondition) -> bool {
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
        PathCondition::SymLink => false,
        PathCondition::Fifo => false,
        PathCondition::Readable => false, // TODO
        PathCondition::Socket => false,
        PathCondition::NonEmpty => stat.len() > 0,
        PathCondition::UserIdFlag => false,
        PathCondition::Writable => false,   // TODO
        PathCondition::Executable => false, // TODO
    }
}
