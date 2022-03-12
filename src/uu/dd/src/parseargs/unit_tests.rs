// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, iseek, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, oseek, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

use super::*;

use crate::StatusLevel;

#[cfg(not(target_os = "linux"))]
#[test]
fn unimplemented_flags_should_error_non_linux() {
    let mut succeeded = Vec::new();

    // The following flags are only implemented in linux
    for &flag in &[
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
        let matches = uu_app().try_get_matches_from(args).unwrap();

        if parse_iflags(&matches).is_ok() {
            succeeded.push(format!("iflag={}", flag));
        }
        if parse_oflags(&matches).is_ok() {
            succeeded.push(format!("oflag={}", flag));
        }
    }

    assert!(
        succeeded.is_empty(),
        "The following flags did not panic as expected: {:?}",
        succeeded
    );
}

#[test]
fn unimplemented_flags_should_error() {
    let mut succeeded = Vec::new();

    // The following flags are not implemented
    for &flag in &["cio", "nocache", "nolinks", "text", "binary"] {
        let args = vec![
            String::from("dd"),
            format!("--iflag={}", flag),
            format!("--oflag={}", flag),
        ];
        let matches = uu_app().try_get_matches_from(args).unwrap();

        if parse_iflags(&matches).is_ok() {
            succeeded.push(format!("iflag={}", flag));
        }
        if parse_oflags(&matches).is_ok() {
            succeeded.push(format!("oflag={}", flag));
        }
    }

    assert!(
        succeeded.is_empty(),
        "The following flags did not panic as expected: {:?}",
        succeeded
    );
}

#[test]
fn test_status_level_absent() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--of=bar.file"),
    ];

    let matches = uu_app().try_get_matches_from(args).unwrap();
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

    let matches = uu_app().try_get_matches_from(args).unwrap();
    let st = parse_status_level(&matches).unwrap().unwrap();

    assert_eq!(st, StatusLevel::None);
}

