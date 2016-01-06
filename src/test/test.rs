#![crate_name = "uu_test"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) mahkoh (ju.orth [at] gmail [dot] com)
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate libc;

use std::collections::HashMap;
use std::ffi::OsString;
use std::env::{args_os};
use std::str::{from_utf8};

static NAME: &'static str = "test";

// TODO: decide how to handle non-UTF8 input for all the utils
// Definitely don't use [u8], try keeping it as OsStr or OsString instead
pub fn uumain(_: Vec<String>) -> i32 {
    let args = args_os().collect::<Vec<OsString>>();
    // This is completely disregarding valid windows paths that aren't valid unicode
    let args = args.iter().map(|a| a.to_str().unwrap().as_bytes()).collect::<Vec<&[u8]>>();
    if args.is_empty() {
        return 2;
    }
    let args =
        if !args[0].ends_with(NAME.as_bytes()) {
            &args[1..]
        } else {
            &args[..]
        };
    let args = match args[0] {
        b"[" => match args[args.len() - 1] {
            b"]" => &args[1..args.len() - 1],
            _ => return 2,
        },
        _ => &args[1..args.len()],
    };
    let mut error = false;
    let retval = 1 - parse_expr(args, &mut error) as i32;
    if error {
        2
    } else {
        retval
    }
}

fn one(args: &[&[u8]]) -> bool {
    args[0].len() > 0
}

fn two(args: &[&[u8]], error: &mut bool) -> bool {
    match args[0] {
        b"!" => !one(&args[1..]),
        b"-b" => path(args[1], PathCondition::BlockSpecial),
        b"-c" => path(args[1], PathCondition::CharacterSpecial),
        b"-d" => path(args[1], PathCondition::Directory),
        b"-e" => path(args[1], PathCondition::Exists),
        b"-f" => path(args[1], PathCondition::Regular),
        b"-g" => path(args[1], PathCondition::GroupIDFlag),
        b"-h" => path(args[1], PathCondition::SymLink),
        b"-L" => path(args[1], PathCondition::SymLink),
        b"-n" => one(&args[1..]),
        b"-p" => path(args[1], PathCondition::FIFO),
        b"-r" => path(args[1], PathCondition::Readable),
        b"-S" => path(args[1], PathCondition::Socket),
        b"-s" => path(args[1], PathCondition::NonEmpty),
        b"-t" => isatty(args[1]),
        b"-u" => path(args[1], PathCondition::UserIDFlag),
        b"-w" => path(args[1], PathCondition::Writable),
        b"-x" => path(args[1], PathCondition::Executable),
        b"-z" => !one(&args[1..]),
        _ => {
            *error = true;
            false
        }
    }
}

fn three(args: &[&[u8]], error: &mut bool) -> bool {
    match args[1] {
        b"=" => args[0] == args[2],
        b"==" => args[0] == args[2],
        b"!=" => args[0] != args[2],
        b"-eq" => integers(args[0], args[2], IntegerCondition::Equal),
        b"-ne" => integers(args[0], args[2], IntegerCondition::Unequal),
        b"-gt" => integers(args[0], args[2], IntegerCondition::Greater),
        b"-ge" => integers(args[0], args[2], IntegerCondition::GreaterEqual),
        b"-lt" => integers(args[0], args[2], IntegerCondition::Less),
        b"-le" => integers(args[0], args[2], IntegerCondition::LessEqual),
        _ => match args[0] {
            b"!" => !two(&args[1..], error),
            _ => {
                *error = true;
                false
            }
        }
    }
}

