// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ctty, ctable, iseek, oseek, iconvflags, oconvflags parseargs

#[cfg(test)]
mod unit_tests;

use super::*;
use std::error::Error;
use uucore::error::UError;
use uucore::parse_size::ParseSizeError;
use uucore::show_warning;

pub type Matches = ArgMatches;

/// Parser Errors describe errors with parser input
#[derive(Debug, PartialEq)]
pub enum ParseError {
    MultipleFmtTable,
    MultipleUCaseLCase,
    MultipleBlockUnblock,
    MultipleExclNoCreate,
    FlagNoMatch(String),
    ConvFlagNoMatch(String),
    MultiplierStringParseFailure(String),
    MultiplierStringOverflow(String),
    BlockUnblockWithoutCBS,
    StatusLevelNotRecognized(String),
    Unimplemented(String),
    BsOutOfRange,
    IbsOutOfRange,
    ObsOutOfRange,
    CbsOutOfRange,
}

impl ParseError {
    /// Replace the argument, if any, with the given string, consuming self.
    fn with_arg(self, s: String) -> Self {
        match self {
            Self::MultipleFmtTable => Self::MultipleFmtTable,
            Self::MultipleUCaseLCase => Self::MultipleUCaseLCase,
            Self::MultipleBlockUnblock => Self::MultipleBlockUnblock,
            Self::MultipleExclNoCreate => Self::MultipleExclNoCreate,
            Self::FlagNoMatch(_) => Self::FlagNoMatch(s),
            Self::ConvFlagNoMatch(_) => Self::ConvFlagNoMatch(s),
            Self::MultiplierStringParseFailure(_) => Self::MultiplierStringParseFailure(s),
            Self::MultiplierStringOverflow(_) => Self::MultiplierStringOverflow(s),
            Self::BlockUnblockWithoutCBS => Self::BlockUnblockWithoutCBS,
            Self::StatusLevelNotRecognized(_) => Self::StatusLevelNotRecognized(s),
            Self::Unimplemented(_) => Self::Unimplemented(s),
            Self::BsOutOfRange => Self::BsOutOfRange,
            Self::IbsOutOfRange => Self::IbsOutOfRange,
            Self::ObsOutOfRange => Self::ObsOutOfRange,
            Self::CbsOutOfRange => Self::CbsOutOfRange,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultipleFmtTable => {
                write!(
                    f,
                    "Only one of conv=ascii conv=ebcdic or conv=ibm may be specified"
                )
            }
            Self::MultipleUCaseLCase => {
                write!(f, "Only one of conv=lcase or conv=ucase may be specified")
            }
            Self::MultipleBlockUnblock => {
                write!(f, "Only one of conv=block or conv=unblock may be specified")
            }
            Self::MultipleExclNoCreate => {
                write!(f, "Only one ov conv=excl or conv=nocreat may be specified")
            }
            Self::FlagNoMatch(arg) => {
                write!(f, "Unrecognized iflag=FLAG or oflag=FLAG -> {}", arg)
            }
            Self::ConvFlagNoMatch(arg) => {
                write!(f, "Unrecognized conv=CONV -> {}", arg)
            }
            Self::MultiplierStringParseFailure(arg) => {
                write!(f, "Unrecognized byte multiplier -> {}", arg)
            }
            Self::MultiplierStringOverflow(arg) => {
                write!(
                    f,
                    "Multiplier string would overflow on current system -> {}",
                    arg
                )
            }
            Self::BlockUnblockWithoutCBS => {
                write!(f, "conv=block or conv=unblock specified without cbs=N")
            }
            Self::StatusLevelNotRecognized(arg) => {
                write!(f, "status=LEVEL not recognized -> {}", arg)
            }
            ParseError::BsOutOfRange => {
                write!(f, "bs=N cannot fit into memory")
            }
            ParseError::IbsOutOfRange => {
                write!(f, "ibs=N cannot fit into memory")
            }
            ParseError::ObsOutOfRange => {
                write!(f, "obs=N cannot fit into memory")
            }
            ParseError::CbsOutOfRange => {
                write!(f, "cbs=N cannot fit into memory")
            }
            Self::Unimplemented(arg) => {
                write!(f, "feature not implemented on this system -> {}", arg)
            }
        }
    }
}

impl Error for ParseError {}

impl UError for ParseError {
    fn code(&self) -> i32 {
        1
    }
}

