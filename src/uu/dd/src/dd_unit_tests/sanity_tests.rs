use super::*;

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

make_spec_test!(
    random_73k_test_not_a_multiple_obs_gt_ibs,
    "random-73k-not-a-multiple-obs-gt-ibs",
    Input {
        src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
        non_ascii: false,
        ibs: 521,
        xfer_stats: StatusLevel::None,
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: File::create(format!("./test-resources/FAILED-{}.test", "random-73k-not-a-multiple-obs-gt-ibs")).unwrap(),
        obs: 1031,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
    format!("./test-resources/FAILED-{}.test", "random-73k-not-a-multiple-obs-gt-ibs")
);

make_spec_test!(
    random_73k_test_obs_lt_not_a_multiple_ibs,
    "random-73k-obs-lt-not-a-multiple-ibs",
    Input {
        src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
        non_ascii: false,
        ibs: 1031,
        xfer_stats: StatusLevel::None,
        cflags: icf!(),
        iflags: DEFAULT_IFLAGS,
    },
    Output {
        dst: File::create(format!("./test-resources/FAILED-{}.test", "random-73k-obs-lt-not-a-multiple-ibs")).unwrap(),
        obs: 521,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    },
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
    format!("./test-resources/FAILED-{}.test", "random-73k-obs-lt-not-a-multiple-ibs")
);
