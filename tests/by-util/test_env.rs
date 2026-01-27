// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) bamf chdir rlimit prlimit COMSPEC cout cerr FFFD winsize xpixel ypixel Secho sighandler
#![allow(clippy::missing_errors_doc)]

#[cfg(unix)]
use nix::libc;
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
#[cfg(unix)]
use uutests::util::PATH;
#[cfg(unix)]
use uutests::util::TerminalSimulation;
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
        let mut cmd = new_ucmd!();
        if signals.is_empty() {
            cmd.arg("--ignore-signal");
        } else {
            cmd.arg(format!("--ignore-signal={}", signals.join(",")));
        }
        let mut child = cmd.args(&["sleep", "1000"]).run_no_wait();
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
    new_ucmd!().arg("--definitely-invalid").fails_with_code(125);
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_flags_after_command() {
    new_ucmd!()
        // This would cause an error if -u=v were processed because it's malformed
        .args(&["echo", "-u=v"])
        .succeeds()
        .no_stderr()
        .stdout_is("-u=v\n");

    new_ucmd!()
        // Ensure the string isn't split
        // cSpell:disable
        .args(&["printf", "%s-%s", "-Sfoo bar"])
        .succeeds()
        .no_stderr()
        .stdout_is("-Sfoo bar-");
    // cSpell:enable

    new_ucmd!()
        // Ensure -- is recognized
        .args(&["-i", "--", "-u=v"])
        .succeeds()
        .no_stderr()
        .stdout_is("-u=v\n");

    new_ucmd!()
        // Recognize echo as the command after a flag that takes a value
        .args(&["-C", "..", "echo", "-u=v"])
        .succeeds()
        .no_stderr()
        .stdout_is("-u=v\n");

    new_ucmd!()
        // Recognize echo as the command after a flag that takes an inline value
        .args(&["-C..", "echo", "-u=v"])
        .succeeds()
        .no_stderr()
        .stdout_is("-u=v\n");

    new_ucmd!()
        // Recognize echo as the command after a flag that takes a value after another flag
        .args(&["-iC", "..", "echo", "-u=v"])
        .succeeds()
        .no_stderr()
        .stdout_is("-u=v\n");

    new_ucmd!()
        // Similar to the last two combined
        .args(&["-iC..", "echo", "-u=v"])
        .succeeds()
        .no_stderr()
        .stdout_is("-u=v\n");
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
#[cfg(unix)]
fn test_env_permissions() {
    // Try to execute `empty` in test fixture, that does not have exec permission.
    new_ucmd!()
        .arg("./empty")
        .fails_with_code(126)
        .stderr_is("env: './empty': Permission denied\n");
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
        .succeeds()
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
        .succeeds()
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
        new_ucmd!().arg("-u").arg(var).fails().stderr_only(format!(
            "env: cannot unset {}: Invalid argument\n",
            var.quote()
        ));
    }
}

#[test]
fn test_single_name_value_pair() {
    new_ucmd!()
        .arg("FOO=bar")
        .succeeds()
        .stdout_str()
        .lines()
        .any(|line| line == "FOO=bar");
}

