// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore contenta edgecase behaviour

use uutests::{at_and_ucmd, new_ucmd};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn unexpand_init_0() {
    new_ucmd!()
        .args(&["-t4"])
        .pipe_in(" 1\n  2\n   3\n    4\n")
        .succeeds()
        .stdout_is(" 1\n  2\n   3\n\t4\n");
}

#[test]
fn unexpand_init_1() {
    new_ucmd!()
        .args(&["-t4"])
        .pipe_in("     5\n      6\n       7\n        8\n")
        .succeeds()
        .stdout_is("\t 5\n\t  6\n\t   7\n\t\t8\n");
}

#[test]
fn unexpand_init_list_0() {
    new_ucmd!()
        .args(&["-t2,4"])
        .pipe_in(" 1\n  2\n   3\n    4\n")
        .succeeds()
        .stdout_is(" 1\n\t2\n\t 3\n\t\t4\n");
}

#[test]
fn unexpand_init_list_1() {
    // Once the list is exhausted, spaces are not converted anymore
    new_ucmd!()
        .args(&["-t2,4"])
        .pipe_in("     5\n      6\n       7\n        8\n")
        .succeeds()
        .stdout_is("\t\t 5\n\t\t  6\n\t\t   7\n\t\t    8\n");
}

#[test]
fn unexpand_flag_a_0() {
    new_ucmd!()
        .args(&["--"])
        .pipe_in("e     E\nf      F\ng       G\nh        H\n")
        .succeeds()
        .stdout_is("e     E\nf      F\ng       G\nh        H\n");
}

#[test]
fn unexpand_flag_a_1() {
    new_ucmd!()
        .args(&["-a"])
        .pipe_in("e     E\nf      F\ng       G\nh        H\n")
        .succeeds()
        .stdout_is("e     E\nf      F\ng\tG\nh\t H\n");
}

#[test]
fn unexpand_flag_a_2() {
    new_ucmd!()
        .args(&["-t8"])
        .pipe_in("e     E\nf      F\ng       G\nh        H\n")
        .succeeds()
        .stdout_is("e     E\nf      F\ng\tG\nh\t H\n");
}

#[test]
fn unexpand_first_only_0() {
    new_ucmd!()
        .args(&["-t3"])
        .pipe_in("        A     B")
        .succeeds()
        .stdout_is("\t\t  A\t  B");
}

#[test]
fn unexpand_first_only_1() {
    new_ucmd!()
        .args(&["-t3", "--first-only"])
        .pipe_in("        A     B")
        .succeeds()
        .stdout_is("\t\t  A     B");
}

#[test]
fn unexpand_first_only_2() {
    new_ucmd!()
        .args(&["-t3", "-f"])
        .pipe_in("        A     B")
        .succeeds()
        .stdout_is("\t\t  A     B");
}

#[test]
fn unexpand_first_only_3() {
    new_ucmd!()
        .args(&["-f", "-t8"])
        .pipe_in("        A     B")
        .succeeds()
        .stdout_is("\tA     B");
}

#[test]
fn unexpand_trailing_space_0() {
    // evil
    // Individual spaces before fields starting with non blanks should not be
    // converted, unless they are at the beginning of the line.
    new_ucmd!()
        .args(&["-t4"])
        .pipe_in("123 \t1\n123 1\n123 \n123 ")
        .succeeds()
        .stdout_is("123\t\t1\n123 1\n123 \n123 ");
}

#[test]
fn unexpand_trailing_space_1() {
    // super evil
    new_ucmd!()
        .args(&["-t1"])
        .pipe_in(" abc d e  f  g ")
        .succeeds()
        .stdout_is("\tabc d e\t\tf\t\tg ");
}

#[test]
fn unexpand_spaces_follow_tabs_0() {
    // The two first spaces can be included into the first tab.
    new_ucmd!()
        .pipe_in("  \t\t   A")
        .succeeds()
        .stdout_is("\t\t   A");
}

#[test]
fn unexpand_spaces_follow_tabs_1() {
    // evil
    // Explanation of what is going on here:
    //      'a' -> 'a'          // first tabstop (1)
    //    ' \t' -> '\t'         // second tabstop (4)
    //      ' ' -> '\t'         // third tabstop (5)
    // '  B \t' -> '  B \t'     // after the list is exhausted, nothing must change
    new_ucmd!()
        .args(&["-t1,4,5"])
        .pipe_in("a \t   B \t")
        .succeeds()
        .stdout_is("a\t\t  B \t");
}

#[test]
fn unexpand_spaces_after_fields() {
    new_ucmd!()
        .args(&["-a"])
        .pipe_in("   \t        A B C D             A\t\n")
        .succeeds()
        .stdout_is("\t\tA B C D\t\t    A\t\n");
}

#[test]
fn unexpand_read_from_file() {
    new_ucmd!().arg("with_spaces.txt").arg("-t4").succeeds();
}

