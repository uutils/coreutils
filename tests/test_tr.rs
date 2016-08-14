use common::util::*;

static UTIL_NAME: &'static str = "tr";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_toupper() {
    new_ucmd()
        .args(&["a-z", "A-Z"]).pipe_in("!abcd!").run().stdout_is("!ABCD!");
}

#[test]
fn test_small_set2() {
    new_ucmd()
        .args(&["0-9", "X"]).pipe_in("@0123456789").run().stdout_is("@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    new_ucmd()
        .args(&[", ┬─┬", "╯︵┻━┻"])
        .pipe_in("(,°□°）, ┬─┬").run()
        .stdout_is("(╯°□°）╯︵┻━┻");
}

#[test]
fn test_delete() {
    new_ucmd()
        .args(&["-d", "a-z"]).pipe_in("aBcD").run().stdout_is("BD");
}

#[test]
fn test_delete_complement() {
    new_ucmd()
        .args(&["-d", "-c", "a-z"]).pipe_in("aBcD").run().stdout_is("ac");
}
