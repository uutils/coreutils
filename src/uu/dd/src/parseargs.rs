// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ctty, ctable, iconvflags, oconvflags

#[cfg(test)]
mod unit_tests;

use super::*;
use std::error::Error;
use uucore::error::UError;

pub type Matches = clap::ArgMatches<'static>;

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

/// Some flags specified as part of a conv=CONV[,CONV]... block
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
            "none" => Ok(StatusLevel::None),
            "noxfer" => Ok(StatusLevel::Noxfer),
            "progress" => Ok(StatusLevel::Progress),
            _ => Err(ParseError::StatusLevelNotRecognized(s.to_string())),
        }
    }
}

/// Parse bytes using str::parse, then map error if needed.
fn parse_bytes_only(s: &str) -> Result<usize, ParseError> {
    s.parse()
        .map_err(|_| ParseError::MultiplierStringParseFailure(s.to_string()))
}

/// Parse byte and multiplier like 512, 5KiB, or 1G.
/// Uses uucore::parse_size, and adds the 'w' and 'c' suffixes which are mentioned
/// in dd's info page.
fn parse_bytes_with_opt_multiplier(s: &str) -> Result<usize, ParseError> {
    if let Some(idx) = s.rfind('c') {
        parse_bytes_only(&s[..idx])
    } else if let Some(idx) = s.rfind('w') {
        let partial = parse_bytes_only(&s[..idx])?;

        partial
            .checked_mul(2)
            .ok_or_else(|| ParseError::MultiplierStringOverflow(s.to_string()))
    } else {
        uucore::parse_size::parse_size(s).map_err(|e| match e {
            uucore::parse_size::ParseSizeError::ParseFailure(s) => {
                ParseError::MultiplierStringParseFailure(s)
            }
            uucore::parse_size::ParseSizeError::SizeTooBig(s) => {
                ParseError::MultiplierStringOverflow(s)
            }
        })
    }
}

pub fn parse_ibs(matches: &Matches) -> Result<usize, ParseError> {
    if let Some(mixed_str) = matches.value_of(options::BS) {
        parse_bytes_with_opt_multiplier(mixed_str)
    } else if let Some(mixed_str) = matches.value_of(options::IBS) {
        parse_bytes_with_opt_multiplier(mixed_str)
    } else {
        Ok(512)
    }
}

fn parse_cbs(matches: &Matches) -> Result<Option<usize>, ParseError> {
    if let Some(s) = matches.value_of(options::CBS) {
        let bytes = parse_bytes_with_opt_multiplier(s)?;
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}

pub fn parse_status_level(matches: &Matches) -> Result<Option<StatusLevel>, ParseError> {
    match matches.value_of(options::STATUS) {
        Some(s) => {
            let st = s.parse()?;
            Ok(Some(st))
        }
        None => Ok(None),
    }
}

pub fn parse_obs(matches: &Matches) -> Result<usize, ParseError> {
    if let Some(mixed_str) = matches.value_of("bs") {
        parse_bytes_with_opt_multiplier(mixed_str)
    } else if let Some(mixed_str) = matches.value_of("obs") {
        parse_bytes_with_opt_multiplier(mixed_str)
    } else {
        Ok(512)
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
    let mut flags = Vec::new();

    if let Some(comma_str) = matches.value_of(tag) {
        for s in comma_str.split(',') {
            let flag = s.parse()?;
            flags.push(flag);
        }
    }

    Ok(flags)
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
                }
            }
            ConvFlag::FmtAtoE => {
                if fmt.is_some() {
                    return Err(ParseError::MultipleFmtTable);
                } else {
                    fmt = Some(flag);
                }
            }
            ConvFlag::FmtAtoI => {
                if fmt.is_some() {
                    return Err(ParseError::MultipleFmtTable);
                } else {
                    fmt = Some(flag);
                }
            }
            ConvFlag::UCase => {
                if case.is_some() {
                    return Err(ParseError::MultipleUCaseLCase);
                } else {
                    case = Some(flag)
                }
            }
            ConvFlag::LCase => {
                if case.is_some() {
                    return Err(ParseError::MultipleUCaseLCase);
                } else {
                    case = Some(flag)
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
) -> Result<Option<usize>, ParseError> {
    if let Some(amt) = matches.value_of(options::SKIP) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if iflags.skip_bytes {
            Ok(Some(n))
        } else {
            Ok(Some(ibs * n))
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
) -> Result<Option<usize>, ParseError> {
    if let Some(amt) = matches.value_of(options::SEEK) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if oflags.seek_bytes {
            Ok(Some(n))
        } else {
            Ok(Some(obs * n))
        }
    } else {
        Ok(None)
    }
}

/// Parse the value of count=N and the type of N implied by iflags
pub fn parse_count(iflags: &IFlags, matches: &Matches) -> Result<Option<CountType>, ParseError> {
    if let Some(amt) = matches.value_of(options::COUNT) {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if iflags.count_bytes {
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
