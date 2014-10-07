#![crate_name = "uutest"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) mahkoh (ju.orth [at] gmail [dot] com)
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate libc;
extern crate num;

use std::os::{args_as_bytes};
use std::str::{from_utf8};
use num::bigint::{BigInt};

static NAME: &'static str = "test";

// TODO: decide how to handle non-UTF8 input for all the utils
pub fn uumain(_: Vec<String>) -> int {
    let args = args_as_bytes();
    let args: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
    if args.len() == 0 {
        return 2;
    }
    let args =
        if !args[0].ends_with(NAME.as_bytes()) {
            args.slice_from(1)
        } else {
            args.as_slice()
        };
    let args = match args[0] {
        b"[" => match args[args.len() - 1] {
            b"]" => args.slice(1, args.len() - 1),
            _ => return 2,
        },
        _ => args.slice(1, args.len()),
    };
    1 - dispatch(args) as int
}

fn one(args: &[&[u8]]) -> bool {
    args[0].len() > 0
}

fn two(args: &[&[u8]]) -> bool {
    match args[0] {
        b"!" => !one(args.slice_from(1)),
        b"-b" => path(args[1], BlockSpecial),
        b"-c" => path(args[1], CharacterSpecial),
        b"-d" => path(args[1], Directory),
        b"-e" => path(args[1], Exists),
        b"-f" => path(args[1], Regular),
        b"-g" => path(args[1], GroupIDFlag),
        b"-h" => path(args[1], SymLink),
        b"-L" => path(args[1], SymLink),
        b"-n" => one(args.slice_from(1)),
        b"-p" => path(args[1], FIFO),
        b"-r" => path(args[1], Readable),
        b"-S" => path(args[1], Socket),
        b"-s" => path(args[1], NonEmpty),
        b"-t" => isatty(args[1]),
        b"-u" => path(args[1], UserIDFlag),
        b"-w" => path(args[1], Writable),
        b"-x" => path(args[1], Executable),
        b"-z" => !one(args.slice_from(1)),
        _ => false,
    }
}

fn three(args: &[&[u8]]) -> bool {
    match args[1] {
        b"=" => args[0] == args[2],
        b"!=" => args[0] != args[2],
        b"-eq" => integers(args[0], args[2], Equal),
        b"-ne" => integers(args[0], args[2], Unequal),
        b"-gt" => integers(args[0], args[2], Greater),
        b"-ge" => integers(args[0], args[2], GreaterEqual),
        b"-lt" => integers(args[0], args[2], Less),
        b"-le" => integers(args[0], args[2], LessEqual),
        b"-a" => one(args.slice_to(1)) && one(args.slice_from(2)),
        b"-o" => one(args.slice_to(1)) || one(args.slice_from(2)),
        _ => match args[0] {
            b"!" => !two(args.slice_from(1)),
            _ => false,
        }
    }
}

fn four(args: &[&[u8]]) -> bool {
    let (val, len) = match args[0] {
        b"!" => (!three(args.slice_from(1)), 4),
        _ => (three(args), 3)
    };
    if len < args.len() {
        match args[len] {
            b"-a" => val && dispatch(args.slice_from(len + 1)),
            b"-o" => val || dispatch(args.slice_from(len + 1)),
            _ => false
        }
    } else {
        val
    }
}

