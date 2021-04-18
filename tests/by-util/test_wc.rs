use crate::common::util::*;

#[test]
fn test_count_bytes_large_stdin() {
    for &n in &[
        0,
        1,
        42,
        16 * 1024 - 7,
        16 * 1024 - 1,
        16 * 1024,
        16 * 1024 + 1,
        16 * 1024 + 3,
        32 * 1024,
        64 * 1024,
        80 * 1024,
        96 * 1024,
        112 * 1024,
        128 * 1024,
    ] {
        let data = vec_of_size(n);
        let expected = format!("{}\n", n);
        new_ucmd!()
            .args(&["-c"])
            .pipe_in(data)
            .succeeds()
            .stdout_is_bytes(&expected.as_bytes());
    }
}

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .run()
        .stdout_is("  13 109 772\n");
}

#[test]
fn test_utf8() {
    new_ucmd!()
        .args(&["-lwmcL"])
        .pipe_in_fixture("UTF_8_test.txt")
        .run()
        .stdout_is("   300  4969 22781 22213    79\n");
    // GNU returns "  300  2086 22219 22781    79"
    // TODO: we should fix that to match GNU's behavior
}

#[test]
fn test_stdin_line_len_regression() {
    new_ucmd!()
        .args(&["-L"])
        .pipe_in("\n123456")
        .run()
        .stdout_is("6\n");
}

#[test]
fn test_stdin_only_bytes() {
    new_ucmd!()
        .args(&["-c"])
        .pipe_in_fixture("lorem_ipsum.txt")
        .run()
        .stdout_is("772\n");
}

#[test]
fn test_stdin_all_counts() {
    new_ucmd!()
        .args(&["-c", "-m", "-l", "-L", "-w"])
        .pipe_in_fixture("alice_in_wonderland.txt")
        .run()
        .stdout_is("   5  57 302 302  66\n");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg("moby_dick.txt")
        .run()
        .stdout_is("   18  204 1115 moby_dick.txt\n");
}

#[test]
fn test_single_only_lines() {
    new_ucmd!()
        .args(&["-l", "moby_dick.txt"])
        .run()
        .stdout_is("18 moby_dick.txt\n");
}

#[test]
fn test_single_all_counts() {
    new_ucmd!()
        .args(&["-c", "-l", "-L", "-m", "-w", "alice_in_wonderland.txt"])
        .run()
        .stdout_is("   5  57 302 302  66 alice_in_wonderland.txt\n");
}

#[test]
fn test_multiple_default() {
    new_ucmd!()
        .args(&[
            "lorem_ipsum.txt",
            "moby_dick.txt",
            "alice_in_wonderland.txt",
        ])
        .run()
        .stdout_is(
            "   13  109  772 lorem_ipsum.txt\n   18  204 1115 moby_dick.txt\n    5   57  302 \
             alice_in_wonderland.txt\n   36  370 2189 total\n",
        );
}
