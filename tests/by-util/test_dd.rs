// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, availible, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, iseek, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, oseek, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat abcdefghijklm abcdefghi nabcde nabcdefg abcdefg fifoname seekable

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
#[cfg(all(unix, not(feature = "feat_selinux")))]
use uutests::util::run_ucmd_as_root_with_stdin_stdout;
#[cfg(all(not(windows), feature = "printf"))]
use uutests::util::{UCommand, get_tests_binary};
use uutests::util_name;

use regex::Regex;
use uucore::io::OwnedFileDescriptorOrHandle;

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
#[cfg(all(
    unix,
    not(target_os = "macos"),
    not(target_os = "freebsd"),
    feature = "printf"
))]
use std::process::{Command, Stdio};
#[cfg(not(windows))]
use std::thread::sleep;
#[cfg(not(windows))]
use std::time::Duration;
use tempfile::tempfile;

macro_rules! inf {
    ($fname:expr) => {
        format!("if={}", $fname)
    };
}

macro_rules! of {
    ($fname:expr) => {
        format!("of={}", $fname)
    };
}

macro_rules! fixture_path {
    ($fname:expr) => {{ PathBuf::from(format!("./tests/fixtures/dd/{}", $fname)) }};
}

macro_rules! assert_fixture_exists {
    ($fname:expr) => {{
        let fpath = fixture_path!($fname);
        assert!(fpath.exists(), "Fixture missing: {fpath:?}");
    }};
}

#[cfg(any(target_os = "linux", target_os = "android"))]
macro_rules! assert_fixture_not_exists {
    ($fname:expr) => {{
        let fpath = PathBuf::from(format!("./fixtures/dd/{}", $fname));
        assert!(!fpath.exists(), "Fixture present: {fpath:?}");
    }};
}

macro_rules! build_test_file {
    ($fp:expr, $data:expr) => {{
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open($fp)
            .unwrap()
            .write_all($data)
            .unwrap()
    }};
}

macro_rules! cmp_file (
    ($spec:expr, $test:expr) =>
    {
        let specfile_len = $spec.metadata().unwrap().len();
        let testfile_len = $test.metadata().unwrap().len();
        assert_eq!(testfile_len, specfile_len);

        let spec = BufReader::new($spec);
        let test = BufReader::new($test);

        for (b_spec, b_test) in spec.bytes().zip(test.bytes())
        {
            assert_eq!(b_spec.unwrap(),
                       b_test.unwrap());
        }
    };
);

