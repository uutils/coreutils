#![crate_name = "uu_stat"]

// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

extern crate users;

extern crate getopts;
use getopts::Options;

#[macro_use]
mod fsext;
use fsext::*;

#[macro_use]
extern crate uucore;

use std::{fs, iter, cmp};
use std::io::Write;
use std::borrow::Cow;
// use std::error::Error;
use std::os::unix::fs::{FileTypeExt, MetadataExt};

#[cfg(test)]
mod test_stat;

macro_rules! check_bound {
    ($str: ident, $bound:expr, $beg: expr, $end: expr) => (
        if $end >= $bound {
            return Err(format!("‘{}’: invalid directive", &$str[$beg..$end]));
        }

    )
}
macro_rules! fill_string {
    ($str: ident, $c: expr, $cnt: expr) => (
        iter::repeat($c).take($cnt).map(|c| $str.push(c)).all(|_| true)
    )
}

macro_rules! extend_digits {
    ($str: ident, $min: expr) => (
        if $min > $str.len() {
            let mut pad = String::with_capacity($min);
            fill_string!(pad, '0', $min - $str.len());
            pad.push_str($str);
            pad.into()
        } else {
            $str.into()
        }
    )
}

macro_rules! pad_and_print {
    ($result: ident, $str: ident, $left: expr, $width: expr, $padding: expr) => (
        if $str.len() < $width {
            if $left {
                $result.push_str($str.as_ref());
                fill_string!($result, $padding, $width - $str.len());
            } else {
                fill_string!($result, $padding, $width - $str.len());
                $result.push_str($str.as_ref());
            }
        } else {
            $result.push_str($str.as_ref());
        }
        print!("{}", $result);
    )
}
macro_rules! print_adjusted {
    ($str: ident, $left: expr, $width: expr, $padding: expr) => (
        let field_width = cmp::max($width, $str.len());
        let mut result = String::with_capacity(field_width);
        pad_and_print!(result, $str, $left, field_width, $padding);
    );
    ($str: ident, $left: expr, $need_prefix: expr, $prefix: expr, $width: expr, $padding: expr) => (
        let mut field_width = cmp::max($width, $str.len());
        let mut result = String::with_capacity(field_width + $prefix.len());
        if $need_prefix {
            result.push_str($prefix);
            field_width -= $prefix.len();
        }
        pad_and_print!(result, $str, $left, field_width, $padding);
    )
}

static NAME: &'static str = "stat";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub const F_ALTER: u8 = 0b1;
pub const F_ZERO: u8 = 0b10;
pub const F_LEFT: u8 = 0b100;
pub const F_SPACE: u8 = 0b1000;
pub const F_SIGN: u8 = 0b10000;
// unused at present
pub const F_GROUP: u8 = 0b100000;

