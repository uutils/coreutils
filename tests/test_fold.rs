use common::util::*;

static UTIL_NAME: &'static str = "fold";

#[test]
fn test_default_80_column_wrap() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg("lorem_ipsum.txt")
                  .run()
                  .stdout;

    assert_eq!(out, at.read("lorem_ipsum_80_column.expected"));
}

#[test]
fn test_40_column_hard_cutoff() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg("-w")
                  .arg("40")
                  .arg("lorem_ipsum.txt")
                  .run()
                  .stdout;

    assert_eq!(out, at.read("lorem_ipsum_40_column_hard.expected"));
}

#[test]
fn test_40_column_word_boundary() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg("-s")
                  .arg("-w")
                  .arg("40")
                  .arg("lorem_ipsum.txt")
                  .run()
                  .stdout;

    assert_eq!(out, at.read("lorem_ipsum_40_column_word.expected"));
}
