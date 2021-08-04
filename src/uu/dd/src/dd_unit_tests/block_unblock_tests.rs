// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

use super::*;

#[cfg(unix)]
macro_rules! make_block_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $block:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            print_level: None,
                            count: None,
                            cflags: IConvFlags {
                                block: $block,
                                ..IConvFlags::default()
                            },
                            iflags: IFlags::default(),
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: OConvFlags::default(),
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
);

#[cfg(unix)]
macro_rules! make_unblock_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $unblock:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            print_level: None,
                            count: None,
                            cflags: IConvFlags {
                                unblock: $unblock,
                                ..IConvFlags::default()
                            },
                            iflags: IFlags::default(),
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: OConvFlags::default(),
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
);

#[test]
fn block_test_no_nl() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, 3u8];
    let res = block(buf, 4, &mut rs);

    assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
}

#[test]
fn block_test_no_nl_short_record() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, 3u8];
    let res = block(buf, 8, &mut rs);

    assert_eq!(
        res,
        vec![vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],]
    );
}

#[test]
fn block_test_no_nl_trunc() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, 3u8, 4u8];
    let res = block(buf, 4, &mut rs);

    // Commented section(s) should be truncated and appear for reference only.
    assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8 /*, 4u8*/],]);
    assert_eq!(rs.records_truncated, 1);
}

#[test]
fn block_test_nl_gt_cbs_trunc() {
    let mut rs = ReadStat::default();
    let buf = vec![
        0u8, 1u8, 2u8, 3u8, 4u8, NEWLINE, 0u8, 1u8, 2u8, 3u8, 4u8, NEWLINE, 5u8, 6u8, 7u8, 8u8,
    ];
    let res = block(buf, 4, &mut rs);

    assert_eq!(
        res,
        vec![
            // Commented section(s) should be truncated and appear for reference only.
            vec![0u8, 1u8, 2u8, 3u8],
            // vec![4u8, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, 2u8, 3u8],
            // vec![4u8, SPACE, SPACE, SPACE],
            vec![5u8, 6u8, 7u8, 8u8],
        ]
    );
    assert_eq!(rs.records_truncated, 2);
}

#[test]
fn block_test_surrounded_nl() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, 5u8, 6u8, 7u8, 8u8];
    let res = block(buf, 8, &mut rs);

    assert_eq!(
        res,
        vec![
            vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
            vec![4u8, 5u8, 6u8, 7u8, 8u8, SPACE, SPACE, SPACE],
        ]
    );
}

#[test]
fn block_test_multiple_nl_same_cbs_block() {
    let mut rs = ReadStat::default();
    let buf = vec![
        0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, NEWLINE, 5u8, 6u8, 7u8, 8u8, 9u8,
    ];
    let res = block(buf, 8, &mut rs);

    assert_eq!(
        res,
        vec![
            vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
            vec![4u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![5u8, 6u8, 7u8, 8u8, 9u8, SPACE, SPACE, SPACE],
        ]
    );
}

#[test]
fn block_test_multiple_nl_diff_cbs_block() {
    let mut rs = ReadStat::default();
    let buf = vec![
        0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, 5u8, 6u8, 7u8, NEWLINE, 8u8, 9u8,
    ];
    let res = block(buf, 8, &mut rs);

    assert_eq!(
        res,
        vec![
            vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
            vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
            vec![8u8, 9u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
        ]
    );
}

#[test]
fn block_test_end_nl_diff_cbs_block() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, 3u8, NEWLINE];
    let res = block(buf, 4, &mut rs);

    assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
}

#[test]
fn block_test_end_nl_same_cbs_block() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, NEWLINE];
    let res = block(buf, 4, &mut rs);

    assert_eq!(res, vec![vec![0u8, 1u8, 2u8, SPACE]]);
}

#[test]
fn block_test_double_end_nl() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, NEWLINE, NEWLINE];
    let res = block(buf, 4, &mut rs);

    assert_eq!(
        res,
        vec![vec![0u8, 1u8, 2u8, SPACE], vec![SPACE, SPACE, SPACE, SPACE],]
    );
}