#[derive(Debug, PartialEq)]
pub enum OutputType {
    Str,
    Integer,
    Unsigned,
    UnsignedHex,
    UnsignedOct,
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum Token {
    Char(char),
    Directive {
        flag: u8,
        width: usize,
        precision: i32,
        format: char,
    },
}

trait ScanNum {
    /// Return (F, offset)
    fn scan_num<F>(&self) -> Option<(F, usize)> where F: std::str::FromStr;
}

impl ScanNum for str {
    fn scan_num<F>(&self) -> Option<(F, usize)>
        where F: std::str::FromStr
    {
        let mut chars = self.chars();
        let mut i = 0;
        while let Some(c) = chars.next() {
            match c {
                '-' | '+' | '0'...'9' => i += 1,
                _ => break,
            }
        }
        if i > 0 {
            F::from_str(&self[..i]).ok().map(|x| (x, i))
        } else {
            None
        }
    }
}

pub struct Stater {
    follow: bool,
    showfs: bool,
    from_user: bool,
    files: Vec<String>,
    default_tokens: Vec<Token>,
    default_dev_tokens: Vec<Token>,
}

fn print_it(arg: &str, otype: OutputType, flag: u8, width: usize, precision: i32) {

    // If the precision is given as just '.', the precision is taken to be zero.
    // A negative precision is taken as if the precision were omitted.
    // This gives the minimum number of digits to appear for d, i, o, u, x, and X conversions,
    // the maximum number of characters to be printed from a string for s and S conversions.

    // #
    // The value should be converted to an "alternate form".
    // For o conversions, the first character of the output string  is made  zero  (by  prefixing  a 0 if it was not zero already).
    // For x and X conversions, a nonzero result has the string "0x" (or "0X" for X conversions) prepended to it.

    // 0
    // The value should be zero padded.
    // For d, i, o, u, x, X, a, A, e, E, f, F, g, and G conversions, the converted value is padded on the left with zeros rather than blanks.
    // If the 0 and - flags both appear, the 0 flag is ignored.
    // If a precision  is  given with a numeric conversion (d, i, o, u, x, and X), the 0 flag is ignored.
    // For other conversions, the behavior is undefined.

    // -
    // The converted value is to be left adjusted on the field boundary.  (The default is right justification.)
    // The  converted  value  is padded on the right with blanks, rather than on the left with blanks or zeros.
    // A - overrides a 0 if both are given.

    // ' ' (a space)
    // A blank should be left before a positive number (or empty string) produced by a signed conversion.

    // +
    // A sign (+ or -) should always be placed before a number produced by a signed conversion.
    // By default, a sign  is  used only for negative numbers.
    // A + overrides a space if both are used.

    if otype == OutputType::Unknown {
        return print!("?");
    }

    let left_align = has!(flag, F_LEFT);
    let padding_char = if has!(flag, F_ZERO) && !left_align && precision == -1 {
        '0'
    } else {
        ' '
    };

    let has_sign = has!(flag, F_SIGN) || has!(flag, F_SPACE);

    let should_alter = has!(flag, F_ALTER);
    let prefix = match otype {
        OutputType::UnsignedOct => "0",
        OutputType::UnsignedHex => "0x",
        OutputType::Integer => {
            if has!(flag, F_SIGN) {
                "+"
            } else {
                " "
            }
        }
        _ => "",
    };

    match otype {
        OutputType::Str => {
            let limit = cmp::min(precision, arg.len() as i32);
            let s: &str = if limit >= 0 {
                &arg[..limit as usize]
            } else {
                arg
            };
            print_adjusted!(s, left_align, width, ' ');
        }
        OutputType::Integer => {
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg, min_digits);
            print_adjusted!(extended, left_align, has_sign, prefix, width, padding_char);
        }
        OutputType::Unsigned => {
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg, min_digits);
            print_adjusted!(extended, left_align, width, padding_char);
        }
        OutputType::UnsignedOct => {
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg, min_digits);
            print_adjusted!(extended,
                            left_align,
                            should_alter,
                            prefix,
                            width,
                            padding_char);
        }
        OutputType::UnsignedHex => {
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg, min_digits);
            print_adjusted!(extended,
                            left_align,
                            should_alter,
                            prefix,
                            width,
                            padding_char);
        }
        _ => unreachable!(),
    }
}