#[test]
fn unexpand_read_from_two_file() {
    new_ucmd!()
        .arg("with_spaces.txt")
        .arg("with_spaces.txt")
        .arg("-t4")
        .succeeds();
}

#[test]
fn test_tabs_shortcut() {
    new_ucmd!()
        .arg("-3")
        .pipe_in("   a   b")
        .succeeds()
        .stdout_is("\ta   b");
}

#[test]
fn test_tabs_shortcut_combined_with_all_arg() {
    fn run_cmd(all_arg: &str) {
        new_ucmd!()
            .args(&[all_arg, "-3"])
            .pipe_in("a  b  c")
            .succeeds()
            .stdout_is("a\tb\tc");
    }

    let all_args = vec!["-a", "--all"];

    for arg in all_args {
        run_cmd(arg);
    }
}

#[test]
fn test_comma_separated_tabs_shortcut() {
    new_ucmd!()
        .args(&["-a", "-3,9"])
        .pipe_in("a  b     c")
        .succeeds()
        .stdout_is("a\tb\tc");
}

#[test]
fn test_tabs_cannot_be_zero() {
    new_ucmd!()
        .arg("--tabs=0")
        .fails()
        .stderr_contains("tab size cannot be 0");
}

#[test]
fn test_tabs_must_be_ascending() {
    new_ucmd!()
        .arg("--tabs=1,1")
        .fails()
        .stderr_contains("tab sizes must be ascending");
}

#[test]
fn test_tabs_with_invalid_chars() {
    new_ucmd!()
        .arg("--tabs=x")
        .fails()
        .stderr_contains("tab size contains invalid character(s): 'x'");
    new_ucmd!()
        .arg("--tabs=1x2")
        .fails()
        .stderr_contains("tab size contains invalid character(s): 'x2'");
}

#[test]
fn test_tabs_shortcut_with_too_large_size() {
    let arg = format!("-{}", u128::MAX);
    let expected_error = "tab stop value is too large";

    new_ucmd!().arg(arg).fails().stderr_contains(expected_error);
}

#[test]
fn test_is_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_name = "dir";
    at.mkdir(dir_name);

    ucmd.arg(dir_name)
        .fails()
        .stderr_contains(format!("unexpand: {dir_name}: Is a directory"));
}

#[test]
fn test_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("file", "content");
    at.write("file1", "a        b");

    ucmd.args(&["file", "file1"])
        .succeeds()
        .stdout_is("contenta        b");
}

#[test]
fn test_one_nonexisting_file() {
    new_ucmd!()
        .arg("asdf.txt")
        .fails()
        .stderr_contains("asdf.txt: No such file or directory");
}

#[test]
#[cfg(all(target_os = "linux", not(target_env = "musl")))]
fn test_read_error() {
    new_ucmd!()
        .arg("/proc/self/mem")
        .fails()
        .stderr_contains("unexpand: /proc/self/mem: Input/output error");
}

#[test]
#[cfg(target_os = "linux")]
fn test_non_utf8_filename() {
    use std::os::unix::ffi::OsStringExt;

    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"        a\n").unwrap();

    ucmd.arg(&filename).succeeds().stdout_is("\ta\n");
}

#[test]
fn unexpand_multibyte_utf8_gnu_compat() {
    // Verifies GNU-compatible behavior: column position uses byte count, not display width
    // "1ΔΔΔ5" is 8 bytes (1 + 2*3 + 1), already at tab stop 8
    // So 3 spaces should NOT convert to tab (would need 8 more to reach tab stop 16)
    new_ucmd!()
        .args(&["-a"])
        .pipe_in("1ΔΔΔ5   99999\n")
        .succeeds()
        .stdout_is("1ΔΔΔ5   99999\n");
}

#[test]
fn test_blanks_ext1() {
    // Test case from GNU test suite: blanks-ext1
    // ['blanks-ext1', '-t', '3,+6', {IN=> "\t      "}, {OUT=> "\t\t"}],
    new_ucmd!()
        .args(&["-t", "3,+6"])
        .pipe_in("\t      ")
        .succeeds()
        .stdout_is("\t\t");
}

#[test]
fn test_blanks_ext2() {
    // Test case from GNU test suite: blanks-ext2
    // ['blanks-ext2', '-t', '3,/9', {IN=> "\t      "}, {OUT=> "\t\t"}],
    new_ucmd!()
        .args(&["-t", "3,/9"])
        .pipe_in("\t      ")
        .succeeds()
        .stdout_is("\t\t");
}

