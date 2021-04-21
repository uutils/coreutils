#[cfg(test)]
mod test;

use crate::conversion_tables::*;
use crate::{
    ConvFlagInput, ConvFlagOutput,
    StatusLevel,
};

use std::error::Error;


/// Parser Errors describe errors with input
#[derive(Debug)]
pub enum ParseError
{
    MultipleFmtTable,
    MultipleUCaseLCase,
    MultipleBlockUnblock,
    ConvFlagNoMatch(String),
    NoMatchingMultiplier(String),
    MultiplierStringContainsNoValue(String),
    MultiplierStringWouldOverflow(String),
}

impl std::fmt::Display for ParseError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dd-args: Parse Error")
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
            Err(ParseError::NoMatchingMultiplier(String::from(s))),
    }
}

fn parse_bytes_with_opt_multiplier(s: String) -> Result<usize, ParseError>
{
    if let Some(idx) = s.find(char::is_alphabetic)
    {
        let base: usize = match s[0..idx].parse()
        {
            Ok(val) => val,
            Err(_) => return Err(ParseError::MultiplierStringContainsNoValue(s)),
        };
        let mult = parse_multiplier(&s[idx..])?;

        if let Some(bytes) = base.checked_mul(mult)
        {
            Ok(bytes)
        }
        else
        {
            Err(ParseError::MultiplierStringWouldOverflow(s))
        }
    }
    else
    {
        let bytes: usize = match s.parse()
        {
            Ok(val) => val,
            Err(_) => return Err(ParseError::MultiplierStringContainsNoValue(s)),
        };
        Ok(bytes)
    }
}

pub fn parse_ibs(matches: &getopts::Matches) -> Result<usize, ParseError>
{
    if let Some(mixed_str) = matches.opt_str("bs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else if let Some(mixed_str) = matches.opt_str("ibs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else
    {
        Ok(512)
    }
}

pub fn parse_status_level(matches: &getopts::Matches) -> Result<StatusLevel, ParseError>
{
    // TODO: Impl
    unimplemented!()
}

pub fn parse_obs(matches: &getopts::Matches) -> Result<usize, ParseError>
{
    if let Some(mixed_str) = matches.opt_str("bs")
    {
        parse_bytes_with_opt_multiplier(mixed_str)
    }
    else if let Some(mixed_str) = matches.opt_str("obs")
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
    match (fmt, case)
    {
        // Both specified
        (Some(fmt), Some(case)) =>
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
            },
        // Only one of {ascii, ebcdic, ibm} specified
        (Some(fmt), None) =>
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
            },
        // Only one of {ucase, lcase} specified
        (None, Some(ConvFlag::UCase)) =>
            Some(&ASCII_LCASE_TO_UCASE),
        (None, Some(ConvFlag::LCase)) =>
            Some(&ASCII_UCASE_TO_LCASE),
        (_, _) =>
            None,
   }
}

fn parse_conv_opts(matches: &getopts::Matches) -> Result<Vec<ConvFlag>, ParseError>
{
    let mut flags = Vec::new();

    if let Some(comma_str) = matches.opt_str("conv")
    {
        println!("Parsing conv: {}", comma_str);
        for s in comma_str.split(",")
        {
            let flag = s.parse()?;
            println!("found flag: {:?}", &flag);
            flags.push(flag);
        }
    }

    Ok(flags)
}

/// Parse Conversion Options (Input Variety)
/// Construct and validate a ConvFlagInput
pub fn parse_conv_flag_input(matches: &getopts::Matches) -> Result<ConvFlagInput, ParseError>
{
    let flags = parse_conv_opts(matches)?;

    let mut fmt = None;
    let mut case = None;
    let mut block = false;
    let mut unblock = false;
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
                if !unblock
                {
                    block = true;
                }
                else
                {
                    return Err(ParseError::MultipleBlockUnblock);
                },
            ConvFlag::Unblock =>
                if !block
                {
                    unblock = true;
                }
                else
                {
                    return Err(ParseError::MultipleBlockUnblock);
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

    Ok(ConvFlagInput {
        ctable,
        block,
        unblock,
        swab,
        sync,
        noerror,
    })
}

/// Parse Conversion Options (Output Variety)
/// Construct and validate a ConvFlagOutput
pub fn parse_conv_flag_output(matches: &getopts::Matches) -> Result<ConvFlagOutput, ParseError>
{
    let flags = parse_conv_opts(matches)?;

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
                excl = true,
            ConvFlag::NoCreat =>
                nocreat = true,
            ConvFlag::NoTrunc =>
                notrunc = true,
            ConvFlag::FDataSync =>
                fdatasync = true,
            ConvFlag::FSync =>
                fsync = true,
            _ => {},
       }
    }

    Ok(ConvFlagOutput {
        sparse,
        excl,
        nocreat,
        notrunc,
        fdatasync,
        fsync,
    })
}