impl Stater {
    pub fn generate_tokens(fmtstr: &str, use_printf: bool) -> Result<Vec<Token>, String> {

        let mut tokens = Vec::new();
        let bound = fmtstr.len();
        let chars = fmtstr.chars().collect::<Vec<char>>();

        let mut i = 0_usize;
        while i < bound {

            match chars[i] {
                '%' => {
                    let old = i;

                    i += 1;
                    if i >= bound {
                        tokens.push(Token::Char('%'));
                        continue;
                    }
                    if chars[i] == '%' {
                        tokens.push(Token::Char('%'));
                        i += 1;
                        continue;
                    }

                    let mut flag: u8 = 0;

                    while i < bound {
                        match chars[i] {
                            '#' => flag |= F_ALTER,
                            '0' => flag |= F_ZERO,
                            '-' => flag |= F_LEFT,
                            ' ' => flag |= F_SPACE,
                            '+' => flag |= F_SIGN,
                            //'\'' => flag |= F_GROUP,
                            '\'' => unimplemented!(),
                            'I' => unimplemented!(),
                            _ => break,
                        }
                        i += 1;
                    }
                    check_bound!(fmtstr, bound, old, i);

                    let mut width = 0_usize;
                    let mut precision = -1_i32;
                    let mut j = i;

                    match fmtstr[j..].scan_num::<usize>() {
                        Some((field_width, offset)) => {
                            width = field_width;
                            j += offset;
                        }
                        None => (),
                    }
                    check_bound!(fmtstr, bound, old, j);

                    if chars[j] == '.' {
                        j += 1;
                        check_bound!(fmtstr, bound, old, j);

                        match fmtstr[j..].scan_num::<i32>() {
                            Some((prec, offset)) => {
                                if prec >= 0 {
                                    precision = prec;
                                }
                                j += offset;
                            }
                            None => precision = 0,
                        }
                        check_bound!(fmtstr, bound, old, j);
                    }

                    i = j;
                    tokens.push(Token::Directive {
                        width: width,
                        flag: flag,
                        precision: precision,
                        format: chars[i],
                    })

                }
                '\\' => {
                    if !use_printf {
                        tokens.push(Token::Char('\\'));
                    } else {
                        i += 1;
                        if i >= bound {
                            show_warning!("backslash at end of format");
                            tokens.push(Token::Char('\\'));
                            continue;
                        }
                        match chars[i] {
                            'x' => {
                                // TODO: parse character
                            }
                            '0'...'7' => {
                                // TODO: parse character
                            }
                            '"' => tokens.push(Token::Char('"')),
                            '\\' => tokens.push(Token::Char('\\')),
                            'a' => tokens.push(Token::Char('\x07')),
                            'b' => tokens.push(Token::Char('\x08')),
                            'e' => tokens.push(Token::Char('\x1B')),
                            'f' => tokens.push(Token::Char('\x0c')),
                            'n' => tokens.push(Token::Char('\n')),
                            'r' => tokens.push(Token::Char('\r')),
                            'v' => tokens.push(Token::Char('\x0b')),
                            c => {
                                show_warning!("unrecognized escape '\\{}'", c);
                                tokens.push(Token::Char(c));
                            }
                        }
                    }
                }

                c => tokens.push(Token::Char(c)),
            }
            i += 1;
        }
        if !use_printf && !fmtstr.ends_with('\n') {
            tokens.push(Token::Char('\n'));
        }
        Ok(tokens)
    }

    fn new(matches: getopts::Matches) -> Result<Stater, String> {
        let fmtstr = if matches.opt_present("printf") {
            matches.opt_str("printf").expect("Invalid format string")
        } else {
            matches.opt_str("format").unwrap_or("".to_owned())
        };

        let use_printf = matches.opt_present("printf");
        let terse = matches.opt_present("terse");
        let showfs = matches.opt_present("file-system");

        let default_tokens = if fmtstr.is_empty() {
            Stater::generate_tokens(&Stater::default_fmt(showfs, terse, false), use_printf).unwrap()
        } else {
            match Stater::generate_tokens(&fmtstr, use_printf) {
                Ok(ts) => ts,
                Err(e) => return Err(e),
            }
        };
        let default_dev_tokens = Stater::generate_tokens(&Stater::default_fmt(showfs, terse, true),
                                                         use_printf)
                                     .unwrap();

        Ok(Stater {
            follow: matches.opt_present("dereference"),
            showfs: showfs,
            from_user: !fmtstr.is_empty(),
            files: matches.free,
            default_tokens: default_tokens,
            default_dev_tokens: default_dev_tokens,
        })
    }

    fn exec(&self) -> i32 {
        for f in &self.files {
            self.do_stat(f.as_str());
        }
        0
    }