enum IntegerCondition {
    Equal,
    Unequal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

fn integers(a: &[u8], b: &[u8], cond: IntegerCondition) -> bool {
    let (a, b): (&str, &str) = match (from_utf8(a), from_utf8(b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };
    let (a, b): (BigInt, BigInt) = match (from_str(a), from_str(b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };
    match cond {
        Equal        => a == b,
        Unequal      => a != b,
        Greater      => a >  b,
        GreaterEqual => a >= b,
        Less         => a <  b,
        LessEqual    => a <= b,
    }
}

fn isatty(fd: &[u8]) -> bool {
    use libc::{isatty};
    from_utf8(fd).and_then(|s| from_str(s))
            .map(|i| unsafe { isatty(i) == 1 }).unwrap_or(false)
}

fn dispatch(args: &[&[u8]]) -> bool {
    match args.len() {
        0 => false,
        1 => one(args),
        2 => two(args),
        3 => three(args),
        _ => four(args)
    }
}

#[deriving(Eq, PartialEq)]
enum PathCondition {
    BlockSpecial,
    CharacterSpecial,
    Directory,
    Exists,
    Regular,
    GroupIDFlag,
    SymLink,
    FIFO,
    Readable,
    Socket,
    NonEmpty,
    UserIDFlag,
    Writable,
    Executable,
}

#[cfg(not(windows))]
fn path(path: &[u8], cond: PathCondition) -> bool {
    use libc::{stat, lstat, S_IFMT, S_IFLNK, S_IFBLK, S_IFCHR, S_IFDIR, S_IFREG};
    use libc::{S_IFIFO, mode_t};
    static S_ISUID: mode_t = 0o4000;
    static S_ISGID: mode_t = 0o2000;
    static S_IFSOCK: mode_t = 0o140000;

    enum Permission {
        Read    = 0o4,
        Write   = 0o2,
        Execute = 0o1,
    }
    let perm = |stat: stat, p: Permission| {
        use libc::{getgid, getuid};
        let (uid, gid) = unsafe { (getuid(), getgid()) };
        if uid == stat.st_uid {
            stat.st_mode & (p as mode_t << 6) != 0
        } else if gid == stat.st_gid {
            stat.st_mode & (p as mode_t << 3) != 0
        } else {
            stat.st_mode & (p as mode_t << 0) != 0
        }
    };

    let path = unsafe { path.to_c_str_unchecked() };
    let mut stat = unsafe { std::mem::zeroed() };
    if cond == SymLink {
        if unsafe { lstat(path.as_ptr(), &mut stat) } == 0 {
            if stat.st_mode & S_IFMT == S_IFLNK {
                return true;
            }
        }
        return false;
    }
    if unsafe { libc::stat(path.as_ptr(), &mut stat) } != 0 {
        return false;
    }
    let file_type = stat.st_mode & S_IFMT;
    match cond {
        BlockSpecial     => file_type == S_IFBLK,
        CharacterSpecial => file_type == S_IFCHR,
        Directory        => file_type == S_IFDIR,
        Exists           => true,
        Regular          => file_type == S_IFREG,
        GroupIDFlag      => stat.st_mode & S_ISGID != 0,
        SymLink          => true,
        FIFO             => file_type == S_IFIFO,
        Readable         => perm(stat, Read),
        Socket           => file_type == S_IFSOCK,
        NonEmpty         => stat.st_size > 0,
        UserIDFlag       => stat.st_mode & S_ISUID != 0,
        Writable         => perm(stat, Write),
        Executable       => perm(stat, Execute),
    }
}

#[cfg(windows)]
fn path(path: &[u8], cond: PathCondition) -> bool {
    use std::io::{TypeFile, TypeDirectory, TypeBlockSpecial, TypeNamedPipe};
    use std::io::fs::{stat};
    use std::path::{Path};

    let path = match Path::new_opt(path) {
        Some(p) => p,
        None => return false,
    };
    let stat = match stat(&path) {
        Ok(s) => s,
        _ => return false,
    };
    match cond {
        BlockSpecial     => stat.kind == TypeBlockSpecial,
        CharacterSpecial => false,
        Directory        => stat.kind == TypeDirectory,
        Exists           => true,
        Regular          => stat.kind == TypeFile,
        GroupIDFlag      => false,
        SymLink          => false,
        FIFO             => stat.kind == TypeNamedPipe,
        Readable         => false, // TODO
        Socket           => false, // TODO?
        NonEmpty         => stat.size > 0,
        UserIDFlag       => false,
        Writable         => false, // TODO
        Executable       => false, // TODO
    }
}
