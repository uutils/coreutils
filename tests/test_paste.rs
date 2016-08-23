use common::util::*;


#[test]
fn test_combine_pairs_of_lines() {
    new_ucmd!()
        .args(&["-s", "-d", "\t\n", "html_colors.txt"])
        .run()
        .stdout_is_fixture("html_colors.expected");
}