fn four(args: &[&[u8]], error: &mut bool) -> bool {
    match args[0] {
        b"!" => {
            !three(&args[1..], error)
        }
        _ => {
            *error = true;
            false
        }
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
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let (a, b): (i64, i64) = match (a.parse(), b.parse()) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    match cond {
        IntegerCondition::Equal        => a == b,
        IntegerCondition::Unequal      => a != b,
        IntegerCondition::Greater      => a >  b,
        IntegerCondition::GreaterEqual => a >= b,
        IntegerCondition::Less         => a <  b,
        IntegerCondition::LessEqual    => a <= b,
    }
}

fn isatty(fd: &[u8]) -> bool {
    use libc::{isatty};
    from_utf8(fd).ok().and_then(|s| s.parse().ok())
            .map_or(false, |i| unsafe { isatty(i) == 1 })
}

fn dispatch(args: &mut &[&[u8]], error: &mut bool) -> bool {
    let (val, idx) = match args.len() {
        0 => {
            *error = true;
            (false, 0)
        }
        1 => (one(*args), 1),
        2 => dispatch_two(args, error),
        3 => dispatch_three(args, error),
        _ => dispatch_four(args, error)
    };
    *args = &(*args)[idx..];
    val
}

fn dispatch_two(args: &mut &[&[u8]], error: &mut bool) -> (bool, usize) {
    let val = two(*args, error);
    if *error {
        *error = false;
        (one(*args), 1)
    } else {
        (val, 2)
    }
}

fn dispatch_three(args: &mut &[&[u8]], error: &mut bool) -> (bool, usize) {
    let val = three(*args, error);
    if *error {
        *error = false;
        dispatch_two(args, error)
    } else {
        (val, 3)
    }
}

fn dispatch_four(args: &mut &[&[u8]], error: &mut bool) -> (bool, usize) {
    let val = four(*args, error);
    if *error {
        *error = false;
        dispatch_three(args, error)
    } else {
        (val, 4)
    }
}

#[derive(Clone, Copy)]
enum Precedence {
    Unknown = 0,
    Paren,     // FIXME: this is useless (parentheses have not been implemented)
    Or,
    And,
    BUnOp,
    BinOp,
    UnOp
}

fn parse_expr(mut args: &[&[u8]], error: &mut bool) -> bool {
    if args.len() == 0 {
        false
    } else {
        let hashmap = setup_hashmap();
        let lhs = dispatch(&mut args, error);

        if args.len() > 0 {
            parse_expr_helper(&hashmap, &mut args, lhs, Precedence::Unknown, error)
        } else {
            lhs
        }
    }
}

fn parse_expr_helper<'a>(hashmap: &HashMap<&'a [u8], Precedence>,
                         args: &mut &[&'a [u8]],
                         mut lhs: bool,
                         min_prec: Precedence,
                         error: &mut bool) -> bool {
    let mut prec = *hashmap.get(&args[0]).unwrap_or_else(|| {
        *error = true;
        &min_prec
    });
    while !*error && args.len() > 0 && prec as usize >= min_prec as usize {
        let op = args[0];
        *args = &(*args)[1..];
        let mut rhs = dispatch(args, error);
        while args.len() > 0 {
            let subprec = *hashmap.get(&args[0]).unwrap_or_else(|| {
                *error = true;
                &min_prec
            });
            if subprec as usize <= prec as usize || *error {
                break;
            }
            rhs = parse_expr_helper(hashmap, args, rhs, subprec, error);
        }
        lhs = match prec {
            Precedence::UnOp | Precedence::BUnOp => {
                *error = true;
                false
            }
            Precedence::And => lhs && rhs,
            Precedence::Or => lhs || rhs,
            Precedence::BinOp => three(&[if lhs { b" " } else { b"" }, op, if rhs { b" " } else { b"" }], error),
            Precedence::Paren => unimplemented!(),  // TODO: implement parentheses
            _ => unreachable!()
        };
        if args.len() > 0 {
            prec = *hashmap.get(&args[0]).unwrap_or_else(|| {
                *error = true;
                &min_prec
            });
        }
    }
    lhs
}

#[inline]
fn setup_hashmap<'a>() -> HashMap<&'a [u8], Precedence> {
    let mut hashmap = HashMap::<&'a [u8], Precedence>::new();

    hashmap.insert(b"-b", Precedence::UnOp);
    hashmap.insert(b"-c", Precedence::UnOp);
    hashmap.insert(b"-d", Precedence::UnOp);
    hashmap.insert(b"-e", Precedence::UnOp);
    hashmap.insert(b"-f", Precedence::UnOp);
    hashmap.insert(b"-g", Precedence::UnOp);
    hashmap.insert(b"-h", Precedence::UnOp);
    hashmap.insert(b"-L", Precedence::UnOp);
    hashmap.insert(b"-n", Precedence::UnOp);
    hashmap.insert(b"-p", Precedence::UnOp);
    hashmap.insert(b"-r", Precedence::UnOp);
    hashmap.insert(b"-S", Precedence::UnOp);
    hashmap.insert(b"-s", Precedence::UnOp);
    hashmap.insert(b"-t", Precedence::UnOp);
    hashmap.insert(b"-u", Precedence::UnOp);
    hashmap.insert(b"-w", Precedence::UnOp);
    hashmap.insert(b"-x", Precedence::UnOp);
    hashmap.insert(b"-z", Precedence::UnOp);

    hashmap.insert(b"=", Precedence::BinOp);
    hashmap.insert(b"!=", Precedence::BinOp);
    hashmap.insert(b"-eq", Precedence::BinOp);
    hashmap.insert(b"-ne", Precedence::BinOp);
    hashmap.insert(b"-gt", Precedence::BinOp);
    hashmap.insert(b"-ge", Precedence::BinOp);
    hashmap.insert(b"-lt", Precedence::BinOp);
    hashmap.insert(b"-le", Precedence::BinOp);

    hashmap.insert(b"!", Precedence::BUnOp);

    hashmap.insert(b"-a", Precedence::And);
    hashmap.insert(b"-o", Precedence::Or);

    hashmap.insert(b"(", Precedence::Paren);
    hashmap.insert(b")", Precedence::Paren);

    hashmap
}

#[derive(Eq, PartialEq)]
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
    use std::ffi::CString;

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
            stat.st_mode & ((p as mode_t) << 6) != 0
        } else if gid == stat.st_gid {
            stat.st_mode & ((p as mode_t) << 3) != 0
        } else {
            stat.st_mode & ((p as mode_t)) != 0
        }
    };

    let path = CString::new(path).unwrap();
    let mut stat = unsafe { std::mem::zeroed() };
    if cond == PathCondition::SymLink {
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
        PathCondition::BlockSpecial     => file_type == S_IFBLK,
        PathCondition::CharacterSpecial => file_type == S_IFCHR,
        PathCondition::Directory        => file_type == S_IFDIR,
        PathCondition::Exists           => true,
        PathCondition::Regular          => file_type == S_IFREG,
        PathCondition::GroupIDFlag      => stat.st_mode & S_ISGID != 0,
        PathCondition::SymLink          => true,
        PathCondition::FIFO             => file_type == S_IFIFO,
        PathCondition::Readable         => perm(stat, Permission::Read),
        PathCondition::Socket           => file_type == S_IFSOCK,
        PathCondition::NonEmpty         => stat.st_size > 0,
        PathCondition::UserIDFlag       => stat.st_mode & S_ISUID != 0,
        PathCondition::Writable         => perm(stat, Permission::Write),
        PathCondition::Executable       => perm(stat, Permission::Execute),
    }
}

#[cfg(windows)]
fn path(path: &[u8], cond: PathCondition) -> bool {
    use std::fs::metadata;
    let path = from_utf8(path).unwrap();
    let stat = match metadata(path) {
        Ok(s) => s,
        _ => return false,
    };
    match cond {
        PathCondition::BlockSpecial     => false,
        PathCondition::CharacterSpecial => false,
        PathCondition::Directory        => stat.is_dir(),
        PathCondition::Exists           => true,
        PathCondition::Regular          => stat.is_file(),
        PathCondition::GroupIDFlag      => false,
        PathCondition::SymLink          => false,
        PathCondition::FIFO             => false,
        PathCondition::Readable         => false, // TODO
        PathCondition::Socket           => false,
        PathCondition::NonEmpty         => stat.len() > 0,
        PathCondition::UserIDFlag       => false,
        PathCondition::Writable         => false, // TODO
        PathCondition::Executable       => false, // TODO
    }
}
