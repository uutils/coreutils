// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

#[macro_use]
extern crate uucore;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::fs::display_permissions;
use uucore::fsext::{
    pretty_filetype, pretty_fstype, pretty_time, read_fs_list, statfs, BirthTime, FsMeta,
};
use uucore::libc::mode_t;
use uucore::{entries, format_usage};

use clap::{crate_version, Arg, ArgMatches, Command};
use std::borrow::Cow;
use std::convert::AsRef;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
use std::{cmp, fs, iter};

macro_rules! check_bound {
    ($str: ident, $bound:expr, $beg: expr, $end: expr) => {
        if $end >= $bound {
            return Err(USimpleError::new(
                1,
                format!("{}: invalid directive", $str[$beg..$end].quote()),
            ));
        }
    };
}
macro_rules! fill_string {
    ($str: ident, $c: expr, $cnt: expr) => {
        iter::repeat($c)
            .take($cnt)
            .map(|c| $str.push(c))
            .all(|_| true)
    };
}
macro_rules! extend_digits {
    ($str: expr, $min: expr) => {
        if $min > $str.len() {
            let mut pad = String::with_capacity($min);
            fill_string!(pad, '0', $min - $str.len());
            pad.push_str($str);
            pad.into()
        } else {
            $str.into()
        }
    };
}
macro_rules! pad_and_print {
    ($result: ident, $str: ident, $left: expr, $width: expr, $padding: expr) => {
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
    };
}
macro_rules! print_adjusted {
    ($str: ident, $left: expr, $width: expr, $padding: expr) => {
        let field_width = cmp::max($width, $str.len());
        let mut result = String::with_capacity(field_width);
        pad_and_print!(result, $str, $left, field_width, $padding);
    };
    ($str: ident, $left: expr, $need_prefix: expr, $prefix: expr, $width: expr, $padding: expr) => {
        let mut field_width = cmp::max($width, $str.len());
        let mut result = String::with_capacity(field_width + $prefix.len());
        if $need_prefix {
            result.push_str($prefix);
            field_width -= $prefix.len();
        }
        pad_and_print!(result, $str, $left, field_width, $padding);
    };
}

static ABOUT: &str = "Display file or file system status.";
const USAGE: &str = "{} [OPTION]... FILE...";

pub mod options {
    pub static DEREFERENCE: &str = "dereference";
    pub static FILE_SYSTEM: &str = "file-system";
    pub static FORMAT: &str = "format";
    pub static PRINTF: &str = "printf";
    pub static TERSE: &str = "terse";
}

static ARG_FILES: &str = "files";

pub const F_ALTER: u8 = 1;
pub const F_ZERO: u8 = 1 << 1;
pub const F_LEFT: u8 = 1 << 2;
pub const F_SPACE: u8 = 1 << 3;
pub const F_SIGN: u8 = 1 << 4;
// unused at present
pub const F_GROUP: u8 = 1 << 5;

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

pub trait ScanUtil {
    fn scan_num<F>(&self) -> Option<(F, usize)>
    where
        F: std::str::FromStr;
    fn scan_char(&self, radix: u32) -> Option<(char, usize)>;
}

impl ScanUtil for str {
    fn scan_num<F>(&self) -> Option<(F, usize)>
    where
        F: std::str::FromStr,
    {
        let mut chars = self.chars();
        let mut i = 0;
        match chars.next() {
            Some('-') | Some('+') | Some('0'..='9') => i += 1,
            _ => return None,
        }
        for c in chars {
            match c {
                '0'..='9' => i += 1,
                _ => break,
            }
        }
        if i > 0 {
            F::from_str(&self[..i]).ok().map(|x| (x, i))
        } else {
            None
        }
    }

    fn scan_char(&self, radix: u32) -> Option<(char, usize)> {
        let count = match radix {
            8 => 3_usize,
            16 => 2,
            _ => return None,
        };
        let chars = self.chars().enumerate();
        let mut res = 0_u32;
        let mut offset = 0_usize;
        for (i, c) in chars {
            if i >= count {
                break;
            }
            match c.to_digit(radix) {
                Some(digit) => {
                    let tmp = res * radix + digit;
                    if tmp < 256 {
                        res = tmp;
                    } else {
                        break;
                    }
                }
                None => break,
            }
            offset = i + 1;
        }
        if offset > 0 {
            Some((res as u8 as char, offset))
        } else {
            None
        }
    }
}

