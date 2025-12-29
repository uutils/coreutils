// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ctty, ctable, iseek, oseek, iconvflags, oconvflags parseargs outfile oconv

#[cfg(test)]
mod unit_tests;

use super::{ConversionMode, IConvFlags, IFlags, Num, OConvFlags, OFlags, Settings, StatusLevel};
use crate::conversion_tables::ConversionTable;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::parser::parse_size::{ParseSizeError, Parser as SizeParser};
use uucore::show_warning;
use uucore::translate;

/// Parser Errors describe errors with parser input
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{}", translate!("dd-error-unrecognized-operand", "operand" => .0.clone()))]
    UnrecognizedOperand(String),
    #[error("{}", translate!("dd-error-multiple-format-table"))]
    MultipleFmtTable,
    #[error("{}", translate!("dd-error-multiple-case"))]
    MultipleUCaseLCase,
    #[error("{}", translate!("dd-error-multiple-block"))]
    MultipleBlockUnblock,
    #[error("{}", translate!("dd-error-multiple-excl"))]
    MultipleExclNoCreate,
    #[error("{}", translate!("dd-error-invalid-flag", "flag" => .0.clone(), "cmd" => uucore::execution_phrase()))]
    FlagNoMatch(String),
    #[error("{}", translate!("dd-error-conv-flag-no-match", "flag" => .0.clone()))]
    ConvFlagNoMatch(String),
    #[error("{}", translate!("dd-error-multiplier-parse-failure", "input" => .0.clone()))]
    MultiplierStringParseFailure(String),
    #[error("{}", translate!("dd-error-multiplier-overflow", "input" => .0.clone()))]
    MultiplierStringOverflow(String),
    #[error("{}", translate!("dd-error-block-without-cbs"))]
    BlockUnblockWithoutCBS,
    #[error("{}", translate!("dd-error-status-not-recognized", "level" => .0.clone()))]
    StatusLevelNotRecognized(String),
    #[error("{}", translate!("dd-error-unimplemented", "feature" => .0.clone()))]
    Unimplemented(String),
    #[error("{}", translate!("dd-error-bs-out-of-range", "param" => .0.clone()))]
    BsOutOfRange(String),
    #[error("{}", translate!("dd-error-invalid-number", "input" => .0.clone()))]
    InvalidNumber(String),
    #[error("invalid number: ‘{0}’: {1}")]
    InvalidNumberWithErrMsg(String, String),
}

