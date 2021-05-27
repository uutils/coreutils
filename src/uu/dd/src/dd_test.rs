use super::*;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs;
use md5::{ Md5, Digest, };
use hex_literal::hex;

// use tempfile::tempfile;
// TODO: (Maybe) Use tempfiles in the tests.

const DEFAULT_CFO: OConvFlags = OConvFlags {
    sparse: false,
    excl: false,
    nocreat: false,
    notrunc: false,
    fdatasync: false,
    fsync: false,
};

const DEFAULT_IFLAGS: IFlags = IFlags {
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
    fullblock: false,
    count_bytes: false,
    skip_bytes: false,
};

const DEFAULT_OFLAGS: OFlags = OFlags {
    append: false,
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
    seek_bytes: false,
};

#[macro_export]
macro_rules! icf (
    () =>
    {
        icf!(None)
    };
    ( $ctable:expr ) =>
    {
        IConvFlags {
            ctable: $ctable,
            block: None,
            unblock: None,
            swab: false,
            sync: false,
            noerror: false,
        }
    };
);

macro_rules! make_spec_test (
    ( $test_id:ident, $test_name:expr, $src:expr ) =>
    {
        // When spec not given, output should match input
        make_spec_test!($test_id, $test_name, $src, $src);
    };
    ( $test_id:ident, $test_name:expr, $src:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            xfer_stats: StatusLevel::None,
                            cflags: icf!(),
                            iflags: DEFAULT_IFLAGS,
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: DEFAULT_CFO,
                            oflags: DEFAULT_OFLAGS,
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
    ( $test_id:ident, $test_name:expr, $i:expr, $o:expr, $spec:expr, $tmp_fname:expr ) =>
    {
        #[test]
        fn $test_id()
        {
            dd_fileout($i,$o).unwrap();

            let res = File::open($tmp_fname).unwrap();
            let res = BufReader::new(res);

            let spec = BufReader::new($spec);

            for (b_res, b_spec) in res.bytes().zip(spec.bytes())
            {
                assert_eq!(b_res.unwrap(),
                           b_spec.unwrap());
            }

            fs::remove_file($tmp_fname).unwrap();
        }
    };
);

macro_rules! make_conv_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $ctable:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            xfer_stats: StatusLevel::None,
                            cflags: icf!($ctable),
                            iflags: DEFAULT_IFLAGS,
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: DEFAULT_CFO,
                            oflags: DEFAULT_OFLAGS,
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
);

macro_rules! make_icf_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $icf:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            xfer_stats: StatusLevel::None,
                            cflags: $icf,
                            iflags: DEFAULT_IFLAGS,
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: DEFAULT_CFO,
                            oflags: DEFAULT_OFLAGS,
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

make_conv_test!(
    atoe_conv_spec_test,
    "atoe-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&ASCII_TO_EBCDIC),
    File::open("./test-resources/gnudd-conv-atoe-seq-byte-values.spec").unwrap()
);

make_conv_test!(
    etoa_conv_spec_test,
    "etoa-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&EBCDIC_TO_ASCII),
    File::open("./test-resources/gnudd-conv-etoa-seq-byte-values.spec").unwrap()
);

make_conv_test!(
    atoibm_conv_spec_test,
    "atoibm-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&ASCII_TO_IBM),
    File::open("./test-resources/gnudd-conv-atoibm-seq-byte-values.spec").unwrap()
);

make_conv_test!(
    lcase_ascii_to_ucase_ascii,
    "lcase_ascii_to_ucase_ascii",
    File::open("./test-resources/lcase-ascii.test").unwrap(),
    Some(&ASCII_LCASE_TO_UCASE),
    File::open("./test-resources/ucase-ascii.test").unwrap()
);

make_conv_test!(
    ucase_ascii_to_lcase_ascii,
    "ucase_ascii_to_lcase_ascii",
    File::open("./test-resources/ucase-ascii.test").unwrap(),
    Some(&ASCII_UCASE_TO_LCASE),
    File::open("./test-resources/lcase-ascii.test").unwrap()
);