/// Some flags specified as part of a conv=CONV\[,CONV\]... block
/// relate to the input file, others to the output file.
#[derive(Debug, PartialEq)]
enum ConvFlag {
    // Input
    FmtAtoE,
    FmtEtoA,
    FmtAtoI,
    Block,
    Unblock,
    UCase,
    LCase,
    Swab,
    Sync,
    NoError,
    // Output
    Sparse,
    Excl,
    NoCreat,
    NoTrunc,
    FDataSync,
    FSync,
}

impl std::str::FromStr for ConvFlag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Input
            "ascii" => Ok(Self::FmtEtoA),
            "ebcdic" => Ok(Self::FmtAtoE),
            "ibm" => Ok(Self::FmtAtoI),
            "lcase" => Ok(Self::LCase),
            "ucase" => Ok(Self::UCase),
            "block" => Ok(Self::Block),
            "unblock" => Ok(Self::Unblock),
            "swab" => Ok(Self::Swab),
            "sync" => Ok(Self::Sync),
            "noerror" => Ok(Self::NoError),
            // Output
            "sparse" => Ok(Self::Sparse),
            "excl" => Ok(Self::Excl),
            "nocreat" => Ok(Self::NoCreat),
            "notrunc" => Ok(Self::NoTrunc),
            "fdatasync" => Ok(Self::FDataSync),
            "fsync" => Ok(Self::FSync),
            _ => Err(ParseError::ConvFlagNoMatch(String::from(s))),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Flag {
    // Input only
    FullBlock,
    CountBytes,
    SkipBytes,
    // Either
    #[allow(unused)]
    Cio,
    #[allow(unused)]
    Direct,
    #[allow(unused)]
    Directory,
    #[allow(unused)]
    Dsync,
    #[allow(unused)]
    Sync,
    #[allow(unused)]
    NoCache,
    #[allow(unused)]
    NonBlock,
    #[allow(unused)]
    NoATime,
    #[allow(unused)]
    NoCtty,
    #[allow(unused)]
    NoFollow,
    #[allow(unused)]
    NoLinks,
    #[allow(unused)]
    Binary,
    #[allow(unused)]
    Text,
    // Output only
    Append,
    SeekBytes,
}

impl std::str::FromStr for Flag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Input only
            "fullblock" => Ok(Self::FullBlock),
            "count_bytes" => Ok(Self::CountBytes),
            "skip_bytes" => Ok(Self::SkipBytes),
            // Either
            "cio" =>
            // Ok(Self::Cio),
            {
                Err(ParseError::Unimplemented(s.to_string()))
            }
            "direct" =>
            // Ok(Self::Direct),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::Direct)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "directory" =>
            // Ok(Self::Directory),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::Directory)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "dsync" =>
            // Ok(Self::Dsync),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::Dsync)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "sync" =>
            // Ok(Self::Sync),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::Sync)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "nocache" =>
            // Ok(Self::NoCache),
            {
                Err(ParseError::Unimplemented(s.to_string()))
            }
            "nonblock" =>
            // Ok(Self::NonBlock),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::NonBlock)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "noatime" =>
            // Ok(Self::NoATime),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::NoATime)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "noctty" =>
            // Ok(Self::NoCtty),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::NoCtty)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "nofollow" =>
            // Ok(Self::NoFollow),
            {
                if cfg!(target_os = "linux") {
                    Ok(Self::NoFollow)
                } else {
                    Err(ParseError::Unimplemented(s.to_string()))
                }
            }
            "nolinks" =>
            // Ok(Self::NoLinks),
            {
                Err(ParseError::Unimplemented(s.to_string()))
            }
            "binary" =>
            // Ok(Self::Binary),
            {
                Err(ParseError::Unimplemented(s.to_string()))
            }
            "text" =>
            // Ok(Self::Text),
            {
                Err(ParseError::Unimplemented(s.to_string()))
            }
            // Output only
            "append" => Ok(Self::Append),
            "seek_bytes" => Ok(Self::SeekBytes),
            _ => Err(ParseError::FlagNoMatch(String::from(s))),
        }
    }
}

impl std::str::FromStr for StatusLevel {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "noxfer" => Ok(Self::Noxfer),
            "progress" => Ok(Self::Progress),
            _ => Err(ParseError::StatusLevelNotRecognized(s.to_string())),
        }
    }
}

fn show_zero_multiplier_warning() {
    show_warning!(
        "{} is a zero multiplier; use {} if that is intended",
        "0x".quote(),
        "00x".quote()
    );
}

