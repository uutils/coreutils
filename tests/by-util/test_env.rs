// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) bamf chdir rlimit prlimit COMSPEC

use crate::common::util::TestScenario;
use std::env;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(125);
}

#[test]
fn test_env_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .no_stderr()
        .stdout_contains("Options:");
}

#[test]
fn test_env_version() {
    new_ucmd!()
        .arg("--version")
        .succeeds()
        .no_stderr()
        .stdout_contains(util_name!());
}

#[test]
fn test_echo() {
    let result = new_ucmd!().arg("echo").arg("FOO-bar").succeeds();

    assert_eq!(result.stdout_str().trim(), "FOO-bar");
}

#[test]
fn test_file_option() {
    let out = new_ucmd!()
        .arg("-f")
        .arg("vars.conf.txt")
        .run()
        .stdout_move_str();

    assert_eq!(
        out.lines()
            .filter(|&line| line == "FOO=bar" || line == "BAR=bamf this")
            .count(),
        2
    );
}

#[test]
fn test_combined_file_set() {
    let out = new_ucmd!()
        .arg("-f")
        .arg("vars.conf.txt")
        .arg("FOO=bar.alt")
        .run()
        .stdout_move_str();

    assert_eq!(out.lines().filter(|&line| line == "FOO=bar.alt").count(), 1);
}

#[test]
fn test_combined_file_set_unset() {
    let out = new_ucmd!()
        .arg("-u")
        .arg("BAR")
        .arg("-f")
        .arg("vars.conf.txt")
        .arg("FOO=bar.alt")
        .succeeds()
        .stdout_move_str();

    assert_eq!(
        out.lines()
            .filter(|&line| line == "FOO=bar.alt" || line.starts_with("BAR="))
            .count(),
        1
    );
}

#[test]
fn test_unset_invalid_variables() {
    use uucore::display::Quotable;

    // Cannot test input with \0 in it, since output will also contain \0. rlimit::prlimit fails
    // with this error: Error { kind: InvalidInput, message: "nul byte found in provided data" }
    for var in ["", "a=b"] {
        new_ucmd!().arg("-u").arg(var).run().stderr_only(format!(
            "env: cannot unset {}: Invalid argument\n",
            var.quote()
        ));
    }
}

#[test]
fn test_single_name_value_pair() {
    let out = new_ucmd!().arg("FOO=bar").run();

    assert!(out.stdout_str().lines().any(|line| line == "FOO=bar"));
}

#[test]
fn test_multiple_name_value_pairs() {
    let out = new_ucmd!().arg("FOO=bar").arg("ABC=xyz").run();

    assert_eq!(
        out.stdout_str()
            .lines()
            .filter(|&line| line == "FOO=bar" || line == "ABC=xyz")
            .count(),
        2
    );
}

#[test]
fn test_ignore_environment() {
    let scene = TestScenario::new(util_name!());

    scene.ucmd().arg("-i").succeeds().no_stdout();
    scene.ucmd().arg("-").succeeds().no_stdout();
}

#[test]
fn test_empty_name() {
    new_ucmd!()
        .arg("-i")
        .arg("=xyz")
        .run()
        .stderr_only("env: warning: no name specified for value 'xyz'\n");
}

#[test]
fn test_null_delimiter() {
    let out = new_ucmd!()
        .arg("-i")
        .arg("--null")
        .arg("FOO=bar")
        .arg("ABC=xyz")
        .succeeds()
        .stdout_move_str();

    let mut vars: Vec<_> = out.split('\0').collect();
    assert_eq!(vars.len(), 3);
    vars.sort_unstable();
    assert_eq!(vars[0], "");
    assert_eq!(vars[1], "ABC=xyz");
    assert_eq!(vars[2], "FOO=bar");
}

#[test]
fn test_unset_variable() {
    let out = TestScenario::new(util_name!())
        .ucmd()
        .env("HOME", "FOO")
        .arg("-u")
        .arg("HOME")
        .succeeds()
        .stdout_move_str();

    assert!(!out.lines().any(|line| line.starts_with("HOME=")));
}

#[test]
fn test_fail_null_with_program() {
    new_ucmd!()
        .arg("--null")
        .arg("cd")
        .fails()
        .stderr_contains("cannot specify --null (-0) with command");
}

#[cfg(not(windows))]
#[test]
fn test_change_directory() {
    let scene = TestScenario::new(util_name!());
    let temporary_directory = tempdir().unwrap();
    let temporary_path = std::fs::canonicalize(temporary_directory.path()).unwrap();
    assert_ne!(env::current_dir().unwrap(), temporary_path);

    // command to print out current working directory
    let pwd = "pwd";

    let out = scene
        .ucmd()
        .arg("--chdir")
        .arg(&temporary_path)
        .arg(pwd)
        .succeeds()
        .stdout_move_str();
    assert_eq!(out.trim(), temporary_path.as_os_str());
}

