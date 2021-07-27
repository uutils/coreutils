// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

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
        print_level: None,
        count: None,
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: None,
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 521,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: Some(CountType::Reads(32)),
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1024,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: Some(CountType::Bytes(32 * 1024)),
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: Some(CountType::Reads(16)),
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: Some(CountType::Bytes(12345)),
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: Some(CountType::Reads(32)),
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1024,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: Some(CountType::Bytes(32 * 1024)),
        cflags: IConvFlags::default(),
        iflags: IFlags::default(),
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: OConvFlags::default(),
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
        print_level: None,
        count: None,
        cflags: IConvFlags::default(),
        iflags: IFlags {
            fullblock: true,
            ..IFlags::default()
        },
    },
    Output {
        dst: DST_PLACEHOLDER,
        obs: 1031,
        cflags: OConvFlags::default(),
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

#[test]
#[should_panic]
fn test_nocreat_causes_failure_when_ofile_doesnt_exist() {
    let args = vec![
        String::from("dd"),
        String::from("--conv=nocreat"),
        String::from("--of=not-a-real.file"),
    ];

    let matches = uu_app().get_matches_from_safe(args).unwrap();
    let _ = Output::<File>::new(&matches).unwrap();
}