#[test]
fn test_multiple_name_value_pairs() {
    let out = new_ucmd!().arg("FOO=bar").arg("ABC=xyz").succeeds();

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
        .succeeds()
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
        "env: '\\c' must not appear in double-quoted -S string at position 2\n"
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

    let env_bin = String::from(uutests::util::get_tests_binary()) + " " + util_name!();
    let input = [
        String::from("-i"),
        String::from(r#"-SA="B\_C=D" "#) + env_bin.escape_default().to_string().as_str() + "",
    ];

    let mut output = "A=B C=D\n".to_string();

    // Workaround for the test to pass when coverage is being run.
    // If enabled, the binary called by env_bin will most probably be
    // instrumented for coverage, and thus will set the
    // __LLVM_PROFILE_RT_INIT_ONCE
    if env::var("__LLVM_PROFILE_RT_INIT_ONCE").is_ok() {
        output.push_str("__LLVM_PROFILE_RT_INIT_ONCE=__LLVM_PROFILE_RT_INIT_ONCE\n");
    }

    let out = scene.ucmd().args(&input).succeeds();
    assert_eq!(out.stdout_str(), output);
}

#[test]
#[allow(clippy::cognitive_complexity)] // Ignore clippy lint of too long function sign
fn test_env_parsing_errors() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("-S\\|echo hallo") // no quotes, invalid escape sequence |
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\|' in -S at position 1\n");

    ts.ucmd()
        .arg("-S\\a") // no quotes, invalid escape sequence a
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S at position 1\n");

    ts.ucmd()
        .arg("-S\"\\a\"") // double quotes, invalid escape sequence a
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S at position 2\n");

    ts.ucmd()
        .arg(r#"-S"\a""#) // same as before, just using r#""#
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S at position 2\n");

    ts.ucmd()
        .arg("-S'\\a'") // single quotes, invalid escape sequence a
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\a' in -S at position 2\n");

    ts.ucmd()
        .arg(r"-S\|\&\;") // no quotes, invalid escape sequence |
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\|' in -S at position 1\n");

    ts.ucmd()
        .arg(r"-S\<\&\;") // no quotes, invalid escape sequence <
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\<' in -S at position 1\n");

    ts.ucmd()
        .arg(r"-S\>\&\;") // no quotes, invalid escape sequence >
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\>' in -S at position 1\n");

    ts.ucmd()
        .arg(r"-S\`\&\;") // no quotes, invalid escape sequence `
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S at position 1\n");

    ts.ucmd()
        .arg(r#"-S"\`\&\;""#) // double quotes, invalid escape sequence `
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S at position 2\n");

    ts.ucmd()
        .arg(r"-S'\`\&\;'") // single quotes, invalid escape sequence `
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S at position 2\n");

    ts.ucmd()
        .arg(r"-S\`") // ` escaped without quotes
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S at position 1\n");

    ts.ucmd()
        .arg(r#"-S"\`""#) // ` escaped in double quotes
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S at position 2\n");

    ts.ucmd()
        .arg(r"-S'\`'") // ` escaped in single quotes
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\`' in -S at position 2\n");

    ts.ucmd()
        .args(&[r"-S\🦉"]) // ` escaped in single quotes
        .fails_with_code(125)
        .no_stdout()
        .stderr_is("env: invalid sequence '\\\u{FFFD}' in -S at position 1\n"); // gnu doesn't show the owl. Instead a invalid unicode ?
}

#[test]
fn test_env_with_empty_executable_single_quotes() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&["-S''"]) // empty single quotes, considered as program name
        .fails_with_code(127)
        .no_stdout()
        .stderr_is("env: '': No such file or directory\n"); // gnu version again adds escaping here
}

#[test]
fn test_env_with_empty_executable_double_quotes() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&["-S\"\""]) // empty double quotes, considered as program name
        .fails_with_code(127)
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
        .fails_with_code(125)
        .stderr_contains("env: 'banana': invalid signal");
    ts.ucmd()
        .args(&["--ignore-signal=SIGbanana"])
        .fails_with_code(125)
        .stderr_contains("env: 'SIGbanana': invalid signal");
    ts.ucmd()
        .args(&["--ignore-signal=exit"])
        .fails_with_code(125)
        .stderr_contains("env: 'exit': invalid signal");
    ts.ucmd()
        .args(&["--ignore-signal=SIGexit"])
        .fails_with_code(125)
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
        .fails_with_code(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_stop as i32
        ));
    ts.ucmd()
        .args(&["--ignore-signal=kill", "echo", "hello"])
        .fails_with_code(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_kill as i32
        ));
    ts.ucmd()
        .args(&["--ignore-signal=SToP", "echo", "hello"])
        .fails_with_code(125)
        .stderr_contains(format!(
            "env: failed to set signal action for signal {}: Invalid argument",
            signal_stop as i32
        ));
    ts.ucmd()
        .args(&["--ignore-signal=SIGKILL", "echo", "hello"])
        .fails_with_code(125)
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
#[cfg(unix)]
fn test_env_arg_ignore_signal_all_signals() {
    let mut target = Target::new(&[]);
    target.send_signal(Signal::SIGINT);
    assert!(target.is_alive());
}

