#[cfg(test)]
mod unit_tests;

use crate::conversion_tables::*;
use crate::{
    CountType,
    IConvFlags, OConvFlags,
    StatusLevel,
};
use crate::{
    IFlags, OFlags,
};

use std::error::Error;

pub type Matches = clap::ArgMatches<'static>;

/// Parser Errors describe errors with parser input
#[derive(Debug)]
pub enum ParseError
{
    MultipleFmtTable,
    MultipleUCaseLCase,
    MultipleBlockUnblock,
    MultipleExclNoCreat,
    FlagNoMatch(String),
    ConvFlagNoMatch(String),
    NoMatchingMultiplier(String),
    ByteStringContainsNoValue(String),
    MultiplierStringWouldOverflow(String),
    BlockUnblockWithoutCBS,
    StatusLevelNotRecognized(String),
}

impl std::fmt::Display for ParseError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self
        {
            Self::MultipleFmtTable =>
            {
                write!(f, "Only one of conv=ascii conv=ebcdic or conv=ibm may be specified")
            },
            Self::MultipleUCaseLCase =>
            {
                write!(f, "Only one of conv=lcase or conv=ucase may be specified")
            },
            Self::MultipleBlockUnblock =>
            {
                write!(f, "Only one of conv=block or conv=unblock may be specified")
            },
            Self::MultipleExclNoCreat =>
            {
                write!(f, "Only one ov conv=excl or conv=nocreat may be specified")
            },
            Self::FlagNoMatch(arg) =>
            {
                write!(f, "Unrecognized iflag=FLAG or oflag=FLAG -> {}", arg)
            },
            Self::ConvFlagNoMatch(arg) =>
            {
                write!(f, "Unrecognized conv=CONV -> {}", arg)
            },
            Self::NoMatchingMultiplier(arg) =>
            {
                write!(f, "Unrecognized byte multiplier -> {}", arg)
            },
            Self::ByteStringContainsNoValue(arg) =>
            {
                write!(f, "Unrecognized byte value -> {}", arg)
            },
            Self::MultiplierStringWouldOverflow(arg) =>
            {
                write!(f, "Multiplier string would overflow on current system -> {}", arg)
            },
            Self::BlockUnblockWithoutCBS =>
            {
                write!(f, "conv=block or conv=unblock specified without cbs=N")
            },
            Self::StatusLevelNotRecognized(arg) =>
            {
                write!(f, "status=LEVEL not recognized -> {}", arg)
            },
        }
    }
}

impl Error for ParseError {}

/// Some flags specified as part of a conv=CONV[,CONV]... block
/// relate to the input file, others to the output file.
#[derive(Debug, PartialEq)]
enum ConvFlag
{
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

impl std::str::FromStr for ConvFlag
{
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        match s
        {
            // Input
            "ascii" =>
                Ok(Self::FmtEtoA),
            "ebcdic" =>
                Ok(Self::FmtAtoE),
            "ibm" =>
                Ok(Self::FmtAtoI),
            "lcase" =>
                Ok(Self::LCase),
            "ucase" =>
                Ok(Self::UCase),
            "block" =>
                Ok(Self::Block),
            "unblock" =>
                Ok(Self::Unblock),
            "swab" =>
                Ok(Self::Swab),
            "sync" =>
                Ok(Self::Sync),
            "noerror" =>
                Ok(Self::NoError),
            // Output
            "sparse" =>
                Ok(Self::Sparse),
            "excl" =>
                Ok(Self::Excl),
            "nocreat" =>
                Ok(Self::NoCreat),
            "notrunc" =>
                Ok(Self::NoTrunc),
            "fdatasync" =>
                Ok(Self::FDataSync),
            "fsync" =>
                Ok(Self::FSync),
            _ =>
                Err(ParseError::ConvFlagNoMatch(String::from(s)))
            }
    }
}

enum Flag
{
    // Input only
    FullBlock,
    CountBytes,
    SkipBytes,
    // Either
    Cio,
    Direct,
    Directory,
    Dsync,
    Sync,
    NoCache,
    NonBlock,
    NoATime,
    NoCtty,
    NoFollow,
    NoLinks,
    Binary,
    Text,
    // Output only
    Append,
    SeekBytes,
}

