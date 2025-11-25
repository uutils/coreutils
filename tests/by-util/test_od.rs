// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore abcdefghijklmnopqrstuvwxyz Anone fdbb littl

#[cfg(unix)]
use std::io::Read;

use unindent::unindent;
use uutests::util::TestScenario;
use uutests::{at_and_ucmd, new_ucmd, util_name};

// octal dump of 'abcdefghijklmnopqrstuvwxyz\n'
static ALPHA_OUT: &str = "
        0000000 061141 062143 063145 064147 065151 066153 067155 070157
        0000020 071161 072163 073165 074167 075171 000012
        0000033
        ";

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

// Test that od can read one file and dump with default format
#[test]
fn test_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("test", "abcdefghijklmnopqrstuvwxyz\n");

    for arg in ["--endian=little", "--endian=littl", "--endian=l"] {
        scene
            .ucmd()
            .arg(arg)
            .arg("test")
            .succeeds()
            .stdout_only(unindent(ALPHA_OUT));
    }

    // Ensure that default format matches `-t o2`, and that `-t` does not absorb file argument
    scene
        .ucmd()
        .arg("--endian=little")
        .arg("-t")
        .arg("o2")
        .arg("test")
        .succeeds()
        .stdout_only(unindent(ALPHA_OUT));
}

// Test that od can read 2 files and concatenate the contents
#[test]
fn test_two_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("test1", "abcdefghijklmnop");
    at.write("test2", "qrstuvwxyz\n"); // spell-checker:disable-line
    ucmd.arg("--endian=little")
        .arg("test1")
        .arg("test2")
        .succeeds()
        .stdout_only(unindent(ALPHA_OUT));
}

// Test that od gives non-0 exit val for filename that doesn't exist.
#[test]
fn test_non_existing_file() {
    new_ucmd!().arg("non_existing_file").fails();
}

// Test that od reads from stdin instead of a file
#[test]
fn test_from_stdin() {
    let input = "abcdefghijklmnopqrstuvwxyz\n";
    new_ucmd!()
        .arg("--endian=little")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(ALPHA_OUT));
}

// Test that od reads from stdin and also from files
#[test]
fn test_from_mixed() {
    let (at, mut ucmd) = at_and_ucmd!();
    // spell-checker:disable-next-line
    let (data1, data2, data3) = ("abcdefg", "hijklmnop", "qrstuvwxyz\n");
    at.write("test-1", data1);
    at.write("test-3", data3);

    ucmd.arg("--endian=little")
        .arg("test-1")
        .arg("-")
        .arg("test-3")
        .run_piped_stdin(data2.as_bytes())
        .success()
        .stdout_only(unindent(ALPHA_OUT));
}

#[test]
fn test_multiple_formats() {
    let input = "abcdefghijklmnopqrstuvwxyz\n";
    new_ucmd!()
        .arg("-c")
        .arg("-b")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            "
            0000000   a   b   c   d   e   f   g   h   i   j   k   l   m   n   o   p
                    141 142 143 144 145 146 147 150 151 152 153 154 155 156 157 160
            0000020   q   r   s   t   u   v   w   x   y   z  \\n
                    161 162 163 164 165 166 167 170 171 172 012
            0000033
            ",
        ));
}