#[test]
#[cfg(unix)]
fn test_env_default_signal_pipe() {
    let ts = TestScenario::new(util_name!());
    run_sigpipe_script(&ts, &["--default-signal=PIPE"]);
}

#[test]
#[cfg(unix)]
fn test_env_default_signal_all_signals() {
    let ts = TestScenario::new(util_name!());
    run_sigpipe_script(&ts, &["--default-signal"]);
}

#[test]
#[cfg(unix)]
fn test_env_block_signal_flag() {
    new_ucmd!()
        .env("PATH", PATH)
        .args(&["--block-signal", "true"])
        .succeeds()
        .no_stderr();
}

#[test]
#[cfg(unix)]
fn test_env_list_signal_handling_reports_ignore() {
    let result = new_ucmd!()
        .env("PATH", PATH)
        .args(&["--ignore-signal=INT", "--list-signal-handling", "true"])
        .succeeds();
    let stderr = result.stderr_str();
    assert!(
        stderr.contains("INT") && stderr.contains("IGNORE"),
        "unexpected signal listing: {stderr}"
    );
}

#[cfg(unix)]
fn run_sigpipe_script(ts: &TestScenario, extra_args: &[&str]) {
    let shell = env::var("SHELL").unwrap_or_else(|_| String::from("sh"));
    let _guard = SigpipeGuard::new();
    let mut cmd = ts.ucmd();
    cmd.env("PATH", PATH);
    cmd.args(extra_args);
    cmd.arg(shell);
    cmd.arg("-c");
    cmd.arg("trap - PIPE; seq 999999 2>err | head -n1 > out");
    cmd.succeeds();
    assert_eq!(ts.fixtures.read("out"), "1\n");
    assert_eq!(ts.fixtures.read("err"), "");
}

#[cfg(unix)]
struct SigpipeGuard {
    previous: libc::sighandler_t,
}

#[cfg(unix)]
impl SigpipeGuard {
    fn new() -> Self {
        let previous = unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
        Self { previous }
    }
}

#[cfg(unix)]
impl Drop for SigpipeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::signal(libc::SIGPIPE, self.previous);
        }
    }
}