impl std::str::FromStr for Flag
{
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        match s
        {
            // Input only
            "fullblock" =>
                Ok(Self::FullBlock),
            "count_bytes" =>
                Ok(Self::CountBytes),
            "skip_bytes" =>
                Ok(Self::SkipBytes),
            // Either
            "cio" =>
                Ok(Self::Cio),
            "direct" =>
                Ok(Self::Direct),
            "directory" =>
                Ok(Self::Directory),
            "dsync" =>
                Ok(Self::Dsync),
            "sync" =>
                Ok(Self::Sync),
            "nocache" =>
                Ok(Self::NoCache),
            "nonblock" =>
                Ok(Self::NonBlock),
            "noatime" =>
                Ok(Self::NoATime),
            "noctty" =>
                Ok(Self::NoCtty),
            "nofollow" =>
                Ok(Self::NoFollow),
            "nolinks" =>
                Ok(Self::NoLinks),
            "binary" =>
                Ok(Self::Binary),
            "text" =>
                Ok(Self::Text),
            // Output only
            "append" =>
                Ok(Self::Append),
            "seek_bytes" =>
                Ok(Self::SeekBytes),
            _ =>
                Err(ParseError::FlagNoMatch(String::from(s))),
        }
    }
}

impl std::str::FromStr for StatusLevel
{
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        match s
        {
            "none" =>
                Ok(StatusLevel::None),
            "noxfer" =>
                Ok(StatusLevel::Noxfer),
            "progress" =>
                Ok(StatusLevel::Progress),
            _ =>
                Err(ParseError::StatusLevelNotRecognized(s.to_string())),
        }
    }
}

fn parse_multiplier<'a>(s: &'a str) -> Result<usize, ParseError>
{
    match s
    {
        "c" =>
            Ok(1),
        "w" =>
            Ok(2),
        "b" =>
            Ok(512),
        "kB" =>
            Ok(1000),
        "K" | "KiB" =>
            Ok(1024),
        "MB" =>
            Ok(1000*1000),
        "M" | "MiB" =>
            Ok(1024*1024),
        "GB" =>
            Ok(1000*1000*1000),
        "G" | "GiB" =>
            Ok(1024*1024*1024),
        "TB" =>
            Ok(1000*1000*1000*1000),
        "T" | "TiB" =>
            Ok(1024*1024*1024*1024),
        "PB" =>
            Ok(1000*1000*1000*1000*1000),
        "P" | "PiB" =>
            Ok(1024*1024*1024*1024*1024),
        "EB" =>
            Ok(1000*1000*1000*1000*1000*1000),
        "E" | "EiB" =>
            Ok(1024*1024*1024*1024*1024*1024),
// The following would overflow on my x64 system
//      "ZB" =>
//          Ok(1000*1000*1000*1000*1000*1000*1000),
//      "Z" | "ZiB" =>
//          Ok(1024*1024*1024*1024*1024*1024*1024),
//      "YB" =>
//          Ok(1000*1000*1000*1000*1000*1000*1000*1000),
//      "Y" | "YiB" =>
//          Ok(1024*1024*1024*1024*1024*1024*1024*1024),
        _ =>
            Err(ParseError::NoMatchingMultiplier(s.to_string())),
    }
}

fn parse_bytes_only(s: &str) -> Result<usize, ParseError>
{
    match s.parse()
    {
        Ok(bytes) =>
            Ok(bytes),
        Err(_) =>
            Err(ParseError::ByteStringContainsNoValue(s.to_string())),
    }
}

fn parse_bytes_with_opt_multiplier(s: &str) -> Result<usize, ParseError>
{
    match s.find(char::is_alphabetic)
    {
        Some(idx) =>
        {
            let base = parse_bytes_only(&s[..idx])?;
            let mult = parse_multiplier(&s[idx..])?;

            if let Some(bytes) = base.checked_mul(mult)
            {
                Ok(bytes)
            }
            else
            {
                Err(ParseError::MultiplierStringWouldOverflow(s.to_string()))
            }
        }
        _ =>
            parse_bytes_only(&s),
    }
}