    fn do_stat(&self, file: &str) {

        #[inline]
        fn get_grp_name(gid: u32) -> String {
            if let Some(g) = users::get_group_by_gid(gid) {
                g.name().to_owned()
            } else {
                "UNKNOWN".to_owned()
            }
        }

        #[inline]
        fn get_usr_name(uid: u32) -> String {
            if let Some(g) = users::get_user_by_uid(uid) {
                g.name().to_owned()
            } else {
                "UNKNOWN".to_owned()
            }
        }

        if !self.showfs {
            let result = if self.follow {
                fs::metadata(file)
            } else {
                fs::symlink_metadata(file)
            };
            match result {
                Ok(meta) => {
                    let ftype = meta.file_type();
                    let tokens = if self.from_user ||
                                    !(ftype.is_char_device() || ftype.is_block_device()) {
                        &self.default_tokens
                    } else {
                        &self.default_dev_tokens
                    };
                    let is_symlink = ftype.is_symlink();

                    for t in tokens.into_iter() {
                        match t {
                            &Token::Char(c) => print!("{}", c),
                            &Token::Directive { flag, width, precision, format } => {

                                let arg: String;
                                let otype: OutputType;

                                match format {
                                    // unsigned oct
                                    'a' => {
                                        arg = format!("{:o}", 0o7777 & meta.mode());
                                        otype = OutputType::UnsignedOct;
                                    }
                                    // string
                                    'A' => {
                                        arg = pretty_access(meta.mode());
                                        otype = OutputType::Str;
                                    }
                                    // unsigned
                                    'b' => {
                                        arg = format!("{}", meta.blocks());
                                        otype = OutputType::Unsigned;
                                    }

                                    // unsigned
                                    // FIXME: blocksize differs on various platform
                                    // See coreutils/gnulib/lib/stat-size.h ST_NBLOCKSIZE
                                    'B' => {
                                        // the size in bytes of each block reported by %b
                                        arg = format!("{}", 512);
                                        otype = OutputType::Unsigned;
                                    }

                                    // unsigned
                                    'd' => {
                                        arg = format!("{}", meta.dev());
                                        otype = OutputType::Unsigned;
                                    }
                                    // unsigned hex
                                    'D' => {
                                        arg = format!("{:x}", meta.dev());
                                        otype = OutputType::UnsignedHex;
                                    }
                                    // unsigned hex
                                    'f' => {
                                        arg = format!("{:x}", meta.mode());
                                        otype = OutputType::UnsignedHex;
                                    }
                                    // string
                                    'F' => {
                                        arg = pretty_filetype(meta.mode(), meta.len()).to_owned();
                                        otype = OutputType::Str;
                                    }
                                    // unsigned
                                    'g' => {
                                        arg = format!("{}", meta.gid());
                                        otype = OutputType::Unsigned;
                                    }
                                    // string
                                    'G' => {
                                        arg = get_grp_name(meta.gid());
                                        otype = OutputType::Str;
                                    }
                                    // unsigned
                                    'h' => {
                                        arg = format!("{}", meta.nlink());
                                        otype = OutputType::Unsigned;
                                    }
                                    // unsigned
                                    'i' => {
                                        arg = format!("{}", meta.ino());
                                        otype = OutputType::Unsigned;
                                    }

                                    // string
                                    // FIXME:
                                    'm' => {
                                        // mount point
                                        arg = "/".to_owned();
                                        otype = OutputType::Str;
                                    }

                                    // string
                                    'n' => {
                                        arg = file.to_owned();
                                        otype = OutputType::Str;
                                    }
                                    // string
                                    'N' => {
                                        if is_symlink {
                                            arg = format!("'{}' -> '{}'",
                                                          file,
                                                          fs::read_link(file)
                                                              .expect("Invalid symlink")
                                                              .to_string_lossy());
                                        } else {
                                            arg = format!("'{}'", file);
                                        }
                                        otype = OutputType::Str;
                                    }
                                    // unsigned
                                    'o' => {
                                        arg = format!("{}", meta.blksize());
                                        otype = OutputType::Unsigned;
                                    }
                                    // int
                                    's' => {
                                        arg = format!("{}", meta.len());
                                        otype = OutputType::Integer;
                                    }
                                    // unsigned hex
                                    't' => {
                                        arg = format!("{:x}", meta.rdev() >> 8);
                                        otype = OutputType::UnsignedHex;
                                    }
                                    // unsigned hex
                                    'T' => {
                                        arg = format!("{:x}", meta.rdev() & 0xff);
                                        otype = OutputType::UnsignedHex;
                                    }
                                    // unsigned
                                    'u' => {
                                        arg = format!("{}", meta.uid());
                                        otype = OutputType::Unsigned;
                                    }
                                    // string
                                    'U' => {
                                        arg = get_usr_name(meta.uid());
                                        otype = OutputType::Str;
                                    }

                                    // string
                                    // FIXME:
                                    'w' => {
                                        // time of file birth, human-readable; - if unknown
                                        arg = "-".to_owned();
                                        otype = OutputType::Str;
                                    }

                                    // int
                                    // FIXME:
                                    'W' => {
                                        // time of file birth, seconds since Epoch; 0 if unknown
                                        arg = format!("{}", 0);
                                        otype = OutputType::Integer;
                                    }

                                    // string
                                    'x' => {
                                        arg = pretty_time(meta.atime(), meta.atime_nsec());
                                        otype = OutputType::Str;
                                    }
                                    // int
                                    'X' => {
                                        arg = format!("{}", meta.atime());
                                        otype = OutputType::Integer;
                                    }
                                    // string
                                    'y' => {
                                        arg = pretty_time(meta.mtime(), meta.mtime_nsec());
                                        otype = OutputType::Str;
                                    }
                                    // int
                                    'Y' => {
                                        arg = format!("{}", meta.mtime());
                                        otype = OutputType::Str;
                                    }
                                    // string
                                    'z' => {
                                        arg = pretty_time(meta.ctime(), meta.ctime_nsec());
                                        otype = OutputType::Str;
                                    }
                                    // int
                                    'Z' => {
                                        arg = format!("{}", meta.ctime());
                                        otype = OutputType::Integer;
                                    }

                                    _ => {
                                        arg = "?".to_owned();
                                        otype = OutputType::Unknown;
                                    }
                                }
                                print_it(&arg, otype, flag, width, precision);
                            }
                        }
                    }
                }
                Err(e) => {
                    show_info!("cannot stat '{}': {}", file, e);
                    return;
                }
            }
        } else {
            match statfs(file) {
                Ok(data) => {
                    let tokens = &self.default_tokens;

                    for t in tokens.into_iter() {
                        match t {
                            &Token::Char(c) => print!("{}", c),
                            &Token::Directive { flag, width, precision, format } => {

                                let arg: String;
                                let otype: OutputType;
                                match format {
                                    // int
                                    'a' => {
                                        arg = format!("{}", data.f_bavail);
                                        otype = OutputType::Integer;
                                    }
                                    // int
                                    'b' => {
                                        arg = format!("{}", data.f_blocks);
                                        otype = OutputType::Integer;
                                    }
                                    // unsigned
                                    'c' => {
                                        arg = format!("{}", data.f_files);
                                        otype = OutputType::Unsigned;
                                    }
                                    // int
                                    'd' => {
                                        arg = format!("{}", data.f_ffree);
                                        otype = OutputType::Integer;
                                    }
                                    // int
                                    'f' => {
                                        arg = format!("{}", data.f_bfree);
                                        otype = OutputType::Integer;
                                    }
                                    // hex unsigned
                                    'i' => {
                                        arg = format!("{:x}", data.f_fsid);
                                        otype = OutputType::UnsignedHex;
                                    }
                                    // unsigned
                                    'l' => {
                                        arg = format!("{}", data.f_namelen);
                                        otype = OutputType::Unsigned;
                                    }
                                    // string
                                    'n' => {
                                        arg = file.to_owned();
                                        otype = OutputType::Str;
                                    }
                                    // unsigned
                                    's' => {
                                        arg = format!("{}", data.f_bsize);
                                        otype = OutputType::Unsigned;
                                    }
                                    // unsigned
                                    'S' => {
                                        arg = format!("{}", data.f_frsize);
                                        otype = OutputType::Unsigned;
                                    }
                                    // hex unsigned
                                    't' => {
                                        arg = format!("{:x}", data.f_type);
                                        otype = OutputType::UnsignedHex;
                                    }
                                    // string
                                    'T' => {
                                        arg = pretty_fstype(data.f_type).into_owned();
                                        otype = OutputType::Str;
                                    }
                                    _ => {
                                        arg = "?".to_owned();
                                        otype = OutputType::Unknown;
                                    }
                                }

                                print_it(&arg, otype, flag, width, precision);
                            }
                        }
                    }
                }
                Err(e) => {
                    show_info!("cannot stat '{}': {}", file, e);
                    return;
                }
            }
        }
    }

