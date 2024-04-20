// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, iseek, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, oseek, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat, oconv

use super::*;

use crate::conversion_tables::{
    ASCII_TO_EBCDIC_UCASE_TO_LCASE, ASCII_TO_IBM, EBCDIC_TO_ASCII_LCASE_TO_UCASE,
};
use crate::parseargs::Parser;
use crate::StatusLevel;

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[allow(clippy::useless_vec)]
#[test]
fn unimplemented_flags_should_error_non_linux() {
    let mut succeeded = Vec::new();

    // The following flags are only implemented in linux
    for flag in [
        "direct",
        "directory",
        "dsync",
        "sync",
        "nonblock",
        "noatime",
        "noctty",
        "nofollow",
    ] {
        let args = vec![format!("iflag={}", flag)];

        if Parser::new()
            .parse(&args.iter().map(AsRef::as_ref).collect::<Vec<_>>()[..])
            .is_ok()
        {
            succeeded.push(format!("iflag={}", flag));
        }

        let args = vec![format!("oflag={}", flag)];

        if Parser::new()
            .parse(&args.iter().map(AsRef::as_ref).collect::<Vec<_>>()[..])
            .is_ok()
        {
            succeeded.push(format!("iflag={}", flag));
        }
    }

    assert!(
        succeeded.is_empty(),
        "The following flags did not panic as expected: {:?}",
        succeeded
    );
}

#[test]
#[allow(clippy::useless_vec)]
fn unimplemented_flags_should_error() {
    let mut succeeded = Vec::new();

    // The following flags are not implemented
    for flag in ["cio", "nolinks", "text", "binary"] {
        let args = vec![format!("iflag={flag}")];

        if Parser::new()
            .parse(&args.iter().map(AsRef::as_ref).collect::<Vec<_>>()[..])
            .is_ok()
        {
            succeeded.push(format!("iflag={flag}"));
        }

        let args = vec![format!("oflag={flag}")];

        if Parser::new()
            .parse(&args.iter().map(AsRef::as_ref).collect::<Vec<_>>()[..])
            .is_ok()
        {
            succeeded.push(format!("iflag={flag}"));
        }
    }

    assert!(
        succeeded.is_empty(),
        "The following flags did not panic as expected: {succeeded:?}"
    );
}

#[test]
fn test_status_level_absent() {
    let args = &["if=foo.file", "of=bar.file"];

    assert_eq!(Parser::new().parse(args).unwrap().status, None);
}

