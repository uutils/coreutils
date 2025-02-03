// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) bamf chdir rlimit prlimit COMSPEC cout cerr FFFD
#![allow(clippy::missing_errors_doc)]

#[cfg(unix)]
use nix::sys::signal::Signal;
#[cfg(feature = "echo")]
use regex::Regex;
use std::env;
use std::path::Path;
#[cfg(unix)]
use std::process::Command;
use tempfile::tempdir;
use uutests::new_ucmd;
use uutests::util::TestScenario;
#[cfg(unix)]
use uutests::util::UChild;
use uutests::util_name;

#[cfg(unix)]
struct Target {
    child: UChild,
}
#[cfg(unix)]
impl Target {
    fn new(signals: &[&str]) -> Self {
        let mut child = new_ucmd!()
            .args(&[
                format!("--ignore-signal={}", signals.join(",")).as_str(),
                "sleep",
                "1000",
            ])
            .run_no_wait();
        child.delay(500);
        Self { child }
    }
    fn send_signal(&mut self, signal: Signal) {
        let _ = Command::new("kill")
            .args(&[
                format!("-{}", signal as i32),
                format!("{}", self.child.id()),
            ])
            .spawn()
            .expect("failed to send signal")
            .wait();
        self.child.delay(100);
    }
    fn is_alive(&mut self) -> bool {
        self.child.is_alive()
    }
}
#[cfg(unix)]
impl Drop for Target {
    fn drop(&mut self) {
        self.child.kill();
    }
}

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
fn test_env_permissions() {
    new_ucmd!()
        .arg(".")
        .fails()
        .code_is(126)
        .stderr_is("env: '.': Permission denied\n");
}

#[test]
fn test_echo() {
    #[cfg(target_os = "windows")]
    let args = ["cmd", "/d/c", "echo"];
    #[cfg(not(target_os = "windows"))]
    let args = ["echo"];

    let result = new_ucmd!().args(&args).arg("FOO-bar").succeeds();

    assert_eq!(result.stdout_str().trim(), "FOO-bar");
}

#[cfg(target_os = "windows")]
#[test]
fn test_if_windows_batch_files_can_be_executed() {
    let result = new_ucmd!().arg("./runBat.bat").succeeds();

    assert!(result.stdout_str().contains("Hello Windows World!"));
}

#[cfg(feature = "echo")]
#[test]
fn test_debug_1() {
    let ts = TestScenario::new(util_name!());
    let result = ts
        .ucmd()
        .arg("-v")
        .arg(&ts.bin_path)
        .args(&["echo", "hello"])
        .succeeds();
    result.stderr_matches(
        &Regex::new(concat!(
            r"executing: [^\n]+(\/|\\)coreutils(\.exe)?\n",
            r"   arg\[0\]= '[^\n]+(\/|\\)coreutils(\.exe)?'\n",
            r"   arg\[1\]= 'echo'\n",
            r"   arg\[2\]= 'hello'"
        ))
        .unwrap(),
    );
}

#[cfg(feature = "echo")]
#[test]
fn test_debug_2() {
    let ts = TestScenario::new(util_name!());
    let result = ts
        .ucmd()
        .arg("-vv")
        .arg(&ts.bin_path)
        .args(&["echo", "hello2"])
        .succeeds();
    result.stderr_matches(
        &Regex::new(concat!(
            r"input args:\n",
            r"arg\[0\]: 'env'\n",
            r"arg\[1\]: '-vv'\n",
            r"arg\[2\]: '[^\n]+(\/|\\)coreutils(.exe)?'\n",
            r"arg\[3\]: 'echo'\n",
            r"arg\[4\]: 'hello2'\n",
            r"executing: [^\n]+(\/|\\)coreutils(.exe)?\n",
            r"   arg\[0\]= '[^\n]+(\/|\\)coreutils(.exe)?'\n",
            r"   arg\[1\]= 'echo'\n",
            r"   arg\[2\]= 'hello2'"
        ))
        .unwrap(),
    );
}

#[cfg(feature = "echo")]
#[test]
fn test_debug1_part_of_string_arg() {
    let ts = TestScenario::new(util_name!());

    let result = ts
        .ucmd()
        .arg("-vS FOO=BAR")
        .arg(&ts.bin_path)
        .args(&["echo", "hello1"])
        .succeeds();
    result.stderr_matches(
        &Regex::new(concat!(
            r"executing: [^\n]+(\/|\\)coreutils(\.exe)?\n",
            r"   arg\[0\]= '[^\n]+(\/|\\)coreutils(\.exe)?'\n",
            r"   arg\[1\]= 'echo'\n",
            r"   arg\[2\]= 'hello1'"
        ))
        .unwrap(),
    );
}

