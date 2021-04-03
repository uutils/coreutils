use super::*;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs;
use md5::{ Md5, Digest, };
use hex_literal::hex;
// use tempfile::tempfile;
// TODO: (Maybe) Use tempfiles in the tests.

macro_rules! make_hash_test (
    ( $test_id:ident, $test_name:expr, $src:expr, $exp:expr ) =>
    {
        #[test]
        fn $test_id()
        {
            let tmp_fname = format!("./test-resources/FAILED-{}.test", $test_name);

            let i = Input {
                src: $src,
                ibs: 256,
                output_progress: false,
            };

            let o = Output {
                dst: File::create(&tmp_fname).unwrap(),
                obs: 1024,
                conv_table: None,
            };

            dd(i,o).unwrap();

            let res = {
                let res = File::open(&tmp_fname).unwrap();
                let res = BufReader::new(res);

                let mut h = Md5::new();
                for b in res.bytes()
                {
                    h.update([b.unwrap()]);
                }

                h.finalize()
            };

            assert_eq!(hex!($exp), res[..]);

            fs::remove_file(&tmp_fname).unwrap();
        }
    };
    ( $test_id:ident, $test_name:expr, $i:expr, $o:expr, $exp:expr ) =>
    {
        #[test]
        fn $test_id()
        {
            let tmp_fname = format!("./test-resources/FAILED-{}.test", $test_name);

            let o = Output {
                dst: File::create(&tmp_fname).unwrap(),
                obs: $o.obs,
                conv_table: $o.conv_table,
            };

            dd($i,o).unwrap();

            let res = {
                let res = File::open(&tmp_fname).unwrap();
                let res = BufReader::new(res);

                let mut h = Md5::new();
                for b in res.bytes()
                {
                    h.update([b.unwrap()]);
                }

                h.finalize()
            };

            assert_eq!(hex!($exp), res[..]);

            fs::remove_file(&tmp_fname).unwrap();
        }
    };
);

macro_rules! make_spec_test (
    ( $test_id:ident, $test_name:expr, $i:expr, $o:expr, $spec:expr ) =>
    {
        #[test]
        fn $test_id()
        {
            let tmp_fname = format!("./test-resources/FAILED-{}.test", $test_name);

            let o = Output {
                dst: File::create(&tmp_fname).unwrap(),
                obs: $o.obs,
                conv_table: $o.conv_table,
            };

            dd($i,o).unwrap();

            let res = File::open(&tmp_fname).unwrap();
            let res = BufReader::new(res);

            let spec = BufReader::new($spec);

            for (b_res, b_spec) in res.bytes().zip(spec.bytes())
            {
                assert_eq!(b_res.unwrap(),
                           b_spec.unwrap());
            }

            fs::remove_file(&tmp_fname).unwrap();
        }
    };
);

make_hash_test!(
    empty_file_test,
    "stdio-empty-file",
    io::empty(),
    "d41d8cd98f00b204e9800998ecf8427e"
);

make_hash_test!(
    zeros_4k_test,
    "zeros-4k",
    File::open("./test-resources/zeros-620f0b67a91f7f74151bc5be745b7110.test").unwrap(),
    "620f0b67a91f7f74151bc5be745b7110"
);

make_hash_test!(
    ones_4k_test,
    "ones-4k",
    File::open("./test-resources/ones-6ae59e64850377ee5470c854761551ea.test").unwrap(),
    "6ae59e64850377ee5470c854761551ea"
);

make_hash_test!(
    deadbeef_32k_test,
    "deadbeef-32k",
    File::open("./test-resources/deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test").unwrap(),
    "18d99661a1de1fc9af21b0ec2cd67ba3"
);

make_hash_test!(
    random_73k_test,
    "random-73k",
    File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap(),
    "5828891cb1230748e146f34223bbd3b5"
);

make_spec_test!(
    atoe_conv_spec_test,
    "atoe-conv-spec-test",
    Input {
        src: File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
        ibs: 512,
        output_progress: false,
    },
    Output {
        dst: Vec::new(), // unused!
        obs: 512,
        conv_table: Some(ASCII_TO_EBCDIC),
    },
    File::open("./test-resources/gnudd-conv-atoe-seq-byte-values.spec").unwrap()
);

