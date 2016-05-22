use common::util::*;

static UTIL_NAME: &'static str = "wc";

#[test]
fn test_stdin_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.run_piped_stdin(at.read("lorem_ipsum.txt"));
    assert_eq!(result.stdout, "  13 109 772\n");
}

#[test]
fn test_stdin_only_bytes() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c"]).run_piped_stdin(at.read("lorem_ipsum.txt"));
    assert_eq!(result.stdout, " 772\n");
}

#[test]
fn test_stdin_all_counts() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c", "-m", "-l", "-L", "-w"])
                     .run_piped_stdin(at.read("alice_in_wonderland.txt"));
    assert_eq!(result.stdout, "   5  57 302 302  66\n");
}

#[test]
fn test_single_default() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("moby_dick.txt").run();
    assert_eq!(result.stdout, "   18  204 1115 moby_dick.txt\n");
}

#[test]
fn test_single_only_lines() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-l", "moby_dick.txt"]).run();
    assert_eq!(result.stdout, "   18 moby_dick.txt\n");
}

#[test]
fn test_single_all_counts() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c", "-l", "-L", "-m", "-w", "alice_in_wonderland.txt"]).run();
    assert_eq!(result.stdout,
               "   5  57 302 302  66 alice_in_wonderland.txt\n");
}

#[test]
fn test_multiple_default() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["lorem_ipsum.txt", "moby_dick.txt", "alice_in_wonderland.txt"]).run();
    assert_eq!(result.stdout,
               "   13  109  772 lorem_ipsum.txt\n   18  204 1115 moby_dick.txt\n    5   57  302 \
                alice_in_wonderland.txt\n   36  370 2189 total\n");
}
