use common::util::*;

static UTIL_NAME: &'static str = "paste";

#[test]
fn test_combine_pairs_of_lines() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.arg("-s")
                  .arg("-d")
                  .arg("\t\n")
                  .arg("html_colors.txt")
                  .run()
                  .stdout;

    assert_eq!(out, at.read("html_colors.expected"));
}