/// Parse bytes using str::parse, then map error if needed.
fn parse_bytes_only(s: &str) -> Result<u64, ParseError> {
    s.parse()
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
/// # Errors
///
/// If a number cannot be parsed or if the multiplication would cause
/// an overflow.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_bytes_no_x("123").unwrap(), 123);
/// assert_eq!(parse_bytes_no_x("2c").unwrap(), 2 * 1);
/// assert_eq!(parse_bytes_no_x("3w").unwrap(), 3 * 2);
/// assert_eq!(parse_bytes_no_x("2b").unwrap(), 2 * 512);
/// assert_eq!(parse_bytes_no_x("2k").unwrap(), 2 * 1024);
/// ```
fn parse_bytes_no_x(s: &str) -> Result<u64, ParseError> {
    let (num, multiplier) = match (s.find('c'), s.rfind('w'), s.rfind('b')) {
        (None, None, None) => match uucore::parse_size::parse_size(s) {
            Ok(n) => (n, 1),
            Err(ParseSizeError::ParseFailure(s)) => {
                return Err(ParseError::MultiplierStringParseFailure(s))
            }
            Err(ParseSizeError::SizeTooBig(s)) => {
                return Err(ParseError::MultiplierStringOverflow(s))
            }
        },
        (Some(i), None, None) => (parse_bytes_only(&s[..i])?, 1),
        (None, Some(i), None) => (parse_bytes_only(&s[..i])?, 2),
        (None, None, Some(i)) => (parse_bytes_only(&s[..i])?, 512),
        _ => return Err(ParseError::MultiplierStringParseFailure(s.to_string())),
    };
    num.checked_mul(multiplier)
        .ok_or_else(|| ParseError::MultiplierStringOverflow(s.to_string()))
}

/// Parse byte and multiplier like 512, 5KiB, or 1G.
/// Uses uucore::parse_size, and adds the 'w' and 'c' suffixes which are mentioned
/// in dd's info page.
fn parse_bytes_with_opt_multiplier(s: &str) -> Result<u64, ParseError> {
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
        parse_bytes_no_x(parts[0]).map_err(|e| e.with_arg(s.to_string()))
    } else {
        let mut total = 1;
        for part in parts {
            if part == "0" {
                show_zero_multiplier_warning();
            }
            let num = parse_bytes_no_x(part).map_err(|e| e.with_arg(s.to_string()))?;
            total *= num;
        }
        Ok(total)
    }
}

pub fn parse_ibs(matches: &Matches) -> Result<usize, ParseError> {
    if let Some(mixed_str) = matches.value_of(options::BS) {
        parse_bytes_with_opt_multiplier(mixed_str)?
            .try_into()
            .map_err(|_| ParseError::BsOutOfRange)
    } else if let Some(mixed_str) = matches.value_of(options::IBS) {
        parse_bytes_with_opt_multiplier(mixed_str)?
            .try_into()
            .map_err(|_| ParseError::IbsOutOfRange)
    } else {
        Ok(512)
    }
}

pub fn parse_obs(matches: &Matches) -> Result<usize, ParseError> {
    if let Some(mixed_str) = matches.value_of("bs") {
        parse_bytes_with_opt_multiplier(mixed_str)?
            .try_into()
            .map_err(|_| ParseError::BsOutOfRange)
    } else if let Some(mixed_str) = matches.value_of("obs") {
        parse_bytes_with_opt_multiplier(mixed_str)?
            .try_into()
            .map_err(|_| ParseError::ObsOutOfRange)
    } else {
        Ok(512)
    }
}