    // taken from coreutils/src/stat.c
    fn default_fmt(showfs: bool, terse: bool, dev: bool) -> String {

        // SELinux related format is *ignored*

        // 36 is taken randomly
        let mut fmtstr = String::with_capacity(36);
        if showfs {
            if terse {
                fmtstr.push_str("%n %i %l %t %s %S %b %f %a %c %d\n");
            } else {
                fmtstr.push_str("  File: \"%n\"\n    ID: %-8i Namelen: %-7l Type: %T\nBlock \
                                 size: %-10s Fundamental block size: %S\nBlocks: Total: %-10b \
                                 Free: %-10f Available: %a\nInodes: Total: %-10c Free: %d\n");
            }
        } else if terse {
            fmtstr.push_str("%n %s %b %f %u %g %D %i %h %t %T %X %Y %Z %W %o\n");
        } else {
            fmtstr.push_str("  File: %N\n  Size: %-10s\tBlocks: %-10b IO Block: %-6o %F\n");
            if dev {
                fmtstr.push_str("Device: %Dh/%dd\tInode: %-10i  Links: %-5h Device type: %t,%T\n");
            } else {
                fmtstr.push_str("Device: %Dh/%dd\tInode: %-10i  Links: %h\n");
            }
            fmtstr.push_str("Access: (%04a/%10.10A)  Uid: (%5u/%8U)   Gid: (%5g/%8G)\n");
            fmtstr.push_str("Access: %x\nModify: %y\nChange: %z\n Birth: %w\n");
        }
        fmtstr
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    opts.optflag("L", "dereference", "follow links");
    opts.optflag("f",
                 "file-system",
                 "display file system status instead of file status");
    opts.optflag("t", "terse", "print the information in terse form");

    // Omit the unused description as they are too long
    opts.optopt("c", "format", "", "FORMAT");
    opts.optopt("", "printf", "", "FORMAT");


    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            disp_err!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        return help();
    } else if matches.opt_present("version") {
        return version();
    }