#[test]
fn block_test_start_nl() {
    let mut rs = ReadStat::default();
    let buf = vec![NEWLINE, 0u8, 1u8, 2u8, 3u8];
    let res = block(buf, 4, &mut rs);

    assert_eq!(
        res,
        vec![vec![SPACE, SPACE, SPACE, SPACE], vec![0u8, 1u8, 2u8, 3u8],]
    );
}

#[test]
fn block_test_double_surrounded_nl_no_trunc() {
    let mut rs = ReadStat::default();
    let buf = vec![0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8];
    let res = block(buf, 8, &mut rs);

    assert_eq!(
        res,
        vec![
            vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
            vec![SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
        ]
    );
}

#[test]
fn block_test_double_surrounded_nl_double_trunc() {
    let mut rs = ReadStat::default();
    let buf = vec![
        0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8, 8u8,
    ];
    let res = block(buf, 4, &mut rs);

    assert_eq!(
        res,
        vec![
            // Commented section(s) should be truncated and appear for reference only.
            vec![0u8, 1u8, 2u8, 3u8],
            vec![SPACE, SPACE, SPACE, SPACE],
            vec![4u8, 5u8, 6u8, 7u8 /*, 8u8*/],
        ]
    );
    assert_eq!(rs.records_truncated, 1);
}

#[cfg(unix)]
make_block_test!(
    block_cbs16,
    "block-cbs-16",
    File::open("./test-resources/dd-block-cbs16.test").unwrap(),
    Some(16),
    File::open("./test-resources/dd-block-cbs16.spec").unwrap()
);

#[cfg(unix)]
make_block_test!(
    block_cbs16_as_cbs8,
    "block-cbs-16-as-cbs8",
    File::open("./test-resources/dd-block-cbs16.test").unwrap(),
    Some(8),
    File::open("./test-resources/dd-block-cbs8.spec").unwrap()
);

#[cfg(unix)]
make_block_test!(
    block_consecutive_nl,
    "block-consecutive-nl",
    File::open("./test-resources/dd-block-consecutive-nl.test").unwrap(),
    Some(16),
    File::open("./test-resources/dd-block-consecutive-nl-cbs16.spec").unwrap()
);

#[test]
fn unblock_test_full_cbs() {
    let buf = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8];
    let res = unblock(buf, 8);

    assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, NEWLINE],);
}

#[test]
fn unblock_test_all_space() {
    let buf = vec![SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE];
    let res = unblock(buf, 8);

    assert_eq!(res, vec![NEWLINE],);
}

#[test]
fn unblock_test_decoy_spaces() {
    let buf = vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, 7u8];
    let res = unblock(buf, 8);

    assert_eq!(
        res,
        vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, 7u8, NEWLINE],
    );
}

#[test]
fn unblock_test_strip_single_cbs() {
    let buf = vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE];
    let res = unblock(buf, 8);

    assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, NEWLINE],);
}

#[test]
fn unblock_test_strip_multi_cbs() {
    let buf = vec![
        vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
        vec![0u8, 1u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
        vec![0u8, 1u8, 2u8, SPACE, SPACE, SPACE, SPACE, SPACE],
        vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let res = unblock(buf, 8);

    let exp = vec![
        vec![0u8, NEWLINE],
        vec![0u8, 1u8, NEWLINE],
        vec![0u8, 1u8, 2u8, NEWLINE],
        vec![0u8, 1u8, 2u8, 3u8, NEWLINE],
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    assert_eq!(res, exp);
}

#[cfg(unix)]
make_unblock_test!(
    unblock_multi_16,
    "unblock-multi-16",
    File::open("./test-resources/dd-unblock-cbs16.test").unwrap(),
    Some(16),
    File::open("./test-resources/dd-unblock-cbs16.spec").unwrap()
);

#[cfg(unix)]
make_unblock_test!(
    unblock_multi_16_as_8,
    "unblock-multi-16-as-8",
    File::open("./test-resources/dd-unblock-cbs16.test").unwrap(),
    Some(8),
    File::open("./test-resources/dd-unblock-cbs8.spec").unwrap()
);
