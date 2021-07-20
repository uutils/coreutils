/* cspell:disable */

use super::*;

use crate::StatusLevel;

#[cfg(not(target_os = "linux"))]
#[test]
fn unimplemented_flags_should_error_non_unix() {
    let mut unfailed = Vec::new();

    // The following flags are only implemented in linux
    for flag in vec![
        "direct",
        "directory",
        "dsync",
        "sync",
        "nonblock",
        "noatime",
        "noctty",
        "nofollow",
    ] {
        let args = vec![
            String::from("dd"),
            format!("--iflag={}", flag),
            format!("--oflag={}", flag),
        ];
        let matches = uu_app().get_matches_from_safe(args).unwrap();

        match parse_iflags(&matches) {
            Ok(_) => unfailed.push(format!("iflag={}", flag)),
            Err(_) => { /* expected behaviour :-) */ }
        }
        match parse_oflags(&matches) {
            Ok(_) => unfailed.push(format!("oflag={}", flag)),
            Err(_) => { /* expected behaviour :-) */ }
        }
    }

    if !unfailed.is_empty() {
        panic!(
            "The following flags did not panic as expected: {:?}",
            unfailed
        );
    }
}

#[test]
fn unimplemented_flags_should_error() {
    let mut unfailed = Vec::new();

    // The following flags are not implemented
    for flag in vec!["cio", "nocache", "nolinks", "text", "binary"] {
        let args = vec![
            String::from("dd"),
            format!("--iflag={}", flag),
            format!("--oflag={}", flag),
        ];
        let matches = uu_app().get_matches_from_safe(args).unwrap();

        match parse_iflags(&matches) {
            Ok(_) => unfailed.push(format!("iflag={}", flag)),
            Err(_) => { /* expected behaviour :-) */ }
        }
        match parse_oflags(&matches) {
            Ok(_) => unfailed.push(format!("oflag={}", flag)),
            Err(_) => { /* expected behaviour :-) */ }
        }
    }

    if !unfailed.is_empty() {
        panic!(
            "The following flags did not panic as expected: {:?}",
            unfailed
        );
    }
}

#[test]
fn test_status_level_absent() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--of=bar.file"),
    ];

    let matches = uu_app().get_matches_from_safe(args).unwrap();
    let st = parse_status_level(&matches).unwrap();

    assert_eq!(st, None);
}

#[test]
fn test_status_level_none() {
    let args = vec![
        String::from("dd"),
        String::from("--status=none"),
        String::from("--if=foo.file"),
        String::from("--of=bar.file"),
    ];

    let matches = uu_app().get_matches_from_safe(args).unwrap();
    let st = parse_status_level(&matches).unwrap().unwrap();

    assert_eq!(st, StatusLevel::None);
}

#[test]
fn test_status_level_progress() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--of=bar.file"),
        String::from("--status=progress"),
    ];

    let matches = uu_app().get_matches_from_safe(args).unwrap();
    let st = parse_status_level(&matches).unwrap().unwrap();

    assert_eq!(st, StatusLevel::Progress);
}

#[test]
fn test_status_level_noxfer() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--status=noxfer"),
        String::from("--of=bar.file"),
    ];

    let matches = uu_app().get_matches_from_safe(args).unwrap();
    let st = parse_status_level(&matches).unwrap().unwrap();

    assert_eq!(st, StatusLevel::Noxfer);
}

// ----- IConvFlags/Output -----

#[test]
#[should_panic]
fn icf_ctable_error() {
    let args = vec![String::from("dd"), String::from("--conv=ascii,ebcdic,ibm")];

    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let _ = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn icf_case_error() {
    let args = vec![String::from("dd"), String::from("--conv=ucase,lcase")];

    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let _ = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn icf_block_error() {
    let args = vec![String::from("dd"), String::from("--conv=block,unblock")];

    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let _ = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn icf_creat_error() {
    let args = vec![String::from("dd"), String::from("--conv=excl,nocreat")];

    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let _ = parse_conv_flag_output(&matches).unwrap();
}

#[test]
fn parse_icf_token_ibm() {
    let exp = vec![ConvFlag::FmtAtoI];

    let args = vec![String::from("dd"), String::from("--conv=ibm")];
    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let act = parse_flag_list::<ConvFlag>("conv", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(&cf));
    }
}

#[test]
fn parse_icf_tokens_elu() {
    let exp = vec![ConvFlag::FmtEtoA, ConvFlag::LCase, ConvFlag::Unblock];

    let args = vec![
        String::from("dd"),
        String::from("--conv=ebcdic,lcase,unblock"),
    ];
    let matches = uu_app().get_matches_from_safe(args).unwrap();
    let act = parse_flag_list::<ConvFlag>("conv", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(&cf));
    }
}

#[test]
fn parse_icf_tokens_remaining() {
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
    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let act = parse_flag_list::<ConvFlag>("conv", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(&cf));
    }
}

