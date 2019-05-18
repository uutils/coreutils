use common::util::*;

#[test]
fn test_env_help() {
    assert!(new_ucmd!().arg("--help").succeeds().no_stderr().stdout.contains("OPTIONS:"));
}

#[test]
fn test_env_version() {
    assert!(new_ucmd!().arg("--version").succeeds().no_stderr().stdout.contains(util_name!()));
}

#[test]
fn test_echo() {
    // assert!(new_ucmd!().arg("printf").arg("FOO-bar").succeeds().no_stderr().stdout.contains("FOO-bar"));
    let mut cmd = new_ucmd!();
    cmd.arg("echo").arg("FOO-bar");
    println!("cmd={:?}", cmd);

    let result = cmd.run();
    println!("success={:?}", result.success);
    println!("stdout={:?}", result.stdout);
    println!("stderr={:?}", result.stderr);
    assert!(result.success);

    let out = result.stdout.trim_end();

    assert_eq!(out, "FOO-bar");
}

#[test]
fn test_file_option() {
    let out = new_ucmd!()
        .arg("-f").arg("vars.conf.txt")
        .run().stdout;

    assert_eq!(out.lines().filter(|&line| line == "FOO=bar" || line == "BAR=bamf this").count(), 2);
}

#[test]
fn test_combined_file_set() {
    let out = new_ucmd!()
        .arg("-f").arg("vars.conf.txt")
        .arg("FOO=bar.alt")
        .run().stdout;

    assert_eq!(out.lines().filter(|&line| line == "FOO=bar.alt").count(), 1);
}

#[test]
fn test_combined_file_set_unset() {
    let out = new_ucmd!()
        .arg("-u").arg("BAR")
        .arg("-f").arg("vars.conf.txt")
        .arg("FOO=bar.alt")
        .run().stdout;

    assert_eq!(out.lines().filter(|&line| line == "FOO=bar.alt" || line.starts_with("BAR=")).count(), 1);
}

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

#[test]
fn test_fail_null_with_program() {
    let out = new_ucmd!().arg("--null").arg("cd").fails().stderr;
    assert!(out.contains("cannot specify --null (-0) with command"));
}
