// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

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
                            print_level: None,
                            count: None,
                            cflags: icf!($ctable),
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

macro_rules! make_icf_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $icf:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            print_level: None,
                            count: None,
                            cflags: $icf,
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
fn all_valid_ascii_ebcdic_ascii_roundtrip_conv_test() {
    // ASCII->EBCDIC
    let test_name = "all-valid-ascii-to-ebcdic";
    let tmp_fname_ae = format!("./test-resources/FAILED-{}.test", test_name);

    let i = Input {
        src: File::open(
            "./test-resources/all-valid-ascii-chars-37eff01866ba3f538421b30b7cbefcac.test",
        )
        .unwrap(),
        non_ascii: false,
        ibs: 128,
        print_level: None,
        count: None,
        cflags: icf!(Some(&ASCII_TO_EBCDIC)),
        iflags: IFlags::default(),
    };

    let o = Output {
        dst: File::create(&tmp_fname_ae).unwrap(),
        obs: 1024,
        cflags: OConvFlags::default(),
    };

    o.dd_out(i).unwrap();

    // EBCDIC->ASCII
    let test_name = "all-valid-ebcdic-to-ascii";
    let tmp_fname_ea = format!("./test-resources/FAILED-{}.test", test_name);

    let i = Input {
        src: File::open(&tmp_fname_ae).unwrap(),
        non_ascii: false,
        ibs: 256,
        print_level: None,
        count: None,
        cflags: icf!(Some(&EBCDIC_TO_ASCII)),
        iflags: IFlags::default(),
    };

    let o = Output {
        dst: File::create(&tmp_fname_ea).unwrap(),
        obs: 1024,
        cflags: OConvFlags::default(),
    };

    o.dd_out(i).unwrap();

    // Final Comparison
    let res = File::open(&tmp_fname_ea).unwrap();
    let spec =
        File::open("./test-resources/all-valid-ascii-chars-37eff01866ba3f538421b30b7cbefcac.test")
            .unwrap();

    assert_eq!(
        res.metadata().unwrap().len(),
        spec.metadata().unwrap().len()
    );

    let res = BufReader::new(res);
    let spec = BufReader::new(spec);

    let res = BufReader::new(res);

    // Check all bytes match
    for (b_res, b_spec) in res.bytes().zip(spec.bytes()) {
        assert_eq!(b_res.unwrap(), b_spec.unwrap());
    }

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
        sync: None,
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
        sync: None,
        noerror: false,
    },
    File::open("./test-resources/seq-byte-values-odd.spec").unwrap()
);
