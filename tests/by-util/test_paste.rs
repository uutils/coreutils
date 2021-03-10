use crate::common::util::*;

#[test]
fn test_combine_pairs_of_lines() {
    for s in vec!["-s", "--serial"] {
        for d in vec!["-d", "--delimiters"] {
            new_ucmd!()
                .args(&[s, d, "\t\n", "html_colors.txt"])
                .run()
                .stdout_is_fixture("html_colors.expected");
        }
    }
}

#[test]
fn test_multi_stdin() {
    for d in vec!["-d", "--delimiters"] {
        new_ucmd!()
            .args(&[d, "\t\n", "-", "-"])
            .pipe_in_fixture("html_colors.txt")
            .succeeds()
            .stdout_is_fixture("html_colors.expected");
    }
}