#[test]
fn test_status_level_none() {
    let args = &["status=none", "if=foo.file", "of=bar.file"];

    assert_eq!(
        Parser::new().parse(args).unwrap().status,
        Some(StatusLevel::None)
    );
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_all_top_level_args_no_leading_dashes() {
    let args = &[
        "if=foo.file",
        "of=bar.file",
        "ibs=10",
        "obs=10",
        "cbs=1",
        "bs=100",
        "count=2",
        "skip=2",
        "seek=2",
        "iseek=2",
        "oseek=2",
        "status=progress",
        "conv=ascii,ucase",
        "iflag=count_bytes,skip_bytes",
        "oflag=append,seek_bytes",
    ];

    let settings = Parser::new().parse(args).unwrap();

    // ibs=10 and obs=10 are overwritten by bs=100
    assert_eq!(settings.ibs, 100);
    assert_eq!(settings.obs, 100);

    // count=2 iflag=count_bytes
    assert_eq!(settings.count, Some(Num::Bytes(2)));

    // seek=2 oflag=seek_bytes
    assert_eq!(settings.seek, 2);

    // skip=2 iflag=skip_bytes
    assert_eq!(settings.skip, 2);

    // status=progress
    assert_eq!(settings.status, Some(StatusLevel::Progress));

    // conv=ascii,ucase
    assert_eq!(
        settings.iconv,
        IConvFlags {
            // ascii implies unblock
            mode: Some(ConversionMode::ConvertThenUnblock(
                &EBCDIC_TO_ASCII_LCASE_TO_UCASE,
                1
            )),
            ..IConvFlags::default()
        },
    );

    // no conv flags apply to output
    assert_eq!(settings.oconv, OConvFlags::default(),);

    // iconv=count_bytes,skip_bytes
    assert_eq!(
        settings.iflags,
        IFlags {
            count_bytes: true,
            skip_bytes: true,
            ..IFlags::default()
        },
    );

    // oconv=append,seek_bytes
    assert_eq!(
        settings.oflags,
        OFlags {
            append: true,
            seek_bytes: true,
            ..OFlags::default()
        },
    );
}

#[test]
fn test_status_level_progress() {
    let args = &["if=foo.file", "of=bar.file", "status=progress"];

    let settings = Parser::new().parse(args).unwrap();

    assert_eq!(settings.status, Some(StatusLevel::Progress));
}

#[test]
fn test_status_level_noxfer() {
    let args = &["if=foo.file", "status=noxfer", "of=bar.file"];

    let settings = Parser::new().parse(args).unwrap();

    assert_eq!(settings.status, Some(StatusLevel::Noxfer));
}

#[test]
fn test_multiple_flags_options() {
    let args = &[
        "iflag=fullblock,count_bytes",
        "iflag=skip_bytes",
        "oflag=append",
        "oflag=seek_bytes",
        "conv=ascii,ucase",
        "conv=unblock",
        "cbs=512",
    ];
    let settings = Parser::new().parse(args).unwrap();

    // iflag
    assert_eq!(
        settings.iflags,
        IFlags {
            fullblock: true,
            count_bytes: true,
            skip_bytes: true,
            ..Default::default()
        }
    );

    // oflag
    assert_eq!(
        settings.oflags,
        OFlags {
            append: true,
            seek_bytes: true,
            ..Default::default()
        }
    );

    // conv
    assert_eq!(
        settings.iconv,
        IConvFlags {
            mode: Some(ConversionMode::ConvertThenUnblock(
                &EBCDIC_TO_ASCII_LCASE_TO_UCASE,
                512
            )),
            ..Default::default()
        }
    );
}

#[test]
fn test_override_multiple_options() {
    let args = &[
        "if=foo.file",
        "if=correct.file",
        "of=bar.file",
        "of=correct.file",
        "ibs=256",
        "ibs=1024",
        "obs=256",
        "obs=1024",
        "cbs=1",
        "cbs=2",
        "skip=0",
        "skip=2",
        "seek=0",
        "seek=2",
        "iseek=0",
        "iseek=2",
        "oseek=0",
        "oseek=2",
        "status=none",
        "status=noxfer",
        "count=512",
        "count=1024",
        "iflag=count_bytes",
    ];

    let settings = Parser::new().parse(args).unwrap();

    assert_eq!(settings.infile, Some("correct.file".into()));
    assert_eq!(settings.outfile, Some("correct.file".into()));
    assert_eq!(settings.ibs, 1024);
    assert_eq!(settings.obs, 1024);
    assert_eq!(settings.status, Some(StatusLevel::Noxfer));
    assert_eq!(settings.skip, 2048);
    assert_eq!(settings.seek, 2048);
    assert_eq!(settings.count, Some(Num::Bytes(1024)));
}

// // ----- IConvFlags/Output -----

#[test]
fn icf_ctable_error() {
    let args = &["conv=ascii,ebcdic,ibm"];
    assert!(Parser::new().parse(args).is_err());
}

#[test]
fn icf_case_error() {
    let args = &["conv=ucase,lcase"];
    assert!(Parser::new().parse(args).is_err());
}

#[test]
fn icf_block_error() {
    let args = &["conv=block,unblock"];
    assert!(Parser::new().parse(args).is_err());
}

#[test]
fn icf_creat_error() {
    let args = &["conv=excl,nocreat"];
    assert!(Parser::new().parse(args).is_err());
}

#[test]
fn parse_icf_token_ibm() {
    let args = &["conv=ibm"];
    let settings = Parser::new().parse(args).unwrap();

    assert_eq!(
        settings.iconv,
        IConvFlags {
            mode: Some(ConversionMode::ConvertOnly(&ASCII_TO_IBM)),
            ..Default::default()
        }
    );
}

#[test]
fn parse_icf_tokens_elu() {
    let args = &["conv=ebcdic,lcase"];
    let settings = Parser::new().parse(args).unwrap();

    assert_eq!(
        settings.iconv,
        IConvFlags {
            mode: Some(ConversionMode::ConvertOnly(&ASCII_TO_EBCDIC_UCASE_TO_LCASE)),
            ..Default::default()
        }
    );
}

#[test]
fn parse_icf_tokens_remaining() {
    let args = &["conv=ascii,ucase,block,sparse,swab,sync,noerror,excl,nocreat,notrunc,noerror,fdatasync,fsync"];
    assert_eq!(
        Parser::new().read(args),
        Ok(Parser {
            conv: ConvFlags {
                ascii: true,
                ucase: true,
                block: true,
                sparse: true,
                swab: true,
                sync: true,
                noerror: true,
                excl: true,
                nocreat: true,
                notrunc: true,
                fdatasync: true,
                fsync: true,
                ..Default::default()
            },
            is_conv_specified: true,
            ..Default::default()
        })
    );
}

#[test]
fn parse_iflag_tokens() {
    let args = &["iflag=fullblock,count_bytes,skip_bytes"];
    assert_eq!(
        Parser::new().read(args),
        Ok(Parser {
            iflag: IFlags {
                fullblock: true,
                count_bytes: true,
                skip_bytes: true,
                ..Default::default()
            },
            ..Default::default()
        })
    );
}

#[test]
fn parse_oflag_tokens() {
    let args = &["oflag=append,seek_bytes"];
    assert_eq!(
        Parser::new().read(args),
        Ok(Parser {
            oflag: OFlags {
                append: true,
                seek_bytes: true,
                ..Default::default()
            },
            ..Default::default()
        })
    );
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn parse_iflag_tokens_linux() {
    let args = &["iflag=direct,directory,dsync,sync,nonblock,noatime,noctty,nofollow"];
    assert_eq!(
        Parser::new().read(args),
        Ok(Parser {
            iflag: IFlags {
                direct: true,
                directory: true,
                dsync: true,
                sync: true,
                nonblock: true,
                noatime: true,
                noctty: true,
                nofollow: true,
                ..Default::default()
            },
            ..Default::default()
        })
    );
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn parse_oflag_tokens_linux() {
    let args = &["oflag=direct,directory,dsync,sync,nonblock,noatime,noctty,nofollow"];
    assert_eq!(
        Parser::new().read(args),
        Ok(Parser {
            oflag: OFlags {
                direct: true,
                directory: true,
                dsync: true,
                sync: true,
                nonblock: true,
                noatime: true,
                noctty: true,
                nofollow: true,
                ..Default::default()
            },
            ..Default::default()
        })
    );
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
fn test_neg_panic() {
    let bs_str = format!("{}", -1);

    parse_bytes_with_opt_multiplier(&bs_str).unwrap();
}