fn build_ascii_block(n: usize) -> Vec<u8> {
    (0..=127).cycle().take(n).collect()
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

// Sanity Tests
#[test]
fn version() {
    new_ucmd!().args(&["--version"]).succeeds();
}

#[test]
fn help() {
    new_ucmd!().args(&["--help"]).succeeds();
}

#[test]
fn test_stdin_stdout() {
    let input = build_ascii_block(521);
    let output = String::from_utf8(input.clone()).unwrap();
    new_ucmd!()
        .args(&["status=none"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

// Top-Level Items
// count=N, skip=N, status=LEVEL, conv=FLAG, *flag=FLAG
#[test]
fn test_stdin_stdout_count() {
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    output.truncate(256);
    new_ucmd!()
        .args(&["status=none", "count=2", "ibs=128"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_count_bytes() {
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    output.truncate(256);
    new_ucmd!()
        .args(&["status=none", "count=256", "iflag=count_bytes"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_skip() {
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    let _ = output.drain(..256);
    new_ucmd!()
        .args(&["status=none", "skip=2", "ibs=128"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_skip_bytes() {
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    let _ = output.drain(..256);
    new_ucmd!()
        .args(&["status=none", "skip=256", "ibs=128", "iflag=skip_bytes"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_skip_w_multiplier() {
    let input = build_ascii_block(10 * 1024);
    let output = String::from_utf8(input[5 * 1024..].to_vec()).unwrap();
    new_ucmd!()
        .args(&["status=none", "skip=5K", "iflag=skip_bytes"])
        .pipe_in(input)
        .succeeds()
        .stdout_is(output);
}

#[test]
fn test_stdin_stdout_count_w_multiplier() {
    let input = build_ascii_block(5 * 1024);
    let output = String::from_utf8(input[..2 * 1024].to_vec()).unwrap();
    new_ucmd!()
        .args(&["status=none", "count=2KiB", "iflag=count_bytes"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_b_multiplier() {
    // "2b" means 2 * 512, which is 1024.
    new_ucmd!()
        .args(&["bs=2b", "count=1"])
        .pipe_in("a".repeat(1025))
        .succeeds()
        .stdout_is("a".repeat(1024));
}

#[test]
fn test_x_multiplier() {
    // "2x3" means 2 * 3, which is 6.
    new_ucmd!()
        .args(&["bs=2x3", "count=1"])
        .pipe_in("abcdefghi")
        .succeeds()
        .stdout_is("abcdef");
}

#[test]
fn test_zero_multiplier_warning() {
    for arg in ["count", "seek", "skip"] {
        new_ucmd!()
            .args(&[format!("{arg}=0").as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .no_stderr();

        new_ucmd!()
            .args(&[format!("{arg}=00x1").as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .no_stderr();

        new_ucmd!()
            .args(&[format!("{arg}=0x1").as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .stderr_contains("warning: '0x' is a zero multiplier; use '00x' if that is intended");

        new_ucmd!()
            .args(&[format!("{arg}=0x0x1").as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .stderr_is("dd: warning: '0x' is a zero multiplier; use '00x' if that is intended\ndd: warning: '0x' is a zero multiplier; use '00x' if that is intended\n");

        new_ucmd!()
            .args(&[format!("{arg}=1x0x1").as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .stderr_contains("warning: '0x' is a zero multiplier; use '00x' if that is intended");
    }
}

#[test]
fn test_final_stats_noxfer() {
    new_ucmd!()
        .args(&["status=noxfer"])
        .succeeds()
        .stderr_only("0+0 records in\n0+0 records out\n");
}

#[test]
fn test_final_stats_unspec() {
    new_ucmd!()
        .succeeds()
        .stderr_contains("0+0 records in\n0+0 records out\n0 bytes copied, ")
        .stderr_matches(&Regex::new(r"\d(\.\d+)?(e-\d\d)? s, ").unwrap())
        .stderr_contains("0.0 B/s");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_excl_causes_failure_when_present() {
    let fname = "this-file-exists-excl.txt";
    assert_fixture_exists!(&fname);

    let (_fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["of=this-file-exists-excl.txt", "conv=excl"])
        .fails();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_noatime_does_not_update_infile_atime() {
    // NOTE: Not all environments support tracking access time. If this
    // test fails on some systems and passes on others, assume the functionality
    // is not working and the systems that pass it simply don't update file access time.
    let fname = "this-ifile-exists-noatime.txt";
    assert_fixture_exists!(&fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "iflag=noatime", &inf!(fname)]);

    let pre_atime = fix.metadata(fname).accessed().unwrap();

    ucmd.succeeds().no_output();

    let post_atime = fix.metadata(fname).accessed().unwrap();
    assert_eq!(pre_atime, post_atime);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_noatime_does_not_update_ofile_atime() {
    // NOTE: Not all environments support tracking access time. If this
    // test fails on some systems and passes on others, assume the functionality
    // is not working and the systems that pass it simply don't update file access time.
    let fname = "this-ofile-exists-noatime.txt";
    assert_fixture_exists!(&fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "oflag=noatime", &of!(fname)]);

    let pre_atime = fix.metadata(fname).accessed().unwrap();

    ucmd.pipe_in("").succeeds().no_output();

    let post_atime = fix.metadata(fname).accessed().unwrap();
    assert_eq!(pre_atime, post_atime);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_nocreat_causes_failure_when_outfile_not_present() {
    let fname = "this-file-does-not-exist.txt";
    assert_fixture_not_exists!(&fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["conv=nocreat", &of!(&fname)])
        .pipe_in("")
        .fails()
        .stderr_only(
            "dd: failed to open 'this-file-does-not-exist.txt': No such file or directory\n",
        );
    assert!(!fix.file_exists(fname));
}

#[test]
fn test_notrunc_does_not_truncate() {
    // Set up test if needed (eg. after failure)
    let fname = "this-file-exists-notrunc.txt";
    let fpath = fixture_path!(fname);
    match fpath.metadata() {
        Ok(m) if m.len() == 256 => {}
        _ => build_test_file!(&fpath, &build_ascii_block(256)),
    }

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "conv=notrunc", &of!(&fname), "if=null.txt"])
        .succeeds()
        .no_output();

    assert_eq!(256, fix.metadata(fname).len());
}

#[test]
fn test_existing_file_truncated() {
    // Set up test if needed (eg. after failure)
    let fname = "this-file-exists-truncated.txt";
    let fpath = fixture_path!(fname);
    match fpath.metadata() {
        Ok(m) if m.len() == 256 => {}
        _ => build_test_file!(&fpath, &vec![0; 256]),
    }

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "if=null.txt", &of!(fname)])
        .succeeds()
        .no_output();

    assert_eq!(0, fix.metadata(fname).len());
}

#[test]
fn test_null_stats() {
    new_ucmd!()
        .arg("if=null.txt")
        .succeeds()
        .stderr_contains("0+0 records in\n0+0 records out\n0 bytes copied, ")
        .stderr_matches(&Regex::new(r"\d(\.\d+)?(e-\d\d)? s, ").unwrap())
        .stderr_contains("0.0 B/s");
}

#[test]
fn test_null_fullblock() {
    new_ucmd!()
        .args(&["if=null.txt", "status=none", "iflag=fullblock"])
        .succeeds()
        .no_output();
}

#[cfg(unix)]
#[ignore = "See note below before using this test."]
#[test]
fn test_fullblock() {
    let tname = "fullblock-from-urand";
    let tmp_fn = format!("TESTFILE-{tname}.tmp");
    let exp_stats = vec![
        "1+0 records in\n",
        "1+0 records out\n",
        "134217728 bytes (134 MB, 128 MiB) copied,",
    ];
    let exp_stats = exp_stats.into_iter().fold(Vec::new(), |mut acc, s| {
        acc.extend(s.bytes());
        acc
    });

    let ucmd = new_ucmd!()
        .args(&[
            "if=/dev/urandom",
            &of!(&tmp_fn),
            "bs=128M",
            // Note: In order for this test to actually test iflag=fullblock, the bs=VALUE
            // must be big enough to 'overwhelm' the urandom store of bytes.
            // Try executing 'dd if=/dev/urandom bs=128M count=1' (i.e without iflag=fullblock).
            // The stats should contain the line: '0+1 records in' indicating a partial read.
            // Since my system only copies 32 MiB without fullblock, I expect 128 MiB to be
            // a reasonable value for testing most systems.
            "count=1",
            "iflag=fullblock",
        ])
        .succeeds();

    let run_stats = &ucmd.stderr()[..exp_stats.len()];
    assert_eq!(exp_stats, run_stats);
}

// Fileio
#[test]
fn test_ys_to_stdout() {
    let output: Vec<_> = String::from("y\n").bytes().cycle().take(1024).collect();
    let output = String::from_utf8(output).unwrap();

    new_ucmd!()
        .args(&["status=none", "if=y-nl-1k.txt"])
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_zeros_to_stdout() {
    let output = vec![0; 256 * 1024];
    let output = String::from_utf8(output).unwrap();
    new_ucmd!()
        .args(&["status=none", "if=zero-256k.txt"])
        .succeeds()
        .stdout_only(output);
}

#[cfg(target_pointer_width = "32")]
#[test]
fn test_oversized_bs_32_bit() {
    for bs_param in ["bs", "ibs", "obs", "cbs"] {
        new_ucmd!()
            .args(&[format!("{bs_param}=5GB")])
            .fails()
            .no_stdout()
            .code_is(1)
            .stderr_is(format!("dd: {bs_param}=N cannot fit into memory\n"));
    }
}

#[test]
fn test_to_stdout_with_ibs_obs() {
    let output: Vec<_> = String::from("y\n").bytes().cycle().take(1024).collect();
    let output = String::from_utf8(output).unwrap();

    new_ucmd!()
        .args(&["status=none", "if=y-nl-1k.txt", "ibs=521", "obs=1031"])
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_ascii_10k_to_stdout() {
    let output = build_ascii_block(1024 * 1024);
    // build_test_file!("ascii-10k.txt", &output);
    let output = String::from_utf8(output).unwrap();

    new_ucmd!()
        .args(&["status=none", "if=ascii-10k.txt"])
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_zeros_to_file() {
    let tname = "zero-256k";
    let test_fn = format!("{tname}.txt");
    let tmp_fn = format!("TESTFILE-{tname}.tmp");
    assert_fixture_exists!(test_fn);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", &inf!(test_fn), &of!(tmp_fn)])
        .succeeds()
        .no_output();

    cmp_file!(
        File::open(fixture_path!(&test_fn)).unwrap(),
        fix.open(&tmp_fn)
    );
}

#[test]
fn test_to_file_with_ibs_obs() {
    let tname = "zero-256k";
    let test_fn = format!("{tname}.txt");
    let tmp_fn = format!("TESTFILE-{tname}.tmp");
    assert_fixture_exists!(test_fn);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[
        "status=none",
        &inf!(test_fn),
        &of!(tmp_fn),
        "ibs=222",
        "obs=111",
    ])
    .succeeds()
    .no_output();

    cmp_file!(
        File::open(fixture_path!(&test_fn)).unwrap(),
        fix.open(&tmp_fn)
    );
}

#[test]
fn test_ascii_521k_to_file() {
    let tname = "ascii-521k";
    let input = build_ascii_block(512 * 1024);
    let tmp_fn = format!("TESTFILE-{tname}.tmp");

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", &of!(tmp_fn)])
        .pipe_in(input.clone())
        .succeeds()
        .no_output();

    assert_eq!(512 * 1024, fix.metadata(&tmp_fn).len());

    cmp_file!(
        {
            let mut input_f = tempfile().unwrap();
            input_f.write_all(&input).unwrap();
            input_f
        },
        fix.open(&tmp_fn)
    );
}

#[ignore = ""]
#[cfg(unix)]
#[test]
fn test_ascii_5_gibi_to_file() {
    let tname = "ascii-5G";
    let tmp_fn = format!("TESTFILE-{tname}.tmp");

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[
        "status=none",
        "count=5G",
        "iflag=count_bytes",
        "if=/dev/zero",
        &of!(tmp_fn),
    ])
    .succeeds()
    .no_output();

    assert_eq!(5 * 1024 * 1024 * 1024, fix.metadata(&tmp_fn).len());
}

#[test]
fn test_self_transfer() {
    let fname = "self-transfer-256k.txt";
    assert_fixture_exists!(fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "conv=notrunc", &inf!(fname), &of!(fname)]);

    assert!(fix.file_exists(fname));
    assert_eq!(256 * 1024, fix.metadata(fname).len());

    ucmd.succeeds().no_output();

    assert!(fix.file_exists(fname));
    assert_eq!(256 * 1024, fix.metadata(fname).len());
}

#[test]
fn test_unicode_filenames() {
    let tname = "😎💚🦊";
    let test_fn = format!("{tname}.txt");
    let tmp_fn = format!("TESTFILE-{tname}.tmp");
    assert_fixture_exists!(test_fn);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", &inf!(test_fn), &of!(tmp_fn)])
        .succeeds()
        .no_output();

    cmp_file!(
        File::open(fixture_path!(&test_fn)).unwrap(),
        fix.open(&tmp_fn)
    );
}

#[test]
fn test_conv_ascii_implies_unblock() {
    // 0x40 = 0o100 =  64, which gets converted to ' '
    // 0xc1 = 0o301 = 193, which gets converted to 'A'
    //
    // `conv=ascii` implies `conv=unblock`, which means trailing paces
    // are stripped and a newline is appended at the end of each
    // block.
    //
    // `cbs=4` means use a conversion block size of 4 bytes per block.
    new_ucmd!()
        .args(&["conv=ascii", "cbs=4"])
        .pipe_in(b"\x40\xc1\x40\xc1\x40\xc1\x40\x40".to_vec())
        .succeeds()
        .stdout_is(" A A\n A\n");
}

#[test]
fn test_conv_ebcdic_implies_block() {
    // 0x40 = 0o100 =  64, which is the result of converting from ' '
    // 0xc1 = 0o301 = 193, which is the result of converting from 'A'
    //
    // `conv=ebcdic` implies `conv=block`, which means trailing spaces
    // are added to pad each block.
    //
    // `cbs=4` means use a conversion block size of 4 bytes per block.
    new_ucmd!()
        .args(&["conv=ebcdic", "cbs=4"])
        .pipe_in(" A A\n A\n")
        .succeeds()
        .stdout_is_bytes(b"\x40\xc1\x40\xc1\x40\xc1\x40\x40");
}

/// Test for seeking forward N bytes in the output file before copying.
#[test]
fn test_seek_bytes() {
    // Since the output file is stdout, seeking forward by eight bytes
    // results in a prefix of eight null bytes.
    new_ucmd!()
        .args(&["seek=8", "oflag=seek_bytes"])
        .pipe_in("abcdefghijklm\n")
        .succeeds()
        .stdout_is("\0\0\0\0\0\0\0\0abcdefghijklm\n");
}

/// Test for skipping beyond the number of bytes in a file.
#[test]
fn test_skip_beyond_file() {
    new_ucmd!()
        .args(&["bs=1", "skip=5", "count=0", "status=noxfer"])
        .pipe_in("abcd")
        .succeeds()
        .no_stdout()
        .stderr_contains(
            "'standard input': cannot skip to specified offset\n0+0 records in\n0+0 records out\n",
        );
}

#[test]
fn test_seek_do_not_overwrite() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut outfile = at.make_file("outfile");
    outfile.write_all(b"abc").unwrap();
    // Skip the first byte of the input, seek past the first byte of
    // the output, and write only one byte to the output.
    ucmd.args(&[
        "bs=1",
        "skip=1",
        "seek=1",
        "count=1",
        "status=noxfer",
        "of=outfile",
    ])
    .pipe_in("123")
    .succeeds()
    .stderr_is("1+0 records in\n1+0 records out\n")
    .no_stdout();
    assert_eq!(at.read("outfile"), "a2");
}

#[test]
fn test_partial_records_out() {
    new_ucmd!()
        .args(&["bs=2", "status=noxfer"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("abc")
        .stderr_is("1+1 records in\n1+1 records out\n");
}

#[test]
fn test_block_cbs16() {
    new_ucmd!()
        .args(&["conv=block", "cbs=16"])
        .pipe_in_fixture("dd-block-cbs16.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-block-cbs16.spec");
}

#[test]
fn test_block_cbs16_as_cbs8() {
    new_ucmd!()
        .args(&["conv=block", "cbs=8"])
        .pipe_in_fixture("dd-block-cbs16.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-block-cbs8.spec");
}

#[test]
fn test_block_consecutive_nl() {
    new_ucmd!()
        .args(&["conv=block", "cbs=16"])
        .pipe_in_fixture("dd-block-consecutive-nl.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-block-consecutive-nl-cbs16.spec");
}

#[test]
fn test_unblock_multi_16() {
    new_ucmd!()
        .args(&["conv=unblock", "cbs=16"])
        .pipe_in_fixture("dd-unblock-cbs16.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-unblock-cbs16.spec");
}

#[test]
fn test_unblock_multi_16_as_8() {
    new_ucmd!()
        .args(&["conv=unblock", "cbs=8"])
        .pipe_in_fixture("dd-unblock-cbs16.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-unblock-cbs8.spec");
}

#[test]
fn test_atoe_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ebcdic"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-atoe-seq-byte-values.spec");
}

#[test]
fn test_etoa_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ascii"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-etoa-seq-byte-values.spec");
}

#[test]
fn test_atoibm_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ibm"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-atoibm-seq-byte-values.spec");
}

#[test]
fn test_lcase_ascii_to_ucase_ascii() {
    new_ucmd!()
        .args(&["conv=ucase"])
        .pipe_in_fixture("lcase-ascii.test")
        .succeeds()
        .stdout_is_fixture_bytes("ucase-ascii.test");
}

#[test]
fn test_ucase_ascii_to_lcase_ascii() {
    new_ucmd!()
        .args(&["conv=lcase"])
        .pipe_in_fixture("ucase-ascii.test")
        .succeeds()
        .stdout_is_fixture_bytes("lcase-ascii.test");
}

#[test]
fn test_atoe_and_ucase_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ebcdic,ucase"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("ucase-ebcdic.test");
}

#[test]
fn test_atoe_and_lcase_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ebcdic,lcase"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("lcase-ebcdic.test");
}

// TODO I think uppercase and lowercase are unintentionally swapped in
// the code that parses the command-line arguments. See this line from
// `parseargs.rs`:
//
//     (ConvFlag::FmtAtoI, ConvFlag::UCase) => Some(&ASCII_TO_IBM_UCASE_TO_LCASE),
//     (ConvFlag::FmtAtoI, ConvFlag::LCase) => Some(&ASCII_TO_IBM_LCASE_TO_UCASE),
//
// If my reading is correct and that is a typo, then the
// UCASE_TO_LCASE and LCASE_TO_UCASE in those lines should be swapped,
// and the expected output for the following two tests should be
// updated accordingly.
#[test]
fn test_atoibm_and_ucase_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ibm,ucase"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("lcase-ibm.test");
}

#[test]
fn test_atoibm_and_lcase_conv_spec_test() {
    new_ucmd!()
        .args(&["conv=ibm,lcase"])
        .pipe_in_fixture("seq-byte-values-b632a992d3aed5d8d1a59cc5a5a455ba.test")
        .succeeds()
        .stdout_is_fixture_bytes("ucase-ibm.test");
}

#[test]
fn test_swab_256_test() {
    new_ucmd!()
        .args(&["conv=swab"])
        .pipe_in_fixture("seq-byte-values.test")
        .succeeds()
        .stdout_is_fixture_bytes("seq-byte-values-swapped.test");
}

#[test]
fn test_swab_257_test() {
    new_ucmd!()
        .args(&["conv=swab"])
        .pipe_in_fixture("seq-byte-values-odd.test")
        .succeeds()
        .stdout_is_fixture_bytes("seq-byte-values-odd.spec");
}

#[test]
fn test_block_lower() {
    new_ucmd!()
        .args(&["conv=block,lcase", "cbs=8"])
        .pipe_in_fixture("dd-block8-lowercase.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-block8-lowercase.spec");
}

#[test]
fn test_lower_block() {
    new_ucmd!()
        .args(&["conv=lcase,block", "cbs=8"])
        .pipe_in_fixture("dd-block8-lowercase.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-block8-lowercase.spec");
}

#[test]
fn test_unblock_lower() {
    new_ucmd!()
        .args(&["conv=unblock,lcase", "cbs=8"])
        .pipe_in_fixture("dd-unblock8-lowercase.test")
        .succeeds()
        .stdout_is_fixture_bytes("dd-unblock8-lowercase.spec");
}

#[test]
fn test_zeros_4k_conv_sync_obs_gt_ibs() {
    new_ucmd!()
        .args(&["conv=sync", "ibs=521", "obs=1031"])
        .pipe_in_fixture("zeros-620f0b67a91f7f74151bc5be745b7110.test")
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-sync-ibs-521-obs-1031-zeros.spec");
}

#[test]
fn test_zeros_4k_conv_sync_ibs_gt_obs() {
    new_ucmd!()
        .args(&["conv=sync", "ibs=1031", "obs=521"])
        .pipe_in_fixture("zeros-620f0b67a91f7f74151bc5be745b7110.test")
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-sync-ibs-1031-obs-521-zeros.spec");
}

#[test]
fn test_deadbeef_32k_conv_sync_obs_gt_ibs() {
    new_ucmd!()
        .args(&[
            "conv=sync",
            "ibs=521",
            "obs=1031",
            "if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-sync-ibs-521-obs-1031-deadbeef.spec");
}

#[test]
fn test_deadbeef_32k_conv_sync_ibs_gt_obs() {
    new_ucmd!()
        .args(&[
            "conv=sync",
            "ibs=1031",
            "obs=521",
            "if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-sync-ibs-1031-obs-521-deadbeef.spec");
}

#[test]
fn test_random_73k_test_bs_prime_obs_gt_ibs_sync() {
    new_ucmd!()
        .args(&[
            "conv=sync",
            "ibs=521",
            "obs=1031",
            "if=random-5828891cb1230748e146f34223bbd3b5.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-sync-ibs-521-obs-1031-random.spec");
}

#[test]
fn test_random_73k_test_bs_prime_ibs_gt_obs_sync() {
    new_ucmd!()
        .args(&[
            "conv=sync",
            "ibs=1031",
            "obs=521",
            "if=random-5828891cb1230748e146f34223bbd3b5.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-conv-sync-ibs-1031-obs-521-random.spec");
}

#[test]
fn test_identity() {
    new_ucmd!()
        .args(&["if=zeros-620f0b67a91f7f74151bc5be745b7110.test"])
        .succeeds()
        .stdout_is_fixture_bytes("zeros-620f0b67a91f7f74151bc5be745b7110.test");
    new_ucmd!()
        .args(&["if=ones-6ae59e64850377ee5470c854761551ea.test"])
        .succeeds()
        .stdout_is_fixture_bytes("ones-6ae59e64850377ee5470c854761551ea.test");
    new_ucmd!()
        .args(&["if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test"])
        .succeeds()
        .stdout_is_fixture_bytes("deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test");
    new_ucmd!()
        .args(&["if=random-5828891cb1230748e146f34223bbd3b5.test"])
        .succeeds()
        .stdout_is_fixture_bytes("random-5828891cb1230748e146f34223bbd3b5.test");
}

#[test]
fn test_random_73k_test_not_a_multiple_obs_gt_ibs() {
    new_ucmd!()
        .args(&[
            "ibs=521",
            "obs=1031",
            "if=random-5828891cb1230748e146f34223bbd3b5.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("random-5828891cb1230748e146f34223bbd3b5.test");
}

#[test]
fn test_random_73k_test_obs_lt_not_a_multiple_ibs() {
    new_ucmd!()
        .args(&[
            "ibs=1031",
            "obs=521",
            "if=random-5828891cb1230748e146f34223bbd3b5.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("random-5828891cb1230748e146f34223bbd3b5.test");
}

#[cfg(not(windows))]
#[test]
fn test_random_73k_test_lazy_fullblock() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("fifo");
    let child = ucmd
        .args(&[
            "ibs=521",
            "obs=1031",
            "iflag=fullblock",
            "if=fifo",
            "status=noxfer",
        ])
        .run_no_wait();
    let data = at.read_bytes("random-5828891cb1230748e146f34223bbd3b5.test");
    {
        let mut fifo = OpenOptions::new()
            .write(true)
            .open(at.plus("fifo"))
            .unwrap();
        for chunk in data.chunks(521 / 2) {
            fifo.write_all(chunk).unwrap();
            sleep(Duration::from_millis(10));
        }
    }
    child
        .wait()
        .unwrap()
        .success()
        .stdout_is_bytes(&data)
        .stderr_is("142+1 records in\n72+1 records out\n");
}

#[test]
fn test_deadbeef_all_32k_test_count_reads() {
    new_ucmd!()
        .args(&[
            "bs=1024",
            "count=32",
            "if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test");
}

#[test]
fn test_deadbeef_all_32k_test_count_bytes() {
    new_ucmd!()
        .args(&[
            "ibs=531",
            "obs=1031",
            "count=32x1024",
            "oflag=count_bytes",
            "if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test");
}

#[test]
fn test_deadbeef_32k_to_16k_test_count_reads() {
    new_ucmd!()
        .args(&[
            "ibs=1024",
            "obs=1031",
            "count=16",
            "if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-deadbeef-first-16k.spec");
}

#[test]
fn test_deadbeef_32k_to_12345_test_count_bytes() {
    new_ucmd!()
        .args(&[
            "ibs=531",
            "obs=1031",
            "count=12345",
            "iflag=count_bytes",
            "if=deadbeef-18d99661a1de1fc9af21b0ec2cd67ba3.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-deadbeef-first-12345.spec");
}

#[test]
fn test_random_73k_test_count_reads() {
    new_ucmd!()
        .args(&[
            "bs=1024",
            "count=32",
            "if=random-5828891cb1230748e146f34223bbd3b5.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-random-first-32k.spec");
}

#[test]
fn test_random_73k_test_count_bytes() {
    new_ucmd!()
        .args(&[
            "ibs=521",
            "obs=1031",
            "count=32x1024",
            "iflag=count_bytes",
            "if=random-5828891cb1230748e146f34223bbd3b5.test",
        ])
        .succeeds()
        .stdout_is_fixture_bytes("gnudd-random-first-32k.spec");
}

#[test]
fn test_all_valid_ascii_ebcdic_ascii_roundtrip_conv_test() {
    let tmp = new_ucmd!()
        .args(&["ibs=128", "obs=1024", "conv=ebcdic"])
        .pipe_in_fixture("all-valid-ascii-chars-37eff01866ba3f538421b30b7cbefcac.test")
        .succeeds()
        .stdout_move_bytes();
    new_ucmd!()
        .args(&["ibs=256", "obs=1024", "conv=ascii"])
        .pipe_in(tmp)
        .succeeds()
        .stdout_is_fixture_bytes("all-valid-ascii-chars-37eff01866ba3f538421b30b7cbefcac.test");
}

#[test]
fn test_skip_zero() {
    new_ucmd!()
        .args(&["skip=0", "status=noxfer"])
        .succeeds()
        .no_stdout()
        .stderr_is("0+0 records in\n0+0 records out\n");
}

#[test]
fn test_truncated_record() {
    new_ucmd!()
        .args(&["cbs=1", "conv=block", "status=noxfer"])
        .pipe_in("ab")
        .succeeds()
        .stdout_is("a")
        .stderr_is("0+1 records in\n0+1 records out\n1 truncated record\n");
    new_ucmd!()
        .args(&["cbs=1", "conv=block", "status=noxfer"])
        .pipe_in("ab\ncd\n")
        .succeeds()
        .stdout_is("ac")
        .stderr_is("0+1 records in\n0+1 records out\n2 truncated records\n");
}

/// Test that the output file can be `/dev/null`.
#[cfg(unix)]
#[test]
fn test_outfile_dev_null() {
    new_ucmd!().arg("of=/dev/null").succeeds().no_stdout();
}

#[test]
fn test_block_sync() {
    new_ucmd!()
        .args(&["ibs=5", "cbs=5", "conv=block,sync", "status=noxfer"])
        .pipe_in("012\nabcde\n")
        .succeeds()
        // blocks:    1    2
        .stdout_is("012  abcde")
        .stderr_is("2+0 records in\n0+1 records out\n");

    // It seems that a partial record in is represented as an
    // all-spaces block at the end of the output. The "1 truncated
    // record" line is present in the status report due to the line
    // "abcdefg\n" being truncated to "abcde".
    new_ucmd!()
        .args(&["ibs=5", "cbs=5", "conv=block,sync", "status=noxfer"])
        .pipe_in("012\nabcdefg\n")
        .succeeds()
        // blocks:    1    2    3
        .stdout_is("012  abcde     ")
        .stderr_is("2+1 records in\n0+1 records out\n1 truncated record\n");
}

#[test]
fn test_bytes_iseek_bytes_iflag() {
    new_ucmd!()
        .args(&["iseek=10", "iflag=skip_bytes", "bs=2"])
        .pipe_in("0123456789abcdefghijklm")
        .succeeds()
        .stdout_is("abcdefghijklm");
}

#[test]
fn test_bytes_iseek_skip_not_additive() {
    new_ucmd!()
        .args(&["iseek=4", "skip=4", "iflag=skip_bytes", "bs=2"])
        .pipe_in("0123456789abcdefghijklm")
        .succeeds()
        .stdout_is("456789abcdefghijklm");
}

#[test]
fn test_bytes_oseek_bytes_oflag() {
    new_ucmd!()
        .args(&["oseek=8", "oflag=seek_bytes", "bs=2"])
        .pipe_in("abcdefghijklm")
        .succeeds()
        .stdout_is_fixture_bytes("dd-bytes-alphabet-null.spec");
}

#[test]
fn test_bytes_oseek_bytes_trunc_oflag() {
    new_ucmd!()
        .args(&["oseek=8", "oflag=seek_bytes", "bs=2", "count=0"])
        .pipe_in("abcdefghijklm")
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_is_fixture_bytes("dd-bytes-null-trunc.spec");
}

#[test]
fn test_bytes_oseek_seek_not_additive() {
    new_ucmd!()
        .args(&["oseek=8", "seek=8", "oflag=seek_bytes", "bs=2"])
        .pipe_in("abcdefghijklm")
        .succeeds()
        .stdout_is_fixture_bytes("dd-bytes-alphabet-null.spec");
}

#[test]
fn test_final_stats_less_than_one_kb_si() {
    let result = new_ucmd!().pipe_in("0".repeat(999)).succeeds();
    let s = result.stderr_str();
    assert!(s.starts_with("1+1 records in\n1+1 records out\n999 bytes copied,"));
}

#[test]
fn test_final_stats_less_than_one_kb_iec() {
    let result = new_ucmd!().pipe_in("0".repeat(1000)).succeeds();
    let s = result.stderr_str();
    assert!(s.starts_with("1+1 records in\n1+1 records out\n1000 bytes (1.0 kB) copied,"));

    let result = new_ucmd!().pipe_in("0".repeat(1023)).succeeds();
    let s = result.stderr_str();
    assert!(s.starts_with("1+1 records in\n1+1 records out\n1023 bytes (1.0 kB) copied,"));
}

#[test]
fn test_final_stats_more_than_one_kb() {
    let result = new_ucmd!().pipe_in("0".repeat(1024)).succeeds();
    let s = result.stderr_str();
    assert!(s.starts_with("2+0 records in\n2+0 records out\n1024 bytes (1.0 kB, 1.0 KiB) copied,"));
}

#[test]
fn test_final_stats_three_char_limit() {
    let result = new_ucmd!().pipe_in("0".repeat(10_000)).succeeds();
    let s = result.stderr_str();
    assert!(
        s.starts_with("19+1 records in\n19+1 records out\n10000 bytes (10 kB, 9.8 KiB) copied,")
    );

    let result = new_ucmd!().pipe_in("0".repeat(100_000)).succeeds();
    let s = result.stderr_str();
    assert!(
        s.starts_with("195+1 records in\n195+1 records out\n100000 bytes (100 kB, 98 KiB) copied,")
    );
}

#[test]
fn test_invalid_number_arg_gnu_compatibility() {
    let commands = vec!["bs", "cbs", "count", "ibs", "obs", "seek", "skip"];

    for command in commands {
        new_ucmd!()
            .args(&[format!("{command}=")])
            .fails()
            .stderr_is("dd: invalid number: ‘’\n");

        new_ucmd!()
            .args(&[format!("{command}=29d")])
            .fails()
            .stderr_is("dd: invalid number: ‘29d’\n");
    }
}

#[test]
fn test_invalid_flag_arg_gnu_compatibility() {
    let commands = vec!["iflag", "oflag"];

    for command in commands {
        new_ucmd!()
            .args(&[format!("{command}=")])
            .fails()
            .usage_error("invalid input flag: ‘’");

        new_ucmd!()
            .args(&[format!("{command}=29d")])
            .fails()
            .usage_error("invalid input flag: ‘29d’");
    }
}

#[test]
fn test_invalid_file_arg_gnu_compatibility() {
    new_ucmd!()
        .args(&["if="])
        .fails()
        .stderr_is("dd: failed to open '': No such file or directory\n");

    new_ucmd!()
        .args(&["if=81as9bn8as9g302az8ns9.pdf.zip.pl.com"])
        .fails()
        .stderr_is(
            "dd: failed to open '81as9bn8as9g302az8ns9.pdf.zip.pl.com': No such file or directory\n",
        );

    new_ucmd!()
        .args(&["of="])
        .fails()
        .stderr_is("dd: failed to open '': No such file or directory\n");

    new_ucmd!()
        .args(&["of=81as9bn8as9g302az8ns9.pdf.zip.pl.com"])
        .pipe_in("")
        .succeeds();
}

#[test]
fn test_ucase_lcase() {
    new_ucmd!()
        .arg("conv=ucase,lcase")
        .fails()
        .stderr_contains("lcase")
        .stderr_contains("ucase");
}

#[test]
fn test_big_multiplication() {
    new_ucmd!()
        .arg("ibs=10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10x10")
        .fails()
        .stderr_contains("invalid number");
}

/// Test for count, seek, and skip given in units of bytes.
#[test]
fn test_bytes_suffix() {
    new_ucmd!()
        .args(&["count=3B", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("abc");
    new_ucmd!()
        .args(&["skip=3B", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("def");
    new_ucmd!()
        .args(&["iseek=3B", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("def");
    new_ucmd!()
        .args(&["seek=3B", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("\0\0\0abcdef");
    new_ucmd!()
        .args(&["oseek=3B", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("\0\0\0abcdef");
}

#[test]
// the recursive nature of the suffix allows any string with a 'B' in it treated as bytes.
fn test_bytes_suffix_recursive() {
    new_ucmd!()
        .args(&["count=2Bx2", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("abcd");
    new_ucmd!()
        .args(&["skip=2Bx2", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("ef");
    new_ucmd!()
        .args(&["iseek=2Bx2", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("ef");
    new_ucmd!()
        .args(&["seek=2Bx2", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("\0\0\0\0abcdef");
    new_ucmd!()
        .args(&["oseek=2Bx2", "status=none"])
        .pipe_in("abcdef")
        .succeeds()
        .stdout_only("\0\0\0\0abcdef");
}

/// Test for "conv=sync" with a slow reader.
#[cfg(not(windows))]
#[test]
fn test_sync_delayed_reader() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("fifo");
    let child = ucmd
        .args(&["ibs=16", "obs=32", "conv=sync", "if=fifo", "status=noxfer"])
        .run_no_wait();
    {
        let mut fifo = OpenOptions::new()
            .write(true)
            .open(at.plus("fifo"))
            .unwrap();
        for _ in 0..8 {
            fifo.write_all(&[0xF; 8]).unwrap();
            sleep(Duration::from_millis(10));
        }
    }
    // Expected output is 0xFFFFFFFF00000000FFFFFFFF00000000...
    let mut expected: [u8; 8 * 16] = [0; 8 * 16];
    for i in 0..8 {
        for j in 0..8 {
            expected[16 * i + j] = 0xF;
        }
    }

    child
        .wait()
        .unwrap()
        .success()
        .stdout_is_bytes(expected)
        .stderr_is("0+8 records in\n4+0 records out\n");
}

/// Test for making a sparse copy of the input file.
#[test]
fn test_sparse() {
    let (at, mut ucmd) = at_and_ucmd!();

    // Create a file and make it a large sparse file.
    //
    // On common Linux filesystems, setting the length to one megabyte
    // should cause the file to become a sparse file, but it depends
    // on the system.
    std::fs::File::create(at.plus("infile"))
        .unwrap()
        .set_len(1024 * 1024)
        .unwrap();

    // Perform a sparse copy.
    ucmd.args(&["bs=32K", "if=infile", "of=outfile", "conv=sparse"])
        .succeeds();

    // The number of bytes in the file should be accurate though the
    // number of blocks stored on disk may be zero.
    assert_eq!(at.metadata("infile").len(), at.metadata("outfile").len());
}

/// Test that a seek on an output FIFO results in a read.
#[test]
#[cfg(unix)]
fn test_seek_output_fifo() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("fifo");

    let mut ucmd = ts.ucmd();
    let child = ucmd
        .args(&["count=0", "seek=1", "of=fifo", "status=noxfer"])
        .run_no_wait();

    std::fs::write(at.plus("fifo"), vec![0; 512]).unwrap();

    child
        .wait()
        .unwrap()
        .success()
        .stderr_only("0+0 records in\n0+0 records out\n");
}

/// Test that a skip on an input FIFO results in a read.
#[test]
#[cfg(unix)]
fn test_skip_input_fifo() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("fifo");

    let mut ucmd = ts.ucmd();
    let child = ucmd
        .args(&["count=0", "skip=1", "if=fifo", "status=noxfer"])
        .run_no_wait();

    std::fs::write(at.plus("fifo"), vec![0; 512]).unwrap();

    child
        .wait()
        .unwrap()
        .success()
        .stderr_only("0+0 records in\n0+0 records out\n");
}

/// Test for reading part of stdin from each of two child processes.
#[cfg(all(not(windows), feature = "printf"))]
#[test]
fn test_multiple_processes_reading_stdin() {
    // TODO Investigate if this is possible on Windows.
    let printf = format!("{} printf 'abcdef\n'", get_tests_binary());
    let dd_skip = format!("{} dd bs=1 skip=3 count=0", get_tests_binary());
    let dd = format!("{} dd", get_tests_binary());
    UCommand::new()
        .arg(format!("{printf} | ( {dd_skip} && {dd} ) 2> /dev/null"))
        .succeeds()
        .stdout_only("def\n");
}

/// Test that discarding system file cache fails for stdin.
#[test]
#[cfg(target_os = "linux")]
fn test_nocache_stdin_error() {
    #[cfg(not(target_env = "musl"))]
    let detail = "Illegal seek";
    #[cfg(target_env = "musl")]
    let detail = "Invalid seek";
    new_ucmd!()
        .args(&["iflag=nocache", "count=0", "status=noxfer"])
        .fails_with_code(1)
        .stderr_only(format!("dd: failed to discard cache for: 'standard input': {detail}\n0+0 records in\n0+0 records out\n"));
}

/// Test that dd fails when no number in count.
#[test]
fn test_empty_count_number() {
    new_ucmd!()
        .args(&["count=B"])
        .fails_with_code(1)
        .stderr_only("dd: invalid number: ‘B’\n");
}

/// Test for discarding system file cache.
#[test]
#[cfg(target_os = "linux")]
fn test_nocache_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write_bytes("f", b"a".repeat(1 << 20).as_slice());
    ucmd.args(&["if=f", "of=/dev/null", "iflag=nocache", "status=noxfer"])
        .succeeds()
        .stderr_only("2048+0 records in\n2048+0 records out\n");
}

#[test]
#[cfg(unix)]
#[cfg(not(feature = "feat_selinux"))]
// Disabled on SELinux for now
fn test_skip_past_dev() {
    // NOTE: This test intends to trigger code which can only be reached with root permissions.
    let ts = TestScenario::new(util_name!());

    if !ts.fixtures.file_exists("/dev/sda1") {
        print!("Test skipped; no /dev/sda1 device found");
    } else if let Ok(result) = run_ucmd_as_root_with_stdin_stdout(
        &ts,
        &["bs=1", "skip=10000000000000000", "count=0", "status=noxfer"],
        Some("/dev/sda1"),
        None,
    ) {
        result.stderr_contains("dd: 'standard input': cannot skip: Invalid argument");
        result.stderr_contains("0+0 records in");
        result.stderr_contains("0+0 records out");
        result.code_is(1);
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
#[cfg(unix)]
#[cfg(not(feature = "feat_selinux"))]
fn test_seek_past_dev() {
    // NOTE: This test intends to trigger code which can only be reached with root permissions.
    let ts = TestScenario::new(util_name!());

    if !ts.fixtures.file_exists("/dev/sda1") {
        print!("Test skipped; no /dev/sda1 device found");
    } else if let Ok(result) = run_ucmd_as_root_with_stdin_stdout(
        &ts,
        &["bs=1", "seek=10000000000000000", "count=0", "status=noxfer"],
        None,
        Some("/dev/sda1"),
    ) {
        result.stderr_contains("dd: 'standard output': cannot seek: Invalid argument");
        result.stderr_contains("0+0 records in");
        result.stderr_contains("0+0 records out");
        result.code_is(1);
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
#[cfg(all(
    unix,
    not(target_os = "macos"),
    not(target_os = "freebsd"),
    feature = "printf"
))]
fn test_reading_partial_blocks_from_fifo() {
    // Create the FIFO.
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("fifo");
    let fifoname = at.plus_as_string("fifo");

    // Start a `dd` process that reads from the fifo (so it will wait
    // until the writer process starts).
    let mut reader_command = Command::new(get_tests_binary());
    let child = reader_command
        .args(["dd", "ibs=3", "obs=3", &format!("if={fifoname}")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("LC_ALL", "C")
        .spawn()
        .unwrap();

    // Start different processes to write to the FIFO, with a small
    // pause in between.
    let mut writer_command = Command::new("sh");
    let _ = writer_command
        .args([
            "-c",
            &format!("(printf \"ab\"; sleep 0.1; printf \"cd\") > {fifoname}"),
        ])
        .spawn()
        .unwrap()
        .wait();

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.stdout, b"abcd");
    let expected = b"0+2 records in\n1+1 records out\n4 bytes copied";
    assert!(output.stderr.starts_with(expected));
}

#[test]
#[cfg(all(
    unix,
    not(target_os = "macos"),
    not(target_os = "freebsd"),
    feature = "printf"
))]
fn test_reading_partial_blocks_from_fifo_unbuffered() {
    // Create the FIFO.
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("fifo");
    let fifoname = at.plus_as_string("fifo");

    // Start a `dd` process that reads from the fifo (so it will wait
    // until the writer process starts).
    //
    // `bs=N` takes precedence over `ibs=N` and `obs=N`.
    let mut reader_command = Command::new(get_tests_binary());
    let child = reader_command
        .args(["dd", "bs=3", "ibs=1", "obs=1", &format!("if={fifoname}")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("LC_ALL", "C")
        .spawn()
        .unwrap();

    // Start different processes to write to the FIFO, with a small
    // pause in between.
    let mut writer_command = Command::new("sh");
    let _ = writer_command
        .args([
            "-c",
            &format!("(printf \"ab\"; sleep 0.1; printf \"cd\") > {fifoname}"),
        ])
        .spawn()
        .unwrap()
        .wait();

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.stdout, b"abcd");
    let expected = b"0+2 records in\n0+2 records out\n4 bytes copied";
    assert!(output.stderr.starts_with(expected));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_iflag_directory_fails_when_file_is_passed_via_std_in() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.make_file("input");
    let filename = at.plus_as_string("input");
    new_ucmd!()
        .args(&["iflag=directory", "count=0"])
        .set_stdin(std::process::Stdio::from(File::open(filename).unwrap()))
        .fails()
        .stderr_only("dd: setting flags for 'standard input': Not a directory\n");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_iflag_directory_passes_when_dir_is_redirected() {
    new_ucmd!()
        .args(&["iflag=directory", "count=0"])
        .set_stdin(std::process::Stdio::from(File::open(".").unwrap()))
        .succeeds();
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_iflag_directory_fails_when_file_is_piped_via_std_in() {
    new_ucmd!()
        .arg("iflag=directory")
        .pipe_in("")
        .fails()
        .stderr_only("dd: setting flags for 'standard input': Not a directory\n");
}

#[test]
fn test_stdin_stdout_not_rewound_even_when_connected_to_seekable_file() {
    use std::process::Stdio;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write("in", "abcde");

    let stdin = OwnedFileDescriptorOrHandle::open_file(
        OpenOptions::new().read(true),
        at.plus("in").as_path(),
    )
    .unwrap();
    let stdout = OwnedFileDescriptorOrHandle::open_file(
        OpenOptions::new().create(true).write(true),
        at.plus("out").as_path(),
    )
    .unwrap();
    let stderr = OwnedFileDescriptorOrHandle::open_file(
        OpenOptions::new().create(true).write(true),
        at.plus("err").as_path(),
    )
    .unwrap();

    ts.ucmd()
        .args(&["bs=1", "skip=1", "count=1"])
        .set_stdin(Stdio::from(stdin.try_clone().unwrap()))
        .set_stdout(Stdio::from(stdout.try_clone().unwrap()))
        .set_stderr(Stdio::from(stderr.try_clone().unwrap()))
        .succeeds();

    ts.ucmd()
        .args(&["bs=1", "skip=1"])
        .set_stdin(stdin)
        .set_stdout(stdout)
        .set_stderr(stderr)
        .succeeds();

    let err_file_content = std::fs::read_to_string(at.plus_as_string("err")).unwrap();
    println!("stderr:\n{err_file_content}");

    let out_file_content = std::fs::read_to_string(at.plus_as_string("out")).unwrap();
    println!("stdout:\n{out_file_content}");
    assert_eq!(out_file_content, "bde");
}

#[test]
fn test_wrong_number_err_msg() {
    new_ucmd!()
        .args(&["count=kBb"])
        .fails()
        .stderr_contains("dd: invalid number: 'kBb'\n");

    new_ucmd!()
        .args(&["count=1kBb555"])
        .fails()
        .stderr_contains("dd: invalid number: '1kBb555'\n");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_oflag_direct_partial_block() {
    // Test for issue #9003: dd should handle partial blocks with oflag=direct
    // This reproduces the scenario where writing a partial block with O_DIRECT fails

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // Create input file with size that's not a multiple of block size
    // This will trigger the partial block write issue
    let input_file = "test_direct_input.iso";
    let output_file = "test_direct_output.img";
    let block_size = 8192; // 8K blocks
    let input_size = block_size * 3 + 511; // 3 full blocks + 511 byte partial block

    // Create test input file with known pattern
    let input_data = vec![0x42; input_size]; // Use non-zero pattern for better verification
    at.write_bytes(input_file, &input_data);

    // Get full paths for the dd command
    let input_path = at.plus(input_file);
    let output_path = at.plus(output_file);

    // Test with oflag=direct - should succeed with the fix
    new_ucmd!()
        .args(&[
            format!("if={}", input_path.display()),
            format!("of={}", output_path.display()),
            "oflag=direct".to_string(),
            format!("bs={block_size}"),
            "status=none".to_string(),
        ])
        .succeeds()
        .stdout_is("")
        .stderr_is("");
    assert!(output_path.exists());
    let output_size = output_path.metadata().unwrap().len() as usize;
    assert_eq!(output_size, input_size);

    // Verify content matches input
    let output_content = std::fs::read(&output_path).unwrap();
    assert_eq!(output_content.len(), input_size);
    assert_eq!(output_content, input_data);

    // Clean up
    at.remove(input_file);
    at.remove(output_file);
}