make_conv_test!(
    // conv=ebcdic,ucase
    atoe_and_ucase_conv_spec_test,
    "atoe-and-ucase-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&ASCII_TO_EBCDIC_LCASE_TO_UCASE),
    File::open("./test-resources/ucase-ebcdic.test").unwrap()
);

make_conv_test!(
    // conv=ebcdic,lcase
    atoe_and_lcase_conv_spec_test,
    "atoe-and-lcase-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&ASCII_TO_EBCDIC_UCASE_TO_LCASE),
    File::open("./test-resources/lcase-ebcdic.test").unwrap()
);

make_conv_test!(
    // conv=ibm,ucase
    atoibm_and_ucase_conv_spec_test,
    "atoibm-and-ucase-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&ASCII_TO_IBM_UCASE_TO_LCASE),
    File::open("./test-resources/lcase-ibm.test").unwrap()
);

make_conv_test!(
    // conv=ibm,lcase
    atoibm_and_lcase_conv_spec_test,
    "atoibm-and-lcase-conv-spec-test",
    File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
    Some(&ASCII_TO_IBM_LCASE_TO_UCASE),
    File::open("./test-resources/ucase-ibm.test").unwrap()
);

#[test]
fn all_valid_ascii_ebcdic_ascii_roundtrip_conv_test()
{
    // ASCII->EBCDIC
    let test_name = "all-valid-ascii-to-ebcdic";
    let tmp_fname_ae = format!("./test-resources/FAILED-{}.test", test_name);

    let i = Input {
        src: File::open("./test-resources/all-valid-ascii-chars-37eff01866ba3f538421b30b7cbefcac.test").unwrap(),
        non_ascii: false,
        ibs: 128,
        xfer_stats: StatusLevel::None,
        cflags: icf!(Some(&ASCII_TO_EBCDIC)),
        iflags: DEFAULT_IFLAGS,
    };

    let o = Output {
        dst: File::create(&tmp_fname_ae).unwrap(),
        obs: 1024,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    };

    dd_fileout(i,o).unwrap();

    // EBCDIC->ASCII
    let test_name = "all-valid-ebcdic-to-ascii";
    let tmp_fname_ea = format!("./test-resources/FAILED-{}.test", test_name);

    let i = Input {
        src: File::open(&tmp_fname_ae).unwrap(),
        non_ascii: false,
        ibs: 256,
        xfer_stats: StatusLevel::None,
        cflags: icf!(Some(&EBCDIC_TO_ASCII)),
        iflags: DEFAULT_IFLAGS,
    };

    let o = Output {
        dst: File::create(&tmp_fname_ea).unwrap(),
        obs: 1024,
        cflags: DEFAULT_CFO,
        oflags: DEFAULT_OFLAGS,
    };

    dd_fileout(i,o).unwrap();

    let res = {
        let res = File::open(&tmp_fname_ea).unwrap();
        let res = BufReader::new(res);

        let mut h = Md5::new();
        for b in res.bytes()
        {
            h.update([b.unwrap()]);
        }

        h.finalize()
    };

    assert_eq!(hex!("37eff01866ba3f538421b30b7cbefcac"), res[..]);

    fs::remove_file(&tmp_fname_ae).unwrap();
    fs::remove_file(&tmp_fname_ea).unwrap();
}

make_icf_test!(
    swab_256_test,
    "swab-256",
    File::open("./test-resources/seq-byte-values.test").unwrap(),
    IConvFlags {
        ctable: None,
        block: None,
        unblock: None,
        swab: true,
        sync: false,
        noerror: false,
    },
    File::open("./test-resources/seq-byte-values-swapped.test").unwrap()
);

make_icf_test!(
    swab_257_test,
    "swab-257",
    File::open("./test-resources/seq-byte-values-odd.test").unwrap(),
    IConvFlags {
        ctable: None,
        block: None,
        unblock: None,
        swab: true,
        sync: false,
        noerror: false,
    },
    File::open("./test-resources/seq-byte-values-odd.spec").unwrap()
);

fn block_test_basic()
{
    let mut buf = vec![0u8, 1u8, 2u8, 3u8];
    let res = block(&buf, 4);

    assert_eq!(res, vec![buf]);
}
