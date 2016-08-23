use common::util::*;


#[test]
fn test_count_up() {
    new_ucmd!()
        .args(&["10"]).run().stdout_is("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down() {
    new_ucmd!()
        .args(&["--", "5", "-1", "1"]).run().stdout_is("5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator() {
    new_ucmd!()
        .args(&["-s", ",", "-t", "!", "2", "6"]).run().stdout_is("2,3,4,5,6!");
}

#[test]
fn test_equalize_widths() {
    new_ucmd!()
        .args(&["-w", "5", "10"]).run().stdout_is("05\n06\n07\n08\n09\n10\n");
}