#[cfg(windows)]
#[test]
fn test_change_directory() {
    let scene = TestScenario::new(util_name!());
    let temporary_directory = tempdir().unwrap();

    let temporary_path = temporary_directory.path();
    let temporary_path = temporary_path
        .strip_prefix(r"\\?\")
        .unwrap_or(temporary_path);

    let env_cd = env::current_dir().unwrap();
    let env_cd = env_cd.strip_prefix(r"\\?\").unwrap_or(&env_cd);

    assert_ne!(env_cd, temporary_path);

    // COMSPEC is a variable that contains the full path to cmd.exe
    let cmd_path = env::var("COMSPEC").unwrap();

    // command to print out current working directory
    let pwd = [&*cmd_path, "/C", "cd"];

    let out = scene
        .ucmd()
        .arg("--chdir")
        .arg(temporary_path)
        .args(&pwd)
        .succeeds()
        .stdout_move_str();
    assert_eq!(out.trim(), temporary_path.as_os_str());
}

#[test]
fn test_fail_change_directory() {
    let scene = TestScenario::new(util_name!());
    let some_non_existing_path = "some_nonexistent_path";
    assert!(!Path::new(some_non_existing_path).is_dir());

    let out = scene
        .ucmd()
        .arg("--chdir")
        .arg(some_non_existing_path)
        .arg("pwd")
        .fails()
        .stderr_move_str();
    assert!(out.contains("env: cannot change directory to "));
}

#[test]
fn test_split_string_into_args_one_argument_no_quotes() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .arg("-S echo hello world")
        .succeeds()
        .stdout_move_str();
    assert_eq!(out, "hello world\n");
}

#[test]
fn test_split_string_into_args_one_argument() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .arg("-S echo \"hello world\"")
        .succeeds()
        .stdout_move_str();
    assert_eq!(out, "hello world\n");
}

#[test]
fn test_split_string_into_args_s_escaping_challenge() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .args(&[r#"-S echo "hello \"great\" world""#])
        .succeeds()
        .stdout_move_str();
    assert_eq!(out, "hello \"great\" world\n");
}

#[test]
fn test_split_string_into_args_s_escaped_c_not_allowed() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .args(&[r#"-S'"\\c"'"#])
        .fails()
        .stderr_move_str();
    assert_eq!(out, "env: '\\c' must not appear in double-quoted -S string\n");
}

#[test]
fn test_split_string_into_args_s_whitespace_handling() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .args(&["-Sprintf x%sx\\n A \t B \x0B\x0C\r\n"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(out, "xAx\nxBx\n");
}

#[test]
fn test_split_string_into_args_long_option_whitespace_handling() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .args(&["--split-string printf x%sx\\n A \t B \x0B\x0C\r\n"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(out, "xAx\nxBx\n");
}

#[test]
fn test_split_string_into_args_debug_output_whitespace_handling() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .args(&["-vS printf x%sx\\n A \t B \x0B\x0C\r\n"])
        .succeeds();
    assert_eq!(out.stdout_str(), "xAx\nxBx\n");
    assert_eq!(out.stderr_str(), "input args:\narg[0]: env\narg[1]: -vS printf x%sx\\n A \t B \u{b}\u{c}\r\n\nexecutable: printf\narg[0]: x%sx\n\narg[1]: A\narg[2]: B\n");
}

#[test]
fn test_split_string_misc() {
    use ::env::parse_args_from_str;

    assert_eq!(
        vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"],
        parse_args_from_str(r#"A=B FOO=AR  sh -c "echo \$A\$FOO""#).0,
    );
    assert_eq!(
        vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"],
        parse_args_from_str(r#"A=B FOO=AR  sh -c 'echo $A$FOO'"#).0,
    );
    assert_eq!(
        vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"],
        parse_args_from_str(r#"A=B FOO=AR  sh -c 'echo $A$FOO'"#).0,
    );

    assert_eq!(
        vec!["-i", "A=B ' C"],
        parse_args_from_str(r#"-i A='B \' C'"#).0,
    );
}

#[test]
fn test_split_string_environment_vars_test() {
    /*assert_eq!(
        vec!["FOO=BAR", "sh", "-c", "echo xBARx =$FOO="],
        ::env::parse_args_from_str(r#"FOO=BAR sh -c "echo x${FOO}x =\$FOO=""#).0,
    );*/
}
