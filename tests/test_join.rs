use common::util::*;


#[test]
fn empty_files() {
    new_ucmd!()
        .arg("empty.txt")
        .arg("empty.txt")
        .succeeds().stdout_only("");

    new_ucmd!()
        .arg("empty.txt")
        .arg("fields_1.txt")
        .succeeds().stdout_only("");

    new_ucmd!()
        .arg("fields_1.txt")
        .arg("empty.txt")
        .succeeds().stdout_only("");
}

#[test]
fn empty_intersection() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-2")
        .arg("2")
        .succeeds().stdout_only("");
}

#[test]
fn default_arguments() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .succeeds().stdout_only_fixture("default.expected");
}

#[test]
fn different_fields() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .arg("-j")
        .arg("2")
        .succeeds().stdout_only_fixture("different_fields.expected");

    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .arg("-1")
        .arg("2")
        .arg("-2")
        .arg("2")
        .succeeds().stdout_only_fixture("different_fields.expected");
}

#[test]
fn different_field() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_3.txt")
        .arg("-2")
        .arg("2")
        .succeeds().stdout_only_fixture("different_field.expected");
}

#[test]
fn unpaired_lines() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_3.txt")
        .arg("-a")
        .arg("1")
        .succeeds().stdout_only_fixture("fields_2.txt");

    new_ucmd!()
        .arg("fields_3.txt")
        .arg("fields_2.txt")
        .arg("-1")
        .arg("2")
        .arg("-a")
        .arg("2")
        .succeeds().stdout_only_fixture("unpaired_lines.expected");
}

#[test]
fn case_insensitive() {
    new_ucmd!()
        .arg("capitalized.txt")
        .arg("fields_3.txt")
        .arg("-i")
        .succeeds().stdout_only_fixture("case_insensitive.expected");
}
