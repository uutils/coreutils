// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore datetime

use uucore::error::{UResult, USimpleError};

use clap::builder::ValueParser;
use uucore::display::Quotable;
use uucore::fs::display_permissions;
use uucore::fsext::{pretty_filetype, pretty_fstype, read_fs_list, statfs, BirthTime, FsMeta};
use uucore::libc::mode_t;
use uucore::{
    entries, format_usage, help_about, help_section, help_usage, show_error, show_warning,
};

use chrono::{DateTime, Local};
use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

const ABOUT: &str = help_about!("stat.md");
const USAGE: &str = help_usage!("stat.md");
const LONG_USAGE: &str = help_section!("long usage", "stat.md");

mod options {
    pub const DEREFERENCE: &str = "dereference";
    pub const FILE_SYSTEM: &str = "file-system";
    pub const FORMAT: &str = "format";
    pub const PRINTF: &str = "printf";
    pub const TERSE: &str = "terse";
    pub const FILES: &str = "files";
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
struct Flags {
    alter: bool,
    zero: bool,
    left: bool,
    space: bool,
    sign: bool,
    group: bool,
}

/// checks if the string is within the specified bound,
/// if it gets out of bound, error out by printing sub-string from index `beg` to`end`,
/// where `beg` & `end` is the beginning and end index of sub-string, respectively
fn check_bound(slice: &str, bound: usize, beg: usize, end: usize) -> UResult<()> {
    if end >= bound {
        return Err(USimpleError::new(
            1,
            format!("{}: invalid directive", slice[beg..end].quote()),
        ));
    }
    Ok(())
}

enum Padding {
    Zero,
    Space,
}

/// pads the string with zeroes or spaces and prints it
///
/// # Example
/// ```ignore
/// uu_stat::pad_and_print("1", false, 5, Padding::Zero) == "00001";
/// ```
/// currently only supports '0' & ' ' as the padding character
/// because the format specification of print! does not support general
/// fill characters.
fn pad_and_print(result: &str, left: bool, width: usize, padding: Padding) {
    match (left, padding) {
        (false, Padding::Zero) => print!("{result:0>width$}"),
        (false, Padding::Space) => print!("{result:>width$}"),
        (true, Padding::Zero) => print!("{result:0<width$}"),
        (true, Padding::Space) => print!("{result:<width$}"),
    };
}

#[derive(Debug)]
pub enum OutputType {
    Str(String),
    Integer(i64),
    Unsigned(u64),
    UnsignedHex(u64),
    UnsignedOct(u32),
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Char(char),
    Directive {
        flag: Flags,
        width: usize,
        precision: Option<usize>,
        format: char,
    },
}

trait ScanUtil {
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
            Some('-' | '+' | '0'..='9') => i += 1,
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
            8 => 3,
            16 => 2,
            _ => return None,
        };
        let chars = self.chars().enumerate();
        let mut res = 0;
        let mut offset = 0;
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

fn group_num(s: &str) -> Cow<str> {
    let is_negative = s.starts_with('-');
    assert!(is_negative || s.chars().take(1).all(|c| c.is_ascii_digit()));
    assert!(s.chars().skip(1).all(|c| c.is_ascii_digit()));
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

struct Stater {
    follow: bool,
    show_fs: bool,
    from_user: bool,
    files: Vec<OsString>,
    mount_list: Option<Vec<String>>,
    default_tokens: Vec<Token>,
    default_dev_tokens: Vec<Token>,
}

/// Prints a formatted output based on the provided output type, flags, width, and precision.
///
/// # Arguments
///
/// * `output` - A reference to the OutputType enum containing the value to be printed.
/// * `flags` - A Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed output.
/// * `precision` - An Option containing the precision value.
///
/// This function delegates the printing process to more specialized functions depending on the output type.
fn print_it(output: &OutputType, flags: Flags, width: usize, precision: Option<usize>) {
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
    let padding_char = determine_padding_char(&flags, &precision);

    match output {
        OutputType::Str(s) => print_str(s, &flags, width, precision),
        OutputType::Integer(num) => print_integer(*num, &flags, width, precision, padding_char),
        OutputType::Unsigned(num) => print_unsigned(*num, &flags, width, precision, padding_char),
        OutputType::UnsignedOct(num) => {
            print_unsigned_oct(*num, &flags, width, precision, padding_char);
        }
        OutputType::UnsignedHex(num) => {
            print_unsigned_hex(*num, &flags, width, precision, padding_char);
        }
        OutputType::Unknown => print!("?"),
    }
}

/// Determines the padding character based on the provided flags and precision.
///
/// # Arguments
///
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `precision` - An Option containing the precision value.
///
/// # Returns
///
/// * Padding - An instance of the Padding enum representing the padding character.
fn determine_padding_char(flags: &Flags, precision: &Option<usize>) -> Padding {
    if flags.zero && !flags.left && precision.is_none() {
        Padding::Zero
    } else {
        Padding::Space
    }
}

/// Prints a string value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `s` - The string to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed string.
/// * `precision` - An Option containing the precision value.
fn print_str(s: &str, flags: &Flags, width: usize, precision: Option<usize>) {
    let s = match precision {
        Some(p) if p < s.len() => &s[..p],
        _ => s,
    };
    pad_and_print(s, flags.left, width, Padding::Space);
}

/// Prints an integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed integer.
/// * `precision` - An Option containing the precision value.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_integer(
    num: i64,
    flags: &Flags,
    width: usize,
    precision: Option<usize>,
    padding_char: Padding,
) {
    let num = num.to_string();
    let arg = if flags.group {
        group_num(&num)
    } else {
        Cow::Borrowed(num.as_str())
    };
    let prefix = if flags.sign {
        "+"
    } else if flags.space {
        " "
    } else {
        ""
    };
    let extended = format!(
        "{prefix}{arg:0>precision$}",
        precision = precision.unwrap_or(0)
    );
    pad_and_print(&extended, flags.left, width, padding_char);
}

/// Prints an unsigned integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The unsigned integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed unsigned integer.
/// * `precision` - An Option containing the precision value.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_unsigned(
    num: u64,
    flags: &Flags,
    width: usize,
    precision: Option<usize>,
    padding_char: Padding,
) {
    let num = num.to_string();
    let s = if flags.group {
        group_num(&num)
    } else {
        Cow::Borrowed(num.as_str())
    };
    let s = format!("{s:0>precision$}", precision = precision.unwrap_or(0));
    pad_and_print(&s, flags.left, width, padding_char);
}

