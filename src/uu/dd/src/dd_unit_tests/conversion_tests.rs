use super::*;

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