pub fn parse_ibs(matches: &Matches) -> Result<usize, ParseError>
{
    if let Some(mixed_str) = matches.value_of("bs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else if let Some(mixed_str) = matches.value_of("ibs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else
    {
        Ok(512)
    }
}

fn parse_cbs(matches: &Matches) -> Result<Option<usize>, ParseError>
{
    if let Some(s) = matches.value_of("cbs")
    {
        let bytes = parse_bytes_with_opt_multiplier(s)?;
        Ok(Some(bytes))
    }
    else
    {
        Ok(None)
    }
}

pub fn parse_status_level(matches: &Matches) -> Result<Option<StatusLevel>, ParseError>
{
    match matches.value_of("status")
    {
       Some(s) =>
        {
            let st = s.parse()?;
            Ok(Some(st))
        },
        None =>
            Ok(None),
    }
}

pub fn parse_obs(matches: &Matches) -> Result<usize, ParseError>
{
    if let Some(mixed_str) = matches.value_of("bs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else if let Some(mixed_str) = matches.value_of("obs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else
    {
        Ok(512)
    }
}

fn parse_ctable(fmt: Option<ConvFlag>, case: Option<ConvFlag>) -> Option<&'static ConversionTable>
{
    fn parse_conv_and_case_table(fmt: &ConvFlag, case: &ConvFlag) -> Option<&'static ConversionTable>
    {
        match (fmt, case)
        {
            (ConvFlag::FmtAtoE, ConvFlag::UCase) =>
                Some(&ASCII_TO_EBCDIC_LCASE_TO_UCASE),
            (ConvFlag::FmtAtoE, ConvFlag::LCase) =>
                Some(&ASCII_TO_EBCDIC_UCASE_TO_LCASE),
            (ConvFlag::FmtEtoA, ConvFlag::UCase) =>
                Some(&EBCDIC_TO_ASCII_LCASE_TO_UCASE),
            (ConvFlag::FmtEtoA, ConvFlag::LCase) =>
                Some(&EBCDIC_TO_ASCII_UCASE_TO_LCASE),
            (ConvFlag::FmtAtoI, ConvFlag::UCase) =>
                Some(&ASCII_TO_IBM_UCASE_TO_LCASE),
            (ConvFlag::FmtAtoI, ConvFlag::LCase) =>
                Some(&ASCII_TO_IBM_LCASE_TO_UCASE),
            (_, _) =>
                None,
        }
    }
    fn parse_conv_table_only(fmt: &ConvFlag) -> Option<&'static ConversionTable>
    {
        match fmt
        {
            ConvFlag::FmtAtoE =>
                Some(&ASCII_TO_EBCDIC),
            ConvFlag::FmtEtoA =>
                Some(&EBCDIC_TO_ASCII),
            ConvFlag::FmtAtoI =>
                Some(&ASCII_TO_IBM),
            _ =>
                None,
        }
    }
    // ------------------------------------------------------------------------
    match (fmt, case)
    {
        // Both [ascii | ebcdic | ibm] and [lcase | ucase] specified
        (Some(fmt), Some(case)) =>
            parse_conv_and_case_table(&fmt, &case),
        // Only [ascii | ebcdic | ibm] specified
        (Some(fmt), None) =>
            parse_conv_table_only(&fmt),
        // Only [lcase | ucase] specified
        (None, Some(ConvFlag::UCase)) =>
            Some(&ASCII_LCASE_TO_UCASE),
        (None, Some(ConvFlag::LCase)) =>
            Some(&ASCII_UCASE_TO_LCASE),
        // ST else...
        (_, _) =>
            None,
   }
}

fn parse_flag_list<T: std::str::FromStr<Err = ParseError>>(tag: &str, matches: &Matches) -> Result<Vec<T>, ParseError>
{
    let mut flags = Vec::new();

    if let Some(comma_str) = matches.value_of(tag)
    {
        for s in comma_str.split(",")
        {
            let flag = s.parse()?;
            flags.push(flag);
        }
    }

    Ok(flags)
}

/// Parse Conversion Options (Input Variety)
/// Construct and validate a IConvFlags
pub fn parse_conv_flag_input(matches: &Matches) -> Result<IConvFlags, ParseError>
{
    let flags = parse_flag_list("conv", matches)?;
    let cbs = parse_cbs(matches)?;

    let mut fmt = None;
    let mut case = None;
    let mut block = None;
    let mut unblock = None;
    let mut swab = false;
    let mut sync = false;
    let mut noerror = false;

    for flag in flags
    {
        match flag
        {
            ConvFlag::FmtEtoA =>
                if let Some(_) = fmt
                {
                    return Err(ParseError::MultipleFmtTable);
                }
                else
                {
                    fmt = Some(flag);
                },
            ConvFlag::FmtAtoE =>
                if let Some(_) = fmt
                {
                    return Err(ParseError::MultipleFmtTable);
                }
                else
                {
                    fmt = Some(flag);
                },
            ConvFlag::FmtAtoI =>
                if let Some(_) = fmt
                {
                    return Err(ParseError::MultipleFmtTable);
                }
                else
                {
                    fmt = Some(flag);
                },
            ConvFlag::UCase =>
                if let Some(_) = case
                {
                    return Err(ParseError::MultipleUCaseLCase);
                }
                else
                {
                    case = Some(flag)
                },
            ConvFlag::LCase =>
                if let Some(_) = case
                {
                    return Err(ParseError::MultipleUCaseLCase);
                }
                else
                {
                    case = Some(flag)
                },
            ConvFlag::Block =>
                match (cbs, unblock)
                {
                    (Some(cbs), None) =>
                        block = Some(cbs),
                    (None, _) =>
                        return Err(ParseError::BlockUnblockWithoutCBS),
                    (_, Some(_)) =>
                        return Err(ParseError::MultipleBlockUnblock),
                },
            ConvFlag::Unblock =>
                match (cbs, block)
                {
                    (Some(cbs), None) =>
                        unblock = Some(cbs),
                    (None, _) =>
                        return Err(ParseError::BlockUnblockWithoutCBS),
                    (_, Some(_)) =>
                        return Err(ParseError::MultipleBlockUnblock),
                },
            ConvFlag::Swab =>
                swab = true,
            ConvFlag::Sync =>
                sync = true,
            ConvFlag::NoError =>
                noerror = true,
            _ => {},
        }
    }

    let ctable = parse_ctable(fmt, case);
    let sync = if sync && (block.is_some() || unblock.is_some())
    {
        Some(' ' as u8)
    }
    else if sync
    {
        Some(0u8)
    }
    else
    {
        None
    };

    Ok(IConvFlags {
        ctable,
        block,
        unblock,
        swab,
        sync,
        noerror,
    })
}

/// Parse Conversion Options (Output Variety)
/// Construct and validate a OConvFlags
pub fn parse_conv_flag_output(matches: &Matches) -> Result<OConvFlags, ParseError>
{
    let flags = parse_flag_list("conv", matches)?;

    let mut sparse = false;
    let mut excl = false;
    let mut nocreat = false;
    let mut notrunc = false;
    let mut fdatasync = false;
    let mut fsync = false;

    for flag in flags
    {
        match flag
        {
            ConvFlag::Sparse =>
                sparse = true,
            ConvFlag::Excl =>
                if !nocreat
                {
                    excl = true;
                }
                else
                {
                    return Err(ParseError::MultipleExclNoCreat);
                },
            ConvFlag::NoCreat =>
                if !excl
                {
                    nocreat = true;
                }
                else
                {
                    return Err(ParseError::MultipleExclNoCreat);
                },
            ConvFlag::NoTrunc =>
                notrunc = true,
            ConvFlag::FDataSync =>
                fdatasync = true,
            ConvFlag::FSync =>
                fsync = true,
            _ => {},
       }
    }

    Ok(OConvFlags {
        sparse,
        excl,
        nocreat,
        notrunc,
        fdatasync,
        fsync,
    })
}

/// Parse IFlags struct from CL-input
pub fn parse_iflags(matches: &Matches) -> Result<IFlags, ParseError>
{
    let mut cio = false;
    let mut direct = false;
    let mut directory = false;
    let mut dsync = false;
    let mut sync = false;
    let mut nocache = false;
    let mut nonblock = false;
    let mut noatime = false;
    let mut noctty = false;
    let mut nofollow = false;
    let mut nolinks = false;
    let mut binary = false;
    let mut text = false;
    let mut fullblock = false;
    let mut count_bytes = false;
    let mut skip_bytes = false;

    let flags = parse_flag_list("iflag", matches)?;

    for flag in flags
    {
        match flag
        {
            Flag::Cio =>
                cio = true,
            Flag::Direct =>
                direct = true,
            Flag::Directory =>
                directory = true,
            Flag::Dsync =>
                dsync = true,
            Flag::Sync =>
                sync = true,
            Flag::NoCache =>
                nocache = true,
            Flag::NonBlock =>
                nonblock = true,
            Flag::NoATime =>
                noatime = true,
            Flag::NoCtty =>
                noctty = true,
            Flag::NoFollow =>
                nofollow = true,
            Flag::NoLinks =>
                nolinks = true,
            Flag::Binary =>
                binary = true,
            Flag::Text =>
                text = true,
            Flag::FullBlock =>
                fullblock = true,
            Flag::CountBytes =>
                count_bytes = true,
            Flag::SkipBytes =>
                skip_bytes = true,
            _ => {},
        }
    }

    Ok(IFlags{
        cio,
        direct,
        directory,
        dsync,
        sync,
        nocache,
        nonblock,
        noatime,
        noctty,
        nofollow,
        nolinks,
        binary,
        text,
        fullblock,
        count_bytes,
        skip_bytes,
    })
}

/// Parse OFlags struct from CL-input
pub fn parse_oflags(matches: &Matches) -> Result<OFlags, ParseError>
{
    let mut append = false;
    let mut cio = false;
    let mut direct = false;
    let mut directory = false;
    let mut dsync = false;
    let mut sync = false;
    let mut nocache = false;
    let mut nonblock = false;
    let mut noatime = false;
    let mut noctty = false;
    let mut nofollow = false;
    let mut nolinks = false;
    let mut binary = false;
    let mut text = false;
    let mut seek_bytes = false;

    let flags = parse_flag_list("oflag", matches)?;

    for flag in flags
    {
        match flag
        {
            Flag::Append =>
                append = true,
            Flag::Cio =>
                cio = true,
            Flag::Direct =>
                direct = true,
            Flag::Directory =>
                directory = true,
            Flag::Dsync =>
                dsync = true,
            Flag::Sync =>
                sync = true,
            Flag::NoCache =>
                nocache = true,
            Flag::NonBlock =>
                nonblock = true,
            Flag::NoATime =>
                noatime = true,
            Flag::NoCtty =>
                noctty = true,
            Flag::NoFollow =>
                nofollow = true,
            Flag::NoLinks =>
                nolinks = true,
            Flag::Binary =>
                binary = true,
            Flag::Text =>
                text = true,
            Flag::SeekBytes =>
                seek_bytes = true,
            _ => {},
        }
    }

    Ok(OFlags {
        append,
        cio,
        direct,
        directory,
        dsync,
        sync,
        nocache,
        nonblock,
        noatime,
        noctty,
        nofollow,
        nolinks,
        binary,
        text,
        seek_bytes,
    })
}

/// Parse the amount of the input file to skip.
pub fn parse_skip_amt(ibs: &usize, iflags: &IFlags, matches: &Matches) -> Result<Option<usize>, ParseError>
{
    if let Some(amt) = matches.value_of("skip")
    {
        if iflags.skip_bytes
        {
            let n = parse_bytes_with_opt_multiplier(amt)?;
            Ok(Some(n))
        }
        else
        {
            let n = parse_bytes_with_opt_multiplier(amt)?;
            Ok(Some(ibs*n))
        }
    }
    else
    {
        Ok(None)
    }
}

/// Parse the amount of the output file to seek.
pub fn parse_seek_amt(obs: &usize, oflags: &OFlags, matches: &Matches) -> Result<Option<usize>, ParseError>
{
    if let Some(amt) = matches.value_of("seek")
    {
        if oflags.seek_bytes
        {
            let n = parse_bytes_with_opt_multiplier(amt)?;
            Ok(Some(n))
        }
        else
        {
            let n = parse_bytes_with_opt_multiplier(amt)?;
            Ok(Some(obs*n))
        }
    }
    else
    {
        Ok(None)
    }
}

/// Parse the value of count=N and the type of N implied by iflags
pub fn parse_count(iflags: &IFlags, matches:  &Matches) -> Result<Option<CountType>, ParseError>
{
    if let Some(amt) = matches.value_of("count")
    {
        let n = parse_bytes_with_opt_multiplier(amt)?;
        if iflags.count_bytes
        {
            Ok(Some(CountType::Bytes(n)))
        }
        else
        {
            Ok(Some(CountType::Reads(n)))
        }
    }
    else
    {
        Ok(None)
    }
}

/// Parse whether the args indicate the input is not ascii
pub fn parse_input_non_ascii(matches: &Matches) -> Result<bool, ParseError>
{
    if let Some(conv_opts) = matches.value_of("conv")
    {
        Ok(conv_opts.contains("ascii"))
    }
    else
    {
        Ok(false)
    }
}