fn parse_cbs(matches: &Matches) -> Result<Option<usize>, ParseError> {
    if let Some(s) = matches.value_of(options::CBS) {
        let bytes = parse_bytes_with_opt_multiplier(s)?
            .try_into()
            .map_err(|_| ParseError::CbsOutOfRange)?;
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}

pub(crate) fn parse_status_level(matches: &Matches) -> Result<Option<StatusLevel>, ParseError> {
    match matches.value_of(options::STATUS) {
        Some(s) => {
            let st = s.parse()?;
            Ok(Some(st))
        }
        None => Ok(None),
    }
}

fn parse_ctable(fmt: Option<ConvFlag>, case: Option<ConvFlag>) -> Option<&'static ConversionTable> {
    fn parse_conv_and_case_table(
        fmt: &ConvFlag,
        case: &ConvFlag,
    ) -> Option<&'static ConversionTable> {
        match (fmt, case) {
            (ConvFlag::FmtAtoE, ConvFlag::UCase) => Some(&ASCII_TO_EBCDIC_LCASE_TO_UCASE),
            (ConvFlag::FmtAtoE, ConvFlag::LCase) => Some(&ASCII_TO_EBCDIC_UCASE_TO_LCASE),
            (ConvFlag::FmtEtoA, ConvFlag::UCase) => Some(&EBCDIC_TO_ASCII_LCASE_TO_UCASE),
            (ConvFlag::FmtEtoA, ConvFlag::LCase) => Some(&EBCDIC_TO_ASCII_UCASE_TO_LCASE),
            (ConvFlag::FmtAtoI, ConvFlag::UCase) => Some(&ASCII_TO_IBM_UCASE_TO_LCASE),
            (ConvFlag::FmtAtoI, ConvFlag::LCase) => Some(&ASCII_TO_IBM_LCASE_TO_UCASE),
            (_, _) => None,
        }
    }
    fn parse_conv_table_only(fmt: &ConvFlag) -> Option<&'static ConversionTable> {
        match fmt {
            ConvFlag::FmtAtoE => Some(&ASCII_TO_EBCDIC),
            ConvFlag::FmtEtoA => Some(&EBCDIC_TO_ASCII),
            ConvFlag::FmtAtoI => Some(&ASCII_TO_IBM),
            _ => None,
        }
    }
    // ------------------------------------------------------------------------
    match (fmt, case) {
        // Both [ascii | ebcdic | ibm] and [lcase | ucase] specified
        (Some(fmt), Some(case)) => parse_conv_and_case_table(&fmt, &case),
        // Only [ascii | ebcdic | ibm] specified
        (Some(fmt), None) => parse_conv_table_only(&fmt),
        // Only [lcase | ucase] specified
        (None, Some(ConvFlag::UCase)) => Some(&ASCII_LCASE_TO_UCASE),
        (None, Some(ConvFlag::LCase)) => Some(&ASCII_UCASE_TO_LCASE),
        // ST else...
        (_, _) => None,
    }
}

fn parse_flag_list<T: std::str::FromStr<Err = ParseError>>(
    tag: &str,
    matches: &Matches,
) -> Result<Vec<T>, ParseError> {
    matches
        .values_of(tag)
        .unwrap_or_default()
        .map(|f| f.parse())
        .collect()
}

/// Parse Conversion Options (Input Variety)
/// Construct and validate a IConvFlags
pub fn parse_conv_flag_input(matches: &Matches) -> Result<IConvFlags, ParseError> {
    let mut iconvflags = IConvFlags::default();
    let mut fmt = None;
    let mut case = None;
    let mut is_sync = false;

    let flags = parse_flag_list(options::CONV, matches)?;
    let cbs = parse_cbs(matches)?;

    for flag in flags {
        match flag {
            ConvFlag::FmtEtoA => {
                if fmt.is_some() {
                    return Err(ParseError::MultipleFmtTable);
                } else {
                    fmt = Some(flag);
                    // From the GNU documentation:
                    //
                    // > ‘ascii’
                    // >
                    // > Convert EBCDIC to ASCII, using the conversion
                    // > table specified by POSIX. This provides a 1:1
                    // > translation for all 256 bytes. This implies
                    // > ‘conv=unblock’; input is converted to ASCII
                    // > before trailing spaces are deleted.
                    //
                    // -- https://www.gnu.org/software/coreutils/manual/html_node/dd-invocation.html
                    if cbs.is_some() {
                        iconvflags.unblock = cbs;
                    }
                }
            }
            ConvFlag::FmtAtoE => {
                if fmt.is_some() {
                    return Err(ParseError::MultipleFmtTable);
                } else {
                    fmt = Some(flag);
                    // From the GNU documentation:
                    //
                    // > ‘ebcdic’
                    // >
                    // > Convert ASCII to EBCDIC. This is the inverse
                    // > of the ‘ascii’ conversion. This implies
                    // > ‘conv=block’; trailing spaces are added before
                    // > being converted to EBCDIC.
                    //
                    // -- https://www.gnu.org/software/coreutils/manual/html_node/dd-invocation.html
                    if cbs.is_some() {
                        iconvflags.block = cbs;
                    }
                }
            }
            ConvFlag::FmtAtoI => {
                if fmt.is_some() {
                    return Err(ParseError::MultipleFmtTable);
                } else {
                    fmt = Some(flag);
                }
            }
            ConvFlag::UCase | ConvFlag::LCase => {
                if case.is_some() {
                    return Err(ParseError::MultipleUCaseLCase);
                } else {
                    case = Some(flag);
                }
            }
            ConvFlag::Block => match (cbs, iconvflags.unblock) {
                (Some(cbs), None) => iconvflags.block = Some(cbs),
                (None, _) => return Err(ParseError::BlockUnblockWithoutCBS),
                (_, Some(_)) => return Err(ParseError::MultipleBlockUnblock),
            },
            ConvFlag::Unblock => match (cbs, iconvflags.block) {
                (Some(cbs), None) => iconvflags.unblock = Some(cbs),
                (None, _) => return Err(ParseError::BlockUnblockWithoutCBS),
                (_, Some(_)) => return Err(ParseError::MultipleBlockUnblock),
            },
            ConvFlag::Swab => iconvflags.swab = true,
            ConvFlag::Sync => is_sync = true,
            ConvFlag::NoError => iconvflags.noerror = true,
            _ => {}
        }
    }

    // The final conversion table depends on both
    // fmt (eg. ASCII -> EBCDIC)
    // case (eg. UCASE -> LCASE)
    // So the final value can't be set until all flags are parsed.
    let ctable = parse_ctable(fmt, case);

    // The final value of sync depends on block/unblock
    // block implies sync with ' '
    // unblock implies sync with 0
    // So the final value can't be set until all flags are parsed.
    let sync = if is_sync && (iconvflags.block.is_some() || iconvflags.unblock.is_some()) {
        Some(b' ')
    } else if is_sync {
        Some(0u8)
    } else {
        None
    };

    Ok(IConvFlags {
        ctable,
        sync,
        ..iconvflags
    })
}