make_spec_test!(
    etoa_conv_spec_test,
    "etoa-conv-spec-test",
    Input {
        src: File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
        ibs: 512,
        output_progress: false,
    },
    Output {
        dst: Vec::new(), // unused!
        obs: 512,
        conv_table: Some(EBCDIC_TO_ASCII),
    },
    File::open("./test-resources/gnudd-conv-etoa-seq-byte-values.spec").unwrap()
);

make_spec_test!(
    atoibm_conv_spec_test,
    "atoibm-conv-spec-test",
    Input {
        src: File::open("./test-resources/seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test").unwrap(),
        ibs: 512,
        output_progress: false,
    },
    Output {
        dst: Vec::new(), // unused!
        obs: 512,
        conv_table: Some(ASCII_TO_IBM),
    },
    File::open("./test-resources/gnudd-conv-atoibm-seq-byte-values.spec").unwrap()
);

make_spec_test!(
    lcase_ascii_to_ucase_ascii,
    "lcase_ascii_to_ucase_ascii",
    Input {
        src: File::open("./test-resources/lcase-ascii.test").unwrap(),
        ibs: 512,
        output_progress: false,
    },
    Output {
        dst: Vec::new(), // unused!
        obs: 512,
        conv_table: Some(LCASE_TO_UCASE),
    },
    File::open("./test-resources/ucase-ascii.test").unwrap()
);

make_spec_test!(
    ucase_ascii_to_lcase_ascii,
    "ucase_ascii_to_lcase_ascii",
    Input {
        src: File::open("./test-resources/ucase-ascii.test").unwrap(),
        ibs: 512,
        output_progress: false,
    },
    Output {
        dst: Vec::new(), // unused!
        obs: 512,
        conv_table: Some(UCASE_TO_LCASE),
    },
    File::open("./test-resources/lcase-ascii.test").unwrap()
);

#[test]
fn all_valid_ascii_ebcdic_ascii_roundtrip_conv_test()
{
    // ASCII->EBCDIC
    let test_name = "all-valid-ascii-to-ebcdic";
    let tmp_fname_ae = format!("./test-resources/FAILED-{}.test", test_name);

    let i = Input {
        src: File::open("./test-resources/all-valid-ascii-chars-37eff01866ba3f538421b30b7cbefcac.test").unwrap(),
        ibs: 256,
        output_progress: false,
    };

    let o = Output {
        dst: File::create(&tmp_fname_ae).unwrap(),
        obs: 1024,
        conv_table: Some(ASCII_TO_EBCDIC),
    };

    dd(i,o).unwrap();

    // EBCDIC->ASCII
    let test_name = "all-valid-ebcdic-to-ascii";
    let tmp_fname_ea = format!("./test-resources/FAILED-{}.test", test_name);

    let i = Input {
        src: File::open(&tmp_fname_ae).unwrap(),
        ibs: 256,
        output_progress: false,
    };

    let o = Output {
        dst: File::create(&tmp_fname_ea).unwrap(),
        obs: 1024,
        conv_table: Some(EBCDIC_TO_ASCII),
    };

    dd(i,o).unwrap();

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

// #[test]
// fn copy_zerofile_from_args()
// {
//     let spec = File::open("./test-resources/zeros-620f0b67a91f7f74151bc5be745b7110.test").unwrap();
//     let tmp_fname = format!("./test-resources/{}", "zeros-from-args.test");
//
//     let args = vec![
//         String::from("if=./test-resources/zeros-620f0b67a91f7f74151bc5be745b7110.test"),
//         String::from(&tmp_fname),
//         ];
//     let args = Args { args };
//
//     uumain(args);
//
//     let res = File::open(&tmp_fname).unwrap();
//     let res = BufReader::new(res);
//
//     let spec = BufReader::new(spec);
//
//     for (b_res, b_spec) in res.bytes().zip(spec.bytes())
//     {
//         assert_eq!(b_res.unwrap(),
//                    b_spec.unwrap());
//     }
//
//     fs::remove_file(&tmp_fname).unwrap();
// }

//use rand::prelude::*;
//#[test]
//fn make_test_data()
//{
//    let mut f = File::create("./test-resources/random-walk-through-the-ascii-ranged-forest.test").unwrap();
//    // let mut rng = rand::thread_rng();

//    for _ in 0..65536 {
//        f.write(&[c]).unwrap();
//    }
//}
