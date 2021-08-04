use crate::common::util::*;
// spell-checker:ignore (ToDO) taaaa tbbbb tcccc

#[test]
fn test_with_tab() {
    new_ucmd!()
        .arg("with-tab.txt")
        .succeeds()
        .stdout_contains("        ")
        .stdout_does_not_contain("\t");
}

#[test]
fn test_with_trailing_tab() {
    new_ucmd!()
        .arg("with-trailing-tab.txt")
        .succeeds()
        .stdout_contains("with tabs=>  ")
        .stdout_does_not_contain("\t");
}

#[test]
fn test_with_trailing_tab_i() {
    new_ucmd!()
        .arg("with-trailing-tab.txt")
        .arg("-i")
        .succeeds()
        .stdout_contains("        // with tabs=>\t");
}

#[test]
fn test_with_tab_size() {
    new_ucmd!()
        .arg("with-tab.txt")
        .arg("--tabs=10")
        .succeeds()
        .stdout_contains("          ");
}

#[test]
fn test_with_space() {
    new_ucmd!()
        .arg("with-spaces.txt")
        .succeeds()
        .stdout_contains("    return");
}

#[test]
fn test_with_multiple_files() {
    new_ucmd!()
        .arg("with-spaces.txt")
        .arg("with-tab.txt")
        .succeeds()
        .stdout_contains("    return")
        .stdout_contains("        ");
}

#[test]
fn test_tabs_space_separated_list() {
    new_ucmd!()
        .args(&["--tabs", "3 6 9"])
        .pipe_in("a\tb\tc\td\te")
        .succeeds()
        .stdout_is("a  b  c  d e");
}

#[test]
fn test_tabs_mixed_style_list() {
    new_ucmd!()
        .args(&["--tabs", ", 3,6 9"])
        .pipe_in("a\tb\tc\td\te")
        .succeeds()
        .stdout_is("a  b  c  d e");
}

#[test]
fn test_tabs_empty_string() {
    new_ucmd!()
        .args(&["--tabs", ""])
        .pipe_in("a\tb\tc")
        .succeeds()
        .stdout_is("a       b       c");
}

#[test]
fn test_tabs_comma_only() {
    new_ucmd!()
        .args(&["--tabs", ","])
        .pipe_in("a\tb\tc")
        .succeeds()
        .stdout_is("a       b       c");
}

#[test]
fn test_tabs_space_only() {
    new_ucmd!()
        .args(&["--tabs", " "])
        .pipe_in("a\tb\tc")
        .succeeds()
        .stdout_is("a       b       c");
}

#[test]
fn test_tabs_slash() {
    new_ucmd!()
        .args(&["--tabs", "/"])
        .pipe_in("a\tb\tc")
        .succeeds()
        .stdout_is("a       b       c");
}

#[test]
fn test_tabs_plus() {
    new_ucmd!()
        .args(&["--tabs", "+"])
        .pipe_in("a\tb\tc")
        .succeeds()
        .stdout_is("a       b       c");
}

#[test]
fn test_tabs_trailing_slash() {
    new_ucmd!()
        .arg("--tabs=1,/5")
        .pipe_in("\ta\tb\tc")
        .succeeds()
        //          0         1
        //          01234567890
        .stdout_is(" a   b    c");
}

#[test]
fn test_tabs_trailing_slash_long_columns() {
    new_ucmd!()
        .arg("--tabs=1,/3")
        .pipe_in("\taaaa\tbbbb\tcccc")
        .succeeds()
        //          0         1
        //          01234567890123456
        .stdout_is(" aaaa bbbb  cccc");
}

#[test]
fn test_tabs_trailing_plus() {
    new_ucmd!()
        .arg("--tabs=1,+5")
        .pipe_in("\ta\tb\tc")
        .succeeds()
        //          0         1
        //          012345678901
        .stdout_is(" a    b    c");
}

#[test]
fn test_tabs_trailing_plus_long_columns() {
    new_ucmd!()
        .arg("--tabs=1,+3")
        .pipe_in("\taaaa\tbbbb\tcccc")
        .succeeds()
        //          0         1
        //          012345678901234567
        .stdout_is(" aaaa  bbbb  cccc");
}

#[test]
fn test_tabs_must_be_ascending() {
    new_ucmd!()
        .arg("--tabs=1,1")
        .fails()
        .stderr_contains("tab sizes must be ascending");
}

#[test]
fn test_tabs_keep_last_trailing_specifier() {
    // If there are multiple trailing specifiers, use only the last one
    // before the number.
    new_ucmd!()
        .arg("--tabs=1,+/+/5")
        .pipe_in("\ta\tb\tc")
        .succeeds()
        //          0         1
        //          01234567890
        .stdout_is(" a   b    c");
}

#[test]
fn test_tabs_comma_separated_no_numbers() {
    new_ucmd!()
        .arg("--tabs=+,/,+,/")
        .pipe_in("\ta\tb\tc")
        .succeeds()
        .stdout_is("        a       b       c");
}