/// Prints an unsigned octal integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The unsigned octal integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed unsigned octal integer.
/// * `precision` - An Option containing the precision value.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_unsigned_oct(
    num: u32,
    flags: &Flags,
    width: usize,
    precision: Option<usize>,
    padding_char: Padding,
) {
    let prefix = if flags.alter { "0" } else { "" };
    let s = format!(
        "{prefix}{num:0>precision$o}",
        precision = precision.unwrap_or(0)
    );
    pad_and_print(&s, flags.left, width, padding_char);
}

/// Prints an unsigned hexadecimal integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The unsigned hexadecimal integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed unsigned hexadecimal integer.
/// * `precision` - An Option containing the precision value.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_unsigned_hex(
    num: u64,
    flags: &Flags,
    width: usize,
    precision: Option<usize>,
    padding_char: Padding,
) {
    let prefix = if flags.alter { "0x" } else { "" };
    let s = format!(
        "{prefix}{num:0>precision$x}",
        precision = precision.unwrap_or(0)
    );
    pad_and_print(&s, flags.left, width, padding_char);
}

impl Stater {
    fn handle_percent_case(
        chars: &[char],
        i: &mut usize,
        bound: usize,
        format_str: &str,
    ) -> UResult<Token> {
        let old = *i;

        *i += 1;
        if *i >= bound {
            return Ok(Token::Char('%'));
        }
        if chars[*i] == '%' {
            *i += 1;
            return Ok(Token::Char('%'));
        }

        let mut flag = Flags::default();

        while *i < bound {
            match chars[*i] {
                '#' => flag.alter = true,
                '0' => flag.zero = true,
                '-' => flag.left = true,
                ' ' => flag.space = true,
                '+' => flag.sign = true,
                '\'' => flag.group = true,
                'I' => unimplemented!(),
                _ => break,
            }
            *i += 1;
        }
        check_bound(format_str, bound, old, *i)?;

        let mut width = 0;
        let mut precision = None;
        let mut j = *i;

        if let Some((field_width, offset)) = format_str[j..].scan_num::<usize>() {
            width = field_width;
            j += offset;
        }
        check_bound(format_str, bound, old, j)?;

        if chars[j] == '.' {
            j += 1;
            check_bound(format_str, bound, old, j)?;

            match format_str[j..].scan_num::<i32>() {
                Some((value, offset)) => {
                    if value >= 0 {
                        precision = Some(value as usize);
                    }
                    j += offset;
                }
                None => precision = Some(0),
            }
            check_bound(format_str, bound, old, j)?;
        }

        *i = j;
        Ok(Token::Directive {
            width,
            flag,
            precision,
            format: chars[*i],
        })
    }

