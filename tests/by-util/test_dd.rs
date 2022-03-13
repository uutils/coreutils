// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, availible, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, iseek, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, oseek, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat abcdefghijklm abcdefghi nabcde nabcdefg abcdefg

use crate::common::util::*;

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use tempfile::tempfile;

macro_rules! inf {
    ($fname:expr) => {{
        &format!("if={}", $fname)
    }};
}

macro_rules! of {
    ($fname:expr) => {{
        &format!("of={}", $fname)
    }};
}

macro_rules! fixture_path {
    ($fname:expr) => {{
        PathBuf::from(format!("./tests/fixtures/dd/{}", $fname))
    }};
}

macro_rules! assert_fixture_exists {
    ($fname:expr) => {{
        let fpath = fixture_path!($fname);
        assert!(fpath.exists(), "Fixture missing: {:?}", fpath);
    }};
}

#[cfg(target_os = "linux")]
macro_rules! assert_fixture_not_exists {
    ($fname:expr) => {{
        let fpath = PathBuf::from(format!("./fixtures/dd/{}", $fname));
        assert!(!fpath.exists(), "Fixture present: {:?}", fpath);
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
        .run()
        .no_stderr()
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
        .run()
        .no_stderr()
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
        .run()
        .no_stderr()
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
        .run()
        .no_stderr()
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
        .run()
        .no_stderr()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_skip_w_multiplier() {
    let input = build_ascii_block(10 * 1024);
    let output = String::from_utf8(input[5 * 1024..].to_vec()).unwrap();
    new_ucmd!()
        .args(&["status=none", "skip=5K", "iflag=skip_bytes"])
        .pipe_in(input)
        .run()
        .no_stderr()
        .stdout_is(output)
        .success();
}

#[test]
fn test_stdin_stdout_count_w_multiplier() {
    let input = build_ascii_block(5 * 1024);
    let output = String::from_utf8(input[..2 * 1024].to_vec()).unwrap();
    new_ucmd!()
        .args(&["status=none", "count=2KiB", "iflag=count_bytes"])
        .pipe_in(input)
        .run()
        .no_stderr()
        .stdout_is(output)
        .success();
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
            .args(&[format!("{}=0", arg).as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .no_stderr();

        new_ucmd!()
            .args(&[format!("{}=00x1", arg).as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .no_stderr();

        new_ucmd!()
            .args(&[format!("{}=0x1", arg).as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .stderr_contains("warning: '0x' is a zero multiplier; use '00x' if that is intended");

        new_ucmd!()
            .args(&[format!("{}=0x0x1", arg).as_str(), "status=none"])
            .pipe_in("")
            .succeeds()
            .no_stdout()
            .stderr_is("dd: warning: '0x' is a zero multiplier; use '00x' if that is intended\ndd: warning: '0x' is a zero multiplier; use '00x' if that is intended\n");

        new_ucmd!()
            .args(&[format!("{}=1x0x1", arg).as_str(), "status=none"])
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
    let output = vec![
        "0+0 records in",
        "0+0 records out",
        "0 bytes (0 B, 0 B) copied, 0.0 s, 0 B/s",
    ];
    let output = output.into_iter().fold(String::new(), |mut acc, s| {
        acc.push_str(s);
        acc.push('\n');
        acc
    });
    new_ucmd!().run().stderr_only(&output).success();
}

#[cfg(target_os = "linux")]
#[test]
fn test_excl_causes_failure_when_present() {
    let fname = "this-file-exists-excl.txt";
    assert_fixture_exists!(&fname);

    let (_fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["of=this-file-exists-excl.txt", "conv=excl"])
        .fails();
}

#[cfg(target_os = "linux")]
#[test]
fn test_noatime_does_not_update_infile_atime() {
    // NOTE: Not all environments support tracking access time. If this
    // test fails on some systems and passes on others, assume the functionality
    // is not working and the systems that pass it simply don't update file access time.
    let fname = "this-ifile-exists-noatime.txt";
    assert_fixture_exists!(&fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "iflag=noatime", inf!(fname)]);

    let pre_atime = fix.metadata(fname).accessed().unwrap();

    ucmd.run().no_stderr().success();

    let post_atime = fix.metadata(fname).accessed().unwrap();
    assert_eq!(pre_atime, post_atime);
}

#[cfg(target_os = "linux")]
#[test]
fn test_noatime_does_not_update_ofile_atime() {
    // NOTE: Not all environments support tracking access time. If this
    // test fails on some systems and passes on others, assume the functionality
    // is not working and the systems that pass it simply don't update file access time.
    let fname = "this-ofile-exists-noatime.txt";
    assert_fixture_exists!(&fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "oflag=noatime", of!(fname)]);

    let pre_atime = fix.metadata(fname).accessed().unwrap();

    ucmd.pipe_in("").run().no_stderr().success();

    let post_atime = fix.metadata(fname).accessed().unwrap();
    assert_eq!(pre_atime, post_atime);
}

#[cfg(target_os = "linux")]
#[test]
fn test_nocreat_causes_failure_when_outfile_not_present() {
    let fname = "this-file-does-not-exist.txt";
    assert_fixture_not_exists!(&fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["conv=nocreat", of!(&fname)])
        .pipe_in("")
        .fails()
        .stderr_only(
            "dd: failed to open 'this-file-does-not-exist.txt': No such file or directory",
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
    ucmd.args(&["status=none", "conv=notrunc", of!(&fname), "if=null.txt"])
        .run()
        .no_stdout()
        .no_stderr()
        .success();

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
    ucmd.args(&["status=none", "if=null.txt", of!(fname)])
        .run()
        .no_stdout()
        .no_stderr()
        .success();

    assert_eq!(0, fix.metadata(fname).len());
}

#[test]
fn test_null_stats() {
    let stats = vec![
        "0+0 records in\n",
        "0+0 records out\n",
        "0 bytes (0 B, 0 B) copied, 0.0 s, 0 B/s\n",
    ];
    let stats = stats.into_iter().fold(String::new(), |mut acc, s| {
        acc.push_str(s);
        acc
    });

    new_ucmd!()
        .args(&["if=null.txt"])
        .run()
        .stderr_only(stats)
        .success();
}

#[test]
fn test_null_fullblock() {
    new_ucmd!()
        .args(&["if=null.txt", "status=none", "iflag=fullblock"])
        .run()
        .no_stdout()
        .no_stderr()
        .success();
}

#[cfg(unix)]
#[ignore] // See note below before using this test.
#[test]
fn test_fullblock() {
    let tname = "fullblock-from-urand";
    let tmp_fn = format!("TESTFILE-{}.tmp", &tname);
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
            of!(&tmp_fn),
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
        .run();
    ucmd.success();

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
        .run()
        .no_stderr()
        .stdout_is(output)
        .success();
}

#[test]
fn test_zeros_to_stdout() {
    let output = vec![0; 256 * 1024];
    let output = String::from_utf8(output).unwrap();
    new_ucmd!()
        .args(&["status=none", "if=zero-256k.txt"])
        .run()
        .no_stderr()
        .stdout_is(output)
        .success();
}

#[cfg(target_pointer_width = "32")]
#[test]
fn test_oversized_bs_32_bit() {
    for bs_param in &["bs", "ibs", "obs", "cbs"] {
        new_ucmd!()
            .args(&[format!("{}=5GB", bs_param)])
            .run()
            .no_stdout()
            .failure()
            .status_code(1)
            .stderr_is(format!("dd: {}=N cannot fit into memory\n", bs_param));
    }
}

#[test]
fn test_to_stdout_with_ibs_obs() {
    let output: Vec<_> = String::from("y\n").bytes().cycle().take(1024).collect();
    let output = String::from_utf8(output).unwrap();

    new_ucmd!()
        .args(&["status=none", "if=y-nl-1k.txt", "ibs=521", "obs=1031"])
        .run()
        .no_stderr()
        .stdout_is(output)
        .success();
}

#[test]
fn test_ascii_10k_to_stdout() {
    let output = build_ascii_block(1024 * 1024);
    // build_test_file!("ascii-10k.txt", &output);
    let output = String::from_utf8(output).unwrap();

    new_ucmd!()
        .args(&["status=none", "if=ascii-10k.txt"])
        .run()
        .no_stderr()
        .stdout_is(output)
        .success();
}

#[test]
fn test_zeros_to_file() {
    let tname = "zero-256k";
    let test_fn = format!("{}.txt", tname);
    let tmp_fn = format!("TESTFILE-{}.tmp", &tname);
    assert_fixture_exists!(test_fn);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", inf!(test_fn), of!(tmp_fn)])
        .run()
        .no_stderr()
        .no_stdout()
        .success();

    cmp_file!(
        File::open(fixture_path!(&test_fn)).unwrap(),
        fix.open(&tmp_fn)
    );
}

#[test]
fn test_to_file_with_ibs_obs() {
    let tname = "zero-256k";
    let test_fn = format!("{}.txt", tname);
    let tmp_fn = format!("TESTFILE-{}.tmp", &tname);
    assert_fixture_exists!(test_fn);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[
        "status=none",
        inf!(test_fn),
        of!(tmp_fn),
        "ibs=222",
        "obs=111",
    ])
    .run()
    .no_stderr()
    .no_stdout()
    .success();

    cmp_file!(
        File::open(fixture_path!(&test_fn)).unwrap(),
        fix.open(&tmp_fn)
    );
}

#[test]
fn test_ascii_521k_to_file() {
    let tname = "ascii-521k";
    let input = build_ascii_block(512 * 1024);
    let tmp_fn = format!("TESTFILE-{}.tmp", &tname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", of!(tmp_fn)])
        .pipe_in(input.clone())
        .run()
        .no_stderr()
        .no_stdout()
        .success();

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

#[ignore]
#[cfg(unix)]
#[test]
fn test_ascii_5_gibi_to_file() {
    let tname = "ascii-5G";
    let tmp_fn = format!("TESTFILE-{}.tmp", &tname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[
        "status=none",
        "count=5G",
        "iflag=count_bytes",
        "if=/dev/zero",
        of!(tmp_fn),
    ])
    .run()
    .no_stderr()
    .no_stdout()
    .success();

    assert_eq!(5 * 1024 * 1024 * 1024, fix.metadata(&tmp_fn).len());
}

#[test]
fn test_self_transfer() {
    let fname = "self-transfer-256k.txt";
    assert_fixture_exists!(fname);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", "conv=notrunc", inf!(fname), of!(fname)]);

    assert!(fix.file_exists(fname));
    assert_eq!(256 * 1024, fix.metadata(fname).len());

    ucmd.run().no_stdout().no_stderr().success();

    assert!(fix.file_exists(fname));
    assert_eq!(256 * 1024, fix.metadata(fname).len());
}

#[test]
fn test_unicode_filenames() {
    let tname = "😎💚🦊";
    let test_fn = format!("{}.txt", tname);
    let tmp_fn = format!("TESTFILE-{}.tmp", &tname);
    assert_fixture_exists!(test_fn);

    let (fix, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["status=none", inf!(test_fn), of!(tmp_fn)])
        .run()
        .no_stderr()
        .no_stdout()
        .success();

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
fn test_bytes_count_bytes_iflag() {
    new_ucmd!()
        .args(&["conv=swab", "count=14", "iflag=count_bytes"])
        .pipe_in("0123456789abcdefghijklm")
        .succeeds()
        .stdout_is("1032547698badc");
}

#[test]
fn test_bytes_skip_bytes_iflag() {
    new_ucmd!()
        .args(&["skip=10", "iflag=skip_bytes"])
        .pipe_in("0123456789abcdefghijklm")
        .succeeds()
        .stdout_is("abcdefghijklm");
}

#[test]
fn test_bytes_skip_bytes_pipe_iflag() {
    new_ucmd!()
        .args(&["skip=10", "iflag=skip_bytes", "bs=2"])
        .pipe_in("0123456789abcdefghijklm")
        .succeeds()
        .stdout_is("abcdefghijklm");
}

#[test]
fn test_bytes_oseek_bytes_oflag() {
    new_ucmd!()
        .args(&["seek=8", "oflag=seek_bytes", "bs=2"])
        .pipe_in("abcdefghijklm")
        .succeeds()
        .stdout_is_fixture_bytes("dd-bytes-alphabet-null.spec");
}

#[test]
fn test_bytes_oseek_bytes_trunc_oflag() {
    new_ucmd!()
        .args(&["seek=8", "oflag=seek_bytes", "bs=2", "count=0"])
        .pipe_in("abcdefghijklm")
        .succeeds()
        .stdout_is_fixture_bytes("dd-bytes-null-trunc.spec");
}