/// Parse Conversion Options (Output Variety)
/// Construct and validate a OConvFlags
pub fn parse_conv_flag_output(matches: &Matches) -> Result<OConvFlags, ParseError> {
    let mut oconvflags = OConvFlags::default();

    let flags = parse_flag_list(options::CONV, matches)?;

    for flag in flags {
        match flag {
            ConvFlag::Sparse => oconvflags.sparse = true,
            ConvFlag::Excl => {
                if !oconvflags.nocreat {
                    oconvflags.excl = true;
                } else {
                    return Err(ParseError::MultipleExclNoCreate);
                }
            }
            ConvFlag::NoCreat => {
                if !oconvflags.excl {
                    oconvflags.nocreat = true;
                } else {
                    return Err(ParseError::MultipleExclNoCreate);
                }
            }
            ConvFlag::NoTrunc => oconvflags.notrunc = true,
            ConvFlag::FDataSync => oconvflags.fdatasync = true,
            ConvFlag::FSync => oconvflags.fsync = true,
            _ => {}
        }
    }

    Ok(oconvflags)
}

/// Parse IFlags struct from CL-input
pub fn parse_iflags(matches: &Matches) -> Result<IFlags, ParseError> {
    let mut iflags = IFlags::default();

    let flags = parse_flag_list(options::IFLAG, matches)?;

    for flag in flags {
        match flag {
            Flag::Cio => iflags.cio = true,
            Flag::Direct => iflags.direct = true,
            Flag::Directory => iflags.directory = true,
            Flag::Dsync => iflags.dsync = true,
            Flag::Sync => iflags.sync = true,
            Flag::NoCache => iflags.nocache = true,
            Flag::NonBlock => iflags.nonblock = true,
            Flag::NoATime => iflags.noatime = true,
            Flag::NoCtty => iflags.noctty = true,
            Flag::NoFollow => iflags.nofollow = true,
            Flag::NoLinks => iflags.nolinks = true,
            Flag::Binary => iflags.binary = true,
            Flag::Text => iflags.text = true,
            Flag::FullBlock => iflags.fullblock = true,
            Flag::CountBytes => iflags.count_bytes = true,
            Flag::SkipBytes => iflags.skip_bytes = true,
            _ => {}
        }
    }

    Ok(iflags)
}

