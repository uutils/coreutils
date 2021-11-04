// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

use super::*;

macro_rules! make_sync_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $sync:expr, $ibs:expr, $obs:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: $ibs,
                            print_level: None,
                            count: None,
                            cflags: IConvFlags {
                                sync: $sync,
                                ..IConvFlags::default()
                            },
                            iflags: IFlags::default(),
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: $obs,
                            cflags: OConvFlags::default(),
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
);

// Zeros
make_sync_test!(
    zeros_4k_conv_sync_obs_gt_ibs,
    "zeros_4k_conv_sync_obs_gt_ibs",
    File::open("./test-resources/zeros-620f0b67a91f7f74151bc5be745b7110.test").unwrap(),
    Some(0u8),
    521,
    1031,
    File::open("./test-resources/gnudd-conv-sync-ibs-521-obs-1031-zeros.spec").unwrap()
);

make_sync_test!(
    zeros_4k_conv_sync_ibs_gt_obs,
    "zeros_4k_conv_sync_ibs_gt_obs",
    File::open("./test-resources/zeros-620f0b67a91f7f74151bc5be745b7110.test").unwrap(),
    Some(0u8),
    1031,
    521,
    File::open("./test-resources/gnudd-conv-sync-ibs-1031-obs-521-zeros.spec").unwrap()
);

// Deadbeef
make_sync_test!(
    deadbeef_32k_conv_sync_obs_gt_ibs,
    "deadbeef_32k_conv_sync_obs_gt_ibs",
    File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
    Some(0u8),
    521,
    1031,
    File::open("./test-resources/gnudd-conv-sync-ibs-521-obs-1031-deadbeef.spec").unwrap()
);

make_sync_test!(
    deadbeef_32k_conv_sync_ibs_gt_obs,
    "deadbeef_32k_conv_sync_ibs_gt_obs",
    File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
    Some(0u8),
    1031,
    521,
    File::open("./test-resources/gnudd-conv-sync-ibs-1031-obs-521-deadbeef.spec").unwrap()
);

// Random
make_sync_test!(
    random_73k_test_bs_prime_obs_gt_ibs_sync,
    "random-73k-test-bs-prime-obs-gt-ibs-sync",
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
    Some(0u8),
    521,
    1031,
    File::open("./test-resources/gnudd-conv-sync-ibs-521-obs-1031-random.spec").unwrap()
);

make_sync_test!(
    random_73k_test_bs_prime_ibs_gt_obs_sync,
    "random-73k-test-bs-prime-ibs-gt-obs-sync",
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
    Some(0u8),
    1031,
    521,
    File::open("./test-resources/gnudd-conv-sync-ibs-1031-obs-521-random.spec").unwrap()
);

make_sync_test!(
    deadbeef_16_delayed,
    "deadbeef-16-delayed",
    LazyReader {
        src: File::open("./test-resources/deadbeef-16.test").unwrap()
    },
    Some(0u8),
    16,
    32,
    File::open("./test-resources/deadbeef-16.spec").unwrap()
);
