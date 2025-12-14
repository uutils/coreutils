// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore datetime

use uucore::error::{UError, UResult, USimpleError};
use uucore::translate;

use clap::builder::ValueParser;
use uucore::display::Quotable;
use uucore::fs::display_permissions;
use uucore::fsext::{
    FsMeta, MetadataTimeField, StatFs, metadata_get_time, pretty_filetype, pretty_fstype,
    read_fs_list, statfs,
};
use uucore::libc::mode_t;
use uucore::{entries, format_usage, show_error, show_warning};

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fs::{FileType, Metadata};
use std::io::Write;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
use std::{env, fs};

use thiserror::Error;
use uucore::time::{FormatSystemTimeFallback, format_system_time, system_time_to_sec};

#[derive(Debug, Error)]
enum StatError {
    #[error("{}", translate!("stat-error-invalid-quoting-style", "style" => style.clone()))]
    InvalidQuotingStyle { style: String },
    #[error("{}", translate!("stat-error-missing-operand"))]
    MissingOperand,
    #[error("{}", translate!("stat-error-invalid-directive", "directive" => directive.clone()))]
    InvalidDirective { directive: String },
    #[error("{}", translate!("stat-error-cannot-read-filesystem", "error" => error.clone()))]
    CannotReadFilesystem { error: String },
    #[error("{}", translate!("stat-error-stdin-filesystem-mode"))]
    StdinFilesystemMode,
    #[error("{}", translate!("stat-error-cannot-read-filesystem-info", "file" => file.clone(), "error" => error.clone()))]
    CannotReadFilesystemInfo { file: String, error: String },
    #[error("{}", translate!("stat-error-cannot-stat", "file" => file.clone(), "error" => error.clone()))]
    CannotStat { file: String, error: String },
}

impl UError for StatError {
    fn code(&self) -> i32 {
        1
    }
}

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
            StatError::InvalidDirective {
                directive: slice[beg..end].quote().to_string(),
            }
            .to_string(),
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
    }
}

/// Pads and prints raw bytes (Unix-specific) or falls back to string printing
///
/// On Unix systems, this preserves non-UTF8 data by printing raw bytes
/// On other platforms, falls back to lossy string conversion
fn pad_and_print_bytes<W: Write>(
    mut writer: W,
    bytes: &[u8],
    left: bool,
    width: usize,
    precision: Precision,
) -> Result<(), std::io::Error> {
    let display_bytes = match precision {
        Precision::Number(p) if p < bytes.len() => &bytes[..p],
        _ => bytes,
    };

    let display_len = display_bytes.len();
    let padding_needed = width.saturating_sub(display_len);

    let (left_pad, right_pad) = if left {
        (0, padding_needed)
    } else {
        (padding_needed, 0)
    };

    if left_pad > 0 {
        print_padding(&mut writer, left_pad)?;
    }
    writer.write_all(display_bytes)?;
    if right_pad > 0 {
        print_padding(&mut writer, right_pad)?;
    }

    Ok(())
}

