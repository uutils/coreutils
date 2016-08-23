use common::util::*;


#[test]
fn test_single_name_value_pair() {
    let out = new_ucmd!()
        .arg("FOO=bar").run().stdout;

    assert!(out.lines().any(|line| line == "FOO=bar"));
}

#[test]
fn test_multiple_name_value_pairs() {
    let out = new_ucmd!()
        .arg("FOO=bar")
                  .arg("ABC=xyz")
                  .run()
                  .stdout;

    assert_eq!(out.lines().filter(|&line| line == "FOO=bar" || line == "ABC=xyz").count(),
               2);
}

#[test]
fn test_ignore_environment() {
    let scene = TestScenario::new(util_name!());

    let out = scene.ucmd()
                .arg("-i")
                .run()
                .stdout;

    assert_eq!(out, "");

    let out = scene.ucmd()
                .arg("-")
                .run()
                .stdout;

    assert_eq!(out, "");
}

#[test]
fn test_null_delimiter() {
    let out = new_ucmd!()
                  .arg("-i")
                  .arg("--null")
                  .arg("FOO=bar")
                  .arg("ABC=xyz")
                  .run()
                  .stdout;

    let mut vars : Vec<_> = out.split('\0').collect();
    assert_eq!(vars.len(), 3);
    vars.sort();
    assert_eq!(vars[0], "");
    assert_eq!(vars[1], "ABC=xyz");
    assert_eq!(vars[2], "FOO=bar");
}

#[test]
fn test_unset_variable() {
    // This test depends on the HOME variable being pre-defined by the
    // default shell
    let out = TestScenario::new(util_name!())
                  .ucmd_keepenv()
                  .arg("-u")
                  .arg("HOME")
                  .run()
                  .stdout;

    assert_eq!(out.lines().any(|line| line.starts_with("HOME=")), false);
}