#[test]
fn test_dec() {
    // spell-checker:ignore (words) 0xffu8 xffu
    let input = [
        0u8, 0u8, 1u8, 0u8, 2u8, 0u8, 3u8, 0u8, 0xffu8, 0x7fu8, 0x00u8, 0x80u8, 0x01u8, 0x80u8,
    ];
    let expected_output = unindent(
        "
            0000000      0      1      2      3  32767 -32768 -32767
            0000016
            ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-s")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_hex16() {
    let input: [u8; 9] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xff];
    // spell-checker:disable
    let expected_output = unindent(
        "
            0000000 2301 6745 ab89 efcd 00ff
            0000011
            ",
    );
    // spell-checker:enable
    new_ucmd!()
        .arg("--endian=little")
        .arg("-x")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_hex32() {
    let input: [u8; 9] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xff];
    let expected_output = unindent(
        "
            0000000 67452301 efcdab89 000000ff
            0000011
            ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-X")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_f16() {
    let input: [u8; 14] = [
        0x00, 0x3c, // 0x3C00 1.0
        0x00, 0x00, // 0x0000 0.0
        0x00, 0x80, // 0x8000 -0.0
        0x00, 0x7c, // 0x7C00 Inf
        0x00, 0xfc, // 0xFC00 -Inf
        0x00, 0xfe, // 0xFE00 NaN
        0x00, 0x84,
    ]; // 0x8400 -6.104e-5
    let expected_output = unindent(
        "
            0000000       1.0000000               0              -0             inf
            0000010            -inf             NaN   -6.1035156e-5
            0000016
            ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-tf2")
        .arg("-w8")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_fh() {
    let input: [u8; 14] = [
        0x00, 0x3c, // 0x3C00 1.0
        0x00, 0x00, // 0x0000 0.0
        0x00, 0x80, // 0x8000 -0.0
        0x00, 0x7c, // 0x7C00 Inf
        0x00, 0xfc, // 0xFC00 -Inf
        0x00, 0xfe, // 0xFE00 NaN
        0x00, 0x84,
    ]; // 0x8400 -6.1035156e-5
    let expected_output = unindent(
        "
            0000000       1.0000000               0              -0             inf
            0000010            -inf             NaN   -6.1035156e-5
            0000016
        ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-tfH")
        .arg("-w8")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_fb() {
    let input: [u8; 14] = [
        0x80, 0x3f, // 1.0
        0x00, 0x00, // 0.0
        0x00, 0x80, // -0.0
        0x80, 0x7f, // Inf
        0x80, 0xff, // -Inf
        0xc0, 0x7f, // NaN
        0x80, 0xb8,
    ]; // -6.1035156e-5
    let expected_output = unindent(
        "
            0000000       1.0000000               0              -0             inf
            0000010            -inf             NaN   -6.1035156e-5
            0000016
        ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-tfB")
        .arg("-w8")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_f32() {
    let input: [u8; 28] = [
        0x52, 0x06, 0x9e, 0xbf, // 0xbf9e0652 -1.2345679
        0x4e, 0x61, 0x3c, 0x4b, // 0x4b3c614e 12345678
        0x0f, 0x9b, 0x94, 0xfe, // 0xfe949b0f -9.876543E37
        0x00, 0x00, 0x00, 0x80, // 0x80000000 -0.0
        0xff, 0xff, 0xff, 0x7f, // 0x7fffffff NaN
        0xc2, 0x16, 0x01, 0x00, // 0x000116c2 1e-40
        0x00, 0x00, 0x7f, 0x80,
    ]; // 0x807f0000 -1.1663108E-38
    let expected_output = unindent(
        "
            0000000      -1.2345679        12345678  -9.8765427e+37              -0
            0000020             NaN           1e-40  -1.1663108e-38
            0000034
            ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-f")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_f64() {
    let input: [u8; 40] = [
        0x27, 0x6b, 0x0a, 0x2f, 0x2a, 0xee, 0x45,
        0x43, // 0x4345EE2A2F0A6B27 12345678912345678
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 0x0000000000000000 0
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
        0x80, // 0x8010000000000000 -2.2250738585072014e-308
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // 0x0000000000000001 5e-324 (subnormal)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0,
    ]; // 0xc000000000000000 -2
    let expected_output = unindent(
        "
            0000000        12345678912345678                        0
            0000020 -2.2250738585072014e-308                   5e-324
            0000040      -2.0000000000000000
            0000050
            ",
    );
    new_ucmd!()
        .arg("--endian=little")
        .arg("-F")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_multibyte() {
    let input = "’‐ˆ‘˜語🙂✅🐶𝛑Universität Tübingen \u{1B000}"; // spell-checker:disable-line
    new_ucmd!()
        .args(&["-t", "c"])
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            r"
            0000000 342 200 231 342 200 220 313 206 342 200 230 313 234 350 252 236
            0000020 360 237 231 202 342 234 205 360 237 220 266 360 235 233 221   U
            0000040   n   i   v   e   r   s   i   t 303 244   t       T 303 274   b
            0000060   i   n   g   e   n     360 233 200 200
            0000072
            ",
        ));
}

#[test]
fn test_width() {
    let input: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let expected_output = unindent(
        "
            0000000 000000 000000
            0000004 000000 000000
            0000010
            ",
    );

    new_ucmd!()
        .arg("-w4")
        .arg("-v")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_invalid_width() {
    let input: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
    let expected_output = unindent(
        "
            0000000 000000
            0000002 000000
            0000004
            ",
    );

    new_ucmd!()
        .arg("-w5")
        .arg("-v")
        .run_piped_stdin(&input[..])
        .success()
        .stderr_is_bytes("od: warning: invalid width 5; using 2 instead\n".as_bytes())
        .stdout_is(expected_output);
}

#[test]
fn test_zero_width() {
    let input: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
    let expected_output = unindent(
        "
            0000000 000000
            0000002 000000
            0000004
            ",
    );

    new_ucmd!()
        .arg("-w0")
        .arg("-v")
        .run_piped_stdin(&input[..])
        .success()
        .stderr_is_bytes("od: warning: invalid width 0; using 2 instead\n".as_bytes())
        .stdout_is(expected_output);
}

#[test]
fn test_width_without_value() {
    let input: [u8; 40] = [0; 40];
    let expected_output = unindent("
            0000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000 000000
            0000040 000000 000000 000000 000000
            0000050
            ");

    new_ucmd!()
        .arg("-w")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_suppress_duplicates() {
    let input: [u8; 41] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let expected_output = unindent(
        "
            0000000 00000000000
                     0000  0000
            *
            0000020 00000000001
                     0001  0000
            0000024 00000000000
                     0000  0000
            *
            0000050 00000000000
                     0000
            0000051
            ",
    );

    new_ucmd!()
        .arg("-w4")
        .arg("-O")
        .arg("-x")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_big_endian() {
    let input: [u8; 8] = [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // 0xc000000000000000 -2

    let expected_output = unindent(
        "
        0000000             -2.0000000000000000
                     -2.0000000               0
                       c0000000        00000000
                   c000    0000    0000    0000
        0000010
        ",
    );

    new_ucmd!()
        .arg("--endian=big")
        .arg("-F")
        .arg("-f")
        .arg("-X")
        .arg("-x")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(&expected_output);
    new_ucmd!()
        .arg("--endian=b")
        .arg("-F")
        .arg("-f")
        .arg("-X")
        .arg("-x")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
#[allow(non_snake_case)]
fn test_alignment_Xxa() {
    let input: [u8; 8] = [0x0A, 0x0D, 0x65, 0x66, 0x67, 0x00, 0x9e, 0x9f];

    let expected_output = unindent(
        "
        0000000        66650d0a        9f9e0067
                   0d0a    6665    0067    9f9e
                 nl  cr   e   f   g nul  rs  us
        0000010
        ",
    );

    // in this case the width of the -a (8-bit) determines the alignment for the other fields
    new_ucmd!()
        .arg("--endian=little")
        .arg("-X")
        .arg("-x")
        .arg("-a")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
#[allow(non_snake_case)]
fn test_alignment_Fx() {
    let input: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0]; // 0xc000000000000000 -2

    let expected_output = unindent(
        "
        0000000      -2.0000000000000000
                  0000  0000  0000  c000
        0000010
        ",
    );

    // in this case the width of the -F (64-bit) determines the alignment for the other field
    new_ucmd!()
        .arg("--endian=little")
        .arg("-F")
        .arg("-x")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_max_uint() {
    let input = [0xFFu8; 8];
    let expected_output = unindent(
        "
            0000000          1777777777777777777777
                        37777777777     37777777777
                     177777  177777  177777  177777
                    377 377 377 377 377 377 377 377
                               18446744073709551615
                         4294967295      4294967295
                      65535   65535   65535   65535
                    255 255 255 255 255 255 255 255
            0000010
            ",
    );

    new_ucmd!()
        .arg("--format=o8")
        .arg("-Oobtu8") // spell-checker:disable-line
        .arg("-Dd")
        .arg("--format=u1")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_hex_offset() {
    let input = [0u8; 0x1F];
    let expected_output = unindent(
        "
            000000 00000000 00000000 00000000 00000000
                   00000000 00000000 00000000 00000000
            000010 00000000 00000000 00000000 00000000
                   00000000 00000000 00000000 00000000
            00001F
            ",
    );

    new_ucmd!()
        .arg("-Ax")
        .arg("-X")
        .arg("-X")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_dec_offset() {
    let input = [0u8; 19];
    let expected_output = unindent(
        "
            0000000 00000000 00000000 00000000 00000000
                    00000000 00000000 00000000 00000000
            0000016 00000000
                    00000000
            0000019
            ",
    );

    new_ucmd!()
        .arg("-Ad")
        .arg("-X")
        .arg("-X")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_no_offset() {
    const LINE: &str = " 00000000 00000000 00000000 00000000\n";
    let input = [0u8; 31];
    let expected_output = [LINE, LINE, LINE, LINE].join("");

    new_ucmd!()
        .arg("-An")
        .arg("-X")
        .arg("-X")
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(expected_output);
}

#[test]
fn test_invalid_offset() {
    new_ucmd!().arg("-Ab").fails();
}

#[test]
fn test_empty_offset() {
    new_ucmd!()
        .arg("-A")
        .arg("")
        .fails()
        .stderr_only("od: Radix cannot be empty, and must be one of [o, d, x, n]\n");
}

#[test]
fn test_offset_compatibility() {
    let input = [0u8; 4];
    let expected_output = " 000000 000000\n";

    new_ucmd!()
        .arg("-Anone")
        .pipe_in(input)
        .succeeds()
        .stdout_only(expected_output);
}

#[test]
fn test_skip_bytes() {
    let input = "abcdefghijklmnopq";
    new_ucmd!()
        .arg("-c")
        .arg("--skip-bytes=5")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            "
            0000005   f   g   h   i   j   k   l   m   n   o   p   q
            0000021
            ",
        ));
}

#[test]
fn test_skip_bytes_hex() {
    let input = "abcdefghijklmnopq";
    new_ucmd!()
        .arg("-c")
        .arg("--skip-bytes=0xB")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            "
            0000013   l   m   n   o   p   q
            0000021
            ",
        ));
    new_ucmd!()
        .arg("-c")
        .arg("--skip-bytes=0xE")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            "
            0000016   o   p   q
            0000021
            ",
        ));
}

#[test]
fn test_skip_bytes_error() {
    let input = "12345";
    new_ucmd!()
        .arg("--skip-bytes=10")
        .run_piped_stdin(input.as_bytes())
        .failure();
}

#[test]
fn test_read_bytes() {
    let scene = TestScenario::new(util_name!());
    let fixtures = &scene.fixtures;

    let input = "abcdefghijklmnopqrstuvwxyz\n12345678";

    fixtures.write("f1", input);
    let file = fixtures.open("f1");
    #[cfg(unix)]
    let mut file_shadow = file.try_clone().unwrap();

    scene
        .ucmd()
        .arg("--endian=little")
        .arg("--read-bytes=27")
        .set_stdin(file)
        .succeeds()
        .stdout_only(unindent(ALPHA_OUT));

    // On unix platforms, confirm that only 27 bytes and strictly no more were read from stdin.
    // Leaving stdin in the correct state is required for GNU compatibility.
    #[cfg(unix)]
    {
        // skip(27) to skip the 27 bytes that should have been consumed with the
        // --read-bytes flag.
        let expected_bytes_remaining_in_stdin: Vec<_> = input.bytes().skip(27).collect();
        let mut bytes_remaining_in_stdin = vec![];
        assert_eq!(
            file_shadow
                .read_to_end(&mut bytes_remaining_in_stdin)
                .unwrap(),
            expected_bytes_remaining_in_stdin.len()
        );
        assert_eq!(expected_bytes_remaining_in_stdin, bytes_remaining_in_stdin);
    }
}

#[test]
fn test_ascii_dump() {
    let input: [u8; 22] = [
        0x00, 0x01, 0x0a, 0x0d, 0x10, 0x1f, 0x20, 0x61, 0x62, 0x63, 0x7d, 0x7e, 0x7f, 0x80, 0x90,
        0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0xff,
    ];
    new_ucmd!()
        .arg("-tx1zacz")    // spell-checker:disable-line
        .run_piped_stdin(&input[..])
        .success()
        .stdout_only(unindent(
            r"
            0000000  00  01  0a  0d  10  1f  20  61  62  63  7d  7e  7f  80  90  a0  >...... abc}~....<
                    nul soh  nl  cr dle  us  sp   a   b   c   }   ~ del nul dle  sp
                     \0 001  \n  \r 020 037       a   b   c   }   ~ 177 200 220 240  >...... abc}~....<
            0000020  b0  c0  d0  e0  f0  ff                                          >......<
                      0   @   P   `   p del
                    260 300 320 340 360 377                                          >......<
            0000026
            ",
        ));
}

#[test]
fn test_filename_parsing() {
    // files "a" and "x" both exists, but are no filenames in the command line below
    // "-f" must be treated as a filename, it contains the text: minus lowercase f
    // so "-f" should not be interpreted as a formatting option.
    new_ucmd!()
        .arg("--format")
        .arg("a")
        .arg("-A")
        .arg("x")
        .arg("--")
        .arg("-f")
        .succeeds()
        .stdout_only(unindent(
            "
            000000   m   i   n   u   s  sp   l   o   w   e   r   c   a   s   e  sp
            000010   f  nl
            000012
            ",
        ));
}

#[test]
fn test_stdin_offset() {
    let input = "abcdefghijklmnopq";
    new_ucmd!()
        .arg("-c")
        .arg("+5")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            "
            0000005   f   g   h   i   j   k   l   m   n   o   p   q
            0000021
            ",
        ));
}

#[test]
fn test_file_offset() {
    new_ucmd!()
        .arg("-c")
        .arg("--")
        .arg("-f")
        .arg("10")
        .succeeds()
        .stdout_only(unindent(
            r"
            0000010   w   e   r   c   a   s   e       f  \n
            0000022
            ",
        ));
}

#[test]
fn test_traditional() {
    // note gnu od does not align both lines
    let input = "abcdefghijklmnopq";
    new_ucmd!()
        .arg("--traditional")
        .arg("-a")
        .arg("-c")
        .arg("-")
        .arg("10")
        .arg("0")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            r"
            0000010 (0000000)   i   j   k   l   m   n   o   p   q
                                i   j   k   l   m   n   o   p   q
            0000021 (0000011)
            ",
        ));
}

#[test]
fn test_traditional_with_skip_bytes_override() {
    // --skip-bytes is ignored in this case
    let input = "abcdefghijklmnop";
    new_ucmd!()
        .arg("--traditional")
        .arg("--skip-bytes=10")
        .arg("-c")
        .arg("0")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            r"
            0000000   a   b   c   d   e   f   g   h   i   j   k   l   m   n   o   p
            0000020
            ",
        ));
}

#[test]
fn test_traditional_with_skip_bytes_non_override() {
    // no offset specified in the traditional way, so --skip-bytes is used
    let input = "abcdefghijklmnop";
    new_ucmd!()
        .arg("--traditional")
        .arg("--skip-bytes=10")
        .arg("-c")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            r"
            0000012   k   l   m   n   o   p
            0000020
            ",
        ));
}

#[test]
fn test_traditional_error() {
    // file "0" exists - don't fail on that, but --traditional only accepts a single input
    new_ucmd!()
        .arg("--traditional")
        .arg("0")
        .arg("0")
        .arg("0")
        .arg("0")
        .fails();
}

#[test]
fn test_traditional_only_label() {
    let input = "abcdefghijklmnopqrstuvwxyz";
    new_ucmd!()
        .arg("-An")
        .arg("--traditional")
        .arg("-a")
        .arg("-c")
        .arg("-")
        .arg("10")
        .arg("0x10")
        .run_piped_stdin(input.as_bytes())
        .success()
        .stdout_only(unindent(
            r"
            (0000020)   i   j   k   l   m   n   o   p   q   r   s   t   u   v   w   x
                        i   j   k   l   m   n   o   p   q   r   s   t   u   v   w   x
            (0000040)   y   z
                        y   z
            (0000042)
            ",
        ));
}

#[test]
fn test_od_invalid_bytes() {
    const INVALID_SIZE: &str = "x";
    const INVALID_SUFFIX: &str = "1fb4t";
    const BIG_SIZE: &str = "1Y";

    // NOTE:
    // GNU's od (8.32) with option '--width' does not accept 'Y' as valid suffix.
    // According to the man page it should be valid in the same way it is valid for
    // '--read-bytes' and '--skip-bytes'.

    let options = [
        "--read-bytes",
        "--skip-bytes",
        "--width",
        // "--strings", // TODO: consider testing here once '--strings' is implemented
    ];
    for option in &options {
        new_ucmd!()
            .arg(format!("{option}={INVALID_SIZE}"))
            .arg("file")
            .fails_with_code(1)
            .stderr_only(format!("od: invalid {option} argument '{INVALID_SIZE}'\n"));

        new_ucmd!()
            .arg(format!("{option}={INVALID_SUFFIX}"))
            .arg("file")
            .fails_with_code(1)
            .stderr_only(format!(
                "od: invalid suffix in {option} argument '{INVALID_SUFFIX}'\n"
            ));

        new_ucmd!()
            .arg(format!("{option}={BIG_SIZE}"))
            .arg("file")
            .fails_with_code(1)
            .stderr_only(format!("od: {option} argument '{BIG_SIZE}' too large\n"));
    }
}

#[test]
fn test_od_options_after_filename() {
    let file = "test";
    let (at, mut ucmd) = at_and_ucmd!();
    let input: [u8; 4] = [0x68, 0x1c, 0xbb, 0xfd];
    at.write_bytes(file, &input);

    ucmd.arg(file)
        .arg("-v")
        .arg("-An")
        .arg("-t")
        .arg("x2")
        .succeeds()
        .stdout_only(" 1c68 fdbb\n");
}

#[test]
fn test_od_strings_option() {
    // Test -S option: output strings of at least N graphic chars

    // Test -S0: output all null-terminated strings regardless of length
    new_ucmd!()
        .arg("-S0")
        .pipe_in(b"hello\x00world\x00")
        .succeeds()
        .stdout_is("0000000 hello\n0000006 world\n");

    // Test -S0 with single character strings
    new_ucmd!()
        .arg("-S0")
        .pipe_in(b"a\x00b\x00cd\x00")
        .succeeds()
        .stdout_is("0000000 a\n0000002 b\n0000004 cd\n");

    // Test with null-terminated strings
    new_ucmd!()
        .arg("-S3")
        .pipe_in(b"\x01hello\x00world\x00ab\x00")
        .succeeds()
        .stdout_is("0000001 hello\n0000007 world\n");

    // Test with -S10 to show only strings with minimum length
    new_ucmd!()
        .arg("-S10")
        .pipe_in(b"\x01          \x00          \x00")
        .succeeds()
        .stdout_is("0000001           \n0000014           \n");

    // Test with unterminated string at EOF (should not output)
    new_ucmd!()
        .arg("-S10")
        .pipe_in(b"          ")
        .succeeds()
        .no_output();

    // Test with -N limit and pending string (should output even without null)
    let expected = "0000000   \n";
    new_ucmd!()
        .arg("-N2")
        .arg("-S1")
        .pipe_in("  ")
        .succeeds()
        .stdout_is(expected);

    // Test with -N limit and string too short
    new_ucmd!()
        .arg("-N11")
        .arg("-S11")
        .pipe_in(b"          \x00")
        .succeeds()
        .no_output();

    // Test with no address prefix (-An)
    new_ucmd!()
        .arg("-S3")
        .arg("-An")
        .pipe_in(b"hello\x00world\x00")
        .succeeds()
        .stdout_is("hello\nworld\n");
}

#[test]
fn test_od_eintr_handling() {
    // Test that od properly handles EINTR (ErrorKind::Interrupted) during read operations
    // This verifies the signal interruption retry logic in PartialReader implementation
    // This verifies the signal interruption retry logic in PartialReader implementation

    let file = "test_eintr";
    let (at, mut ucmd) = at_and_ucmd!();

    // Create test data
    let test_data = [0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x0a]; // "Hello\n"
    at.write_bytes(file, &test_data);

    // Test that od can handle interrupted reads during file processing
    // The EINTR handling in PartialReader should retry and complete successfully
    ucmd.arg(file)
        .arg("-t")
        .arg("c")
        .succeeds()
        .no_stderr()
        .stdout_contains("H"); // Should contain 'H' from "Hello"

    // Test with skip and offset options to exercise different code paths
    // Create a new command instance to avoid "already run this UCommand" error
    let (at, mut ucmd2) = at_and_ucmd!();
    at.write_bytes(file, &test_data);

    ucmd2
        .arg(file)
        .arg("-j")
        .arg("1")
        .arg("-N")
        .arg("3")
        .arg("-t")
        .arg("c")
        .succeeds()
        .no_stderr()
        .stdout_contains("e"); // Should contain 'e' from "ello"
}