#[test]
fn disallow_equals_sign_on_short_unset_option() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("-u=")
        .fails_with_code(125)
        .stderr_contains("env: cannot unset '=': Invalid argument");
    ts.ucmd()
        .arg("-u=A1B2C3")
        .fails_with_code(125)
        .stderr_contains("env: cannot unset '=A1B2C3': Invalid argument");
    ts.ucmd().arg("--split-string=A1B=2C3=").succeeds();
    ts.ucmd()
        .arg("--unset=")
        .fails_with_code(125)
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
    pub fn quote(s: &str) -> std::borrow::Cow<'_, str> {
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

    use env::{
        EnvError,
        native_int_str::{Convert, NCvt, from_native_int_representation_owned},
    };

    fn split(input: &str) -> Result<Vec<OsString>, EnvError> {
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
                    assert_eq!(
                        expected,
                        actual.as_slice(),
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
            Err(EnvError::EnvInvalidBackslashAtEndOfStringInMinusS(
                1,
                "Delimiter".into()
            ))
        );
        assert_eq!(
            split(" \\"),
            Err(EnvError::EnvInvalidBackslashAtEndOfStringInMinusS(
                2,
                "Delimiter".into()
            ))
        );
        assert_eq!(
            split("a\\"),
            Err(EnvError::EnvInvalidBackslashAtEndOfStringInMinusS(
                2,
                "Unquoted".into()
            ))
        );
    }

    #[test]
    fn split_errors() {
        assert_eq!(
            split("'abc"),
            Err(EnvError::EnvMissingClosingQuote(4, '\''))
        );
        assert_eq!(split("\""), Err(EnvError::EnvMissingClosingQuote(1, '"')));
        assert_eq!(split("'\\"), Err(EnvError::EnvMissingClosingQuote(2, '\'')));
        assert_eq!(split("'\\"), Err(EnvError::EnvMissingClosingQuote(2, '\'')));
        assert_eq!(
            split(r#""$""#),
            Err(EnvError::EnvParsingOfMissingVariable(2)),
        );
    }

    #[test]
    fn split_error_fail_with_unknown_escape_sequences() {
        assert_eq!(
            split("\\a"),
            Err(EnvError::EnvInvalidSequenceBackslashXInMinusS(1, 'a'))
        );
        assert_eq!(
            split("\"\\a\""),
            Err(EnvError::EnvInvalidSequenceBackslashXInMinusS(2, 'a'))
        );
        assert_eq!(
            split("'\\a'"),
            Err(EnvError::EnvInvalidSequenceBackslashXInMinusS(2, 'a'))
        );
        assert_eq!(
            split(r#""\a""#),
            Err(EnvError::EnvInvalidSequenceBackslashXInMinusS(2, 'a'))
        );
        assert_eq!(
            split(r"\🦉"),
            Err(EnvError::EnvInvalidSequenceBackslashXInMinusS(
                1, '\u{FFFD}'
            ))
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
            NativeStr, from_native_int_representation, from_native_int_representation_owned,
            to_native_int_representation,
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

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_false() {
    let scene = TestScenario::new("util");

    let out = scene.ccmd("env").arg("sh").arg("is_a_tty.sh").succeeds();
    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "stdin is not a tty\nstdout is not a tty\nstderr is not a tty\n"
    );
    std::assert_eq!(
        String::from_utf8_lossy(out.stderr()),
        "This is an error message.\n"
    );
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_true() {
    let scene = TestScenario::new("util");

    let out = scene
        .ccmd("env")
        .arg("sh")
        .arg("is_a_tty.sh")
        .terminal_simulation(true)
        .succeeds();
    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "stdin is a tty\r\nterminal size: 30 80\r\nstdout is a tty\r\nstderr is a tty\r\n"
    );
    std::assert_eq!(
        String::from_utf8_lossy(out.stderr()),
        "This is an error message.\r\n"
    );
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_for_stdin_only() {
    let scene = TestScenario::new("util");

    let out = scene
        .ccmd("env")
        .arg("sh")
        .arg("is_a_tty.sh")
        .terminal_sim_stdio(TerminalSimulation {
            stdin: true,
            stdout: false,
            stderr: false,
            ..Default::default()
        })
        .succeeds();
    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "stdin is a tty\nterminal size: 30 80\nstdout is not a tty\nstderr is not a tty\n"
    );
    std::assert_eq!(
        String::from_utf8_lossy(out.stderr()),
        "This is an error message.\n"
    );
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_for_stdout_only() {
    let scene = TestScenario::new("util");

    let out = scene
        .ccmd("env")
        .arg("sh")
        .arg("is_a_tty.sh")
        .terminal_sim_stdio(TerminalSimulation {
            stdin: false,
            stdout: true,
            stderr: false,
            ..Default::default()
        })
        .succeeds();
    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "stdin is not a tty\r\nstdout is a tty\r\nstderr is not a tty\r\n"
    );
    std::assert_eq!(
        String::from_utf8_lossy(out.stderr()),
        "This is an error message.\n"
    );
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_for_stderr_only() {
    let scene = TestScenario::new("util");

    let out = scene
        .ccmd("env")
        .arg("sh")
        .arg("is_a_tty.sh")
        .terminal_sim_stdio(TerminalSimulation {
            stdin: false,
            stdout: false,
            stderr: true,
            ..Default::default()
        })
        .succeeds();
    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "stdin is not a tty\nstdout is not a tty\nstderr is a tty\n"
    );
    std::assert_eq!(
        String::from_utf8_lossy(out.stderr()),
        "This is an error message.\r\n"
    );
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_size_information() {
    let scene = TestScenario::new("util");

    let out = scene
        .ccmd("env")
        .arg("sh")
        .arg("is_a_tty.sh")
        .terminal_sim_stdio(TerminalSimulation {
            size: Some(libc::winsize {
                ws_col: 40,
                ws_row: 10,
                ws_xpixel: 40 * 8,
                ws_ypixel: 10 * 10,
            }),
            stdout: true,
            stdin: true,
            stderr: true,
        })
        .succeeds();
    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "stdin is a tty\r\nterminal size: 10 40\r\nstdout is a tty\r\nstderr is a tty\r\n"
    );
    std::assert_eq!(
        String::from_utf8_lossy(out.stderr()),
        "This is an error message.\r\n"
    );
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_pty_sends_eot_automatically() {
    let scene = TestScenario::new("util");

    let mut cmd = scene.ccmd("env");
    cmd.timeout(std::time::Duration::from_secs(10));
    cmd.args(&["cat", "-"]);
    cmd.terminal_simulation(true);
    let child = cmd.run_no_wait();
    let out = child.wait().unwrap(); // cat would block if there is no eot

    std::assert_eq!(String::from_utf8_lossy(out.stderr()), "");
    std::assert_eq!(String::from_utf8_lossy(out.stdout()), "\r\n");
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_pty_pipes_into_data_and_sends_eot_automatically() {
    let scene = TestScenario::new("util");

    let message = "Hello stdin forwarding!";

    let mut cmd = scene.ccmd("env");
    cmd.args(&["cat", "-"]);
    cmd.terminal_simulation(true);
    cmd.pipe_in(message);
    let child = cmd.run_no_wait();
    let out = child.wait().unwrap();

    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        format!("{message}\r\n")
    );
    std::assert_eq!(String::from_utf8_lossy(out.stderr()), "");
}

#[test]
#[cfg(not(windows))]
fn test_emoji_env_vars() {
    new_ucmd!()
        .arg("🎯_VAR=Hello 🌍")
        .arg("printenv")
        .arg("🎯_VAR")
        .succeeds()
        .stdout_contains("Hello 🌍");
}

#[cfg(unix)]
#[test]
fn test_simulation_of_terminal_pty_write_in_data_and_sends_eot_automatically() {
    let scene = TestScenario::new("util");

    let mut cmd = scene.ccmd("env");
    cmd.args(&["cat", "-"]);
    cmd.terminal_simulation(true);
    let mut child = cmd.run_no_wait();
    child.write_in("Hello stdin forwarding via write_in!");
    let out = child.wait().unwrap();

    std::assert_eq!(
        String::from_utf8_lossy(out.stdout()),
        "Hello stdin forwarding via write_in!\r\n"
    );
    std::assert_eq!(String::from_utf8_lossy(out.stderr()), "");
}

#[test]
fn test_env_french() {
    new_ucmd!()
        .arg("--verbo")
        .env("LANG", "fr_FR")
        .fails()
        .stderr_contains("erreur : argument inattendu");
}

#[test]
fn test_shebang_error() {
    new_ucmd!()
        .arg("\'-v \'")
        .fails()
        .stderr_contains("use -[v]S to pass options in shebang lines");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_braced_variable_with_default_value() {
    new_ucmd!()
        .arg("-Secho ${UNSET_VAR_UNLIKELY_12345:fallback}")
        .succeeds()
        .stdout_is("fallback\n");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_braced_variable_with_default_when_set() {
    new_ucmd!()
        .env("TEST_VAR_12345", "actual")
        .arg("-Secho ${TEST_VAR_12345:fallback}")
        .succeeds()
        .stdout_is("actual\n");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_simple_braced_variable() {
    new_ucmd!()
        .env("TEST_VAR_12345", "value")
        .arg("-Secho ${TEST_VAR_12345}")
        .succeeds()
        .stdout_is("value\n");
}

#[test]
fn test_braced_variable_error_missing_closing_brace() {
    new_ucmd!()
        .arg("-Secho ${FOO")
        .fails_with_code(125)
        .stderr_contains("Missing closing brace");
}

#[test]
fn test_braced_variable_error_missing_closing_brace_after_default() {
    new_ucmd!()
        .arg("-Secho ${FOO:-value")
        .fails_with_code(125)
        .stderr_contains("Missing closing brace after default value");
}

#[test]
fn test_braced_variable_error_starts_with_digit() {
    new_ucmd!()
        .arg("-Secho ${1FOO}")
        .fails_with_code(125)
        .stderr_contains("Unexpected character: '1'");
}

#[test]
fn test_braced_variable_error_unexpected_character() {
    new_ucmd!()
        .arg("-Secho ${FOO?}")
        .fails_with_code(125)
        .stderr_contains("Unexpected character: '?'");
}

#[test]
#[cfg(unix)]
fn test_non_utf8_env_vars() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let non_utf8_value = OsString::from_vec(b"hello\x80world".to_vec());
    new_ucmd!()
        .env("NON_UTF8_VAR", &non_utf8_value)
        .succeeds()
        .stdout_contains_bytes(b"NON_UTF8_VAR=hello\x80world");
}

#[test]
#[cfg(unix)]
fn test_ignore_signal_pipe_broken_pipe_regression() {
    // Test that --ignore-signal=PIPE properly ignores SIGPIPE in child processes.
    // When SIGPIPE is ignored, processes should handle broken pipes gracefully
    // instead of being terminated by the signal.
    //
    // Regression test for: https://github.com/uutils/coreutils/issues/9617

    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let scene = TestScenario::new(util_name!());

    // Helper function to simulate a broken pipe scenario (like "yes | head -n1")
    let test_sigpipe_behavior = |use_ignore_signal: bool| -> i32 {
        let mut cmd = Command::new(&scene.bin_path);
        cmd.arg("env");

        if use_ignore_signal {
            cmd.arg("--ignore-signal=PIPE");
        }

        cmd.arg("yes").stdout(Stdio::piped()).stderr(Stdio::null());

        let mut child = cmd.spawn().expect("Failed to spawn env process");

        // Read exactly one line then close the pipe to trigger SIGPIPE
        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            // Pipe closes when reader is dropped, sending SIGPIPE to writing process
        }

        match child.wait() {
            Ok(status) => {
                // Process terminated by signal (likely SIGPIPE = 13)
                // Unix convention: signal death = 128 + signal_number
                status.code().unwrap_or(141) // 128 + 13
            }
            Err(_) => 141,
        }
    };

    // Test without signal ignoring - should be killed by SIGPIPE
    let normal_exit_code = test_sigpipe_behavior(false);
    println!("Normal 'env yes' exit code: {normal_exit_code}");

    // Test with --ignore-signal=PIPE - should handle broken pipe gracefully
    let ignore_signal_exit_code = test_sigpipe_behavior(true);
    println!("With --ignore-signal=PIPE exit code: {ignore_signal_exit_code}");

    // Verify the --ignore-signal=PIPE flag changes the behavior
    assert!(
        ignore_signal_exit_code != 141,
        "--ignore-signal=PIPE had no effect! Process was still killed by SIGPIPE (exit code 141). Normal: {normal_exit_code}, --ignore-signal: {ignore_signal_exit_code}"
    );

    // Expected behavior:
    assert_eq!(
        normal_exit_code, 141,
        "Without --ignore-signal, process should be killed by SIGPIPE"
    );
    assert_ne!(
        ignore_signal_exit_code, 141,
        "With --ignore-signal=PIPE, process should NOT be killed by SIGPIPE"
    );

    // Process should exit gracefully when SIGPIPE is ignored
    assert!(
        ignore_signal_exit_code == 0 || ignore_signal_exit_code == 1,
        "With --ignore-signal=PIPE, process should exit gracefully (0 or 1), got: {ignore_signal_exit_code}"
    );
}