/// Parse OFlags struct from CL-input
pub fn parse_oflags(matches: &Matches) -> Result<OFlags, ParseError> {
    let mut oflags = OFlags::default();

    let flags = parse_flag_list(options::OFLAG, matches)?;

    for flag in flags {
        match flag {
            Flag::Append => oflags.append = true,
            Flag::Cio => oflags.cio = true,
            Flag::Direct => oflags.direct = true,
            Flag::Directory => oflags.directory = true,
            Flag::Dsync => oflags.dsync = true,
            Flag::Sync => oflags.sync = true,
            Flag::NoCache => oflags.nocache = true,
            Flag::NonBlock => oflags.nonblock = true,
            Flag::NoATime => oflags.noatime = true,
            Flag::NoCtty => oflags.noctty = true,
            Flag::NoFollow => oflags.nofollow = true,
            Flag::NoLinks => oflags.nolinks = true,
            Flag::Binary => oflags.binary = true,
            Flag::Text => oflags.text = true,
            Flag::SeekBytes => oflags.seek_bytes = true,
            _ => {}
        }
    }

    Ok(oflags)
}

/// Parse the amount of the input file to skip.
pub fn parse_skip_amt(
    ibs: &usize,
    iflags: &IFlags,
    matches: &Matches,
) -> Result<Option<u64>, ParseError> {
    if let Some(amt) = matches.value_of(options::SKIP) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if iflags.skip_bytes || amt.ends_with('B') {
            Ok(Some(n))
        } else {
            Ok(Some(*ibs as u64 * n))
        }
    } else {
        Ok(None)
    }
}

/// Parse the amount of the output file to seek.
pub fn parse_seek_amt(
    obs: &usize,
    oflags: &OFlags,
    matches: &Matches,
) -> Result<Option<u64>, ParseError> {
    if let Some(amt) = matches.value_of(options::SEEK) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if oflags.seek_bytes || amt.ends_with('B') {
            Ok(Some(n))
        } else {
            Ok(Some(*obs as u64 * n))
        }
    } else {
        Ok(None)
    }
}

/// Parse the amount of the input file to seek.
pub fn parse_iseek_amt(
    ibs: &usize,
    iflags: &IFlags,
    matches: &Matches,
) -> Result<Option<u64>, ParseError> {
    if let Some(amt) = matches.value_of(options::ISEEK) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if iflags.skip_bytes || amt.ends_with('B') {
            Ok(Some(n))
        } else {
            Ok(Some(*ibs as u64 * n))
        }
    } else {
        Ok(None)
    }
}

/// Parse the amount of the input file to seek.
pub fn parse_oseek_amt(
    obs: &usize,
    oflags: &OFlags,
    matches: &Matches,
) -> Result<Option<u64>, ParseError> {
    if let Some(amt) = matches.value_of(options::OSEEK) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if oflags.seek_bytes || amt.ends_with('B') {
            Ok(Some(n))
        } else {
            Ok(Some(*obs as u64 * n))
        }
    } else {
        Ok(None)
    }
}

/// Parse the value of count=N and the type of N implied by iflags
pub fn parse_count(iflags: &IFlags, matches: &Matches) -> Result<Option<CountType>, ParseError> {
    if let Some(amt) = matches.value_of(options::COUNT) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if iflags.count_bytes || amt.ends_with('B') {
            Ok(Some(CountType::Bytes(n)))
        } else {
            Ok(Some(CountType::Reads(n)))
        }
    } else {
        Ok(None)
    }
}

/// Parse whether the args indicate the input is not ascii
pub fn parse_input_non_ascii(matches: &Matches) -> Result<bool, ParseError> {
    if let Some(conv_opts) = matches.value_of(options::CONV) {
        Ok(conv_opts.contains("ascii"))
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {

    use crate::parseargs::parse_bytes_with_opt_multiplier;

    #[test]
    fn test_parse_bytes_with_opt_multiplier() {
        assert_eq!(parse_bytes_with_opt_multiplier("123").unwrap(), 123);
        assert_eq!(parse_bytes_with_opt_multiplier("123c").unwrap(), 123); // 123 * 1
        assert_eq!(parse_bytes_with_opt_multiplier("123w").unwrap(), 123 * 2);
        assert_eq!(parse_bytes_with_opt_multiplier("123b").unwrap(), 123 * 512);
        assert_eq!(parse_bytes_with_opt_multiplier("123x3").unwrap(), 123 * 3);
        assert_eq!(parse_bytes_with_opt_multiplier("123k").unwrap(), 123 * 1024);
        assert_eq!(parse_bytes_with_opt_multiplier("1x2x3").unwrap(), 6); // 1 * 2 * 3

        assert_eq!(
            parse_bytes_with_opt_multiplier("1wx2cx3w").unwrap(),
            2 * 2 * (3 * 2) // (1 * 2) * (2 * 1) * (3 * 2)
        );
        assert!(parse_bytes_with_opt_multiplier("123asdf").is_err());
    }
}