/// print padding based on a writer W and n size
/// writer is genric to be any buffer like: `std::io::stdout`
/// n is the calculated padding size
fn print_padding<W: Write>(writer: &mut W, n: usize) -> Result<(), std::io::Error> {
    for _ in 0..n {
        writer.write_all(b" ")?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum OutputType<'a> {
    Str(String),
    OsStr(&'a OsString),
    Integer(i64),
    Unsigned(u64),
    UnsignedHex(u64),
    UnsignedOct(u32),
    Float(f64),
    Unknown,
}

#[derive(Default)]
enum QuotingStyle {
    Locale,
    Shell,
    #[default]
    ShellEscapeAlways,
    Quote,
}

impl std::str::FromStr for QuotingStyle {
    type Err = StatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "locale" => Ok(Self::Locale),
            "shell" => Ok(Self::Shell),
            "shell-escape-always" => Ok(Self::ShellEscapeAlways),
            // The others aren't exposed to the user
            _ => Err(StatError::InvalidQuotingStyle {
                style: s.to_string(),
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Precision {
    NotSpecified,
    NoNumber,
    Number(usize),
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Char(char),
    Byte(u8),
    Directive {
        flag: Flags,
        width: usize,
        precision: Precision,
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
    /// Scans for a number at the beginning of the string
    /// Returns the parsed number and the character count
    /// Since we only deal with ASCII characters (+, -, 0-9), character count equals byte count
    fn scan_num<F>(&self) -> Option<(F, usize)>
    where
        F: std::str::FromStr,
    {
        let mut chars = self.chars();
        let count = chars
            .next()
            .filter(|&c| c.is_ascii_digit() || c == '-' || c == '+')
            .map_or(0, |_| 1 + chars.take_while(char::is_ascii_digit).count());

        if count > 0 {
            F::from_str(&self[..count]).ok().map(|x| (x, count))
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

fn group_num(s: &str) -> Cow<'_, str> {
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
    mount_list: Option<Vec<OsString>>,
    default_tokens: Vec<Token>,
    default_dev_tokens: Vec<Token>,
}

/// Prints a formatted output based on the provided output type, flags, width, and precision.
///
/// # Arguments
///
/// * `output` - A reference to the [`OutputType`] enum containing the value to be printed.
/// * `flags` - A Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed output.
/// * `precision` - How many digits of precision, if any.
///
/// This function delegates the printing process to more specialized functions depending on the output type.
fn print_it(output: &OutputType, flags: Flags, width: usize, precision: Precision) {
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
    let padding_char = determine_padding_char(&flags);

    match output {
        OutputType::Str(s) => print_str(s, &flags, width, precision),
        OutputType::OsStr(s) => print_os_str(s, &flags, width, precision),
        OutputType::Integer(num) => print_integer(*num, &flags, width, precision, padding_char),
        OutputType::Unsigned(num) => print_unsigned(*num, &flags, width, precision, padding_char),
        OutputType::UnsignedOct(num) => {
            print_unsigned_oct(*num, &flags, width, precision, padding_char);
        }
        OutputType::UnsignedHex(num) => {
            print_unsigned_hex(*num, &flags, width, precision, padding_char);
        }
        OutputType::Float(num) => {
            print_float(*num, &flags, width, precision, padding_char);
        }
        OutputType::Unknown => print!("?"),
    }
}

/// Determines the padding character based on the provided flags and precision.
///
/// # Arguments
///
/// * `flags` - A reference to the Flags struct containing formatting flags.
///
/// # Returns
///
/// * Padding - An instance of the Padding enum representing the padding character.
fn determine_padding_char(flags: &Flags) -> Padding {
    if flags.zero && !flags.left {
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
/// * `precision` - How many digits of precision, if any.
fn print_str(s: &str, flags: &Flags, width: usize, precision: Precision) {
    let s = match precision {
        Precision::Number(p) if p < s.len() => &s[..p],
        _ => s,
    };
    pad_and_print(s, flags.left, width, Padding::Space);
}

/// Prints a `OsString` value based on the provided flags, width, and precision.
/// for unix it converts it to bytes then tries to print it if failed print the lossy string version
/// for windows, `OsString` uses UTF-16 internally which doesn't map directly to bytes like Unix,
/// so we fall back to lossy string conversion to handle invalid UTF-8 sequences gracefully
///
/// # Arguments
///
/// * `s` - The `OsString` to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed string.
/// * `precision` - How many digits of precision, if any.
fn print_os_str(s: &OsString, flags: &Flags, width: usize, precision: Precision) {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = s.as_bytes();

        if pad_and_print_bytes(std::io::stdout(), bytes, flags.left, width, precision).is_err() {
            // if an error occurred while trying to print bytes fall back to normal lossy string so it can be printed
            let fallback_string = s.to_string_lossy();
            print_str(&fallback_string, flags, width, precision);
        }
    }
    #[cfg(not(unix))]
    {
        let lossy_string = s.to_string_lossy();
        print_str(&lossy_string, flags, width, precision);
    }
}

fn quote_file_name(file_name: &str, quoting_style: &QuotingStyle) -> String {
    match quoting_style {
        QuotingStyle::Locale | QuotingStyle::Shell => {
            let escaped = file_name.replace('\'', r"\'");
            format!("'{escaped}'")
        }
        QuotingStyle::ShellEscapeAlways => {
            let quote = if file_name.contains('\'') { '"' } else { '\'' };
            format!("{quote}{file_name}{quote}")
        }
        QuotingStyle::Quote => file_name.to_string(),
    }
}

fn get_quoted_file_name(
    display_name: &str,
    file: &OsString,
    file_type: &FileType,
    from_user: bool,
) -> Result<String, i32> {
    let quoting_style = env::var("QUOTING_STYLE")
        .ok()
        .and_then(|style| style.parse().ok())
        .unwrap_or_default();

    if file_type.is_symlink() {
        let quoted_display_name = quote_file_name(display_name, &quoting_style);
        match fs::read_link(file) {
            Ok(dst) => {
                let quoted_dst = quote_file_name(&dst.to_string_lossy(), &quoting_style);
                Ok(format!("{quoted_display_name} -> {quoted_dst}"))
            }
            Err(e) => {
                show_error!("{e}");
                Err(1)
            }
        }
    } else {
        let style = if from_user {
            quoting_style
        } else {
            QuotingStyle::Quote
        };
        Ok(quote_file_name(display_name, &style))
    }
}

fn process_token_filesystem(t: &Token, meta: &StatFs, display_name: &str) {
    match *t {
        Token::Byte(byte) => write_raw_byte(byte),
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

/// Prints an integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed integer.
/// * `precision` - How many digits of precision, if any.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_integer(
    num: i64,
    flags: &Flags,
    width: usize,
    precision: Precision,
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
    let extended = match precision {
        Precision::NotSpecified => format!("{prefix}{arg}"),
        Precision::NoNumber => format!("{prefix}{arg}"),
        Precision::Number(p) => format!("{prefix}{arg:0>p$}"),
    };
    pad_and_print(&extended, flags.left, width, padding_char);
}

/// Truncate a float to the given number of digits after the decimal point.
fn precision_trunc(num: f64, precision: Precision) -> String {
    // GNU `stat` doesn't round, it just seems to truncate to the
    // given precision:
    //
    //     $ stat -c "%.5Y" /dev/pts/ptmx
    //     1736344012.76399
    //     $ stat -c "%.4Y" /dev/pts/ptmx
    //     1736344012.7639
    //     $ stat -c "%.3Y" /dev/pts/ptmx
    //     1736344012.763
    //
    // Contrast this with `printf`, which seems to round the
    // numbers:
    //
    //     $ printf "%.5f\n" 1736344012.76399
    //     1736344012.76399
    //     $ printf "%.4f\n" 1736344012.76399
    //     1736344012.7640
    //     $ printf "%.3f\n" 1736344012.76399
    //     1736344012.764
    //
    let num_str = num.to_string();
    let n = num_str.len();
    match (num_str.find('.'), precision) {
        (None, Precision::NotSpecified) => num_str,
        (None, Precision::NoNumber) => num_str,
        (None, Precision::Number(0)) => num_str,
        (None, Precision::Number(p)) => format!("{num_str}.{zeros}", zeros = "0".repeat(p)),
        (Some(i), Precision::NotSpecified) => num_str[..i].to_string(),
        (Some(_), Precision::NoNumber) => num_str,
        (Some(i), Precision::Number(0)) => num_str[..i].to_string(),
        (Some(i), Precision::Number(p)) if p < n - i => num_str[..i + 1 + p].to_string(),
        (Some(i), Precision::Number(p)) => {
            format!("{num_str}{zeros}", zeros = "0".repeat(p - (n - i - 1)))
        }
    }
}

fn print_float(num: f64, flags: &Flags, width: usize, precision: Precision, padding_char: Padding) {
    let prefix = if flags.sign {
        "+"
    } else if flags.space {
        " "
    } else {
        ""
    };
    let num_str = precision_trunc(num, precision);
    let extended = format!("{prefix}{num_str}");
    pad_and_print(&extended, flags.left, width, padding_char);
}

/// Prints an unsigned integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The unsigned integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed unsigned integer.
/// * `precision` - How many digits of precision, if any.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_unsigned(
    num: u64,
    flags: &Flags,
    width: usize,
    precision: Precision,
    padding_char: Padding,
) {
    let num = num.to_string();
    let s = if flags.group {
        group_num(&num)
    } else {
        Cow::Borrowed(num.as_str())
    };
    let s = match precision {
        Precision::NotSpecified => s,
        Precision::NoNumber => s,
        Precision::Number(p) => format!("{s:0>p$}").into(),
    };
    pad_and_print(&s, flags.left, width, padding_char);
}

/// Prints an unsigned octal integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The unsigned octal integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed unsigned octal integer.
/// * `precision` - How many digits of precision, if any.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_unsigned_oct(
    num: u32,
    flags: &Flags,
    width: usize,
    precision: Precision,
    padding_char: Padding,
) {
    let prefix = if flags.alter { "0" } else { "" };
    let s = match precision {
        Precision::NotSpecified => format!("{prefix}{num:o}"),
        Precision::NoNumber => format!("{prefix}{num:o}"),
        Precision::Number(p) => format!("{prefix}{num:0>p$o}"),
    };
    pad_and_print(&s, flags.left, width, padding_char);
}

/// Prints an unsigned hexadecimal integer value based on the provided flags, width, and precision.
///
/// # Arguments
///
/// * `num` - The unsigned hexadecimal integer value to be printed.
/// * `flags` - A reference to the Flags struct containing formatting flags.
/// * `width` - The width of the field for the printed unsigned hexadecimal integer.
/// * `precision` - How many digits of precision, if any.
/// * `padding_char` - The padding character as determined by `determine_padding_char`.
fn print_unsigned_hex(
    num: u64,
    flags: &Flags,
    width: usize,
    precision: Precision,
    padding_char: Padding,
) {
    let prefix = if flags.alter { "0x" } else { "" };
    let s = match precision {
        Precision::NotSpecified => format!("{prefix}{num:x}"),
        Precision::NoNumber => format!("{prefix}{num:x}"),
        Precision::Number(p) => format!("{prefix}{num:0>p$x}"),
    };
    pad_and_print(&s, flags.left, width, padding_char);
}

fn write_raw_byte(byte: u8) {
    std::io::stdout().write_all(&[byte]).unwrap();
}

impl Stater {
    fn process_flags(chars: &[char], i: &mut usize, bound: usize, flag: &mut Flags) {
        while *i < bound {
            match chars[*i] {
                '#' => flag.alter = true,
                '0' => flag.zero = true,
                '-' => flag.left = true,
                ' ' => flag.space = true,
                // This is not documented but the behavior seems to be
                // the same as a space. For example `stat -c "%I5s" f`
                // prints "    0".
                'I' => flag.space = true,
                '+' => flag.sign = true,
                '\'' => flag.group = true,
                _ => break,
            }
            *i += 1;
        }
    }

    /// Converts a character index to a byte index in a UTF-8 string
    /// This is necessary because Rust strings are UTF-8 encoded, so character positions
    /// don't always align with byte positions for multi-byte characters
    fn char_index_to_byte_index(format_str: &str, char_index: usize) -> usize {
        format_str
            .char_indices()
            .nth(char_index)
            .map_or(format_str.len(), |(byte_idx, _)| byte_idx)
    }

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

        Self::process_flags(chars, i, bound, &mut flag);

        let mut width = 0;
        let mut precision = Precision::NotSpecified;
        let mut j = *i;

        let j_byte = Self::char_index_to_byte_index(format_str, j);
        if let Some((field_width, offset)) = format_str[j_byte..].scan_num::<usize>() {
            width = field_width;
            j += offset;

            // Reject directives like `%<NUMBER>` by checking if width has been parsed.
            if j >= bound || chars[j] == '%' {
                let invalid_directive: String = chars[old..=j.min(bound - 1)].iter().collect();
                return Err(USimpleError::new(
                    1,
                    StatError::InvalidDirective {
                        directive: invalid_directive.quote().to_string(),
                    }
                    .to_string(),
                ));
            }
        }
        check_bound(format_str, bound, old, j)?;

        if chars[j] == '.' {
            j += 1;
            check_bound(format_str, bound, old, j)?;

            let j_byte = Self::char_index_to_byte_index(format_str, j);
            match format_str[j_byte..].scan_num::<i32>() {
                Some((value, offset)) => {
                    if value >= 0 {
                        precision = Precision::Number(value as usize);
                    }
                    j += offset;
                }
                None => precision = Precision::NoNumber,
            }
            check_bound(format_str, bound, old, j)?;
        }

        *i = j;

        // Check for multi-character specifiers (e.g., `%Hd`, `%Lr`)
        if *i + 1 < bound {
            if let Some(&next_char) = chars.get(*i + 1) {
                if (chars[*i] == 'H' || chars[*i] == 'L') && (next_char == 'd' || next_char == 'r')
                {
                    let specifier = format!("{}{next_char}", chars[*i]);
                    *i += 1;
                    return Ok(Token::Directive {
                        flag,
                        width,
                        precision,
                        format: specifier.chars().next().unwrap(),
                    });
                }
            }
        }

        Ok(Token::Directive {
            flag,
            width,
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
            show_warning!("{}", translate!("stat-warning-backslash-end-format"));
            return Token::Char('\\');
        }
        match chars[*i] {
            'a' => Token::Byte(0x07),   // BEL
            'b' => Token::Byte(0x08),   // Backspace
            'f' => Token::Byte(0x0C),   // Form feed
            'n' => Token::Byte(0x0A),   // Line feed
            'r' => Token::Byte(0x0D),   // Carriage return
            't' => Token::Byte(0x09),   // Horizontal tab
            '\\' => Token::Byte(b'\\'), // Backslash
            '\'' => Token::Byte(b'\''), // Single quote
            '"' => Token::Byte(b'"'),   // Double quote
            '0'..='7' => {
                // Parse octal escape sequence (up to 3 digits)
                let mut value = 0u8;
                let mut count = 0;
                while *i < bound && count < 3 {
                    if let Some(digit) = chars[*i].to_digit(8) {
                        value = value * 8 + digit as u8;
                        *i += 1;
                        count += 1;
                    } else {
                        break;
                    }
                }
                *i -= 1; // Adjust index to account for the outer loop increment
                Token::Byte(value)
            }
            'x' => {
                // Parse hexadecimal escape sequence (\xNN format)
                // Uses UTF-8 safe byte indexing to handle multi-byte characters properly
                if *i + 1 < bound {
                    let byte_index = Self::char_index_to_byte_index(format_str, *i + 1);
                    if let Some((c, offset)) = format_str[byte_index..].scan_char(16) {
                        *i += offset;
                        Token::Byte(c as u8)
                    } else {
                        show_warning!("{}", translate!("stat-warning-unrecognized-escape-x"));
                        Token::Byte(b'x')
                    }
                } else {
                    show_warning!("{}", translate!("stat-warning-incomplete-hex-escape"));
                    Token::Byte(b'x')
                }
            }
            other => {
                show_warning!(
                    "{}",
                    translate!("stat-warning-unrecognized-escape", "escape" => other)
                );
                Token::Byte(other as u8)
            }
        }
    }

    fn generate_tokens(format_str: &str, use_printf: bool) -> UResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let chars = format_str.chars().collect::<Vec<char>>();
        let bound = chars.len();
        let mut i = 0;
        while i < bound {
            match chars.get(i) {
                Some('%') => tokens.push(Self::handle_percent_case(
                    &chars, &mut i, bound, format_str,
                )?),
                Some('\\') => {
                    if use_printf {
                        tokens.push(Self::handle_escape_sequences(
                            &chars, &mut i, bound, format_str,
                        ));
                    } else {
                        tokens.push(Token::Char('\\'));
                    }
                }
                Some(c) => tokens.push(Token::Char(*c)),
                None => break,
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
            return Err(Box::new(StatError::MissingOperand) as Box<dyn UError>);
        }
        let format_str = if matches.contains_id(options::PRINTF) {
            matches
                .get_one::<String>(options::PRINTF)
                .expect("Invalid format string")
        } else {
            matches
                .get_one::<String>(options::FORMAT)
                .map_or("", |s| s.as_str())
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
                    USimpleError::new(
                        e.code(),
                        StatError::CannotReadFilesystem {
                            error: e.to_string(),
                        }
                        .to_string(),
                    )
                })?
                .iter()
                .map(|mi| mi.mount_dir.clone())
                .collect::<Vec<_>>();
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

    fn find_mount_point<P: AsRef<Path>>(&self, p: P) -> Option<&OsString> {
        let path = p.as_ref().canonicalize().ok()?;
        self.mount_list
            .as_ref()?
            .iter()
            .find(|root| path.starts_with(root))
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

    #[allow(clippy::too_many_arguments)]
    fn process_token_files(
        &self,
        t: &Token,
        meta: &Metadata,
        display_name: &str,
        file: &OsString,
        file_type: &FileType,
        from_user: bool,
        _follow_symbolic_links: bool,
    ) -> Result<(), i32> {
        match *t {
            Token::Byte(byte) => write_raw_byte(byte),
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
                    'A' => OutputType::Str(display_permissions(meta, true)),
                    // number of blocks allocated (see %B)
                    'b' => OutputType::Unsigned(meta.blocks()),

                    // the size in bytes of each block reported by %b
                    // FIXME: blocksize differs on various platform
                    // See coreutils/gnulib/lib/stat-size.h ST_NBLOCKSIZE // spell-checker:disable-line
                    'B' => OutputType::Unsigned(512),
                    // SELinux security context string
                    'C' => {
                        #[cfg(feature = "selinux")]
                        {
                            if uucore::selinux::is_selinux_enabled() {
                                match uucore::selinux::get_selinux_security_context(
                                    Path::new(file),
                                    _follow_symbolic_links,
                                ) {
                                    Ok(ctx) => OutputType::Str(ctx),
                                    Err(_) => OutputType::Str(translate!(
                                        "stat-selinux-failed-get-context"
                                    )),
                                }
                            } else {
                                OutputType::Str(translate!("stat-selinux-unsupported-system"))
                            }
                        }
                        #[cfg(not(feature = "selinux"))]
                        {
                            OutputType::Str(translate!("stat-selinux-unsupported-os"))
                        }
                    }
                    // device number in decimal
                    'd' => OutputType::Unsigned(meta.dev()),
                    // device number in hex
                    'D' => OutputType::UnsignedHex(meta.dev()),
                    // raw mode in hex
                    'f' => OutputType::UnsignedHex(meta.mode() as u64),
                    // file type
                    'F' => OutputType::Str(pretty_filetype(meta.mode() as mode_t, meta.len())),
                    // group ID of owner
                    'g' => OutputType::Unsigned(meta.gid() as u64),
                    // group name of owner
                    'G' => {
                        let group_name =
                            entries::gid2grp(meta.gid()).unwrap_or_else(|_| "UNKNOWN".to_owned());
                        OutputType::Str(group_name)
                    }
                    // number of hard links
                    'h' => OutputType::Unsigned(meta.nlink()),
                    // inode number
                    'i' => OutputType::Unsigned(meta.ino()),
                    // mount point
                    'm' => match self.find_mount_point(file) {
                        Some(s) => OutputType::OsStr(s),
                        None => OutputType::Str(String::new()),
                    },
                    // file name
                    'n' => OutputType::Str(display_name.to_string()),
                    // quoted file name with dereference if symbolic link
                    'N' => {
                        let file_name =
                            get_quoted_file_name(display_name, file, file_type, from_user)?;
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
                        let user_name =
                            entries::uid2usr(meta.uid()).unwrap_or_else(|_| "UNKNOWN".to_owned());
                        OutputType::Str(user_name)
                    }

                    // time of file birth, human-readable; - if unknown
                    'w' => OutputType::Str(pretty_time(meta, MetadataTimeField::Birth)),

                    // time of file birth, seconds since Epoch; 0 if unknown
                    'W' => OutputType::Integer(
                        metadata_get_time(meta, MetadataTimeField::Birth)
                            .map_or(0, |x| system_time_to_sec(x).0),
                    ),

                    // time of last access, human-readable
                    'x' => OutputType::Str(pretty_time(meta, MetadataTimeField::Access)),
                    // time of last access, seconds since Epoch
                    'X' => {
                        let (sec, nsec) = metadata_get_time(meta, MetadataTimeField::Access)
                            .map_or((0, 0), system_time_to_sec);
                        OutputType::Float(sec as f64 + nsec as f64 / 1_000_000_000.0)
                    }
                    // time of last data modification, human-readable
                    'y' => OutputType::Str(pretty_time(meta, MetadataTimeField::Modification)),
                    // time of last data modification, seconds since Epoch
                    'Y' => {
                        let (sec, nsec) = metadata_get_time(meta, MetadataTimeField::Modification)
                            .map_or((0, 0), system_time_to_sec);
                        OutputType::Float(sec as f64 + nsec as f64 / 1_000_000_000.0)
                    }
                    // time of last status change, human-readable
                    'z' => OutputType::Str(pretty_time(meta, MetadataTimeField::Change)),
                    // time of last status change, seconds since Epoch
                    'Z' => {
                        let (sec, nsec) = metadata_get_time(meta, MetadataTimeField::Change)
                            .map_or((0, 0), system_time_to_sec);
                        OutputType::Float(sec as f64 + nsec as f64 / 1_000_000_000.0)
                    }
                    'R' => {
                        let major = meta.rdev() >> 8;
                        let minor = meta.rdev() & 0xff;
                        OutputType::Str(format!("{major},{minor}"))
                    }
                    'r' => OutputType::Unsigned(meta.rdev()),
                    'H' => OutputType::Unsigned(meta.rdev() >> 8), // Major in decimal
                    'L' => OutputType::Unsigned(meta.rdev() & 0xff), // Minor in decimal

                    _ => OutputType::Unknown,
                };
                print_it(&output, flag, width, precision);
            }
        }
        Ok(())
    }

    fn do_stat(&self, file: &OsStr, stdin_is_fifo: bool) -> i32 {
        let display_name = file.to_string_lossy();
        let file = if cfg!(unix) && display_name == "-" {
            if self.show_fs {
                show_error!("{}", StatError::StdinFilesystemMode);
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
            match statfs(&file) {
                Ok(meta) => {
                    let tokens = &self.default_tokens;

                    // Usage
                    for t in tokens {
                        process_token_filesystem(t, &meta, &display_name);
                    }
                }
                Err(error) => {
                    show_error!(
                        "{}",
                        StatError::CannotReadFilesystemInfo {
                            file: display_name.quote().to_string(),
                            error
                        }
                    );
                    return 1;
                }
            }
        } else {
            let follow_symbolic_links = self.follow || stdin_is_fifo && display_name == "-";
            let result = if follow_symbolic_links {
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
                        if let Err(code) = self.process_token_files(
                            t,
                            &meta,
                            &display_name,
                            &file,
                            &file_type,
                            self.from_user,
                            follow_symbolic_links,
                        ) {
                            return code;
                        }
                    }
                }
                Err(e) => {
                    show_error!(
                        "{}",
                        StatError::CannotStat {
                            file: display_name.quote().to_string(),
                            error: e.to_string()
                        }
                    );
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
                format!(
                    "  {}: \"%n\"\n    {}: %-8i {}: %-7l {}: %T\n{} \
                         {}: %-10s {} {}: %S\n{}: {}: %-10b \
                         {}: %-10f {}: %a\n{}: {}: %-10c {}: %d\n",
                    translate!("stat-word-file"),
                    translate!("stat-word-id"),
                    translate!("stat-word-namelen"),
                    translate!("stat-word-type"),
                    translate!("stat-word-block"),
                    translate!("stat-word-size"),
                    translate!("stat-word-fundamental"),
                    translate!("stat-word-block-size"),
                    translate!("stat-word-blocks"),
                    translate!("stat-word-total"),
                    translate!("stat-word-free"),
                    translate!("stat-word-available"),
                    translate!("stat-word-inodes"),
                    translate!("stat-word-total"),
                    translate!("stat-word-free")
                )
            }
        } else if terse {
            "%n %s %b %f %u %g %D %i %h %t %T %X %Y %Z %W %o\n".into()
        } else {
            let device_line = if show_dev_type {
                format!(
                    "{}: %Dh/%dd\t{}: %-10i  {}: %-5h {} {}: %t,%T\n",
                    translate!("stat-word-device"),
                    translate!("stat-word-inode"),
                    translate!("stat-word-links"),
                    translate!("stat-word-device"),
                    translate!("stat-word-type")
                )
            } else {
                format!(
                    "{}: %Dh/%dd\t{}: %-10i  {}: %h\n",
                    translate!("stat-word-device"),
                    translate!("stat-word-inode"),
                    translate!("stat-word-links")
                )
            };

            format!(
                "  {}: %N\n  {}: %-10s\t{}: %-10b {} {}: %-6o %F\n{}{}: (%04a/%10.10A)  {}: (%5u/%8U)   {}: (%5g/%8G)\n{}: %x\n{}: %y\n{}: %z\n {}: %w\n",
                translate!("stat-word-file"),
                translate!("stat-word-size"),
                translate!("stat-word-blocks"),
                translate!("stat-word-io"),
                translate!("stat-word-block"),
                device_line,
                translate!("stat-word-access"),
                translate!("stat-word-uid"),
                translate!("stat-word-gid"),
                translate!("stat-word-access"),
                translate!("stat-word-modify"),
                translate!("stat-word-change"),
                translate!("stat-word-birth")
            )
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

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
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("stat-about"))
        .after_help(translate!("stat-after-help"))
        .override_usage(format_usage(&translate!("stat-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help(translate!("stat-help-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .help(translate!("stat-help-file-system"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TERSE)
                .short('t')
                .long(options::TERSE)
                .help(translate!("stat-help-terse"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORMAT)
                .short('c')
                .long(options::FORMAT)
                .help(translate!("stat-help-format"))
                .value_name("FORMAT"),
        )
        .arg(
            Arg::new(options::PRINTF)
                .long(options::PRINTF)
                .value_name("FORMAT")
                .help(translate!("stat-help-printf")),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}

const PRETTY_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S.%N %z";

fn pretty_time(meta: &Metadata, md_time_field: MetadataTimeField) -> String {
    if let Some(time) = metadata_get_time(meta, md_time_field) {
        let mut tmp = Vec::new();
        if format_system_time(
            &mut tmp,
            time,
            PRETTY_DATETIME_FORMAT,
            FormatSystemTimeFallback::Float,
        )
        .is_ok()
        {
            return String::from_utf8(tmp).unwrap();
        }
    }
    "-".to_string()
}

#[cfg(test)]
mod tests {
    use crate::{pad_and_print_bytes, print_padding, quote_file_name};

    use super::{Flags, Precision, ScanUtil, Stater, Token, group_num, precision_trunc};

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
                precision: Precision::Number(2),
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
                precision: Precision::NoNumber,
                format: 'w',
            },
            Token::Char('\n'),
        ];
        assert_eq!(&expected, &Stater::generate_tokens(s, false).unwrap());
    }

    #[test]
    fn printf_format() {
        let s = r#"%-# 15a\t\r\"\\\a\b\x1B\f\x0B%+020.-23w\x12\167\132\112\n"#;
        let expected = vec![
            Token::Directive {
                flag: Flags {
                    left: true,
                    alter: true,
                    space: true,
                    ..Default::default()
                },
                width: 15,
                precision: Precision::NotSpecified,
                format: 'a',
            },
            Token::Byte(b'\t'),
            Token::Byte(b'\r'),
            Token::Byte(b'"'),
            Token::Byte(b'\\'),
            Token::Byte(b'\x07'),
            Token::Byte(b'\x08'),
            Token::Byte(b'\x1B'),
            Token::Byte(b'\x0C'),
            Token::Byte(b'\x0B'),
            Token::Directive {
                flag: Flags {
                    sign: true,
                    zero: true,
                    ..Default::default()
                },
                width: 20,
                precision: Precision::NotSpecified,
                format: 'w',
            },
            Token::Byte(b'\x12'),
            Token::Byte(b'w'),
            Token::Byte(b'Z'),
            Token::Byte(b'J'),
            Token::Byte(b'\n'),
        ];
        assert_eq!(&expected, &Stater::generate_tokens(s, true).unwrap());
    }

    #[test]
    fn test_precision_trunc() {
        assert_eq!(precision_trunc(123.456, Precision::NotSpecified), "123");
        assert_eq!(precision_trunc(123.456, Precision::NoNumber), "123.456");
        assert_eq!(precision_trunc(123.456, Precision::Number(0)), "123");
        assert_eq!(precision_trunc(123.456, Precision::Number(1)), "123.4");
        assert_eq!(precision_trunc(123.456, Precision::Number(2)), "123.45");
        assert_eq!(precision_trunc(123.456, Precision::Number(3)), "123.456");
        assert_eq!(precision_trunc(123.456, Precision::Number(4)), "123.4560");
        assert_eq!(precision_trunc(123.456, Precision::Number(5)), "123.45600");
    }

    #[test]
    fn test_pad_and_print_bytes() {
        // testing non-utf8 with normal settings
        let mut buffer = Vec::new();
        let bytes = b"\x80\xFF\x80";
        pad_and_print_bytes(&mut buffer, bytes, false, 3, Precision::NotSpecified).unwrap();
        assert_eq!(&buffer, b"\x80\xFF\x80");

        // testing left padding
        let mut buffer = Vec::new();
        let bytes = b"\x80\xFF\x80";
        pad_and_print_bytes(&mut buffer, bytes, false, 5, Precision::NotSpecified).unwrap();
        assert_eq!(&buffer, b"  \x80\xFF\x80");

        // testing right padding
        let mut buffer = Vec::new();
        let bytes = b"\x80\xFF\x80";
        pad_and_print_bytes(&mut buffer, bytes, true, 5, Precision::NotSpecified).unwrap();
        assert_eq!(&buffer, b"\x80\xFF\x80  ");
    }

    #[test]
    fn test_print_padding() {
        let mut buffer = Vec::new();
        print_padding(&mut buffer, 5).unwrap();
        assert_eq!(&buffer, b"     ");
    }

    #[test]
    fn test_quote_file_name() {
        let file_name = "nice' file";
        assert_eq!(
            quote_file_name(file_name, &crate::QuotingStyle::ShellEscapeAlways),
            "\"nice' file\""
        );

        let file_name = "nice\" file";
        assert_eq!(
            quote_file_name(file_name, &crate::QuotingStyle::ShellEscapeAlways),
            "\'nice\" file\'"
        );
    }
}