#[test]
fn parse_iflag_tokens() {
    let exp = vec![
        Flag::FullBlock,
        Flag::CountBytes,
        Flag::SkipBytes,
        // Flag::Cio,
        Flag::Direct,
        Flag::Directory,
        Flag::Dsync,
        Flag::Sync,
        // Flag::NoCache,
        Flag::NonBlock,
        Flag::NoATime,
        Flag::NoCtty,
        Flag::NoFollow,
        // Flag::NoLinks,
        // Flag::Binary,
        // Flag::Text,
        Flag::Append,
        Flag::SeekBytes,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--iflag=fullblock,count_bytes,skip_bytes,direct,directory,dsync,sync,nonblock,noatime,noctty,nofollow,append,seek_bytes"),
        // String::from("--iflag=fullblock,count_bytes,skip_bytes,cio,direct,directory,dsync,sync,nocache,nonblock,noatime,noctty,nofollow,nolinks,binary,text,append,seek_bytes"),
    ];
    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let act = parse_flag_list::<Flag>("iflag", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(&cf));
    }
}

#[test]
fn parse_oflag_tokens() {
    let exp = vec![
        Flag::FullBlock,
        Flag::CountBytes,
        Flag::SkipBytes,
        // Flag::Cio,
        Flag::Direct,
        Flag::Directory,
        Flag::Dsync,
        Flag::Sync,
        // Flag::NoCache,
        Flag::NonBlock,
        Flag::NoATime,
        Flag::NoCtty,
        Flag::NoFollow,
        // Flag::NoLinks,
        // Flag::Binary,
        // Flag::Text,
        Flag::Append,
        Flag::SeekBytes,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--oflag=fullblock,count_bytes,skip_bytes,direct,directory,dsync,sync,nonblock,noatime,noctty,nofollow,append,seek_bytes"),
        // String::from("--oflag=fullblock,count_bytes,skip_bytes,cio,direct,directory,dsync,sync,nocache,nonblock,noatime,noctty,nofollow,nolinks,binary,text,append,seek_bytes"),
    ];
    let matches = uu_app().get_matches_from_safe(args).unwrap();

    let act = parse_flag_list::<Flag>("oflag", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
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
            // let bs_str = String::from($bs_str);
            assert_eq!($bs, parse_bytes_with_opt_multiplier($bs_str).unwrap())
        }
    }
);

test_byte_parser!(test_bytes_n, "765", 765);
test_byte_parser!(test_bytes_c, "13c", 13);

test_byte_parser!(test_bytes_w, "1w", 2);

test_byte_parser!(test_bytes_b, "1b", 512);

test_byte_parser!(test_bytes_k, "1kB", 1000);
test_byte_parser!(test_bytes_K, "1K", 1024);
test_byte_parser!(test_bytes_Ki, "1KiB", 1024);

test_byte_parser!(test_bytes_MB, "2MB", 2 * 1000 * 1000);
test_byte_parser!(test_bytes_M, "2M", 2 * 1024 * 1024);
test_byte_parser!(test_bytes_Mi, "2MiB", 2 * 1024 * 1024);

test_byte_parser!(test_bytes_GB, "3GB", 3 * 1000 * 1000 * 1000);
test_byte_parser!(test_bytes_G, "3G", 3 * 1024 * 1024 * 1024);
test_byte_parser!(test_bytes_Gi, "3GiB", 3 * 1024 * 1024 * 1024);

test_byte_parser!(test_bytes_TB, "4TB", 4 * 1000 * 1000 * 1000 * 1000);
test_byte_parser!(test_bytes_T, "4T", 4 * 1024 * 1024 * 1024 * 1024);
test_byte_parser!(test_bytes_Ti, "4TiB", 4 * 1024 * 1024 * 1024 * 1024);

test_byte_parser!(test_bytes_PB, "5PB", 5 * 1000 * 1000 * 1000 * 1000 * 1000);
test_byte_parser!(test_bytes_P, "5P", 5 * 1024 * 1024 * 1024 * 1024 * 1024);
test_byte_parser!(test_bytes_Pi, "5PiB", 5 * 1024 * 1024 * 1024 * 1024 * 1024);

test_byte_parser!(
    test_bytes_EB,
    "6EB",
    6 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000
);
test_byte_parser!(
    test_bytes_E,
    "6E",
    6 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024
);
test_byte_parser!(
    test_bytes_Ei,
    "6EiB",
    6 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024
);

#[test]
#[should_panic]
#[allow(non_snake_case)]
fn test_KB_multiplier_error() {
    // KB is not valid (kB, K, and KiB are)
    let bs_str = "2000KB";

    parse_bytes_with_opt_multiplier(bs_str).unwrap();
}

#[test]
#[should_panic]
fn test_overflow_panic() {
    let bs_str = format!("{}KiB", usize::MAX);

    parse_bytes_with_opt_multiplier(&bs_str).unwrap();
}

#[test]
#[should_panic]
fn test_neg_panic() {
    let bs_str = format!("{}KiB", -1);

    parse_bytes_with_opt_multiplier(&bs_str).unwrap();
}
