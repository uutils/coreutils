use super::*;

use crate::{
    build_app,
    SYNTAX, SUMMARY, LONG_HELP,
    ConvFlagInput, ConvFlagOutput,
    StatusLevel,
};

// ----- ConvFlagInput/Output -----

#[test]
fn build_cfi()
{
    let cfi_expd = ConvFlagInput {
        ctable: Some(&ASCII_TO_IBM),
        block: false,
        unblock: false,
        swab: false,
        sync: false,
        noerror: false,
    };

    let args = vec![
        String::from("dd"),
        String::from("--conv=ibm"),
    ];

    let matches = build_app!().parse(args);

    let cfi_parsed = parse_conv_flag_input(&matches).unwrap();

    unimplemented!()
    // assert_eq!(cfi_expd, cfi_parsed);
}

#[test]
#[should_panic]
fn cfi_ctable_error()
{
    let args = vec![
        String::from("dd"),
        String::from("--conv=ascii,ebcdic,ibm"),
    ];

    let matches = build_app!().parse(args);

    let cfi_parsed = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn cfi_case_error()
{
    let args = vec![
        String::from("dd"),
        String::from("--conv=ucase,lcase"),
    ];

    let matches = build_app!().parse(args);

    let cfi_parsed = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn cfi_block_error()
{
    let args = vec![
        String::from("dd"),
        String::from("--conv=block,unblock"),
    ];

    let matches = build_app!().parse(args);

    let cfi_parsed = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn cfi_creat_error()
{
    let args = vec![
        String::from("dd"),
        String::from("--conv=excl,nocreat"),
    ];

    let matches = build_app!().parse(args);

    let cfi_parsed = parse_conv_flag_output(&matches).unwrap();
}

#[test]
fn parse_cfi_token_ibm()
{
    let exp = vec![
        ConvFlag::FmtAtoI,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--conv=ibm"),
    ];
    let matches = build_app!().parse(args);

    let act = parse_conv_opts(&matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp
    {
        assert!(exp.contains(&cf));
    }
}

#[test]
fn parse_cfi_tokens_elu()
{
    let exp = vec![
        ConvFlag::FmtEtoA,
        ConvFlag::LCase,
        ConvFlag::Unblock,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--conv=ebcdic,lcase,unblock"),
    ];
    let matches = build_app!().parse(args);
    let act = parse_conv_opts(&matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp
    {
        assert!(exp.contains(&cf));
    }
}

#[test]
fn parse_cfi_tokens_remaining()
{
    let exp = vec![
        ConvFlag::FmtAtoE,
        ConvFlag::UCase,
        ConvFlag::Block,
        ConvFlag::Sparse,
        ConvFlag::Swab,
        ConvFlag::Sync,
        ConvFlag::NoError,
        ConvFlag::Excl,
        ConvFlag::NoCreat,
        ConvFlag::NoTrunc,
        ConvFlag::NoError,
        ConvFlag::FDataSync,
        ConvFlag::FSync,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--conv=ascii,ucase,block,sparse,swab,sync,noerror,excl,nocreat,notrunc,noerror,fdatasync,fsync"),
    ];
    let matches = build_app!().parse(args);
    let act = parse_conv_opts(&matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp
    {
        assert!(exp.contains(&cf));
    }
}

// ----- Multiplier Strings etc. -----
macro_rules! test_byte_parser (
    ( $test_name:ident, $bs_str:expr, $bs:expr ) =>
    {
        #[allow(non_snake_case)]
        #[test]
        fn $test_name()
        {
            let bs_str = String::from($bs_str);
            assert_eq!($bs, parse_bytes_with_opt_multiplier(bs_str).unwrap())
        }
    }
);

test_byte_parser!(
    test_bytes_n,
    "765",
    765
);
test_byte_parser!(
    test_bytes_c,
    "13c",
    13
);

test_byte_parser!(
    test_bytes_w,
    "1w",
    2
);

test_byte_parser!(
    test_bytes_b,
    "1b",
    512
);

test_byte_parser!(
    test_bytes_k,
    "1kB",
    1000
);
test_byte_parser!(
    test_bytes_K,
    "1K",
    1024
);
test_byte_parser!(
    test_bytes_Ki,
    "1KiB",
    1024
);

test_byte_parser!(
    test_bytes_MB,
    "2MB",
    2*1000*1000
);
test_byte_parser!(
    test_bytes_M,
    "2M",
    2*1024*1024
);
test_byte_parser!(
    test_bytes_Mi,
    "2MiB",
    2*1024*1024
);

test_byte_parser!(
    test_bytes_GB,
    "3GB",
    3*1000*1000*1000
);
test_byte_parser!(
    test_bytes_G,
    "3G",
    3*1024*1024*1024
);
test_byte_parser!(
    test_bytes_Gi,
    "3GiB",
    3*1024*1024*1024
);

test_byte_parser!(
    test_bytes_TB,
    "4TB",
    4*1000*1000*1000*1000
);
test_byte_parser!(
    test_bytes_T,
    "4T",
    4*1024*1024*1024*1024
);
test_byte_parser!(
    test_bytes_Ti,
    "4TiB",
    4*1024*1024*1024*1024
);

test_byte_parser!(
    test_bytes_PB,
    "5PB",
    5*1000*1000*1000*1000*1000
);
test_byte_parser!(
    test_bytes_P,
    "5P",
    5*1024*1024*1024*1024*1024
);
test_byte_parser!(
    test_bytes_Pi,
    "5PiB",
    5*1024*1024*1024*1024*1024
);

test_byte_parser!(
    test_bytes_EB,
    "6EB",
    6*1000*1000*1000*1000*1000*1000
);
test_byte_parser!(
    test_bytes_E,
    "6E",
    6*1024*1024*1024*1024*1024*1024
);
test_byte_parser!(
    test_bytes_Ei,
    "6EiB",
    6*1024*1024*1024*1024*1024*1024
);

#[test]
#[should_panic]
#[allow(non_snake_case)]
fn test_KB_multiplier_error()
{
    // KB is not valid (kB, K, and KiB are)
    let bs_str = String::from("2000KB");

    parse_bytes_with_opt_multiplier(bs_str).unwrap();
}

#[test]
#[should_panic]
fn test_overflow_panic()
{
    let bs_str = format!("{}KiB", usize::MAX);

    parse_bytes_with_opt_multiplier(bs_str).unwrap();
}

#[test]
#[should_panic]
fn test_neg_panic()
{
    let bs_str = format!("{}KiB", -1);

    parse_bytes_with_opt_multiplier(bs_str).unwrap();
}
