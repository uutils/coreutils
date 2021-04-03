use super::*;

#[derive(Debug)]
enum ParseError
{
    MultiplierString(String),
    MultiplierStringWouldOverflow(String),
}

impl std::fmt::Display for ParseError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dd-args: Parse Error")
    }
}

impl Error for ParseError {}

fn parse_multiplier<'a>(s: &'a str) -> Result<usize, Box<dyn Error>>
{
    let s = s.trim();

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
            Err(Box::new(ParseError::MultiplierString(String::from(s)))),
    }
}

fn parse_bytes_with_opt_multiplier(s: String) -> Result<usize, Box<dyn Error>>
{
    if let Some(idx) = s.find(' ')
    {
        let base: usize = s[0..idx].parse()?;
        let mult = parse_multiplier(&s[idx..])?;

        if let Some(bytes) = base.checked_mul(mult)
        {
            Ok(bytes)
        }
        else
        {
            Err(Box::new(ParseError::MultiplierStringWouldOverflow(s)))
        }
    }
    else
    {
        let bytes: usize = s.parse()?;

        Ok(bytes)
    }
}

pub fn parse_ibs(matches: &getopts::Matches) -> Result<usize, Box<dyn Error>>
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

pub fn parse_progress_level(matches: &getopts::Matches) -> Result<bool, Box<dyn Error>>
{
    // TODO: Implement this stub proc
    Ok(false)
}

pub fn parse_obs(matches: &getopts::Matches) -> Result<usize, Box<dyn Error>>
{
    if let Some(str_with_prefixes) = matches.opt_str("bs")
    {
        // TODO: Parse a string containing the number with potential k, kB, kiB, ... multiplier
        // possibly implemented elsewhere, but probably not in exactly the dd style
        panic!()
    }
    else if let Some(str_with_prefixes) = matches.opt_str("obs")
    {
        // TODO: Parse a string containing the number with potential k, kB, kiB, ... multiplier
        // possibly implemented elsewhere, but probably not in exactly the dd style
        panic!()
    }
    else
    {
        Ok(512)
    }
}

pub fn parse_conv_table(matches: &getopts::Matches) -> Result<Option<ConversionTable>, Box<dyn Error>>
{
    // TODO: Complete this stub fn
    Ok(None)
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test_byte_parser (
        ( $test_name:ident, $bs_str:expr, $bs:expr ) =>
        {
            #[test]
            fn $test_name()
            {
                let bs_str = String::from($bs_str);
                assert_eq!($bs, parse_bytes_with_opt_multiplier(bs_str).unwrap())
            }
        }
    );

    #[test]
    fn test_input_parser()
    {
        panic!()
    }

    #[test]
    fn test_output_parser()
    {
        panic!()
    }

    #[test]
    fn test_conv_options_parser()
    {
        panic!()
    }

    test_byte_parser!(
        test_bytes_n,
        "765",
        765
    );
    test_byte_parser!(
        test_bytes_c,
        "13 c",
        13
    );

    test_byte_parser!(
        test_bytes_w,
        "1 w",
        2
    );

    test_byte_parser!(
        test_bytes_b,
        "1 b",
        512
    );

    test_byte_parser!(
        test_bytes_k,
        "1 kB",
        1000
    );
    test_byte_parser!(
        test_bytes_K,
        "1 K",
        1024
    );
    test_byte_parser!(
        test_bytes_Ki,
        "1 KiB",
        1024
    );

    test_byte_parser!(
        test_bytes_MB,
        "1 MB",
        1000*1000
    );
    test_byte_parser!(
        test_bytes_M,
        "1 M",
        1024*1024
    );
    test_byte_parser!(
        test_bytes_Mi,
        "1 MiB",
        1024*1024
    );

    test_byte_parser!(
        test_bytes_GB,
        "1 GB",
        1000*1000*1000
    );
    test_byte_parser!(
        test_bytes_G,
        "1 G",
        1024*1024*1024
    );
    test_byte_parser!(
        test_bytes_Gi,
        "1 GiB",
        1024*1024*1024
    );

    test_byte_parser!(
        test_bytes_TB,
        "1 TB",
        1000*1000*1000*1000
    );
    test_byte_parser!(
        test_bytes_T,
        "1 T",
        1024*1024*1024*1024
    );
    test_byte_parser!(
        test_bytes_Ti,
        "1 TiB",
        1024*1024*1024*1024
    );

    test_byte_parser!(
        test_bytes_PB,
        "1 PB",
        1000*1000*1000*1000*1000
    );
    test_byte_parser!(
        test_bytes_P,
        "1 P",
        1024*1024*1024*1024*1024
    );
    test_byte_parser!(
        test_bytes_Pi,
        "1 PiB",
        1024*1024*1024*1024*1024
    );

    test_byte_parser!(
        test_bytes_EB,
        "1 EB",
        1000*1000*1000*1000*1000*1000
    );
    test_byte_parser!(
        test_bytes_E,
        "1 E",
        1024*1024*1024*1024*1024*1024
    );
    test_byte_parser!(
        test_bytes_Ei,
        "1 EiB",
        1024*1024*1024*1024*1024*1024
    );

    #[test]
    #[should_panic]
    fn test_KB_multiplier_error()
    {
        let bs_str = String::from("2000 KB");

        parse_bytes_with_opt_multiplier(bs_str).unwrap();
    }

}