pub fn group_num(s: &str) -> Cow<str> {
    let is_negative = s.starts_with('-');
    assert!(is_negative || s.chars().take(1).all(|c| c.is_digit(10)));
    assert!(s.chars().skip(1).all(|c| c.is_digit(10)));
    if s.len() < 4 {
        return s.into();
    }
    let mut res = String::with_capacity((s.len() - 1) / 3);
    let s = if is_negative {
        res.push('-');
        &s[1..]
    } else {
        s
    };
    let mut alone = (s.len() - 1) % 3 + 1;
    res.push_str(&s[..alone]);
    while alone != s.len() {
        res.push(',');
        res.push_str(&s[alone..alone + 3]);
        alone += 3;
    }
    res.into()
}

pub struct Stater {
    follow: bool,
    show_fs: bool,
    from_user: bool,
    files: Vec<String>,
    mount_list: Option<Vec<String>>,
    default_tokens: Vec<Token>,
    default_dev_tokens: Vec<Token>,
}

#[allow(clippy::cognitive_complexity)]
fn print_it(arg: &str, output_type: &OutputType, flag: u8, width: usize, precision: i32) {
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

    if output_type == &OutputType::Unknown {
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
    let prefix = match output_type {
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

    match output_type {
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
            let arg = if has!(flag, F_GROUP) {
                group_num(arg)
            } else {
                Cow::Borrowed(arg)
            };
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg.as_ref(), min_digits);
            print_adjusted!(extended, left_align, has_sign, prefix, width, padding_char);
        }
        OutputType::Unsigned => {
            let arg = if has!(flag, F_GROUP) {
                group_num(arg)
            } else {
                Cow::Borrowed(arg)
            };
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg.as_ref(), min_digits);
            print_adjusted!(extended, left_align, width, padding_char);
        }
        OutputType::UnsignedOct => {
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg, min_digits);
            print_adjusted!(
                extended,
                left_align,
                should_alter,
                prefix,
                width,
                padding_char
            );
        }
        OutputType::UnsignedHex => {
            let min_digits = cmp::max(precision, arg.len() as i32) as usize;
            let extended: Cow<str> = extend_digits!(arg, min_digits);
            print_adjusted!(
                extended,
                left_align,
                should_alter,
                prefix,
                width,
                padding_char
            );
        }
        _ => unreachable!(),
    }
}

