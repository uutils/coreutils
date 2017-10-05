use common::util::*;


#[test]
fn test_toupper() {
    new_ucmd!()
        .args(&["a-z", "A-Z"]).pipe_in("!abcd!").run().stdout_is("!ABCD!");
}

#[test]
fn test_small_set2() {
    new_ucmd!()
        .args(&["0-9", "X"]).pipe_in("@0123456789").run().stdout_is("@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    new_ucmd!()
        .args(&[", ┬─┬", "╯︵┻━┻"])
        .pipe_in("(,°□°）, ┬─┬").run()
        .stdout_is("(╯°□°）╯︵┻━┻");
}

#[test]
fn test_delete() {
    new_ucmd!()
        .args(&["-d", "a-z"]).pipe_in("aBcD").run().stdout_is("BD");
}

#[test]
fn test_delete_complement() {
    new_ucmd!()
        .args(&["-d", "-c", "a-z"]).pipe_in("aBcD").run().stdout_is("ac");
}

#[test]
fn test_squeeze() {
    new_ucmd!()
        .args(&["-s", "a-z"]).pipe_in("aaBBcDcc").run().stdout_is("aBBcDc");
}


#[test]
fn test_squeeze_complement() {
    new_ucmd!()
        .args(&["-sc", "a-z"]).pipe_in("aaBBcDcc").run().stdout_is("aaBcDcc");
}

#[test]
fn test_delete_and_squeeze() {
    new_ucmd!()
        .args(&["-ds", "a-z", "A-Z"]).pipe_in("abBcB").run().stdout_is("B");
}

#[test]
fn test_delete_and_squeeze_complement() {
    new_ucmd!()
        .args(&["-dsc", "a-z", "A-Z"]).pipe_in("abBcB").run().stdout_is("abc");
}

#[test]
fn test_set1_longer_than_set2() {
    new_ucmd!()
        .args(&["abc", "xy"]).pipe_in("abcde").run().stdout_is("xyyde");
}

#[test]
fn test_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["ab", "xyz"]).pipe_in("abcde").run().stdout_is("xycde");
}

#[test]
fn test_truncate() {
    new_ucmd!()
        .args(&["-t", "abc", "xy"]).pipe_in("abcde").run().stdout_is("xycde");
}

#[test]
fn test_truncate_with_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["-t", "ab", "xyz"]).pipe_in("abcde").run().stdout_is("xycde");
}