#[test]
fn test_extended_tabstop_syntax() {
    let test_cases = [
        // Standalone /N: tabs at multiples of N
        ("-t /9", "         ", "\t"),            // 9 spaces -> 1 tab
        ("-t /9", "                  ", "\t\t"), // 18 spaces -> 2 tabs
        // Standalone +N: tabs at multiples of N
        ("-t +6", "      ", "\t"),         // 6 spaces -> 1 tab
        ("-t +6", "            ", "\t\t"), // 12 spaces -> 2 tabs
        // 3,/0 and 3,+0 should behave like just 3
        ("-t 3,/0", "          ", "\t\t\t "), // 10 spaces -> 3 tabs + 1 space
        ("-t 3,+0", "          ", "\t\t\t "), // 10 spaces -> 3 tabs + 1 space
        ("-t 3", "          ", "\t\t\t "),    // 10 spaces -> 3 tabs + 1 space
        // 3,/0 with text
        ("-t 3,/0", "   test", "\ttest"), // 3 spaces + text -> 1 tab + text
        // 3,+6 means tab stops at 3, 9, 15, 21, ...
        ("-t 3,+6", "                    ", "\t\t\t     "), // 20 spaces -> 3 tabs + 5 spaces
    ];

    for (args, input, expected) in test_cases {
        new_ucmd!()
            .args(&args.split_whitespace().collect::<Vec<_>>())
            .pipe_in(input)
            .succeeds()
            .stdout_is(expected);
    }
}

#[test]
fn test_buffered_read_edgecase_behaviour() {
    // reads are done in 4096 chunks. Tests edgecase spaces around chunk bounds
    let test_cases = [
        {
            // input has newlines in first chunk and has leading spaces after newline in chunk
            let mut input = vec![b'0'; 180];
            input.push(b'\n');
            input.extend([b' '; 8]);
            input.extend([b'0'; 3897]);
            input.push(b'\n'); // 180 '0' -> 'n' -> 8 spaces -> 3897 '0' -> \n

            let mut expected = vec![b'0'; 180];
            expected.push(b'\n');
            expected.push(b'\t');
            expected.extend([b'0'; 3897]);
            expected.push(b'\n'); // 180 '0' -> 'n' -> 1 tab -> 3897 '0' -> \n
            (input, expected)
        },
        {
            // input has newline after first chunk with leading spaces
            let mut input = vec![b'0'; 4096];
            input.extend("\n        0000\n".as_bytes()); // 4096 '0' -> \n -> 8 spaces -> 4 '0' -> \n

            let mut expected = vec![b'0'; 4096];
            expected.extend("\n\t0000\n".as_bytes()); // 4096 '0' -> \n -> 1 tab -> 4 '0' -> \n
            (input, expected)
        },
        {
            // fixture has newlines in first chunk and has leading spaces after newline in chunk
            let mut input = vec![b'0'; 4095];
            input.extend("\n        0000\n".as_bytes()); // 4095 '0' -> \n -> 8 spaces -> 4 '0' -> \n

            let mut expected = vec![b'0'; 4095];
            expected.extend("\n\t0000\n".as_bytes()); // 4095 '0' -> \n -> 1 tab -> 4 '0' -> \n
            (input, expected)
        },
        {
            // input has trailing spaces in the first chunk (should not be unexpanded) into newline with leading
            // spaces which should be unexpanded
            let mut input = vec![b'0'; 4088];
            input.extend("        \n        0000\n".as_bytes()); // 4088 '0' -> 8 spaces -> \n -> 8 spaces -> 4 '0' -> \n

            let mut expected = vec![b'0'; 4088];
            expected.extend("        \n\t0000\n".as_bytes()); // 4088 '0' -> 8 spaces -> \n -> 8 spaces -> 4 '0' -> \n
            (input, expected)
        },
        {
            // input has a trailing normal chars after new line in first chunk into leading spaces for new
            // chunk (should not be unexpanded)
            let mut input = vec![b'0'; 4087];
            input.extend("\n00000000        \n".as_bytes()); // 4087 '0' -> \n -> 8 '0' -> 8 spaces -> \n

            let mut expected = vec![b'0'; 4087];
            expected.extend("\n00000000        \n".as_bytes()); // 4087 '0' -> \n -> 8 '0' -> 8 spaces -> \n
            (input, expected)
        },
        {
            // input has a trailing blanks after new line in first chunk (should be unexpanded) into leading spaces for new
            // chunk (should be unexpanded)
            let mut input = vec![b'0'; 4087];
            input.extend("\n                \n".as_bytes()); // 4087 '0' -> 16 spaces -> \n

            let mut expected = vec![b'0'; 4087];
            expected.extend("\n\t\t\n".as_bytes()); // 4087 '0' -> 2 tabs -> \n
            (input, expected)
        },
        {
            // input has a trailing blanks after new line in first chunk (should be unexpanded) into leading spaces for new
            // chunk (should be unexpanded) (tests space counting is done over chunk bounds)
            let mut input = vec![b'0'; 4091];
            input.extend("\n        \n".as_bytes()); // 4091 '0' -> 8 spaces -> \n

            let mut expected = vec![b'0'; 4091];
            expected.extend("\n\t\n".as_bytes()); // 4091 '0' -> 1 tab -> \n
            (input, expected)
        },
    ];

    for (input, expected) in test_cases {
        new_ucmd!()
            .pipe_in(input)
            .succeeds()
            .stdout_only(String::from_utf8(expected).unwrap());
    }
}