impl Stater {
    pub fn generate_tokens(format_str: &str, use_printf: bool) -> UResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let bound = format_str.len();
        let chars = format_str.chars().collect::<Vec<char>>();
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
                            '\'' => flag |= F_GROUP,
                            'I' => unimplemented!(),
                            _ => break,
                        }
                        i += 1;
                    }
                    check_bound!(format_str, bound, old, i);

                    let mut width = 0_usize;
                    let mut precision = -1_i32;
                    let mut j = i;

                    if let Some((field_width, offset)) = format_str[j..].scan_num::<usize>() {
                        width = field_width;
                        j += offset;
                    }
                    check_bound!(format_str, bound, old, j);

                    if chars[j] == '.' {
                        j += 1;
                        check_bound!(format_str, bound, old, j);

                        match format_str[j..].scan_num::<i32>() {
                            Some((value, offset)) => {
                                if value >= 0 {
                                    precision = value;
                                }
                                j += offset;
                            }
                            None => precision = 0,
                        }
                        check_bound!(format_str, bound, old, j);
                    }

                    i = j;
                    tokens.push(Token::Directive {
                        width,
                        flag,
                        precision,
                        format: chars[i],
                    });
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
                            'x' if i + 1 < bound => {
                                if let Some((c, offset)) = format_str[i + 1..].scan_char(16) {
                                    tokens.push(Token::Char(c));
                                    i += offset;
                                } else {
                                    show_warning!("unrecognized escape '\\x'");
                                    tokens.push(Token::Char('x'));
                                }
                            }
                            '0'..='7' => {
                                let (c, offset) = format_str[i..].scan_char(8).unwrap();
                                tokens.push(Token::Char(c));
                                i += offset - 1;
                            }
                            '"' => tokens.push(Token::Char('"')),
                            '\\' => tokens.push(Token::Char('\\')),
                            'a' => tokens.push(Token::Char('\x07')),
                            'b' => tokens.push(Token::Char('\x08')),
                            'e' => tokens.push(Token::Char('\x1B')),
                            'f' => tokens.push(Token::Char('\x0C')),
                            'n' => tokens.push(Token::Char('\n')),
                            'r' => tokens.push(Token::Char('\r')),
                            't' => tokens.push(Token::Char('\t')),
                            'v' => tokens.push(Token::Char('\x0B')),
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
        if !use_printf && !format_str.ends_with('\n') {
            tokens.push(Token::Char('\n'));
        }
        Ok(tokens)
    }

    fn new(matches: &ArgMatches) -> UResult<Self> {
        let files: Vec<String> = matches
            .values_of(ARG_FILES)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default();

        let format_str = if matches.is_present(options::PRINTF) {
            matches
                .value_of(options::PRINTF)
                .expect("Invalid format string")
        } else {
            matches.value_of(options::FORMAT).unwrap_or("")
        };

        let use_printf = matches.is_present(options::PRINTF);
        let terse = matches.is_present(options::TERSE);
        let show_fs = matches.is_present(options::FILE_SYSTEM);

        let default_tokens = if format_str.is_empty() {
            Self::generate_tokens(&Self::default_format(show_fs, terse, false), use_printf)?
        } else {
            Self::generate_tokens(format_str, use_printf)?
        };
        let default_dev_tokens =
            Self::generate_tokens(&Self::default_format(show_fs, terse, true), use_printf)?;

        let mount_list = if show_fs {
            // mount points aren't displayed when showing filesystem information
            None
        } else {
            let mut mount_list = read_fs_list()
                .iter()
                .map(|mi| mi.mount_dir.clone())
                .collect::<Vec<String>>();
            // Reverse sort. The longer comes first.
            mount_list.sort();
            mount_list.reverse();
            Some(mount_list)
        };

        Ok(Self {
            follow: matches.is_present(options::DEREFERENCE),
            show_fs,
            from_user: !format_str.is_empty(),
            files,
            default_tokens,
            default_dev_tokens,
            mount_list,
        })
    }

    fn find_mount_point<P: AsRef<Path>>(&self, p: P) -> Option<String> {
        let path = match p.as_ref().canonicalize() {
            Ok(s) => s,
            Err(_) => return None,
        };
        if let Some(ref mount_list) = self.mount_list {
            for root in mount_list.iter() {
                if path.starts_with(root) {
                    return Some(root.clone());
                }
            }
        }
        None
    }

    fn exec(&self) -> i32 {
        let mut ret = 0;
        for f in &self.files {
            ret |= self.do_stat(f.as_str());
        }
        ret
    }

    fn do_stat(&self, file: &str) -> i32 {
        if !self.show_fs {
            let result = if self.follow {
                fs::metadata(file)
            } else {
                fs::symlink_metadata(file)
            };
            match result {
                Ok(meta) => {
                    let file_type = meta.file_type();
                    let tokens = if self.from_user
                        || !(file_type.is_char_device() || file_type.is_block_device())
                    {
                        &self.default_tokens
                    } else {
                        &self.default_dev_tokens
                    };

                    for t in tokens.iter() {
                        match *t {
                            Token::Char(c) => print!("{}", c),
                            Token::Directive {
                                flag,
                                width,
                                precision,
                                format,
                            } => {
                                let arg: String;
                                let output_type: OutputType;

                                match format {
                                    // access rights in octal
                                    'a' => {
                                        arg = format!("{:o}", 0o7777 & meta.mode());
                                        output_type = OutputType::UnsignedOct;
                                    }
                                    // access rights in human readable form
                                    'A' => {
                                        arg = display_permissions(&meta, true);
                                        output_type = OutputType::Str;
                                    }
                                    // number of blocks allocated (see %B)
                                    'b' => {
                                        arg = format!("{}", meta.blocks());
                                        output_type = OutputType::Unsigned;
                                    }

                                    // the size in bytes of each block reported by %b
                                    // FIXME: blocksize differs on various platform
                                    // See coreutils/gnulib/lib/stat-size.h ST_NBLOCKSIZE // spell-checker:disable-line
                                    'B' => {
                                        // the size in bytes of each block reported by %b
                                        arg = format!("{}", 512);
                                        output_type = OutputType::Unsigned;
                                    }

                                    // device number in decimal
                                    'd' => {
                                        arg = format!("{}", meta.dev());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // device number in hex
                                    'D' => {
                                        arg = format!("{:x}", meta.dev());
                                        output_type = OutputType::UnsignedHex;
                                    }
                                    // raw mode in hex
                                    'f' => {
                                        arg = format!("{:x}", meta.mode());
                                        output_type = OutputType::UnsignedHex;
                                    }
                                    // file type
                                    'F' => {
                                        arg = pretty_filetype(meta.mode() as mode_t, meta.len())
                                            .to_owned();
                                        output_type = OutputType::Str;
                                    }
                                    // group ID of owner
                                    'g' => {
                                        arg = format!("{}", meta.gid());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // group name of owner
                                    'G' => {
                                        arg = entries::gid2grp(meta.gid())
                                            .unwrap_or_else(|_| "UNKNOWN".to_owned());
                                        output_type = OutputType::Str;
                                    }
                                    // number of hard links
                                    'h' => {
                                        arg = format!("{}", meta.nlink());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // inode number
                                    'i' => {
                                        arg = format!("{}", meta.ino());
                                        output_type = OutputType::Unsigned;
                                    }

                                    // mount point
                                    'm' => {
                                        arg = self.find_mount_point(file).unwrap();
                                        output_type = OutputType::Str;
                                    }

                                    // file name
                                    'n' => {
                                        arg = file.to_owned();
                                        output_type = OutputType::Str;
                                    }
                                    // quoted file name with dereference if symbolic link
                                    'N' => {
                                        if file_type.is_symlink() {
                                            let dst = match fs::read_link(file) {
                                                Ok(path) => path,
                                                Err(e) => {
                                                    println!("{}", e);
                                                    return 1;
                                                }
                                            };
                                            arg = format!("{} -> {}", file.quote(), dst.quote());
                                        } else {
                                            arg = file.to_string();
                                        }
                                        output_type = OutputType::Str;
                                    }
                                    // optimal I/O transfer size hint
                                    'o' => {
                                        arg = format!("{}", meta.blksize());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // total size, in bytes
                                    's' => {
                                        arg = format!("{}", meta.len());
                                        output_type = OutputType::Integer;
                                    }
                                    // major device type in hex, for character/block device special
                                    // files
                                    't' => {
                                        arg = format!("{:x}", meta.rdev() >> 8);
                                        output_type = OutputType::UnsignedHex;
                                    }
                                    // minor device type in hex, for character/block device special
                                    // files
                                    'T' => {
                                        arg = format!("{:x}", meta.rdev() & 0xff);
                                        output_type = OutputType::UnsignedHex;
                                    }
                                    // user ID of owner
                                    'u' => {
                                        arg = format!("{}", meta.uid());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // user name of owner
                                    'U' => {
                                        arg = entries::uid2usr(meta.uid())
                                            .unwrap_or_else(|_| "UNKNOWN".to_owned());
                                        output_type = OutputType::Str;
                                    }

                                    // time of file birth, human-readable; - if unknown
                                    'w' => {
                                        arg = meta.pretty_birth();
                                        output_type = OutputType::Str;
                                    }

                                    // time of file birth, seconds since Epoch; 0 if unknown
                                    'W' => {
                                        arg = meta.birth();
                                        output_type = OutputType::Integer;
                                    }

                                    // time of last access, human-readable
                                    'x' => {
                                        arg = pretty_time(meta.atime(), meta.atime_nsec());
                                        output_type = OutputType::Str;
                                    }
                                    // time of last access, seconds since Epoch
                                    'X' => {
                                        arg = format!("{}", meta.atime());
                                        output_type = OutputType::Integer;
                                    }
                                    // time of last data modification, human-readable
                                    'y' => {
                                        arg = pretty_time(meta.mtime(), meta.mtime_nsec());
                                        output_type = OutputType::Str;
                                    }
                                    // time of last data modification, seconds since Epoch
                                    'Y' => {
                                        arg = format!("{}", meta.mtime());
                                        output_type = OutputType::Str;
                                    }
                                    // time of last status change, human-readable
                                    'z' => {
                                        arg = pretty_time(meta.ctime(), meta.ctime_nsec());
                                        output_type = OutputType::Str;
                                    }
                                    // time of last status change, seconds since Epoch
                                    'Z' => {
                                        arg = format!("{}", meta.ctime());
                                        output_type = OutputType::Integer;
                                    }

                                    _ => {
                                        arg = "?".to_owned();
                                        output_type = OutputType::Unknown;
                                    }
                                }
                                print_it(&arg, &output_type, flag, width, precision);
                            }
                        }
                    }
                }
                Err(e) => {
                    show_error!("cannot stat {}: {}", file.quote(), e);
                    return 1;
                }
            }
        } else {
            match statfs(file) {
                Ok(meta) => {
                    let tokens = &self.default_tokens;

                    for t in tokens.iter() {
                        match *t {
                            Token::Char(c) => print!("{}", c),
                            Token::Directive {
                                flag,
                                width,
                                precision,
                                format,
                            } => {
                                let arg: String;
                                let output_type: OutputType;
                                match format {
                                    // free blocks available to non-superuser
                                    'a' => {
                                        arg = format!("{}", meta.avail_blocks());
                                        output_type = OutputType::Integer;
                                    }
                                    // total data blocks in file system
                                    'b' => {
                                        arg = format!("{}", meta.total_blocks());
                                        output_type = OutputType::Integer;
                                    }
                                    // total file nodes in file system
                                    'c' => {
                                        arg = format!("{}", meta.total_file_nodes());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // free file nodes in file system
                                    'd' => {
                                        arg = format!("{}", meta.free_file_nodes());
                                        output_type = OutputType::Integer;
                                    }
                                    // free blocks in file system
                                    'f' => {
                                        arg = format!("{}", meta.free_blocks());
                                        output_type = OutputType::Integer;
                                    }
                                    // file system ID in hex
                                    'i' => {
                                        arg = format!("{:x}", meta.fsid());
                                        output_type = OutputType::UnsignedHex;
                                    }
                                    // maximum length of filenames
                                    'l' => {
                                        arg = format!("{}", meta.namelen());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // file name
                                    'n' => {
                                        arg = file.to_owned();
                                        output_type = OutputType::Str;
                                    }
                                    // block size (for faster transfers)
                                    's' => {
                                        arg = format!("{}", meta.io_size());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // fundamental block size (for block counts)
                                    'S' => {
                                        arg = format!("{}", meta.block_size());
                                        output_type = OutputType::Unsigned;
                                    }
                                    // file system type in hex
                                    't' => {
                                        arg = format!("{:x}", meta.fs_type());
                                        output_type = OutputType::UnsignedHex;
                                    }
                                    // file system type in human readable form
                                    'T' => {
                                        arg = pretty_fstype(meta.fs_type()).into_owned();
                                        output_type = OutputType::Str;
                                    }
                                    _ => {
                                        arg = "?".to_owned();
                                        output_type = OutputType::Unknown;
                                    }
                                }

                                print_it(&arg, &output_type, flag, width, precision);
                            }
                        }
                    }
                }
                Err(e) => {
                    show_error!(
                        "cannot read file system information for {}: {}",
                        file.quote(),
                        e
                    );
                    return 1;
                }
            }
        }
        0
    }

    fn default_format(show_fs: bool, terse: bool, show_dev_type: bool) -> String {
        // SELinux related format is *ignored*

        let mut format_str = String::with_capacity(36);
        if show_fs {
            if terse {
                format_str.push_str("%n %i %l %t %s %S %b %f %a %c %d\n");
            } else {
                format_str.push_str(
                    "  File: \"%n\"\n    ID: %-8i Namelen: %-7l Type: %T\nBlock \
                     size: %-10s Fundamental block size: %S\nBlocks: Total: %-10b \
                     Free: %-10f Available: %a\nInodes: Total: %-10c Free: %d\n",
                );
            }
        } else if terse {
            format_str.push_str("%n %s %b %f %u %g %D %i %h %t %T %X %Y %Z %W %o\n");
        } else {
            format_str.push_str("  File: %N\n  Size: %-10s\tBlocks: %-10b IO Block: %-6o %F\n");
            if show_dev_type {
                format_str
                    .push_str("Device: %Dh/%dd\tInode: %-10i  Links: %-5h Device type: %t,%T\n");
            } else {
                format_str.push_str("Device: %Dh/%dd\tInode: %-10i  Links: %h\n");
            }
            format_str.push_str("Access: (%04a/%10.10A)  Uid: (%5u/%8U)   Gid: (%5g/%8G)\n");
            format_str.push_str("Access: %x\nModify: %y\nChange: %z\n Birth: %w\n");
        }
        format_str
    }
}

fn get_long_usage() -> String {
    String::from(
        "
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
for details about the options it supports.
",
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let long_usage = get_long_usage();

    let matches = uu_app().after_help(&long_usage[..]).get_matches_from(args);

    let stater = Stater::new(&matches)?;
    let exit_status = stater.exec();
    if exit_status == 0 {
        Ok(())
    } else {
        Err(exit_status.into())
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help("follow links"),
        )
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .help("display file system status instead of file status"),
        )
        .arg(
            Arg::new(options::TERSE)
                .short('t')
                .long(options::TERSE)
                .help("print the information in terse form"),
        )
        .arg(
            Arg::new(options::FORMAT)
                .short('c')
                .long(options::FORMAT)
                .help(
                    "use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT",
                )
                .value_name("FORMAT"),
        )
        .arg(
            Arg::new(options::PRINTF)
                .long(options::PRINTF)
                .value_name("FORMAT")
                .help(
                    "like --format, but interpret backslash escapes,
            and do not output a mandatory trailing newline;
            if you want a newline, include \n in FORMAT",
                ),
        )
        .arg(
            Arg::new(ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1),
        )
}