#[cfg(feature = "echo")]
#[test]
fn test_debug2_part_of_string_arg() {
    let ts = TestScenario::new(util_name!());
    let result = ts
        .ucmd()
        .arg("-vvS FOO=BAR")
        .arg(&ts.bin_path)
        .args(&["echo", "hello2"])
        .succeeds();
    result.stderr_matches(
        &Regex::new(concat!(
            r"input args:\n",
            r"arg\[0\]: 'env'\n",
            r"arg\[1\]: '-vvS FOO=BAR'\n",
            r"arg\[2\]: '[^\n]+(\/|\\)coreutils(.exe)?'\n",
            r"arg\[3\]: 'echo'\n",
            r"arg\[4\]: 'hello2'\n",
            r"executing: [^\n]+(\/|\\)coreutils(.exe)?\n",
            r"   arg\[0\]= '[^\n]+(\/|\\)coreutils(.exe)?'\n",
            r"   arg\[1\]= 'echo'\n",
            r"   arg\[2\]= 'hello2'"
        ))
        .unwrap(),
    );
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

#[cfg(not(target_os = "windows"))] // windows has no executable "echo", its only supported as part of a batch-file
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

#[cfg(not(target_os = "windows"))] // windows has no executable "echo", its only supported as part of a batch-file
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

#[cfg(not(target_os = "windows"))] // windows has no executable "echo", its only supported as part of a batch-file
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

    let out = scene.ucmd().args(&[r#"-S"\c""#]).fails().stderr_move_str();
    assert_eq!(
        out,
        "env: '\\c' must not appear in double-quoted -S string\n"
    );
}

#[cfg(not(target_os = "windows"))] // no printf available
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

#[cfg(not(target_os = "windows"))] // no printf available
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

#[cfg(not(target_os = "windows"))] // no printf available
#[test]
fn test_split_string_into_args_debug_output_whitespace_handling() {
    let scene = TestScenario::new(util_name!());

    let out = scene
        .ucmd()
        .args(&["-vvS printf x%sx\\n A \t B \x0B\x0C\r\n"])
        .succeeds();
    assert_eq!(out.stdout_str(), "xAx\nxBx\n");
    assert_eq!(
        out.stderr_str(),
        "input args:\narg[0]: 'env'\narg[1]: $\
        '-vvS printf x%sx\\\\n A \\t B \\x0B\\x0C\\r\\n'\nexecuting: printf\
        \n   arg[0]= 'printf'\n   arg[1]= $'x%sx\\n'\n   arg[2]= 'A'\n   arg[3]= 'B'\n"
    );
}

// FixMe: This test fails on MACOS:
// thread 'test_env::test_gnu_e20' panicked at 'assertion failed: `(left == right)`
// left: `"A=B C=D\n__CF_USER_TEXT_ENCODING=0x1F5:0x0:0x0\n"`,
// right: `"A=B C=D\n"`', tests/by-util/test_env.rs:369:5
#[cfg(not(target_os = "macos"))]
#[test]
fn test_gnu_e20() {
    let scene = TestScenario::new(util_name!());

    let env_bin = String::from(uutests::util::get_tests_binary().as_str()) + " " + util_name!();

    let (input, output) = (
        [
            String::from("-i"),
            String::from(r#"-SA="B\_C=D" "#) + env_bin.escape_default().to_string().as_str() + "",
        ],
        "A=B C=D\n",
    );

    let out = scene.ucmd().args(&input).succeeds();
    assert_eq!(out.stdout_str(), output);
}

#[test]
#[allow(clippy::cognitive_complexity)] // Ignore clippy lint of too long function sign
fn test_env_parsing_errors() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("-S\\|echo hallo") // no quotes, invalid escape sequence |
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\|' in -S\n");

    ts.ucmd()
        .arg("-S\\a") // no quotes, invalid escape sequence a
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S\n");

    ts.ucmd()
        .arg("-S\"\\a\"") // double quotes, invalid escape sequence a
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S\n");

    ts.ucmd()
        .arg(r#"-S"\a""#) // same as before, just using r#""#
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S\n");

    ts.ucmd()
        .arg("-S'\\a'") // single quotes, invalid escape sequence a
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S\n");

    ts.ucmd()
        .arg(r"-S\|\&\;") // no quotes, invalid escape sequence |
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\|' in -S\n");

    ts.ucmd()
        .arg(r"-S\<\&\;") // no quotes, invalid escape sequence <
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\<' in -S\n");

    ts.ucmd()
        .arg(r"-S\>\&\;") // no quotes, invalid escape sequence >
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\>' in -S\n");

    ts.ucmd()
        .arg(r"-S\`\&\;") // no quotes, invalid escape sequence `
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S\n");

    ts.ucmd()
        .arg(r#"-S"\`\&\;""#) // double quotes, invalid escape sequence `
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S\n");

    ts.ucmd()
        .arg(r"-S'\`\&\;'") // single quotes, invalid escape sequence `
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S\n");

    ts.ucmd()
        .arg(r"-S\`") // ` escaped without quotes
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S\n");

    ts.ucmd()
        .arg(r#"-S"\`""#) // ` escaped in double quotes
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S\n");

    ts.ucmd()
        .arg(r"-S'\`'") // ` escaped in single quotes
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S\n");

    ts.ucmd()
        .args(&[r"-S\🦉"]) // ` escaped in single quotes
        .fails()
        .code_is(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\\u{FFFD}' in -S\n"); // gnu doesn't show the owl. Instead a invalid unicode ?
}

#[test]
fn test_env_with_empty_executable_single_quotes() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&["-S''"]) // empty single quotes, considered as program name
        .fails()
        .code_is(127)
        .no_stdout()
        .stderr_is("env: '': No such file or directory\n"); // gnu version again adds escaping here
}

#[test]
fn test_env_with_empty_executable_double_quotes() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&["-S\"\""]) // empty double quotes, considered as program name
        .fails()
        .code_is(127)
        .no_stdout()
        .stderr_is("env: '': No such file or directory\n");
}

#[test]
#[cfg(all(unix, feature = "dirname", feature = "echo"))]
fn test_env_overwrite_arg0() {
    let ts = TestScenario::new(util_name!());

    let bin = ts.bin_path.clone();

    ts.ucmd()
        .args(&["--argv0", "echo"])
        .arg(&bin)
        .args(&["-n", "hello", "world!"])
        .succeeds()
        .stdout_is("hello world!")
        .stderr_is("");

    ts.ucmd()
        .args(&["-a", "dirname"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb\n")
        .stderr_is("");
}

#[test]
#[cfg(all(unix, feature = "echo"))]
fn test_env_arg_argv0_overwrite() {
    let ts = TestScenario::new(util_name!());

    let bin = &ts.bin_path;

    // overwrite --argv0 by --argv0
    ts.ucmd()
        .args(&["--argv0", "dirname"])
        .args(&["--argv0", "echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // overwrite -a by -a
    ts.ucmd()
        .args(&["-a", "dirname"])
        .args(&["-a", "echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // overwrite --argv0 by -a
    ts.ucmd()
        .args(&["--argv0", "dirname"])
        .args(&["-a", "echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // overwrite -a by --argv0
    ts.ucmd()
        .args(&["-a", "dirname"])
        .args(&["--argv0", "echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");
}

#[test]
#[cfg(all(unix, feature = "echo"))]
fn test_env_arg_argv0_overwrite_mixed_with_string_args() {
    let ts = TestScenario::new(util_name!());

    let bin = &ts.bin_path;

    // string arg following normal
    ts.ucmd()
        .args(&["-S--argv0 dirname"])
        .args(&["--argv0", "echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // normal following string arg
    ts.ucmd()
        .args(&["-a", "dirname"])
        .args(&["-S-a echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // one large string arg
    ts.ucmd()
        .args(&["-S--argv0 dirname -a echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // two string args
    ts.ucmd()
        .args(&["-S-a dirname"])
        .args(&["-S--argv0 echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");

    // three args: normal, string, normal
    ts.ucmd()
        .args(&["-a", "sleep"])
        .args(&["-S-a dirname"])
        .args(&["-a", "echo"])
        .arg(bin)
        .args(&["aa/bb/cc"])
        .succeeds()
        .stdout_is("aa/bb/cc\n")
        .stderr_is("");
}

#[test]
#[cfg(unix)]
fn test_env_arg_ignore_signal_invalid_signals() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .args(&["--ignore-signal=banana"])
        .fails()
        .code_is(125)
        .stderr_contains("env: 'banana': invalid signal");
    ts.ucmd()
        .args(&["--ignore-signal=SIGbanana"])
        .fails()
        .code_is(125)
        .stderr_contains("env: 'SIGbanana': invalid signal");
    ts.ucmd()
        .args(&["--ignore-signal=exit"])
        .fails()
        .code_is(125)
        .stderr_contains("env: 'exit': invalid signal");
    ts.ucmd()
        .args(&["--ignore-signal=SIGexit"])
        .fails()
        .code_is(125)
        .stderr_contains("env: 'SIGexit': invalid signal");
}

#[test]
#[cfg(unix)]
fn test_env_arg_ignore_signal_special_signals() {
    let ts = TestScenario::new(util_name!());
    let signal_stop = nix::sys::signal::SIGSTOP;
    let signal_kill = nix::sys::signal::SIGKILL;
    ts.ucmd()
        .args(&["--ignore-signal=stop", "echo", "hello"])
        .fails()
        .code_is(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_stop as i32
        ));
    ts.ucmd()
        .args(&["--ignore-signal=kill", "echo", "hello"])
        .fails()
        .code_is(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_kill as i32
        ));
    ts.ucmd()
        .args(&["--ignore-signal=SToP", "echo", "hello"])
        .fails()
        .code_is(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_stop as i32
        ));
    ts.ucmd()
        .args(&["--ignore-signal=SIGKILL", "echo", "hello"])
        .fails()
        .code_is(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_kill as i32
        ));
}

#[test]
#[cfg(unix)]
fn test_env_arg_ignore_signal_valid_signals() {
    {
        let mut target = Target::new(&["int"]);
        target.send_signal(Signal::SIGINT);
        assert!(target.is_alive());
    }
    {
        let mut target = Target::new(&["usr2"]);
        target.send_signal(Signal::SIGUSR2);
        assert!(target.is_alive());
    }
    {
        let mut target = Target::new(&["int", "usr2"]);
        target.send_signal(Signal::SIGUSR1);
        assert!(!target.is_alive());
    }
}

#[test]
#[cfg(unix)]
fn test_env_arg_ignore_signal_empty() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .args(&["--ignore-signal=", "echo", "hello"])
        .succeeds()
        .no_stderr()
        .stdout_contains("hello");
}

#[test]
fn disallow_equals_sign_on_short_unset_option() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("-u=")
        .fails()
        .code_is(125)
        .stderr_contains("env: cannot unset '=': Invalid argument");
    ts.ucmd()
        .arg("-u=A1B2C3")
        .fails()
        .code_is(125)
        .stderr_contains("env: cannot unset '=A1B2C3': Invalid argument");
    ts.ucmd().arg("--split-string=A1B=2C3=").succeeds();
    ts.ucmd()
        .arg("--unset=")
        .fails()
        .code_is(125)
        .stderr_contains("env: cannot unset '': Invalid argument");
}

#[cfg(test)]
mod tests_split_iterator {

    enum EscapeStyle {
        /// No escaping.
        None,
        /// Wrap in single quotes.
        SingleQuoted,
        /// Single quotes combined with backslash.
        Mixed,
    }

    /// Determines escaping style to use.
    fn escape_style(s: &str) -> EscapeStyle {
        if s.is_empty() {
            return EscapeStyle::SingleQuoted;
        }

        let mut special = false;
        let mut newline = false;
        let mut single_quote = false;

        for c in s.chars() {
            match c {
                '\n' => {
                    newline = true;
                    special = true;
                }
                '\'' => {
                    single_quote = true;
                    special = true;
                }
                '|' | '&' | ';' | '<' | '>' | '(' | ')' | '$' | '`' | '\\' | '"' | ' ' | '\t'
                | '*' | '?' | '[' | '#' | '˜' | '=' | '%' => {
                    special = true;
                }
                _ => (),
            }
        }

        if !special {
            EscapeStyle::None
        } else if newline && !single_quote {
            EscapeStyle::SingleQuoted
        } else {
            EscapeStyle::Mixed
        }
    }

    /// Escapes special characters in a string, so that it will retain its literal
    /// meaning when used as a part of command in Unix shell.
    ///
    /// It tries to avoid introducing any unnecessary quotes or escape characters,
    /// but specifics regarding quoting style are left unspecified.
    pub fn quote(s: &str) -> std::borrow::Cow<str> {
        // We are going somewhat out of the way to provide
        // minimal amount of quoting in typical cases.
        match escape_style(s) {
            EscapeStyle::None => s.into(),
            EscapeStyle::SingleQuoted => format!("'{s}'").into(),
            EscapeStyle::Mixed => {
                let mut quoted = String::new();
                quoted.push('\'');
                for c in s.chars() {
                    if c == '\'' {
                        quoted.push_str("'\\''");
                    } else {
                        quoted.push(c);
                    }
                }
                quoted.push('\'');
                quoted.into()
            }
        }
    }

    /// Joins arguments into a single command line suitable for execution in Unix
    /// shell.
    ///
    /// Each argument is quoted using [`quote`] to preserve its literal meaning when
    /// parsed by Unix shell.
    ///
    /// Note: This function is essentially an inverse of [`split`].
    ///
    /// # Examples
    ///
    /// Logging executed commands in format that can be easily copied and pasted
    /// into an actual shell:
    ///
    /// ```rust,no_run
    /// fn execute(args: &[&str]) {
    ///     use std::process::Command;
    ///     println!("Executing: {}", shell_words::join(args));
    ///     Command::new(&args[0])
    ///         .args(&args[1..])
    ///         .spawn()
    ///         .expect("failed to start subprocess")
    ///         .wait()
    ///         .expect("failed to wait for subprocess");
    /// }
    ///
    /// execute(&["python", "-c", "print('Hello world!')"]);
    /// ```
    ///
    /// [`quote`]: fn.quote.html
    /// [`split`]: fn.split.html
    pub fn join<I, S>(words: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut line = words.into_iter().fold(String::new(), |mut line, word| {
            let quoted = quote(word.as_ref());
            line.push_str(quoted.as_ref());
            line.push(' ');
            line
        });
        line.pop();
        line
    }

    use std::ffi::OsString;

    use env::native_int_str::{from_native_int_representation_owned, Convert, NCvt};
    use env::parse_error::ParseError;

    fn split(input: &str) -> Result<Vec<OsString>, ParseError> {
        ::env::split_iterator::split(&NCvt::convert(input)).map(|vec| {
            vec.into_iter()
                .map(from_native_int_representation_owned)
                .collect()
        })
    }

    fn split_ok(cases: &[(&str, &[&str])]) {
        for (i, &(input, expected)) in cases.iter().enumerate() {
            match split(input) {
                Err(actual) => {
                    panic!(
                        "[{i}] calling split({input:?}):\nexpected: Ok({expected:?})\n  actual: Err({actual:?})\n"
                    );
                }
                Ok(actual) => {
                    assert!(
                        expected == actual.as_slice(),
                        "[{i}] After split({input:?}).unwrap()\nexpected: {expected:?}\n  actual: {actual:?}\n"
                    );
                }
            }
        }
    }

    #[test]
    fn split_empty() {
        split_ok(&[("", &[])]);
    }

    #[test]
    fn split_initial_whitespace_is_removed() {
        split_ok(&[
            ("     a", &["a"]),
            ("\t\t\t\tbar", &["bar"]),
            ("\t \nc", &["c"]),
        ]);
    }

    #[test]
    fn split_trailing_whitespace_is_removed() {
        split_ok(&[
            ("a  ", &["a"]),
            ("b\t", &["b"]),
            ("c\t \n \n \n", &["c"]),
            ("d\n\n", &["d"]),
        ]);
    }

    #[test]
    fn split_carriage_return() {
        split_ok(&[("c\ra\r'\r'\r", &["c", "a", "\r"])]);
    }

    #[test]
    fn split_() {
        split_ok(&[("\\'\\'", &["''"])]);
    }

    #[test]
    fn split_single_quotes() {
        split_ok(&[
            (r"''", &[r""]),
            (r"'a'", &[r"a"]),
            (r"'\\'", &[r"\"]),
            (r"' \\ '", &[r" \ "]),
            (r"'#'", &[r"#"]),
        ]);
    }

    #[test]
    fn split_double_quotes() {
        split_ok(&[
            (r#""""#, &[""]),
            (r#""""""#, &[""]),
            (r#""a b c' d""#, &["a b c' d"]),
            (r#""\$""#, &["$"]),
            (r#""`""#, &["`"]),
            (r#""\"""#, &["\""]),
            (r#""\\""#, &["\\"]),
            ("\"\n\"", &["\n"]),
            ("\"\\\n\"", &[""]),
        ]);
    }

    #[test]
    fn split_unquoted() {
        split_ok(&[
            (r"\\|\\&\\;", &[r"\|\&\;"]),
            (r"\\<\\>", &[r"\<\>"]),
            (r"\\(\\)", &[r"\(\)"]),
            (r"\$", &[r"$"]),
            (r#"\""#, &[r#"""#]),
            (r"\'", &[r"'"]),
            ("\\\n", &[]),
            (" \\\n \n", &[]),
            ("a\nb\nc", &["a", "b", "c"]),
            ("a\\\nb\\\nc", &["abc"]),
            ("foo bar baz", &["foo", "bar", "baz"]),
        ]);
    }

    #[test]
    fn split_trailing_backslash() {
        assert_eq!(
            split("\\"),
            Err(ParseError::InvalidBackslashAtEndOfStringInMinusS {
                pos: 1,
                quoting: "Delimiter".into()
            })
        );
        assert_eq!(
            split(" \\"),
            Err(ParseError::InvalidBackslashAtEndOfStringInMinusS {
                pos: 2,
                quoting: "Delimiter".into()
            })
        );
        assert_eq!(
            split("a\\"),
            Err(ParseError::InvalidBackslashAtEndOfStringInMinusS {
                pos: 2,
                quoting: "Unquoted".into()
            })
        );
    }

    #[test]
    fn split_errors() {
        assert_eq!(
            split("'abc"),
            Err(ParseError::MissingClosingQuote { pos: 4, c: '\'' })
        );
        assert_eq!(
            split("\""),
            Err(ParseError::MissingClosingQuote { pos: 1, c: '"' })
        );
        assert_eq!(
            split("'\\"),
            Err(ParseError::MissingClosingQuote { pos: 2, c: '\'' })
        );
        assert_eq!(
            split("'\\"),
            Err(ParseError::MissingClosingQuote { pos: 2, c: '\'' })
        );
        assert_eq!(
            split(r#""$""#),
            Err(ParseError::ParsingOfVariableNameFailed {
                pos: 2,
                msg: "Missing variable name".into()
            }),
        );
    }

    #[test]
    fn split_error_fail_with_unknown_escape_sequences() {
        assert_eq!(
            split("\\a"),
            Err(ParseError::InvalidSequenceBackslashXInMinusS { pos: 1, c: 'a' })
        );
        assert_eq!(
            split("\"\\a\""),
            Err(ParseError::InvalidSequenceBackslashXInMinusS { pos: 2, c: 'a' })
        );
        assert_eq!(
            split("'\\a'"),
            Err(ParseError::InvalidSequenceBackslashXInMinusS { pos: 2, c: 'a' })
        );
        assert_eq!(
            split(r#""\a""#),
            Err(ParseError::InvalidSequenceBackslashXInMinusS { pos: 2, c: 'a' })
        );
        assert_eq!(
            split(r"\🦉"),
            Err(ParseError::InvalidSequenceBackslashXInMinusS {
                pos: 1,
                c: '\u{FFFD}'
            })
        );
    }

    #[test]
    fn split_comments() {
        split_ok(&[
            (r" x # comment ", &["x"]),
            (r" w1#w2 ", &["w1#w2"]),
            (r"'not really a # comment'", &["not really a # comment"]),
            (" a # very long comment \n b # another comment", &["a", "b"]),
        ]);
    }

    #[test]
    fn test_quote() {
        assert_eq!(quote(""), "''");
        assert_eq!(quote("'"), "''\\'''");
        assert_eq!(quote("abc"), "abc");
        assert_eq!(quote("a \n  b"), "'a \n  b'");
        assert_eq!(quote("X'\nY"), "'X'\\''\nY'");
    }

    #[test]
    fn test_join() {
        assert_eq!(join(["a", "b", "c"]), "a b c");
        assert_eq!(join([" ", "$", "\n"]), "' ' '$' '\n'");
    }

    #[test]
    fn join_followed_by_split_is_identity() {
        let cases: Vec<&[&str]> = vec![
            &["a"],
            &["python", "-c", "print('Hello world!')"],
            &["echo", " arg with spaces ", "arg \' with \" quotes"],
            &["even newlines are quoted correctly\n", "\n", "\n\n\t "],
            &["$", "`test`"],
            &["cat", "~user/log*"],
            &["test", "'a \"b", "\"X'"],
            &["empty", "", "", ""],
        ];
        for argv in cases {
            let args = join(argv);
            assert_eq!(split(&args).unwrap(), argv);
        }
    }
}

mod test_raw_string_parser {
    use std::{
        borrow::Cow,
        ffi::{OsStr, OsString},
    };

    use env::{
        native_int_str::{
            from_native_int_representation, from_native_int_representation_owned,
            to_native_int_representation, NativeStr,
        },
        string_expander::StringExpander,
        string_parser,
    };

    const LEN_OWL: usize = if cfg!(target_os = "windows") { 2 } else { 4 };

    #[test]
    fn test_ascii_only_take_one_look_at_correct_data_and_end_behavior() {
        let input = "hello";
        let cow = to_native_int_representation(OsStr::new(input));
        let mut uut = StringExpander::new(&cow);
        for c in input.chars() {
            assert_eq!(c, uut.get_parser().peek().unwrap());
            uut.take_one().unwrap();
        }
        assert_eq!(
            uut.get_parser().peek(),
            Err(string_parser::Error {
                peek_position: 5,
                err_type: string_parser::ErrorType::EndOfInput
            })
        );
        uut.take_one().unwrap_err();
        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            input
        );
        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            ""
        );
    }

    #[test]
    fn test_multi_byte_codes_take_one_look_at_correct_data_and_end_behavior() {
        let input = OsString::from("🦉🦉🦉x🦉🦉x🦉x🦉🦉🦉🦉");
        let cow = to_native_int_representation(input.as_os_str());
        let mut uut = StringExpander::new(&cow);
        for _i in 0..3 {
            assert_eq!(uut.get_parser().peek().unwrap(), '\u{FFFD}');
            uut.take_one().unwrap();
            assert_eq!(uut.get_parser().peek().unwrap(), 'x');
            uut.take_one().unwrap();
        }
        assert_eq!(uut.get_parser().peek().unwrap(), '\u{FFFD}');
        uut.take_one().unwrap();
        assert_eq!(
            uut.get_parser().peek(),
            Err(string_parser::Error {
                peek_position: 10 * LEN_OWL + 3,
                err_type: string_parser::ErrorType::EndOfInput
            })
        );
        uut.take_one().unwrap_err();
        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            input
        );
        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            ""
        );
    }

    #[test]
    fn test_multi_byte_codes_put_one_ascii_start_middle_end_try_invalid_ascii() {
        let input = OsString::from("🦉🦉🦉x🦉🦉x🦉x🦉🦉🦉🦉");
        let cow = to_native_int_representation(input.as_os_str());
        let owl: char = '🦉';
        let mut uut = StringExpander::new(&cow);
        uut.put_one_char('a');
        for _i in 0..3 {
            assert_eq!(uut.get_parser().peek().unwrap(), '\u{FFFD}');
            uut.take_one().unwrap();
            uut.put_one_char('a');
            assert_eq!(uut.get_parser().peek().unwrap(), 'x');
            uut.take_one().unwrap();
            uut.put_one_char('a');
        }
        assert_eq!(uut.get_parser().peek().unwrap(), '\u{FFFD}');
        uut.take_one().unwrap();
        uut.put_one_char(owl);
        uut.put_one_char('a');
        assert_eq!(
            uut.get_parser().peek(),
            Err(string_parser::Error {
                peek_position: LEN_OWL * 10 + 3,
                err_type: string_parser::ErrorType::EndOfInput
            })
        );
        uut.take_one().unwrap_err();
        uut.put_one_char('a');
        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            "a🦉🦉🦉axa🦉🦉axa🦉axa🦉🦉🦉🦉🦉aa"
        );
        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            ""
        );
    }

    #[test]
    fn test_multi_byte_codes_skip_one_take_one_skip_until_ascii_char_or_end() {
        let input = OsString::from("🦉🦉🦉x🦉🦉x🦉x🦉🦉🦉🦉");
        let cow = to_native_int_representation(input.as_os_str());
        let mut uut = StringExpander::new(&cow);

        uut.skip_one().unwrap(); // skip 🦉🦉🦉
        let p = LEN_OWL * 3;
        assert_eq!(uut.get_peek_position(), p);

        uut.skip_one().unwrap(); // skip x
        assert_eq!(uut.get_peek_position(), p + 1);
        uut.take_one().unwrap(); // take 🦉🦉
        let p = p + 1 + LEN_OWL * 2;
        assert_eq!(uut.get_peek_position(), p);

        uut.skip_one().unwrap(); // skip x
        assert_eq!(uut.get_peek_position(), p + 1);
        uut.get_parser_mut().skip_until_char_or_end('x'); // skip 🦉
        let p = p + 1 + LEN_OWL;
        assert_eq!(uut.get_peek_position(), p);
        uut.take_one().unwrap(); // take x
        uut.get_parser_mut().skip_until_char_or_end('x'); // skip 🦉🦉🦉🦉 till end
        let p = p + 1 + LEN_OWL * 4;
        assert_eq!(uut.get_peek_position(), p);

        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            "🦉🦉x"
        );
    }

    #[test]
    fn test_multi_byte_codes_skip_multiple_ascii_bounded_good_and_bad() {
        let input = OsString::from("🦉🦉🦉x🦉🦉x🦉x🦉🦉🦉🦉");
        let cow = to_native_int_representation(input.as_os_str());
        let mut uut = StringExpander::new(&cow);

        uut.get_parser_mut().skip_multiple(0);
        assert_eq!(uut.get_peek_position(), 0);
        let p = LEN_OWL * 3;
        uut.get_parser_mut().skip_multiple(p); // skips 🦉🦉🦉
        assert_eq!(uut.get_peek_position(), p);

        uut.take_one().unwrap(); // take x
        assert_eq!(uut.get_peek_position(), p + 1);
        let step = LEN_OWL * 3 + 1;
        uut.get_parser_mut().skip_multiple(step); // skips 🦉🦉x🦉
        let p = p + 1 + step;
        assert_eq!(uut.get_peek_position(), p);
        uut.take_one().unwrap(); // take x

        assert_eq!(uut.get_peek_position(), p + 1);
        let step = 4 * LEN_OWL;
        uut.get_parser_mut().skip_multiple(step); // skips 🦉🦉🦉🦉
        let p = p + 1 + step;
        assert_eq!(uut.get_peek_position(), p);

        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            "xx"
        );
    }

    #[test]
    fn test_multi_byte_codes_put_string_utf8_start_middle_end() {
        let input = OsString::from("🦉🦉🦉x🦉🦉x🦉x🦉🦉🦉🦉");
        let cow = to_native_int_representation(input.as_os_str());
        let mut uut = StringExpander::new(&cow);

        uut.put_string("🦔oo");
        uut.take_one().unwrap(); // takes 🦉🦉🦉
        uut.put_string("oo🦔");
        uut.take_one().unwrap(); // take x
        uut.get_parser_mut().skip_until_char_or_end('\n'); // skips till end
        uut.put_string("o🦔o");

        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            "🦔oo🦉🦉🦉oo🦔xo🦔o"
        );
    }

    #[test]
    fn test_multi_byte_codes_look_at_remaining_start_middle_end() {
        let input = "🦉🦉🦉x🦉🦉x🦉x🦉🦉🦉🦉";
        let cow = to_native_int_representation(OsStr::new(input));
        let mut uut = StringExpander::new(&cow);

        assert_eq!(uut.get_parser().peek_remaining(), OsStr::new(input));
        uut.take_one().unwrap(); // takes 🦉🦉🦉
        assert_eq!(uut.get_parser().peek_remaining(), OsStr::new(&input[12..]));
        uut.get_parser_mut().skip_until_char_or_end('\n'); // skips till end
        assert_eq!(uut.get_parser().peek_remaining(), OsStr::new(""));

        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            "🦉🦉🦉"
        );
    }

    #[test]
    fn test_deal_with_invalid_encoding() {
        let owl_invalid_part;
        let (brace_1, brace_2);
        #[cfg(target_os = "windows")]
        {
            let mut buffer = [0u16; 2];
            let owl = '🦉'.encode_utf16(&mut buffer);
            owl_invalid_part = owl[0];
            brace_1 = '<'.encode_utf16(&mut buffer).to_vec();
            brace_2 = '>'.encode_utf16(&mut buffer).to_vec();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut buffer = [0u8; 4];
            let owl = '🦉'.encode_utf8(&mut buffer);
            owl_invalid_part = owl.bytes().next().unwrap();
            brace_1 = [b'<'].to_vec();
            brace_2 = [b'>'].to_vec();
        }
        let mut input_ux = brace_1;
        input_ux.push(owl_invalid_part);
        input_ux.extend(brace_2);
        let input_str = from_native_int_representation(Cow::Borrowed(&input_ux));
        let mut uut = StringExpander::new(&input_ux);

        assert_eq!(uut.get_parser().peek_remaining(), input_str);
        assert_eq!(uut.get_parser().peek().unwrap(), '<');
        uut.take_one().unwrap(); // takes "<"
        assert_eq!(
            uut.get_parser().peek_remaining(),
            NativeStr::new(&input_str).split_at(1).1
        );
        assert_eq!(uut.get_parser().peek().unwrap(), '\u{FFFD}');
        uut.take_one().unwrap(); // takes owl_b
        assert_eq!(
            uut.get_parser().peek_remaining(),
            NativeStr::new(&input_str).split_at(2).1
        );
        assert_eq!(uut.get_parser().peek().unwrap(), '>');
        uut.get_parser_mut().skip_until_char_or_end('\n');
        assert_eq!(uut.get_parser().peek_remaining(), OsStr::new(""));

        uut.take_one().unwrap_err();
        assert_eq!(
            from_native_int_representation_owned(uut.take_collected_output()),
            NativeStr::new(&input_str).split_at(2).0
        );
    }
}