    fn handle_escape_sequences(
        chars: &[char],
        i: &mut usize,
        bound: usize,
        format_str: &str,
    ) -> Token {
        *i += 1;
        if *i >= bound {
            show_warning!("backslash at end of format");
            return Token::Char('\\');
        }
        match chars[*i] {
            'x' if *i + 1 < bound => {
                if let Some((c, offset)) = format_str[*i + 1..].scan_char(16) {
                    *i += offset;
                    Token::Char(c)
                } else {
                    show_warning!("unrecognized escape '\\x'");
                    Token::Char('x')
                }
            }
            '0'..='7' => {
                let (c, offset) = format_str[*i..].scan_char(8).unwrap();
                *i += offset - 1;
                Token::Char(c)
            }
            '"' => Token::Char('"'),
            '\\' => Token::Char('\\'),
            'a' => Token::Char('\x07'),
            'b' => Token::Char('\x08'),
            'e' => Token::Char('\x1B'),
            'f' => Token::Char('\x0C'),
            'n' => Token::Char('\n'),
            'r' => Token::Char('\r'),
            't' => Token::Char('\t'),
            'v' => Token::Char('\x0B'),
            c => {
                show_warning!("unrecognized escape '\\{}'", c);
                Token::Char(c)
            }
        }
    }

