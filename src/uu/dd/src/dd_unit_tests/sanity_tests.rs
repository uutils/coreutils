use super::*;

const DST_PLACEHOLDER: Vec<u8> = Vec::new();

macro_rules! make_io_test (
    ( $test_id:ident, $test_name:expr, $i:expr, $o:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        $i,
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: $o.obs,
                            cflags: $o.cflags,
                            oflags: $o.oflags,
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
);

make_spec_test!(
    zeros_4k_test,
    "zeros-4k",
    File::open("./test-resources/zeros-620f0b67a91f7f74151bc5be745b7110.test").unwrap()
);

make_spec_test!(
    ones_4k_test,
    "ones-4k",
    File::open("./test-resources/ones-6ae59e64850377ee5470c854761551ea.test").unwrap()
);

make_spec_test!(
    deadbeef_32k_test,
    "deadbeef-32k",
    File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap()
);

make_spec_test!(
    random_73k_test,
    "random-73k",
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap()
);

make_io_test!(
    random_73k_test_not_a_multiple_obs_gt_ibs,
    "random-73k-not-a-multiple-obs-gt-ibs",
    Input {
        src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
        non_ascii: false,
        ibs: 521,
        xfer_stats: None,
        count: None,
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap()
);

make_io_test!(
    random_73k_test_obs_lt_not_a_multiple_ibs,
    "random-73k-obs-lt-not-a-multiple-ibs",
    Input {
        src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
        non_ascii: false,
        ibs: 1031,
        xfer_stats: None,
        count: None,
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 521,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap()
);

make_io_test!(
    deadbeef_all_32k_test_count_reads,
    "deadbeef_all_32k_test_count_reads",
    Input {
        src: File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
        non_ascii: false,
        ibs: 1024,
        xfer_stats: None,
        count: Some(CountType::Reads(32)),
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1024,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap()
);

make_io_test!(
    deadbeef_all_32k_test_count_bytes,
    "deadbeef_all_32k_test_count_bytes",
    Input {
        src: File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
        non_ascii: false,
        ibs: 531,
        xfer_stats: None,
        count: Some(CountType::Bytes(32 * 1024)),
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap()
);

make_io_test!(
    deadbeef_32k_to_16k_test_count_reads,
    "deadbeef_32k_test_count_reads",
    Input {
        src: File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
        non_ascii: false,
        ibs: 1024,
        xfer_stats: None,
        count: Some(CountType::Reads(16)),
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/gnudd-deadbeef-first-16k.spec").unwrap()
);

make_io_test!(
    deadbeef_32k_to_12345_test_count_bytes,
    "deadbeef_32k_to_12345_test_count_bytes",
    Input {
        src: File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
        non_ascii: false,
        ibs: 531,
        xfer_stats: None,
        count: Some(CountType::Bytes(12345)),
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/gnudd-deadbeef-first-12345.spec").unwrap()
);

make_io_test!(
    random_73k_test_count_reads,
    "random-73k-test-count-reads",
    Input {
        src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
        non_ascii: false,
        ibs: 1024,
        xfer_stats: None,
        count: Some(CountType::Reads(32)),
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1024,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/gnudd-random-first-32k.spec").unwrap()
);

make_io_test!(
    random_73k_test_count_bytes,
    "random-73k-test-count-bytes",
    Input {
        src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
        non_ascii: false,
        ibs: 521,
        xfer_stats: None,
        count: Some(CountType::Bytes(32 * 1024)),
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/gnudd-random-first-32k.spec").unwrap()
);

make_io_test!(
    random_73k_test_lazy_fullblock,
    "random-73k-test-lazy-fullblock",
    Input {
        src: LazyReader {
            src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test")
                .unwrap()
        },
        non_ascii: false,
        ibs: 521,
        xfer_stats: None,
        count: None,
        cflags: icf!(),
        iflags: IFlags {
            fullblock: true,
            cio: false,
            direct: false,
            directory: false,
            dsync: false,
            sync: false,
            nocache: false,
            nonblock: false,
            noatime: false,
            noctty: false,
            nofollow: false,
            nolinks: false,
            binary: false,
            text: false,
            count_bytes: false,
            skip_bytes: false,
        },
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap()
);

// Test internal buffer size fn
#[test]
fn bsize_test_primes() {
    let (n, m) = (7901, 7919);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, n * m);
}

#[test]
fn bsize_test_rel_prime_obs_greater() {
    let (n, m) = (7 * 5119, 13 * 5119);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, 7 * 13 * 5119);
}

#[test]
fn bsize_test_rel_prime_ibs_greater() {
    let (n, m) = (13 * 5119, 7 * 5119);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, 7 * 13 * 5119);
}

#[test]
fn bsize_test_3fac_rel_prime() {
    let (n, m) = (11 * 13 * 5119, 7 * 11 * 5119);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, 7 * 11 * 13 * 5119);
}

#[test]
fn bsize_test_ibs_greater() {
    let (n, m) = (512 * 1024, 256 * 1024);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, n);
}

#[test]
fn bsize_test_obs_greater() {
    let (n, m) = (256 * 1024, 512 * 1024);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, m);
}

#[test]
fn bsize_test_bs_eq() {
    let (n, m) = (1024, 1024);
    let res = calc_bsize(n, m);
    assert!(res % n == 0);
    assert!(res % m == 0);

    assert_eq!(res, m);
}