#[test]
fn test_all_top_level_args_no_leading_dashes() {
    let args = vec![
        String::from("dd"),
        String::from("if=foo.file"),
        String::from("of=bar.file"),
        String::from("ibs=10"),
        String::from("obs=10"),
        String::from("cbs=1"),
        String::from("bs=100"),
        String::from("count=2"),
        String::from("skip=2"),
        String::from("seek=2"),
        String::from("iseek=2"),
        String::from("oseek=2"),
        String::from("status=progress"),
        String::from("conv=ascii,ucase"),
        String::from("iflag=count_bytes,skip_bytes"),
        String::from("oflag=append,seek_bytes"),
    ];
    let args = args
        .into_iter()
        .fold(Vec::new(), append_dashes_if_not_present);

    let matches = uu_app().try_get_matches_from(args).unwrap();

    assert_eq!(100, parse_ibs(&matches).unwrap());
    assert_eq!(100, parse_obs(&matches).unwrap());
    assert_eq!(1, parse_cbs(&matches).unwrap().unwrap());
    assert_eq!(
        CountType::Bytes(2),
        parse_count(
            &IFlags {
                count_bytes: true,
                ..IFlags::default()
            },
            &matches
        )
        .unwrap()
        .unwrap()
    );
    assert_eq!(
        200,
        parse_skip_amt(&100, &IFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        200,
        parse_seek_amt(&100, &OFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        200,
        parse_iseek_amt(&100, &IFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        200,
        parse_oseek_amt(&100, &OFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        StatusLevel::Progress,
        parse_status_level(&matches).unwrap().unwrap()
    );
    assert_eq!(
        IConvFlags {
            ctable: Some(&EBCDIC_TO_ASCII_LCASE_TO_UCASE),
            unblock: Some(1), // because ascii implies unblock
            ..IConvFlags::default()
        },
        parse_conv_flag_input(&matches).unwrap()
    );
    assert_eq!(
        OConvFlags::default(),
        parse_conv_flag_output(&matches).unwrap()
    );
    assert_eq!(
        IFlags {
            count_bytes: true,
            skip_bytes: true,
            ..IFlags::default()
        },
        parse_iflags(&matches).unwrap()
    );
    assert_eq!(
        OFlags {
            append: true,
            seek_bytes: true,
            ..OFlags::default()
        },
        parse_oflags(&matches).unwrap()
    );
}

#[test]
fn test_all_top_level_args_with_leading_dashes() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--of=bar.file"),
        String::from("--ibs=10"),
        String::from("--obs=10"),
        String::from("--cbs=1"),
        String::from("--bs=100"),
        String::from("--count=2"),
        String::from("--skip=2"),
        String::from("--seek=2"),
        String::from("--iseek=2"),
        String::from("--oseek=2"),
        String::from("--status=progress"),
        String::from("--conv=ascii,ucase"),
        String::from("--iflag=count_bytes,skip_bytes"),
        String::from("--oflag=append,seek_bytes"),
    ];
    let args = args
        .into_iter()
        .fold(Vec::new(), append_dashes_if_not_present);

    let matches = uu_app().try_get_matches_from(args).unwrap();

    assert_eq!(100, parse_ibs(&matches).unwrap());
    assert_eq!(100, parse_obs(&matches).unwrap());
    assert_eq!(1, parse_cbs(&matches).unwrap().unwrap());
    assert_eq!(
        CountType::Bytes(2),
        parse_count(
            &IFlags {
                count_bytes: true,
                ..IFlags::default()
            },
            &matches
        )
        .unwrap()
        .unwrap()
    );
    assert_eq!(
        200,
        parse_skip_amt(&100, &IFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        200,
        parse_seek_amt(&100, &OFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        200,
        parse_iseek_amt(&100, &IFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        200,
        parse_oseek_amt(&100, &OFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        StatusLevel::Progress,
        parse_status_level(&matches).unwrap().unwrap()
    );
    assert_eq!(
        IConvFlags {
            ctable: Some(&EBCDIC_TO_ASCII_LCASE_TO_UCASE),
            unblock: Some(1), // because ascii implies unblock
            ..IConvFlags::default()
        },
        parse_conv_flag_input(&matches).unwrap()
    );
    assert_eq!(
        OConvFlags::default(),
        parse_conv_flag_output(&matches).unwrap()
    );
    assert_eq!(
        IFlags {
            count_bytes: true,
            skip_bytes: true,
            ..IFlags::default()
        },
        parse_iflags(&matches).unwrap()
    );
    assert_eq!(
        OFlags {
            append: true,
            seek_bytes: true,
            ..OFlags::default()
        },
        parse_oflags(&matches).unwrap()
    );
}

#[test]
fn test_status_level_progress() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--of=bar.file"),
        String::from("--status=progress"),
    ];

    let matches = uu_app().try_get_matches_from(args).unwrap();
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

    let matches = uu_app().try_get_matches_from(args).unwrap();
    let st = parse_status_level(&matches).unwrap().unwrap();

    assert_eq!(st, StatusLevel::Noxfer);
}

#[test]
fn test_multiple_flags_options() {
    let args = vec![
        String::from("dd"),
        String::from("--iflag=fullblock,directory"),
        String::from("--iflag=skip_bytes"),
        String::from("--oflag=direct"),
        String::from("--oflag=dsync"),
        String::from("--conv=ascii,ucase"),
        String::from("--conv=unblock"),
    ];
    let matches = uu_app().try_get_matches_from(args).unwrap();

    // iflag
    let iflags = parse_flag_list::<Flag>(options::IFLAG, &matches).unwrap();
    assert_eq!(
        vec![Flag::FullBlock, Flag::Directory, Flag::SkipBytes],
        iflags
    );

    // oflag
    let oflags = parse_flag_list::<Flag>(options::OFLAG, &matches).unwrap();
    assert_eq!(vec![Flag::Direct, Flag::Dsync], oflags);

    // conv
    let conv = parse_flag_list::<ConvFlag>(options::CONV, &matches).unwrap();
    assert_eq!(
        vec![ConvFlag::FmtEtoA, ConvFlag::UCase, ConvFlag::Unblock],
        conv
    );
}

#[test]
fn test_override_multiple_options() {
    let args = vec![
        String::from("dd"),
        String::from("--if=foo.file"),
        String::from("--if=correct.file"),
        String::from("--of=bar.file"),
        String::from("--of=correct.file"),
        String::from("--ibs=256"),
        String::from("--ibs=1024"),
        String::from("--obs=256"),
        String::from("--obs=1024"),
        String::from("--cbs=1"),
        String::from("--cbs=2"),
        String::from("--skip=0"),
        String::from("--skip=2"),
        String::from("--seek=0"),
        String::from("--seek=2"),
        String::from("--iseek=0"),
        String::from("--iseek=2"),
        String::from("--oseek=0"),
        String::from("--oseek=2"),
        String::from("--status=none"),
        String::from("--status=noxfer"),
        String::from("--count=512"),
        String::from("--count=1024"),
    ];

    let matches = uu_app().try_get_matches_from(args).unwrap();

    // if
    assert_eq!("correct.file", matches.value_of(options::INFILE).unwrap());

    // of
    assert_eq!("correct.file", matches.value_of(options::OUTFILE).unwrap());

    // ibs
    assert_eq!(1024, parse_ibs(&matches).unwrap());

    // obs
    assert_eq!(1024, parse_obs(&matches).unwrap());

    // cbs
    assert_eq!(2, parse_cbs(&matches).unwrap().unwrap());

    // status
    assert_eq!(
        StatusLevel::Noxfer,
        parse_status_level(&matches).unwrap().unwrap()
    );

    // skip
    assert_eq!(
        200,
        parse_skip_amt(&100, &IFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );

    // seek
    assert_eq!(
        200,
        parse_seek_amt(&100, &OFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );

    // iseek
    assert_eq!(
        200,
        parse_iseek_amt(&100, &IFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );

    // oseek
    assert_eq!(
        200,
        parse_oseek_amt(&100, &OFlags::default(), &matches)
            .unwrap()
            .unwrap()
    );

    // count
    assert_eq!(
        CountType::Bytes(1024),
        parse_count(
            &IFlags {
                count_bytes: true,
                ..IFlags::default()
            },
            &matches
        )
        .unwrap()
        .unwrap()
    );
}

// ----- IConvFlags/Output -----

#[test]
#[should_panic]
fn icf_ctable_error() {
    let args = vec![String::from("dd"), String::from("--conv=ascii,ebcdic,ibm")];

    let matches = uu_app().try_get_matches_from(args).unwrap();

    let _ = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn icf_case_error() {
    let args = vec![String::from("dd"), String::from("--conv=ucase,lcase")];

    let matches = uu_app().try_get_matches_from(args).unwrap();

    let _ = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn icf_block_error() {
    let args = vec![String::from("dd"), String::from("--conv=block,unblock")];

    let matches = uu_app().try_get_matches_from(args).unwrap();

    let _ = parse_conv_flag_input(&matches).unwrap();
}

#[test]
#[should_panic]
fn icf_creat_error() {
    let args = vec![String::from("dd"), String::from("--conv=excl,nocreat")];

    let matches = uu_app().try_get_matches_from(args).unwrap();

    let _ = parse_conv_flag_output(&matches).unwrap();
}

#[test]
fn parse_icf_token_ibm() {
    let exp = vec![ConvFlag::FmtAtoI];

    let args = vec![String::from("dd"), String::from("--conv=ibm")];
    let matches = uu_app().try_get_matches_from(args).unwrap();

    let act = parse_flag_list::<ConvFlag>("conv", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
    }
}

#[test]
fn parse_icf_tokens_elu() {
    let exp = vec![ConvFlag::FmtEtoA, ConvFlag::LCase, ConvFlag::Unblock];

    let args = vec![
        String::from("dd"),
        String::from("--conv=ebcdic,lcase,unblock"),
    ];
    let matches = uu_app().try_get_matches_from(args).unwrap();
    let act = parse_flag_list::<ConvFlag>("conv", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
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
    let matches = uu_app().try_get_matches_from(args).unwrap();

    let act = parse_flag_list::<ConvFlag>("conv", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
    }
}

#[test]
fn parse_iflag_tokens() {
    let exp = vec![
        Flag::FullBlock,
        Flag::CountBytes,
        Flag::SkipBytes,
        Flag::Append,
        Flag::SeekBytes,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--iflag=fullblock,count_bytes,skip_bytes,append,seek_bytes"),
    ];
    let matches = uu_app().try_get_matches_from(args).unwrap();

    let act = parse_flag_list::<Flag>("iflag", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
    }
}

#[test]
fn parse_oflag_tokens() {
    let exp = vec![
        Flag::FullBlock,
        Flag::CountBytes,
        Flag::SkipBytes,
        Flag::Append,
        Flag::SeekBytes,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--oflag=fullblock,count_bytes,skip_bytes,append,seek_bytes"),
    ];
    let matches = uu_app().try_get_matches_from(args).unwrap();

    let act = parse_flag_list::<Flag>("oflag", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn parse_iflag_tokens_linux() {
    let exp = vec![
        Flag::Direct,
        Flag::Directory,
        Flag::Dsync,
        Flag::Sync,
        Flag::NonBlock,
        Flag::NoATime,
        Flag::NoCtty,
        Flag::NoFollow,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--iflag=direct,directory,dsync,sync,nonblock,noatime,noctty,nofollow"),
    ];
    let matches = uu_app().try_get_matches_from(args).unwrap();

    let act = parse_flag_list::<Flag>("iflag", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn parse_oflag_tokens_linux() {
    let exp = vec![
        Flag::Direct,
        Flag::Directory,
        Flag::Dsync,
        Flag::Sync,
        Flag::NonBlock,
        Flag::NoATime,
        Flag::NoCtty,
        Flag::NoFollow,
    ];

    let args = vec![
        String::from("dd"),
        String::from("--oflag=direct,directory,dsync,sync,nonblock,noatime,noctty,nofollow"),
    ];
    let matches = uu_app().try_get_matches_from(args).unwrap();

    let act = parse_flag_list::<Flag>("oflag", &matches).unwrap();

    assert_eq!(exp.len(), act.len());
    for cf in &exp {
        assert!(exp.contains(cf));
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

#[cfg(target_pointer_width = "64")]
#[cfg(test)]
mod test_64bit_arch {
    use super::*;

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
    let bs_str = format!("{}", -1);

    parse_bytes_with_opt_multiplier(&bs_str).unwrap();
}