    fn generate_tokens(format_str: &str, use_printf: bool) -> UResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let bound = format_str.len();
        let chars = format_str.chars().collect::<Vec<char>>();
        let mut i = 0;
        while i < bound {
            match chars[i] {
                '%' => tokens.push(Self::handle_percent_case(
                    &chars, &mut i, bound, format_str,
                )?),
                '\\' => {
                    if use_printf {
                        tokens.push(Self::handle_escape_sequences(
                            &chars, &mut i, bound, format_str,
                        ));
                    } else {
                        tokens.push(Token::Char('\\'));
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
        let files: Vec<OsString> = matches
            .get_many::<OsString>(options::FILES)
            .map(|v| v.map(OsString::from).collect())
            .unwrap_or_default();
        if files.is_empty() {
            return Err(Box::new(USimpleError {
                code: 1,
                message: "missing operand\nTry 'stat --help' for more information.".to_string(),
            }));
        }
        let format_str = if matches.contains_id(options::PRINTF) {
            matches
                .get_one::<String>(options::PRINTF)
                .expect("Invalid format string")
        } else {
            matches
                .get_one::<String>(options::FORMAT)
                .map(|s| s.as_str())
                .unwrap_or("")
        };

        let use_printf = matches.contains_id(options::PRINTF);
        let terse = matches.get_flag(options::TERSE);
        let show_fs = matches.get_flag(options::FILE_SYSTEM);

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
                .map_err(|e| {
                    let context = "cannot read table of mounted file systems";
                    USimpleError::new(e.code(), format!("{}: {}", context, e))
                })?
                .iter()
                .map(|mi| mi.mount_dir.clone())
                .collect::<Vec<String>>();
            // Reverse sort. The longer comes first.
            mount_list.sort();
            mount_list.reverse();
            Some(mount_list)
        };

        Ok(Self {
            follow: matches.get_flag(options::DEREFERENCE),
            show_fs,
            from_user: !format_str.is_empty(),
            files,
            default_tokens,
            default_dev_tokens,
            mount_list,
        })
    }

    fn find_mount_point<P: AsRef<Path>>(&self, p: P) -> Option<String> {
        let path = p.as_ref().canonicalize().ok()?;

        for root in self.mount_list.as_ref()? {
            if path.starts_with(root) {
                return Some(root.clone());
            }
        }
        None
    }

    fn exec(&self) -> i32 {
        let mut stdin_is_fifo = false;
        if cfg!(unix) {
            if let Ok(md) = fs::metadata("/dev/stdin") {
                stdin_is_fifo = md.file_type().is_fifo();
            }
        }

        let mut ret = 0;
        for f in &self.files {
            ret |= self.do_stat(f, stdin_is_fifo);
        }
        ret
    }

    #[allow(clippy::cognitive_complexity)]
    fn do_stat(&self, file: &OsStr, stdin_is_fifo: bool) -> i32 {
        let display_name = file.to_string_lossy();
        let file = if cfg!(unix) && display_name == "-" {
            if self.show_fs {
                show_error!("using '-' to denote standard input does not work in file system mode");
                return 1;
            }
            if let Ok(p) = Path::new("/dev/stdin").canonicalize() {
                p.into_os_string()
            } else {
                OsString::from("/dev/stdin")
            }
        } else {
            OsString::from(file)
        };
        if self.show_fs {
            #[cfg(unix)]
            let p = file.as_bytes();
            #[cfg(not(unix))]
            let p = file.into_string().unwrap();
            match statfs(p) {
                Ok(meta) => {
                    let tokens = &self.default_tokens;

                    for t in tokens {
                        match *t {
                            Token::Char(c) => print!("{c}"),
                            Token::Directive {
                                flag,
                                width,
                                precision,
                                format,
                            } => {
                                let output = match format {
                                    // free blocks available to non-superuser
                                    'a' => OutputType::Unsigned(meta.avail_blocks()),
                                    // total data blocks in file system
                                    'b' => OutputType::Unsigned(meta.total_blocks()),
                                    // total file nodes in file system
                                    'c' => OutputType::Unsigned(meta.total_file_nodes()),
                                    // free file nodes in file system
                                    'd' => OutputType::Unsigned(meta.free_file_nodes()),
                                    // free blocks in file system
                                    'f' => OutputType::Unsigned(meta.free_blocks()),
                                    // file system ID in hex
                                    'i' => OutputType::UnsignedHex(meta.fsid()),
                                    // maximum length of filenames
                                    'l' => OutputType::Unsigned(meta.namelen()),
                                    // file name
                                    'n' => OutputType::Str(display_name.to_string()),
                                    // block size (for faster transfers)
                                    's' => OutputType::Unsigned(meta.io_size()),
                                    // fundamental block size (for block counts)
                                    'S' => OutputType::Integer(meta.block_size()),
                                    // file system type in hex
                                    't' => OutputType::UnsignedHex(meta.fs_type() as u64),
                                    // file system type in human readable form
                                    'T' => OutputType::Str(pretty_fstype(meta.fs_type()).into()),
                                    _ => OutputType::Unknown,
                                };

                                print_it(&output, flag, width, precision);
                            }
                        }
                    }
                }
                Err(e) => {
                    show_error!(
                        "cannot read file system information for {}: {}",
                        display_name.quote(),
                        e
                    );
                    return 1;
                }
            }
        } else {
            let result = if self.follow || stdin_is_fifo && display_name == "-" {
                fs::metadata(&file)
            } else {
                fs::symlink_metadata(&file)
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

                    for t in tokens {
                        match *t {
                            Token::Char(c) => print!("{c}"),
                            Token::Directive {
                                flag,
                                width,
                                precision,
                                format,
                            } => {
                                let output = match format {
                                    // access rights in octal
                                    'a' => OutputType::UnsignedOct(0o7777 & meta.mode()),
                                    // access rights in human readable form
                                    'A' => OutputType::Str(display_permissions(&meta, true)),
                                    // number of blocks allocated (see %B)
                                    'b' => OutputType::Unsigned(meta.blocks()),

                                    // the size in bytes of each block reported by %b
                                    // FIXME: blocksize differs on various platform
                                    // See coreutils/gnulib/lib/stat-size.h ST_NBLOCKSIZE // spell-checker:disable-line
                                    'B' => OutputType::Unsigned(512),

                                    // device number in decimal
                                    'd' => OutputType::Unsigned(meta.dev()),
                                    // device number in hex
                                    'D' => OutputType::UnsignedHex(meta.dev()),
                                    // raw mode in hex
                                    'f' => OutputType::UnsignedHex(meta.mode() as u64),
                                    // file type
                                    'F' => OutputType::Str(
                                        pretty_filetype(meta.mode() as mode_t, meta.len())
                                            .to_owned(),
                                    ),
                                    // group ID of owner
                                    'g' => OutputType::Unsigned(meta.gid() as u64),
                                    // group name of owner
                                    'G' => {
                                        let group_name = entries::gid2grp(meta.gid())
                                            .unwrap_or_else(|_| "UNKNOWN".to_owned());
                                        OutputType::Str(group_name)
                                    }
                                    // number of hard links
                                    'h' => OutputType::Unsigned(meta.nlink()),
                                    // inode number
                                    'i' => OutputType::Unsigned(meta.ino()),
                                    // mount point
                                    'm' => OutputType::Str(self.find_mount_point(&file).unwrap()),
                                    // file name
                                    'n' => OutputType::Str(display_name.to_string()),
                                    // quoted file name with dereference if symbolic link
                                    'N' => {
                                        let file_name = if file_type.is_symlink() {
                                            let dst = match fs::read_link(&file) {
                                                Ok(path) => path,
                                                Err(e) => {
                                                    println!("{e}");
                                                    return 1;
                                                }
                                            };
                                            format!("{} -> {}", display_name.quote(), dst.quote())
                                        } else {
                                            display_name.to_string()
                                        };
                                        OutputType::Str(file_name)
                                    }
                                    // optimal I/O transfer size hint
                                    'o' => OutputType::Unsigned(meta.blksize()),
                                    // total size, in bytes
                                    's' => OutputType::Integer(meta.len() as i64),
                                    // major device type in hex, for character/block device special
                                    // files
                                    't' => OutputType::UnsignedHex(meta.rdev() >> 8),
                                    // minor device type in hex, for character/block device special
                                    // files
                                    'T' => OutputType::UnsignedHex(meta.rdev() & 0xff),
                                    // user ID of owner
                                    'u' => OutputType::Unsigned(meta.uid() as u64),
                                    // user name of owner
                                    'U' => {
                                        let user_name = entries::uid2usr(meta.uid())
                                            .unwrap_or_else(|_| "UNKNOWN".to_owned());
                                        OutputType::Str(user_name)
                                    }

                                    // time of file birth, human-readable; - if unknown
                                    'w' => OutputType::Str(
                                        meta.birth()
                                            .map(|(sec, nsec)| pretty_time(sec as i64, nsec as i64))
                                            .unwrap_or(String::from("-")),
                                    ),

                                    // time of file birth, seconds since Epoch; 0 if unknown
                                    'W' => OutputType::Unsigned(meta.birth().unwrap_or_default().0),

                                    // time of last access, human-readable
                                    'x' => OutputType::Str(pretty_time(
                                        meta.atime(),
                                        meta.atime_nsec(),
                                    )),
                                    // time of last access, seconds since Epoch
                                    'X' => OutputType::Integer(meta.atime()),
                                    // time of last data modification, human-readable
                                    'y' => OutputType::Str(pretty_time(
                                        meta.mtime(),
                                        meta.mtime_nsec(),
                                    )),
                                    // time of last data modification, seconds since Epoch
                                    'Y' => OutputType::Integer(meta.mtime()),
                                    // time of last status change, human-readable
                                    'z' => OutputType::Str(pretty_time(
                                        meta.ctime(),
                                        meta.ctime_nsec(),
                                    )),
                                    // time of last status change, seconds since Epoch
                                    'Z' => OutputType::Integer(meta.ctime()),

                                    _ => OutputType::Unknown,
                                };
                                print_it(&output, flag, width, precision);
                            }
                        }
                    }
                }
                Err(e) => {
                    show_error!("cannot stat {}: {}", display_name.quote(), e);
                    return 1;
                }
            }
        }
        0
    }

    fn default_format(show_fs: bool, terse: bool, show_dev_type: bool) -> String {
        // SELinux related format is *ignored*

        if show_fs {
            if terse {
                "%n %i %l %t %s %S %b %f %a %c %d\n".into()
            } else {
                "  File: \"%n\"\n    ID: %-8i Namelen: %-7l Type: %T\nBlock \
                 size: %-10s Fundamental block size: %S\nBlocks: Total: %-10b \
                 Free: %-10f Available: %a\nInodes: Total: %-10c Free: %d\n"
                    .into()
            }
        } else if terse {
            "%n %s %b %f %u %g %D %i %h %t %T %X %Y %Z %W %o\n".into()
        } else {
            [
                "  File: %N\n  Size: %-10s\tBlocks: %-10b IO Block: %-6o %F\n",
                if show_dev_type {
                    "Device: %Dh/%dd\tInode: %-10i  Links: %-5h Device type: %t,%T\n"
                } else {
                    "Device: %Dh/%dd\tInode: %-10i  Links: %h\n"
                },
                "Access: (%04a/%10.10A)  Uid: (%5u/%8U)   Gid: (%5g/%8G)\n",
                "Access: %x\nModify: %y\nChange: %z\n Birth: %w\n",
            ]
            .join("")
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().after_help(LONG_USAGE).try_get_matches_from(args)?;

    let stater = Stater::new(&matches)?;
    let exit_status = stater.exec();
    if exit_status == 0 {
        Ok(())
    } else {
        Err(exit_status.into())
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help("follow links")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .help("display file system status instead of file status")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TERSE)
                .short('t')
                .long(options::TERSE)
                .help("print the information in terse form")
                .action(ArgAction::SetTrue),
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
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}

const PRETTY_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S.%f %z";

fn pretty_time(sec: i64, nsec: i64) -> String {
    // Return the date in UTC
    let tm = chrono::DateTime::from_timestamp(sec, nsec as u32).unwrap_or_default();
    let tm: DateTime<Local> = tm.into();

    tm.format(PRETTY_DATETIME_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use super::{group_num, Flags, ScanUtil, Stater, Token};

    #[test]
    fn test_scanners() {
        assert_eq!(Some((-5, 2)), "-5zxc".scan_num::<i32>());
        assert_eq!(Some((51, 2)), "51zxc".scan_num::<u32>());
        assert_eq!(Some((192, 4)), "+192zxc".scan_num::<i32>());
        assert_eq!(None, "z192zxc".scan_num::<i32>());

        assert_eq!(Some(('a', 3)), "141zxc".scan_char(8));
        assert_eq!(Some(('\n', 2)), "12qzxc".scan_char(8)); // spell-checker:disable-line
        assert_eq!(Some(('\r', 1)), "dqzxc".scan_char(16)); // spell-checker:disable-line
        assert_eq!(None, "z2qzxc".scan_char(8)); // spell-checker:disable-line
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_group_num() {
        assert_eq!("12,379,821,234", group_num("12379821234"));
        assert_eq!("21,234", group_num("21234"));
        assert_eq!("821,234", group_num("821234"));
        assert_eq!("1,821,234", group_num("1821234"));
        assert_eq!("1,234", group_num("1234"));
        assert_eq!("234", group_num("234"));
        assert_eq!("24", group_num("24"));
        assert_eq!("4", group_num("4"));
        assert_eq!("", group_num(""));
        assert_eq!("-5", group_num("-5"));
        assert_eq!("-1,234", group_num("-1234"));
    }

    #[test]
    #[should_panic]
    fn test_group_num_panic_if_invalid_numeric_characters() {
        group_num("³³³³³");
    }

    #[test]
    fn normal_format() {
        let s = "%'010.2ac%-#5.w\n";
        let expected = vec![
            Token::Directive {
                flag: Flags {
                    group: true,
                    zero: true,
                    ..Default::default()
                },
                width: 10,
                precision: Some(2),
                format: 'a',
            },
            Token::Char('c'),
            Token::Directive {
                flag: Flags {
                    left: true,
                    alter: true,
                    ..Default::default()
                },
                width: 5,
                precision: Some(0),
                format: 'w',
            },
            Token::Char('\n'),
        ];
        assert_eq!(&expected, &Stater::generate_tokens(s, false).unwrap());
    }

    #[test]
    fn printf_format() {
        let s = r#"%-# 15a\t\r\"\\\a\b\e\f\v%+020.-23w\x12\167\132\112\n"#;
        let expected = vec![
            Token::Directive {
                flag: Flags {
                    left: true,
                    alter: true,
                    space: true,
                    ..Default::default()
                },
                width: 15,
                precision: None,
                format: 'a',
            },
            Token::Char('\t'),
            Token::Char('\r'),
            Token::Char('"'),
            Token::Char('\\'),
            Token::Char('\x07'),
            Token::Char('\x08'),
            Token::Char('\x1B'),
            Token::Char('\x0C'),
            Token::Char('\x0B'),
            Token::Directive {
                flag: Flags {
                    sign: true,
                    zero: true,
                    ..Default::default()
                },
                width: 20,
                precision: None,
                format: 'w',
            },
            Token::Char('\x12'),
            Token::Char('w'),
            Token::Char('Z'),
            Token::Char('J'),
            Token::Char('\n'),
        ];
        assert_eq!(&expected, &Stater::generate_tokens(s, true).unwrap());
    }
}