/// Contains a temporary state during parsing of the arguments
#[derive(Debug, PartialEq, Default)]
pub struct Parser {
    infile: Option<String>,
    outfile: Option<String>,
    /// The block size option specified on the command-line, if any.
    bs: Option<usize>,
    /// The input block size option specified on the command-line, if any.
    ibs: Option<usize>,
    /// The output block size option specified on the command-line, if any.
    obs: Option<usize>,
    cbs: Option<usize>,
    skip: Num,
    seek: Num,
    count: Option<Num>,
    conv: ConvFlags,
    /// Whether a data-transforming `conv` option has been specified.
    is_conv_specified: bool,
    iflag: IFlags,
    oflag: OFlags,
    status: Option<StatusLevel>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ConvFlags {
    ascii: bool,
    ebcdic: bool,
    ibm: bool,
    ucase: bool,
    lcase: bool,
    block: bool,
    unblock: bool,
    swab: bool,
    sync: bool,
    noerror: bool,
    sparse: bool,
    excl: bool,
    nocreat: bool,
    notrunc: bool,
    fdatasync: bool,
    fsync: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Conversion {
    Ascii,
    Ebcdic,
    Ibm,
}

#[derive(Clone, Copy)]
enum Case {
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
enum Block {
    Block(usize),
    Unblock(usize),
}

/// Return an Unimplemented error when the target is not Linux or Android
macro_rules! linux_only {
    ($s: expr, $val: expr) => {
        if cfg!(any(target_os = "linux", target_os = "android")) {
            $val
        } else {
            return Err(ParseError::Unimplemented($s.to_string()).into());
        }
    };
}

impl Parser {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(
        self,
        operands: impl IntoIterator<Item: AsRef<str>>,
    ) -> Result<Settings, ParseError> {
        self.read(operands)?.validate()
    }

    pub(crate) fn read(
        mut self,
        operands: impl IntoIterator<Item: AsRef<str>>,
    ) -> Result<Self, ParseError> {
        for operand in operands {
            self.parse_operand(operand.as_ref())?;
        }

        Ok(self)
    }

    pub(crate) fn validate(self) -> Result<Settings, ParseError> {
        let conv = self.conv;
        let conversion = match (conv.ascii, conv.ebcdic, conv.ibm) {
            (false, false, false) => None,
            (true, false, false) => Some(Conversion::Ascii),
            (false, true, false) => Some(Conversion::Ebcdic),
            (false, false, true) => Some(Conversion::Ibm),
            _ => return Err(ParseError::MultipleFmtTable),
        };

        let case = match (conv.ucase, conv.lcase) {
            (false, false) => None,
            (true, false) => Some(Case::Upper),
            (false, true) => Some(Case::Lower),
            (true, true) => return Err(ParseError::MultipleUCaseLCase),
        };

        let non_ascii = matches!(conversion, Some(Conversion::Ascii));
        let conversion_table = get_ctable(conversion, case);

        if conv.nocreat && conv.excl {
            return Err(ParseError::MultipleExclNoCreate);
        }

        // The GNU docs state that
        // - ascii implies unblock
        // - ebcdic and ibm imply block
        // This has a side effect in how it's implemented in GNU, because this errors:
        //     conv=block,unblock
        // but these don't:
        //     conv=ascii,block,unblock
        //     conv=block,ascii,unblock
        //     conv=block,unblock,ascii
        //     conv=block conv=unblock conv=ascii
        let block = if let Some(cbs) = self.cbs {
            match conversion {
                Some(Conversion::Ascii) => Some(Block::Unblock(cbs)),
                Some(_) => Some(Block::Block(cbs)),
                None => match (conv.block, conv.unblock) {
                    (false, false) => None,
                    (true, false) => Some(Block::Block(cbs)),
                    (false, true) => Some(Block::Unblock(cbs)),
                    (true, true) => return Err(ParseError::MultipleBlockUnblock),
                },
            }
        } else if conv.block || conv.unblock {
            return Err(ParseError::BlockUnblockWithoutCBS);
        } else {
            None
        };

        let iconv = IConvFlags {
            mode: conversion_mode(conversion_table, block, non_ascii, conv.sync),
            swab: conv.swab,
            sync: if conv.sync {
                if block.is_some() {
                    Some(b' ')
                } else {
                    Some(0u8)
                }
            } else {
                None
            },
            noerror: conv.noerror,
        };

        let oconv = OConvFlags {
            sparse: conv.sparse,
            excl: conv.excl,
            nocreat: conv.nocreat,
            notrunc: conv.notrunc,
            fdatasync: conv.fdatasync,
            fsync: conv.fsync,
        };

        // Input and output block sizes.
        //
        // The `bs` option takes precedence. If either is not
        // provided, `ibs` and `obs` are each 512 bytes by default.
        let (ibs, obs) = match self.bs {
            None => (self.ibs.unwrap_or(512), self.obs.unwrap_or(512)),
            Some(bs) => (bs, bs),
        };

        // Whether to buffer partial output blocks until they are completed.
        //
        // From the GNU `dd` documentation for the `bs=BYTES` option:
        //
        // > [...] if no data-transforming 'conv' option is specified,
        // > input is copied to the output as soon as it's read, even if
        // > it is smaller than the block size.
        //
        let buffered = self.bs.is_none() || self.is_conv_specified;

        let skip = self
            .skip
            .force_bytes_if(self.iflag.skip_bytes)
            .to_bytes(ibs as u64);
        // GNU coreutils has a limit of i64 (intmax_t)
        if skip > i64::MAX as u64 {
            return Err(ParseError::InvalidNumberWithErrMsg(
                format!("{skip}"),
                "Value too large for defined data type".to_string(),
            ));
        }

        let seek = self
            .seek
            .force_bytes_if(self.oflag.seek_bytes)
            .to_bytes(obs as u64);
        // GNU coreutils has a limit of i64 (intmax_t)
        if seek > i64::MAX as u64 {
            return Err(ParseError::InvalidNumberWithErrMsg(
                format!("{seek}"),
                "Value too large for defined data type".to_string(),
            ));
        }

        let count = self.count.map(|c| c.force_bytes_if(self.iflag.count_bytes));

        Ok(Settings {
            skip,
            seek,
            count,
            iconv,
            oconv,
            ibs,
            obs,
            buffered,
            infile: self.infile,
            outfile: self.outfile,
            iflags: self.iflag,
            oflags: self.oflag,
            status: self.status,
        })
    }

    fn parse_operand(&mut self, operand: &str) -> Result<(), ParseError> {
        match operand.split_once('=') {
            None => return Err(ParseError::UnrecognizedOperand(operand.to_string())),
            Some((k, v)) => match k {
                "bs" => self.bs = Some(Self::parse_bytes(k, v)?),
                "cbs" => self.cbs = Some(Self::parse_bytes(k, v)?),
                "conv" => {
                    self.is_conv_specified = true;
                    self.parse_conv_flags(v)?;
                }
                "count" => self.count = Some(Self::parse_n(v)?),
                "ibs" => self.ibs = Some(Self::parse_bytes(k, v)?),
                "if" => self.infile = Some(v.to_string()),
                "iflag" => self.parse_input_flags(v)?,
                "obs" => self.obs = Some(Self::parse_bytes(k, v)?),
                "of" => self.outfile = Some(v.to_string()),
                "oflag" => self.parse_output_flags(v)?,
                "seek" | "oseek" => self.seek = Self::parse_n(v)?,
                "skip" | "iseek" => self.skip = Self::parse_n(v)?,
                "status" => self.status = Some(Self::parse_status_level(v)?),
                _ => return Err(ParseError::UnrecognizedOperand(operand.to_string())),
            },
        }
        Ok(())
    }

    fn parse_n(val: &str) -> Result<Num, ParseError> {
        let n = parse_bytes_with_opt_multiplier(val)?;
        Ok(if val.contains('B') {
            Num::Bytes(n)
        } else {
            Num::Blocks(n)
        })
    }

    fn parse_bytes(arg: &str, val: &str) -> Result<usize, ParseError> {
        parse_bytes_with_opt_multiplier(val)?
            .try_into()
            .map_err(|_| ParseError::BsOutOfRange(arg.to_string()))
    }

    fn parse_status_level(val: &str) -> Result<StatusLevel, ParseError> {
        match val {
            "none" => Ok(StatusLevel::None),
            "noxfer" => Ok(StatusLevel::Noxfer),
            "progress" => Ok(StatusLevel::Progress),
            _ => Err(ParseError::StatusLevelNotRecognized(val.to_string())),
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn parse_input_flags(&mut self, val: &str) -> Result<(), ParseError> {
        let i = &mut self.iflag;
        for f in val.split(',') {
            match f {
                // Common flags
                "cio" => return Err(ParseError::Unimplemented(f.to_string())),
                "direct" => linux_only!(f, i.direct = true),
                "directory" => linux_only!(f, i.directory = true),
                "dsync" => linux_only!(f, i.dsync = true),
                "sync" => linux_only!(f, i.sync = true),
                "nocache" => linux_only!(f, i.nocache = true),
                "nonblock" => linux_only!(f, i.nonblock = true),
                "noatime" => linux_only!(f, i.noatime = true),
                "noctty" => linux_only!(f, i.noctty = true),
                "nofollow" => linux_only!(f, i.nofollow = true),
                "nolinks" => return Err(ParseError::Unimplemented(f.to_string())),
                "binary" => return Err(ParseError::Unimplemented(f.to_string())),
                "text" => return Err(ParseError::Unimplemented(f.to_string())),

                // Input-only flags
                "fullblock" => i.fullblock = true,
                "count_bytes" => i.count_bytes = true,
                "skip_bytes" => i.skip_bytes = true,
                // GNU silently ignores oflags given as iflag.
                "append" | "seek_bytes" => {}
                _ => return Err(ParseError::FlagNoMatch(f.to_string())),
            }
        }
        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    fn parse_output_flags(&mut self, val: &str) -> Result<(), ParseError> {
        let o = &mut self.oflag;
        for f in val.split(',') {
            match f {
                // Common flags
                "cio" => return Err(ParseError::Unimplemented(val.to_string())),
                "direct" => linux_only!(f, o.direct = true),
                "directory" => linux_only!(f, o.directory = true),
                "dsync" => linux_only!(f, o.dsync = true),
                "sync" => linux_only!(f, o.sync = true),
                "nocache" => linux_only!(f, o.nocache = true),
                "nonblock" => linux_only!(f, o.nonblock = true),
                "noatime" => linux_only!(f, o.noatime = true),
                "noctty" => linux_only!(f, o.noctty = true),
                "nofollow" => linux_only!(f, o.nofollow = true),
                "nolinks" => return Err(ParseError::Unimplemented(f.to_string())),
                "binary" => return Err(ParseError::Unimplemented(f.to_string())),
                "text" => return Err(ParseError::Unimplemented(f.to_string())),

                // Output-only flags
                "append" => o.append = true,
                "seek_bytes" => o.seek_bytes = true,
                // GNU silently ignores iflags given as oflag.
                "fullblock" | "count_bytes" | "skip_bytes" => {}
                _ => return Err(ParseError::FlagNoMatch(f.to_string())),
            }
        }
        Ok(())
    }

    fn parse_conv_flags(&mut self, val: &str) -> Result<(), ParseError> {
        let c = &mut self.conv;
        for f in val.split(',') {
            match f {
                // Conversion
                "ascii" => c.ascii = true,
                "ebcdic" => c.ebcdic = true,
                "ibm" => c.ibm = true,

                // Case
                "lcase" => c.lcase = true,
                "ucase" => c.ucase = true,

                // Block
                "block" => c.block = true,
                "unblock" => c.unblock = true,

                // Other input
                "swab" => c.swab = true,
                "sync" => c.sync = true,
                "noerror" => c.noerror = true,

                // Output
                "sparse" => c.sparse = true,
                "excl" => c.excl = true,
                "nocreat" => c.nocreat = true,
                "notrunc" => c.notrunc = true,
                "fdatasync" => c.fdatasync = true,
                "fsync" => c.fsync = true,
                _ => return Err(ParseError::ConvFlagNoMatch(f.to_string())),
            }
        }
        Ok(())
    }
}

impl UError for ParseError {
    fn code(&self) -> i32 {
        1
    }
}

fn show_zero_multiplier_warning() {
    show_warning!(
        "{}",
        translate!("dd-warning-zero-multiplier", "zero" => "0x".quote(), "alternative" => "00x".quote())
    );
}

/// Parse bytes using [`str::parse`], then map error if needed.
fn parse_bytes_only(s: &str, i: usize) -> Result<u64, ParseError> {
    s[..i]
        .parse()
        .map_err(|_| ParseError::MultiplierStringParseFailure(s.to_string()))
}

/// Parse a number of bytes from the given string, assuming no `'x'` characters.
///
/// The `'x'` character means "multiply the number before the `'x'` by
/// the number after the `'x'`". In order to compute the numbers
/// before and after the `'x'`, use this function, which assumes there
/// are no `'x'` characters in the string.
///
/// A suffix `'c'` means multiply by 1, `'w'` by 2, and `'b'` by
/// 512. You can also use standard block size suffixes like `'k'` for
/// 1024.
///
/// If the number would be too large, return [`u64::MAX`] instead.
///
/// # Errors
///
/// If a number cannot be parsed or if the multiplication would cause
/// an overflow.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_bytes_no_x("123", "123").unwrap(), 123);
/// assert_eq!(parse_bytes_no_x("2c", "2c").unwrap(), 2 * 1);
/// assert_eq!(parse_bytes_no_x("3w", "3w").unwrap(), 3 * 2);
/// assert_eq!(parse_bytes_no_x("2b", "2b").unwrap(), 2 * 512);
/// assert_eq!(parse_bytes_no_x("2k", "2k").unwrap(), 2 * 1024);
/// ```
fn parse_bytes_no_x(full: &str, s: &str) -> Result<u64, ParseError> {
    let parser = SizeParser {
        capital_b_bytes: true,
        no_empty_numeric: true,
        ..Default::default()
    };
    let (num, multiplier) = match (s.find('c'), s.rfind('w'), s.rfind('b')) {
        (None, None, None) => match parser.parse_u64(s) {
            Ok(n) => (n, 1),
            Err(ParseSizeError::SizeTooBig(_)) => (u64::MAX, 1),
            Err(_) => return Err(ParseError::InvalidNumber(full.to_string())),
        },
        (Some(i), None, None) => (parse_bytes_only(s, i)?, 1),
        (None, Some(i), None) => (parse_bytes_only(s, i)?, 2),
        (None, None, Some(i)) => (parse_bytes_only(s, i)?, 512),
        _ => return Err(ParseError::MultiplierStringParseFailure(full.to_string())),
    };
    num.checked_mul(multiplier)
        .ok_or_else(|| ParseError::MultiplierStringOverflow(full.to_string()))
}

/// Parse byte and multiplier like 512, 5KiB, or 1G.
/// Uses [`uucore::parser::parse_size`], and adds the 'w' and 'c' suffixes which are mentioned
/// in dd's info page.
pub fn parse_bytes_with_opt_multiplier(s: &str) -> Result<u64, ParseError> {
    // TODO On my Linux system, there seems to be a maximum block size of 4096 bytes:
    //
    //     $ printf "%0.sa" {1..10000} | dd bs=4095 count=1 status=none | wc -c
    //     4095
    //     $ printf "%0.sa" {1..10000} | dd bs=4k count=1 status=none | wc -c
    //     4096
    //     $ printf "%0.sa" {1..10000} | dd bs=4097 count=1 status=none | wc -c
    //     4096
    //     $ printf "%0.sa" {1..10000} | dd bs=5k count=1 status=none | wc -c
    //     4096
    //

    // Split on the 'x' characters. Each component will be parsed
    // individually, then multiplied together.
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 1 {
        parse_bytes_no_x(s, parts[0])
    } else {
        let mut total: u64 = 1;
        for part in parts {
            if part == "0" {
                show_zero_multiplier_warning();
            }
            let num = parse_bytes_no_x(s, part)?;
            total = total
                .checked_mul(num)
                .ok_or_else(|| ParseError::InvalidNumber(s.to_string()))?;
        }
        Ok(total)
    }
}

fn get_ctable(
    conversion: Option<Conversion>,
    case: Option<Case>,
) -> Option<&'static ConversionTable> {
    use crate::conversion_tables::*;
    Some(match (conversion, case) {
        (None, None) => return None,
        (Some(conv), None) => match conv {
            Conversion::Ascii => &EBCDIC_TO_ASCII,
            Conversion::Ebcdic => &ASCII_TO_EBCDIC,
            Conversion::Ibm => &ASCII_TO_IBM,
        },
        (None, Some(case)) => match case {
            Case::Lower => &ASCII_UCASE_TO_LCASE,
            Case::Upper => &ASCII_LCASE_TO_UCASE,
        },
        (Some(conv), Some(case)) => match (conv, case) {
            (Conversion::Ascii, Case::Upper) => &EBCDIC_TO_ASCII_LCASE_TO_UCASE,
            (Conversion::Ascii, Case::Lower) => &EBCDIC_TO_ASCII_UCASE_TO_LCASE,
            (Conversion::Ebcdic, Case::Upper) => &ASCII_TO_EBCDIC_LCASE_TO_UCASE,
            (Conversion::Ebcdic, Case::Lower) => &ASCII_TO_EBCDIC_UCASE_TO_LCASE,
            (Conversion::Ibm, Case::Upper) => &ASCII_TO_IBM_UCASE_TO_LCASE,
            (Conversion::Ibm, Case::Lower) => &ASCII_TO_IBM_LCASE_TO_UCASE,
        },
    })
}

/// Given the various command-line parameters, determine the conversion mode.
///
/// The `conv` command-line option can take many different values,
/// each of which may combine with others. For example, `conv=ascii`,
/// `conv=lcase`, `conv=sync`, and so on. The arguments to this
/// function represent the settings of those various command-line
/// parameters. This function translates those settings to a
/// [`ConversionMode`].
fn conversion_mode(
    ctable: Option<&'static ConversionTable>,
    block: Option<Block>,
    is_ascii: bool,
    is_sync: bool,
) -> Option<ConversionMode> {
    match (ctable, block) {
        (Some(ct), None) => Some(ConversionMode::ConvertOnly(ct)),
        (Some(ct), Some(Block::Block(cbs))) => {
            if is_ascii {
                Some(ConversionMode::ConvertThenBlock(ct, cbs, is_sync))
            } else {
                Some(ConversionMode::BlockThenConvert(ct, cbs, is_sync))
            }
        }
        (Some(ct), Some(Block::Unblock(cbs))) => {
            if is_ascii {
                Some(ConversionMode::ConvertThenUnblock(ct, cbs))
            } else {
                Some(ConversionMode::UnblockThenConvert(ct, cbs))
            }
        }
        (None, Some(Block::Block(cbs))) => Some(ConversionMode::BlockOnly(cbs, is_sync)),
        (None, Some(Block::Unblock(cbs))) => Some(ConversionMode::UnblockOnly(cbs)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {

    use crate::Num;
    use crate::parseargs::{Parser, parse_bytes_with_opt_multiplier};
    use std::matches;
    const BIG: &str = "9999999999999999999999999999999999999999999999999999999999999";

    #[test]
    fn test_parse_bytes_with_opt_multiplier_invalid() {
        assert!(parse_bytes_with_opt_multiplier("123asdf").is_err());
    }

    #[test]
    fn test_parse_bytes_with_opt_multiplier_without_x() {
        assert_eq!(parse_bytes_with_opt_multiplier("123").unwrap(), 123);
        assert_eq!(parse_bytes_with_opt_multiplier("123c").unwrap(), 123); // 123 * 1
        assert_eq!(parse_bytes_with_opt_multiplier("123w").unwrap(), 123 * 2);
        assert_eq!(parse_bytes_with_opt_multiplier("123b").unwrap(), 123 * 512);
        assert_eq!(parse_bytes_with_opt_multiplier("123k").unwrap(), 123 * 1024);
        assert_eq!(parse_bytes_with_opt_multiplier(BIG).unwrap(), u64::MAX);
    }

    #[test]
    fn test_parse_bytes_with_opt_multiplier_with_x() {
        assert_eq!(parse_bytes_with_opt_multiplier("123x3").unwrap(), 123 * 3);
        assert_eq!(parse_bytes_with_opt_multiplier("1x2x3").unwrap(), 6); // 1 * 2 * 3
        assert_eq!(
            parse_bytes_with_opt_multiplier("1wx2cx3w").unwrap(),
            2 * 2 * (3 * 2) // (1 * 2) * (2 * 1) * (3 * 2)
        );
    }
    #[test]
    fn test_parse_n() {
        for arg in ["1x8x4", "1c", "123b", "123w"] {
            assert!(matches!(Parser::parse_n(arg), Ok(Num::Blocks(_))));
        }
        for arg in ["1Bx8x4", "2Bx8", "2Bx8B", "2x8B"] {
            assert!(matches!(Parser::parse_n(arg), Ok(Num::Bytes(_))));
        }
    }
}