    if matches.free.is_empty() {
        disp_err!("missing operand");
        return 1;
    }

    match Stater::new(matches) {
        // FIXME: Handle error
        Ok(stater) => stater.exec(),
        Err(e) => {
            show_info!("{}", e);
            return 1;
        }
    }
}

fn version() -> i32 {
    println!("{} {}", NAME, VERSION);
    0
}

fn help() -> i32 {
    let msg = format!(r#"Usage: {} [OPTION]... FILE...
Display file or file system status.

Mandatory arguments to long options are mandatory for short options too.
  -L, --dereference     follow links
  -f, --file-system     display file system status instead of file status
  -c  --format=FORMAT   use the specified FORMAT instead of the default;
                          output a newline after each use of FORMAT
      --printf=FORMAT   like --format, but interpret backslash escapes,
                          and do not output a mandatory trailing newline;
                          if you want a newline, include \n in FORMAT
  -t, --terse           print the information in terse form
      --help     display this help and exit
      --version  output version information and exit

The valid format sequences for files (without --file-system):

  %a   access rights in octal (note '#' and '0' printf flags)
  %A   access rights in human readable form
  %b   number of blocks allocated (see %B)
  %B   the size in bytes of each block reported by %b
  %C   SELinux security context string
  %d   device number in decimal
  %D   device number in hex
  %f   raw mode in hex
  %F   file type
  %g   group ID of owner
  %G   group name of owner
  %h   number of hard links
  %i   inode number
  %m   mount point
  %n   file name
  %N   quoted file name with dereference if symbolic link
  %o   optimal I/O transfer size hint
  %s   total size, in bytes
  %t   major device type in hex, for character/block device special files
  %T   minor device type in hex, for character/block device special files
  %u   user ID of owner
  %U   user name of owner
  %w   time of file birth, human-readable; - if unknown
  %W   time of file birth, seconds since Epoch; 0 if unknown
  %x   time of last access, human-readable
  %X   time of last access, seconds since Epoch
  %y   time of last data modification, human-readable
  %Y   time of last data modification, seconds since Epoch
  %z   time of last status change, human-readable
  %Z   time of last status change, seconds since Epoch

Valid format sequences for file systems:

  %a   free blocks available to non-superuser
  %b   total data blocks in file system
  %c   total file nodes in file system
  %d   free file nodes in file system
  %f   free blocks in file system
  %i   file system ID in hex
  %l   maximum length of filenames
  %n   file name
  %s   block size (for faster transfers)
  %S   fundamental block size (for block counts)
  %t   file system type in hex
  %T   file system type in human readable form

NOTE: your shell may have its own version of stat, which usually supersedes
the version described here.  Please refer to your shell's documentation
for details about the options it supports."#,
                      NAME);
    println!("{}", msg);
    0
}
