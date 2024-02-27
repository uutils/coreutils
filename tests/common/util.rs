// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//spell-checker: ignore (linux) rlimit prlimit coreutil ggroups uchild uncaptured scmd SHLVL canonicalized

#![allow(dead_code)]

use pretty_assertions::assert_eq;
#[cfg(any(target_os = "linux", target_os = "android"))]
use rlimit::prlimit;
#[cfg(feature = "sleep")]
use rstest::rstest;
#[cfg(unix)]
use std::borrow::Cow;
use std::collections::VecDeque;
#[cfg(not(windows))]
use std::ffi::CString;
use std::ffi::{OsStr, OsString};
use std::fs::{self, hard_link, remove_file, File, OpenOptions};
use std::io::{self, BufWriter, Read, Result, Write};
#[cfg(unix)]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file, PermissionsExt};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
#[cfg(windows)]
use std::path::MAIN_SEPARATOR;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant};
use std::{env, hint, thread};
use tempfile::{Builder, TempDir};

static TESTS_DIR: &str = "tests";
static FIXTURES_DIR: &str = "fixtures";

static ALREADY_RUN: &str = " you have already run this UCommand, if you want to run \
                            another command in the same test, use TestScenario::new instead of \
                            testing();";
static MULTIPLE_STDIN_MEANINGLESS: &str = "Ucommand is designed around a typical use case of: provide args and input stream -> spawn process -> block until completion -> return output streams. For verifying that a particular section of the input stream is what causes a particular behavior, use the Command type directly.";

static NO_STDIN_MEANINGLESS: &str = "Setting this flag has no effect if there is no stdin";

pub const TESTS_BINARY: &str = env!("CARGO_BIN_EXE_coreutils");
pub const PATH: &str = env!("PATH");

/// Default environment variables to run the commands with
const DEFAULT_ENV: [(&str, &str); 2] = [("LC_ALL", "C"), ("TZ", "UTC")];

/// Test if the program is running under CI
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok_and(|s| s.eq_ignore_ascii_case("true"))
}

/// Read a test scenario fixture, returning its bytes
fn read_scenario_fixture<S: AsRef<OsStr>>(tmpd: &Option<Rc<TempDir>>, file_rel_path: S) -> Vec<u8> {
    let tmpdir_path = tmpd.as_ref().unwrap().as_ref().path();
    AtPath::new(tmpdir_path).read_bytes(file_rel_path.as_ref().to_str().unwrap())
}

/// A command result is the outputs of a command (streams and status code)
/// within a struct which has convenience assertion functions about those outputs
#[derive(Debug, Clone)]
pub struct CmdResult {
    /// bin_path provided by `TestScenario` or `UCommand`
    bin_path: PathBuf,
    /// util_name provided by `TestScenario` or `UCommand`
    util_name: Option<String>,
    //tmpd is used for convenience functions for asserts against fixtures
    tmpd: Option<Rc<TempDir>>,
    /// exit status for command (if there is one)
    exit_status: Option<ExitStatus>,
    /// captured standard output after running the Command
    stdout: Vec<u8>,
    /// captured standard error after running the Command
    stderr: Vec<u8>,
}

impl CmdResult {
    pub fn new<S, T, U, V>(
        bin_path: S,
        util_name: Option<T>,
        tmpd: Option<Rc<TempDir>>,
        exit_status: Option<ExitStatus>,
        stdout: U,
        stderr: V,
    ) -> Self
    where
        S: Into<PathBuf>,
        T: AsRef<str>,
        U: Into<Vec<u8>>,
        V: Into<Vec<u8>>,
    {
        Self {
            bin_path: bin_path.into(),
            util_name: util_name.map(|s| s.as_ref().into()),
            tmpd,
            exit_status,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// Apply a function to `stdout` as bytes and return a new [`CmdResult`]
    pub fn stdout_apply<'a, F, R>(&'a self, function: F) -> Self
    where
        F: Fn(&'a [u8]) -> R,
        R: Into<Vec<u8>>,
    {
        Self::new(
            self.bin_path.clone(),
            self.util_name.clone(),
            self.tmpd.clone(),
            self.exit_status,
            function(&self.stdout),
            self.stderr.as_slice(),
        )
    }

    /// Apply a function to `stdout` as `&str` and return a new [`CmdResult`]
    pub fn stdout_str_apply<'a, F, R>(&'a self, function: F) -> Self
    where
        F: Fn(&'a str) -> R,
        R: Into<Vec<u8>>,
    {
        Self::new(
            self.bin_path.clone(),
            self.util_name.clone(),
            self.tmpd.clone(),
            self.exit_status,
            function(self.stdout_str()),
            self.stderr.as_slice(),
        )
    }

    /// Apply a function to `stderr` as bytes and return a new [`CmdResult`]
    pub fn stderr_apply<'a, F, R>(&'a self, function: F) -> Self
    where
        F: Fn(&'a [u8]) -> R,
        R: Into<Vec<u8>>,
    {
        Self::new(
            self.bin_path.clone(),
            self.util_name.clone(),
            self.tmpd.clone(),
            self.exit_status,
            self.stdout.as_slice(),
            function(&self.stderr),
        )
    }

    /// Apply a function to `stderr` as `&str` and return a new [`CmdResult`]
    pub fn stderr_str_apply<'a, F, R>(&'a self, function: F) -> Self
    where
        F: Fn(&'a str) -> R,
        R: Into<Vec<u8>>,
    {
        Self::new(
            self.bin_path.clone(),
            self.util_name.clone(),
            self.tmpd.clone(),
            self.exit_status,
            self.stdout.as_slice(),
            function(self.stderr_str()),
        )
    }

    /// Assert `stdout` as bytes with a predicate function returning a `bool`.
    #[track_caller]
    pub fn stdout_check<'a, F>(&'a self, predicate: F) -> &Self
    where
        F: Fn(&'a [u8]) -> bool,
    {
        assert!(
            predicate(&self.stdout),
            "Predicate for stdout as `bytes` evaluated to false.\nstdout='{:?}'\nstderr='{:?}'\n",
            &self.stdout,
            &self.stderr
        );
        self
    }

    /// Assert `stdout` as `&str` with a predicate function returning a `bool`.
    #[track_caller]
    pub fn stdout_str_check<'a, F>(&'a self, predicate: F) -> &Self
    where
        F: Fn(&'a str) -> bool,
    {
        assert!(
            predicate(self.stdout_str()),
            "Predicate for stdout as `str` evaluated to false.\nstdout='{}'\nstderr='{}'\n",
            self.stdout_str(),
            self.stderr_str()
        );
        self
    }

    /// Assert `stderr` as bytes with a predicate function returning a `bool`.
    #[track_caller]
    pub fn stderr_check<'a, F>(&'a self, predicate: F) -> &Self
    where
        F: Fn(&'a [u8]) -> bool,
    {
        assert!(
            predicate(&self.stderr),
            "Predicate for stderr as `bytes` evaluated to false.\nstdout='{:?}'\nstderr='{:?}'\n",
            &self.stdout,
            &self.stderr
        );
        self
    }

    /// Assert `stderr` as `&str` with a predicate function returning a `bool`.
    #[track_caller]
    pub fn stderr_str_check<'a, F>(&'a self, predicate: F) -> &Self
    where
        F: Fn(&'a str) -> bool,
    {
        assert!(
            predicate(self.stderr_str()),
            "Predicate for stderr as `str` evaluated to false.\nstdout='{}'\nstderr='{}'\n",
            self.stdout_str(),
            self.stderr_str()
        );
        self
    }

    /// Return the exit status of the child process, if any.
    ///
    /// Returns None if the child process is still running or hasn't been started.
    pub fn try_exit_status(&self) -> Option<ExitStatus> {
        self.exit_status
    }

    /// Return the exit status of the child process.
    ///
    /// # Panics
    ///
    /// If the child process is still running or hasn't been started.
    pub fn exit_status(&self) -> ExitStatus {
        self.try_exit_status()
            .expect("Program must be run first or has not finished, yet")
    }

    /// Return the signal the child process received if any.
    ///
    /// # Platform specific behavior
    ///
    /// This method is only available on unix systems.
    #[cfg(unix)]
    pub fn signal(&self) -> Option<i32> {
        self.exit_status().signal()
    }

    /// Assert that the given signal `value` equals the signal the child process received.
    ///
    /// See also [`std::os::unix::process::ExitStatusExt::signal`].
    ///
    /// # Platform specific behavior
    ///
    /// This assertion method is only available on unix systems.
    #[cfg(unix)]
    #[track_caller]
    pub fn signal_is(&self, value: i32) -> &Self {
        let actual = self.signal().unwrap_or_else(|| {
            panic!(
                "Expected process to be terminated by the '{}' signal, but exit status is: '{}'",
                value,
                self.try_exit_status()
                    .map_or("Not available".to_string(), |e| e.to_string())
            )
        });

        assert_eq!(actual, value);
        self
    }

    /// Assert that the given signal `name` equals the signal the child process received.
    ///
    /// Strings like `SIGINT`, `INT` or a number like `15` are all valid names.  See also
    /// [`std::os::unix::process::ExitStatusExt::signal`] and
    /// [`uucore::signals::signal_by_name_or_value`]
    ///
    /// # Platform specific behavior
    ///
    /// This assertion method is only available on unix systems.
    #[cfg(unix)]
    #[track_caller]
    pub fn signal_name_is(&self, name: &str) -> &Self {
        use uucore::signals::signal_by_name_or_value;
        let expected: i32 = signal_by_name_or_value(name)
            .unwrap_or_else(|| panic!("Invalid signal name or value: '{name}'"))
            .try_into()
            .unwrap();

        let actual = self.signal().unwrap_or_else(|| {
            panic!(
                "Expected process to be terminated by the '{}' signal, but exit status is: '{}'",
                name,
                self.try_exit_status()
                    .map_or("Not available".to_string(), |e| e.to_string())
            )
        });

        assert_eq!(actual, expected);
        self
    }

    /// Returns a reference to the program's standard output as a slice of bytes
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Returns the program's standard output as a string slice
    pub fn stdout_str(&self) -> &str {
        std::str::from_utf8(&self.stdout).unwrap()
    }

    /// Returns the program's standard output as a string
    /// consumes self
    pub fn stdout_move_str(self) -> String {
        String::from_utf8(self.stdout).unwrap()
    }

    /// Returns the program's standard output as a vec of bytes
    /// consumes self
    pub fn stdout_move_bytes(self) -> Vec<u8> {
        self.stdout
    }

    /// Returns a reference to the program's standard error as a slice of bytes
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// Returns the program's standard error as a string slice
    pub fn stderr_str(&self) -> &str {
        std::str::from_utf8(&self.stderr).unwrap()
    }

    /// Returns the program's standard error as a string
    /// consumes self
    pub fn stderr_move_str(self) -> String {
        String::from_utf8(self.stderr).unwrap()
    }

    /// Returns the program's standard error as a vec of bytes
    /// consumes self
    pub fn stderr_move_bytes(self) -> Vec<u8> {
        self.stderr
    }

    /// Returns the program's exit code
    /// Panics if not run or has not finished yet for example when run with `run_no_wait()`
    pub fn code(&self) -> i32 {
        self.exit_status().code().unwrap()
    }

    #[track_caller]
    pub fn code_is(&self, expected_code: i32) -> &Self {
        assert_eq!(self.code(), expected_code);
        self
    }

    /// Returns the program's `TempDir`
    /// Panics if not present
    pub fn tmpd(&self) -> Rc<TempDir> {
        match &self.tmpd {
            Some(ptr) => ptr.clone(),
            None => panic!("Command not associated with a TempDir"),
        }
    }

    /// Returns whether the program succeeded
    pub fn succeeded(&self) -> bool {
        self.exit_status.map_or(true, |e| e.success())
    }

    /// asserts that the command resulted in a success (zero) status code
    #[track_caller]
    pub fn success(&self) -> &Self {
        assert!(
            self.succeeded(),
            "Command was expected to succeed.\nstdout = {}\n stderr = {}",
            self.stdout_str(),
            self.stderr_str()
        );
        self
    }

    /// asserts that the command resulted in a failure (non-zero) status code
    #[track_caller]
    pub fn failure(&self) -> &Self {
        assert!(
            !self.succeeded(),
            "Command was expected to fail.\nstdout = {}\n stderr = {}",
            self.stdout_str(),
            self.stderr_str()
        );
        self
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// generally, it's better to use `stdout_only()` instead,
    /// but you might find yourself using this function if
    /// 1.  you can not know exactly what stdout will be or
    /// 2.  you know that stdout will also be empty
    #[track_caller]
    pub fn no_stderr(&self) -> &Self {
        assert!(
            self.stderr.is_empty(),
            "Expected stderr to be empty, but it's:\n{}",
            self.stderr_str()
        );
        self
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// unless asserting there was neither stdout or stderr, `stderr_only` is usually a better choice
    /// generally, it's better to use `stderr_only()` instead,
    /// but you might find yourself using this function if
    /// 1.  you can not know exactly what stderr will be or
    /// 2.  you know that stderr will also be empty
    #[track_caller]
    pub fn no_stdout(&self) -> &Self {
        assert!(
            self.stdout.is_empty(),
            "Expected stdout to be empty, but it's:\n{}",
            self.stdout_str()
        );
        self
    }

    /// Assert that there is output to neither stderr nor stdout.
    #[track_caller]
    pub fn no_output(&self) -> &Self {
        self.no_stdout().no_stderr()
    }

    /// asserts that the command resulted in stdout stream output that equals the
    /// passed in value, trailing whitespace are kept to force strict comparison (#1235)
    /// `stdout_only()` is a better choice unless stderr may or will be non-empty
    #[track_caller]
    pub fn stdout_is<T: AsRef<str>>(&self, msg: T) -> &Self {
        assert_eq!(self.stdout_str(), String::from(msg.as_ref()));
        self
    }

    /// like `stdout_is`, but succeeds if any elements of `expected` matches stdout.
    #[track_caller]
    pub fn stdout_is_any<T: AsRef<str> + std::fmt::Debug>(&self, expected: &[T]) -> &Self {
        assert!(
            expected.iter().any(|msg| self.stdout_str() == msg.as_ref()),
            "stdout was {}\nExpected any of {:#?}",
            self.stdout_str(),
            expected
        );
        self
    }

    /// Like `stdout_is` but newlines are normalized to `\n`.
    #[track_caller]
    pub fn normalized_newlines_stdout_is<T: AsRef<str>>(&self, msg: T) -> &Self {
        let msg = msg.as_ref().replace("\r\n", "\n");
        assert_eq!(self.stdout_str().replace("\r\n", "\n"), msg);
        self
    }

    /// asserts that the command resulted in stdout stream output,
    /// whose bytes equal those of the passed in slice
    #[track_caller]
    pub fn stdout_is_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        assert_eq!(self.stdout, msg.as_ref(),
            "stdout as bytes wasn't equal to expected bytes. Result as strings:\nstdout  ='{:?}'\nexpected='{:?}'",
            std::str::from_utf8(&self.stdout),
            std::str::from_utf8(msg.as_ref()),
        );
        self
    }

    /// like `stdout_is()`, but expects the contents of the file at the provided relative path
    #[track_caller]
    pub fn stdout_is_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_is(String::from_utf8(contents).unwrap())
    }

    /// Assert that the bytes of stdout exactly match those of the given file.
    ///
    /// Contrast this with [`CmdResult::stdout_is_fixture`], which
    /// decodes the contents of the file as a UTF-8 [`String`] before
    /// comparison with stdout.
    ///
    /// # Examples
    ///
    /// Use this method in a unit test like this:
    ///
    /// ```rust,ignore
    /// #[test]
    /// fn test_something() {
    ///     new_ucmd!().succeeds().stdout_is_fixture_bytes("expected.bin");
    /// }
    /// ```
    #[track_caller]
    pub fn stdout_is_fixture_bytes<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_is_bytes(contents)
    }

    /// like `stdout_is_fixture()`, but replaces the data in fixture file based on values provided in `template_vars`
    /// command output
    #[track_caller]
    pub fn stdout_is_templated_fixture<T: AsRef<OsStr>>(
        &self,
        file_rel_path: T,
        template_vars: &[(&str, &str)],
    ) -> &Self {
        let mut contents =
            String::from_utf8(read_scenario_fixture(&self.tmpd, file_rel_path)).unwrap();
        for kv in template_vars {
            contents = contents.replace(kv.0, kv.1);
        }
        self.stdout_is(contents)
    }

    /// like `stdout_is_templated_fixture`, but succeeds if any replacement by `template_vars` results in the actual stdout.
    #[track_caller]
    pub fn stdout_is_templated_fixture_any<T: AsRef<OsStr>>(
        &self,
        file_rel_path: T,
        template_vars: &[Vec<(String, String)>],
    ) {
        let contents = String::from_utf8(read_scenario_fixture(&self.tmpd, file_rel_path)).unwrap();
        let possible_values = template_vars.iter().map(|vars| {
            let mut contents = contents.clone();
            for kv in vars {
                contents = contents.replace(&kv.0, &kv.1);
            }
            contents
        });
        self.stdout_is_any(&possible_values.collect::<Vec<_>>());
    }

    /// assert that the command resulted in stderr stream output that equals the
    /// passed in value.
    ///
    /// `stderr_only` is a better choice unless stdout may or will be non-empty
    #[track_caller]
    pub fn stderr_is<T: AsRef<str>>(&self, msg: T) -> &Self {
        assert_eq!(self.stderr_str(), msg.as_ref());
        self
    }

    /// asserts that the command resulted in stderr stream output,
    /// whose bytes equal those of the passed in slice
    #[track_caller]
    pub fn stderr_is_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        assert_eq!(
            &self.stderr,
            msg.as_ref(),
            "stderr as bytes wasn't equal to expected bytes. Result as strings:\nstderr  ='{:?}'\nexpected='{:?}'",
            std::str::from_utf8(&self.stderr),
            std::str::from_utf8(msg.as_ref())
        );
        self
    }

    /// Like `stdout_is_fixture`, but for stderr
    #[track_caller]
    pub fn stderr_is_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stderr_is(String::from_utf8(contents).unwrap())
    }

    /// asserts that
    /// 1.  the command resulted in stdout stream output that equals the
    ///     passed in value
    /// 2.  the command resulted in empty (zero-length) stderr stream output
    #[track_caller]
    pub fn stdout_only<T: AsRef<str>>(&self, msg: T) -> &Self {
        self.no_stderr().stdout_is(msg)
    }

    /// asserts that
    /// 1.  the command resulted in a stdout stream whose bytes
    ///     equal those of the passed in value
    /// 2.  the command resulted in an empty stderr stream
    #[track_caller]
    pub fn stdout_only_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        self.no_stderr().stdout_is_bytes(msg)
    }

    /// like `stdout_only()`, but expects the contents of the file at the provided relative path
    #[track_caller]
    pub fn stdout_only_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_only_bytes(contents)
    }

    /// asserts that
    /// 1.  the command resulted in stderr stream output that equals the
    ///     passed in value
    /// 2.  the command resulted in empty (zero-length) stdout stream output
    #[track_caller]
    pub fn stderr_only<T: AsRef<str>>(&self, msg: T) -> &Self {
        self.no_stdout().stderr_is(msg)
    }

    /// asserts that
    /// 1.  the command resulted in a stderr stream whose bytes equal the ones
    ///     of the passed value
    /// 2.  the command resulted in an empty stdout stream
    #[track_caller]
    pub fn stderr_only_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        self.no_stdout().stderr_is_bytes(msg)
    }

    #[track_caller]
    pub fn fails_silently(&self) -> &Self {
        assert!(!self.succeeded());
        assert!(self.stderr.is_empty());
        self
    }

    /// asserts that
    /// 1.  the command resulted in stderr stream output that equals the
    ///     the following format
    ///     `"{util_name}: {msg}\nTry '{bin_path} {util_name} --help' for more information."`
    ///     This the expected format when a `UUsageError` is returned or when `show_error!` is called
    ///     `msg` should be the same as the one provided to `UUsageError::new` or `show_error!`
    ///
    /// 2.  the command resulted in empty (zero-length) stdout stream output
    #[track_caller]
    pub fn usage_error<T: AsRef<str>>(&self, msg: T) -> &Self {
        self.stderr_only(format!(
            "{0}: {2}\nTry '{1} {0} --help' for more information.\n",
            self.util_name.as_ref().unwrap(), // This shouldn't be called using a normal command
            self.bin_path.display(),
            msg.as_ref()
        ))
    }

    #[track_caller]
    pub fn stdout_contains<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            self.stdout_str().contains(cmp.as_ref()),
            "'{}' does not contain '{}'",
            self.stdout_str(),
            cmp.as_ref()
        );
        self
    }

    #[track_caller]
    pub fn stdout_contains_line<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            self.stdout_str().lines().any(|line| line == cmp.as_ref()),
            "'{}' does not contain line '{}'",
            self.stdout_str(),
            cmp.as_ref()
        );
        self
    }

    #[track_caller]
    pub fn stderr_contains<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            self.stderr_str().contains(cmp.as_ref()),
            "'{}' does not contain '{}'",
            self.stderr_str(),
            cmp.as_ref()
        );
        self
    }

    #[track_caller]
    pub fn stdout_does_not_contain<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            !self.stdout_str().contains(cmp.as_ref()),
            "'{}' contains '{}' but should not",
            self.stdout_str(),
            cmp.as_ref(),
        );
        self
    }

    #[track_caller]
    pub fn stderr_does_not_contain<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(!self.stderr_str().contains(cmp.as_ref()));
        self
    }

    #[track_caller]
    pub fn stdout_matches(&self, regex: &regex::Regex) -> &Self {
        assert!(
            regex.is_match(self.stdout_str()),
            "Stdout does not match regex:\n{}",
            self.stdout_str()
        );
        self
    }

    #[track_caller]
    pub fn stderr_matches(&self, regex: &regex::Regex) -> &Self {
        assert!(
            regex.is_match(self.stderr_str()),
            "Stderr does not match regex:\n{}",
            self.stderr_str()
        );
        self
    }

    #[track_caller]
    pub fn stdout_does_not_match(&self, regex: &regex::Regex) -> &Self {
        assert!(
            !regex.is_match(self.stdout_str()),
            "Stdout matches regex:\n{}",
            self.stdout_str()
        );
        self
    }
}

pub fn log_info<T: AsRef<str>, U: AsRef<str>>(msg: T, par: U) {
    println!("{}: {}", msg.as_ref(), par.as_ref());
}

pub fn recursive_copy(src: &Path, dest: &Path) -> Result<()> {
    if fs::metadata(src)?.is_dir() {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let mut new_dest = PathBuf::from(dest);
            new_dest.push(entry.file_name());
            if fs::metadata(entry.path())?.is_dir() {
                fs::create_dir(&new_dest)?;
                recursive_copy(&entry.path(), &new_dest)?;
            } else {
                fs::copy(entry.path(), new_dest)?;
            }
        }
    }
    Ok(())
}

pub fn get_root_path() -> &'static str {
    if cfg!(windows) {
        "C:\\"
    } else {
        "/"
    }
}

/// Compares the extended attributes (xattrs) of two files or directories.
///
/// # Returns
///
/// `true` if both paths have the same set of extended attributes, `false` otherwise.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn compare_xattrs<P: AsRef<std::path::Path>>(path1: P, path2: P) -> bool {
    let get_sorted_xattrs = |path: P| {
        xattr::list(path)
            .map(|attrs| {
                let mut attrs = attrs.collect::<Vec<_>>();
                attrs.sort();
                attrs
            })
            .unwrap_or_else(|_| Vec::new())
    };

    get_sorted_xattrs(path1) == get_sorted_xattrs(path2)
}

/// Object-oriented path struct that represents and operates on
/// paths relative to the directory it was constructed for.
#[derive(Clone)]
pub struct AtPath {
    pub subdir: PathBuf,
}

impl AtPath {
    pub fn new(subdir: &Path) -> Self {
        Self {
            subdir: PathBuf::from(subdir),
        }
    }

    pub fn as_string(&self) -> String {
        self.subdir.to_str().unwrap().to_owned()
    }

    pub fn plus<P: AsRef<Path>>(&self, name: P) -> PathBuf {
        let mut pathbuf = self.subdir.clone();
        pathbuf.push(name);
        pathbuf
    }

    pub fn plus_as_string<P: AsRef<Path>>(&self, name: P) -> String {
        self.plus(name).display().to_string()
    }

    fn minus(&self, name: &str) -> PathBuf {
        let prefixed = PathBuf::from(name);
        if prefixed.starts_with(&self.subdir) {
            let mut unprefixed = PathBuf::new();
            for component in prefixed.components().skip(self.subdir.components().count()) {
                unprefixed.push(component.as_os_str().to_str().unwrap());
            }
            unprefixed
        } else {
            prefixed
        }
    }

    pub fn minus_as_string(&self, name: &str) -> String {
        String::from(self.minus(name).to_str().unwrap())
    }

    pub fn set_readonly(&self, name: &str) {
        let metadata = fs::metadata(self.plus(name)).unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(self.plus(name), permissions).unwrap();
    }

    pub fn open(&self, name: &str) -> File {
        log_info("open", self.plus_as_string(name));
        File::open(self.plus(name)).unwrap()
    }

    pub fn read(&self, name: &str) -> String {
        let mut f = self.open(name);
        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .unwrap_or_else(|e| panic!("Couldn't read {name}: {e}"));
        contents
    }

    pub fn read_bytes(&self, name: &str) -> Vec<u8> {
        let mut f = self.open(name);
        let mut contents = Vec::new();
        f.read_to_end(&mut contents)
            .unwrap_or_else(|e| panic!("Couldn't read {name}: {e}"));
        contents
    }

    pub fn write(&self, name: &str, contents: &str) {
        log_info("write(default)", self.plus_as_string(name));
        std::fs::write(self.plus(name), contents)
            .unwrap_or_else(|e| panic!("Couldn't write {name}: {e}"));
    }

    pub fn write_bytes(&self, name: &str, contents: &[u8]) {
        log_info("write(default)", self.plus_as_string(name));
        std::fs::write(self.plus(name), contents)
            .unwrap_or_else(|e| panic!("Couldn't write {name}: {e}"));
    }

    pub fn append(&self, name: &str, contents: &str) {
        log_info("write(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.plus(name))
            .unwrap();
        f.write_all(contents.as_bytes())
            .unwrap_or_else(|e| panic!("Couldn't write(append) {name}: {e}"));
    }

    pub fn append_bytes(&self, name: &str, contents: &[u8]) {
        log_info("write(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.plus(name))
            .unwrap();
        f.write_all(contents)
            .unwrap_or_else(|e| panic!("Couldn't write(append) to {name}: {e}"));
    }

    pub fn truncate(&self, name: &str, contents: &str) {
        log_info("write(truncate)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(self.plus(name))
            .unwrap();
        f.write_all(contents.as_bytes())
            .unwrap_or_else(|e| panic!("Couldn't write(truncate) {name}: {e}"));
    }

    pub fn rename(&self, source: &str, target: &str) {
        let source = self.plus(source);
        let target = self.plus(target);
        log_info("rename", format!("{source:?} {target:?}"));
        std::fs::rename(&source, &target)
            .unwrap_or_else(|e| panic!("Couldn't rename {source:?} -> {target:?}: {e}"));
    }

    pub fn remove(&self, source: &str) {
        let source = self.plus(source);
        log_info("remove", format!("{source:?}"));
        std::fs::remove_file(&source).unwrap_or_else(|e| panic!("Couldn't remove {source:?}: {e}"));
    }

    pub fn copy(&self, source: &str, target: &str) {
        let source = self.plus(source);
        let target = self.plus(target);
        log_info("copy", format!("{source:?} {target:?}"));
        std::fs::copy(&source, &target)
            .unwrap_or_else(|e| panic!("Couldn't copy {source:?} -> {target:?}: {e}"));
    }

    pub fn rmdir(&self, dir: &str) {
        log_info("rmdir", self.plus_as_string(dir));
        fs::remove_dir(self.plus(dir)).unwrap();
    }

    pub fn mkdir<P: AsRef<Path>>(&self, dir: P) {
        let dir = dir.as_ref();
        log_info("mkdir", self.plus_as_string(dir));
        fs::create_dir(self.plus(dir)).unwrap();
    }

    pub fn mkdir_all(&self, dir: &str) {
        log_info("mkdir_all", self.plus_as_string(dir));
        fs::create_dir_all(self.plus(dir)).unwrap();
    }

    pub fn make_file(&self, name: &str) -> File {
        match File::create(self.plus(name)) {
            Ok(f) => f,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn touch<P: AsRef<Path>>(&self, file: P) {
        let file = file.as_ref();
        log_info("touch", self.plus_as_string(file));
        File::create(self.plus(file)).unwrap();
    }

    #[cfg(not(windows))]
    pub fn mkfifo(&self, fifo: &str) {
        let full_path = self.plus_as_string(fifo);
        log_info("mkfifo", &full_path);
        unsafe {
            let fifo_name: CString = CString::new(full_path).expect("CString creation failed.");
            libc::mkfifo(fifo_name.as_ptr(), libc::S_IWUSR | libc::S_IRUSR);
        }
    }

    #[cfg(not(windows))]
    pub fn is_fifo(&self, fifo: &str) -> bool {
        unsafe {
            let name = CString::new(self.plus_as_string(fifo)).unwrap();
            let mut stat: libc::stat = std::mem::zeroed();
            if libc::stat(name.as_ptr(), &mut stat) >= 0 {
                libc::S_IFIFO & stat.st_mode as libc::mode_t != 0
            } else {
                false
            }
        }
    }

    pub fn hard_link(&self, original: &str, link: &str) {
        log_info(
            "hard_link",
            format!(
                "{},{}",
                self.plus_as_string(original),
                self.plus_as_string(link)
            ),
        );
        hard_link(self.plus(original), self.plus(link)).unwrap();
    }

    pub fn symlink_file(&self, original: &str, link: &str) {
        log_info(
            "symlink",
            format!(
                "{},{}",
                self.plus_as_string(original),
                self.plus_as_string(link)
            ),
        );
        symlink_file(self.plus(original), self.plus(link)).unwrap();
    }

    pub fn relative_symlink_file(&self, original: &str, link: &str) {
        #[cfg(windows)]
        let original = original.replace('/', &MAIN_SEPARATOR.to_string());
        log_info(
            "symlink",
            format!("{},{}", &original, &self.plus_as_string(link)),
        );
        symlink_file(original, self.plus(link)).unwrap();
    }

    pub fn symlink_dir(&self, original: &str, link: &str) {
        log_info(
            "symlink",
            format!(
                "{},{}",
                self.plus_as_string(original),
                self.plus_as_string(link)
            ),
        );
        symlink_dir(self.plus(original), self.plus(link)).unwrap();
    }

    pub fn relative_symlink_dir(&self, original: &str, link: &str) {
        #[cfg(windows)]
        let original = original.replace('/', &MAIN_SEPARATOR.to_string());
        log_info(
            "symlink",
            format!("{},{}", &original, &self.plus_as_string(link)),
        );
        symlink_dir(original, self.plus(link)).unwrap();
    }

    pub fn is_symlink(&self, path: &str) -> bool {
        log_info("is_symlink", self.plus_as_string(path));
        match fs::symlink_metadata(self.plus(path)) {
            Ok(m) => m.file_type().is_symlink(),
            Err(_) => false,
        }
    }

    pub fn resolve_link(&self, path: &str) -> String {
        log_info("resolve_link", self.plus_as_string(path));
        match fs::read_link(self.plus(path)) {
            Ok(p) => self.minus_as_string(p.to_str().unwrap()),
            Err(_) => String::new(),
        }
    }

    pub fn read_symlink(&self, path: &str) -> String {
        log_info("read_symlink", self.plus_as_string(path));
        fs::read_link(self.plus(path))
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub fn symlink_metadata(&self, path: &str) -> fs::Metadata {
        match fs::symlink_metadata(self.plus(path)) {
            Ok(m) => m,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn metadata(&self, path: &str) -> fs::Metadata {
        match fs::metadata(self.plus(path)) {
            Ok(m) => m,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        match fs::metadata(self.plus(path)) {
            Ok(m) => m.is_file(),
            Err(_) => false,
        }
    }

    /// Decide whether the named symbolic link exists in the test directory.
    pub fn symlink_exists(&self, path: &str) -> bool {
        match fs::symlink_metadata(self.plus(path)) {
            Ok(m) => m.file_type().is_symlink(),
            Err(_) => false,
        }
    }

    pub fn dir_exists(&self, path: &str) -> bool {
        match fs::metadata(self.plus(path)) {
            Ok(m) => m.is_dir(),
            Err(_) => false,
        }
    }

    pub fn root_dir_resolved(&self) -> String {
        log_info("current_directory_resolved", "");
        let s = self
            .subdir
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        // Due to canonicalize()'s use of GetFinalPathNameByHandleW() on Windows, the resolved path
        // starts with '\\?\' to extend the limit of a given path to 32,767 wide characters.
        //
        // To address this issue, we remove this prepended string if available.
        //
        // Source:
        // http://stackoverflow.com/questions/31439011/getfinalpathnamebyhandle-without-prepended
        let prefix = "\\\\?\\";

        if let Some(stripped) = s.strip_prefix(prefix) {
            String::from(stripped)
        } else {
            s
        }
    }

    /// Set the permissions of the specified file.
    ///
    /// # Panics
    ///
    /// This function panics if there is an error loading the metadata
    /// or setting the permissions of the file.
    #[cfg(not(windows))]
    pub fn set_mode(&self, filename: &str, mode: u32) {
        let path = self.plus(filename);
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(&path, perms).unwrap();
    }
}

/// An environment for running a single uutils test case, serves three functions:
/// 1. centralizes logic for locating the uutils binary and calling the utility
/// 2. provides a unique temporary directory for the test case
/// 3. copies over fixtures for the utility to the temporary directory
///
/// Fixtures can be found under `tests/fixtures/$util_name/`
pub struct TestScenario {
    pub bin_path: PathBuf,
    pub util_name: String,
    pub fixtures: AtPath,
    tmpd: Rc<TempDir>,
}

impl TestScenario {
    pub fn new<T>(util_name: T) -> Self
    where
        T: AsRef<str>,
    {
        let tmpd = Rc::new(TempDir::new().unwrap());
        let ts = Self {
            bin_path: PathBuf::from(TESTS_BINARY),
            util_name: util_name.as_ref().into(),
            fixtures: AtPath::new(tmpd.as_ref().path()),
            tmpd,
        };
        let mut fixture_path_builder = env::current_dir().unwrap();
        fixture_path_builder.push(TESTS_DIR);
        fixture_path_builder.push(FIXTURES_DIR);
        fixture_path_builder.push(util_name.as_ref());
        if let Ok(m) = fs::metadata(&fixture_path_builder) {
            if m.is_dir() {
                recursive_copy(&fixture_path_builder, &ts.fixtures.subdir).unwrap();
            }
        }
        ts
    }

    /// Returns builder for invoking the target uutils binary. Paths given are
    /// treated relative to the environment's unique temporary test directory.
    pub fn ucmd(&self) -> UCommand {
        UCommand::from_test_scenario(self)
    }

    /// Returns builder for invoking any system command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    pub fn cmd<S: Into<PathBuf>>(&self, bin_path: S) -> UCommand {
        let mut command = UCommand::new();
        command.bin_path(bin_path);
        command.temp_dir(self.tmpd.clone());
        command
    }

    /// Returns builder for invoking any uutils command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    pub fn ccmd<S: AsRef<str>>(&self, util_name: S) -> UCommand {
        UCommand::with_util(util_name, self.tmpd.clone())
    }
}

/// A `UCommand` is a builder wrapping an individual Command that provides several additional features:
/// 1. it has convenience functions that are more ergonomic to use for piping in stdin, spawning the command
///       and asserting on the results.
/// 2. it tracks arguments provided so that in test cases which may provide variations of an arg in loops
///     the test failure can display the exact call which preceded an assertion failure.
/// 3. it provides convenience construction methods to set the Command uutils utility and temporary directory.
///
/// Per default `UCommand` runs a command given as an argument in a shell, platform independently.
/// It does so with safety in mind, so the working directory is set to an individual temporary
/// directory and the environment variables are cleared per default.
///
/// The default behavior can be changed with builder methods:
/// * [`UCommand::with_util`]: Run `coreutils UTIL_NAME` instead of the shell
/// * [`UCommand::from_test_scenario`]: Run `coreutils UTIL_NAME` instead of the shell in the
///   temporary directory of the [`TestScenario`]
/// * [`UCommand::current_dir`]: Sets the working directory
/// * ...
#[derive(Debug, Default)]
pub struct UCommand {
    args: VecDeque<OsString>,
    env_vars: Vec<(OsString, OsString)>,
    current_dir: Option<PathBuf>,
    bin_path: Option<PathBuf>,
    util_name: Option<String>,
    has_run: bool,
    ignore_stdin_write_error: bool,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
    bytes_into_stdin: Option<Vec<u8>>,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    limits: Vec<(rlimit::Resource, u64, u64)>,
    stderr_to_stdout: bool,
    timeout: Option<Duration>,
    tmpd: Option<Rc<TempDir>>, // drop last
}

impl UCommand {
    /// Create a new plain [`UCommand`].
    ///
    /// Executes a command that must be given as argument (for example with [`UCommand::arg`] in a
    /// shell (`sh -c` on unix platforms or `cmd /C` on windows).
    ///
    /// Per default the environment is cleared and the working directory is set to an individual
    /// temporary directory for safety purposes.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Create a [`UCommand`] for a specific uutils utility.
    ///
    /// Sets the temporary directory to `tmpd` and the execution binary to the path where
    /// `coreutils` is found.
    pub fn with_util<T>(util_name: T, tmpd: Rc<TempDir>) -> Self
    where
        T: AsRef<str>,
    {
        let mut ucmd = Self::new();
        ucmd.util_name = Some(util_name.as_ref().into());
        ucmd.bin_path(TESTS_BINARY).temp_dir(tmpd);
        ucmd
    }

    /// Create a [`UCommand`] from a [`TestScenario`].
    ///
    /// The temporary directory and uutils utility are inherited from the [`TestScenario`] and the
    /// execution binary is set to `coreutils`.
    pub fn from_test_scenario(scene: &TestScenario) -> Self {
        Self::with_util(&scene.util_name, scene.tmpd.clone())
    }

    /// Set the execution binary.
    ///
    /// Make sure the binary found at this path is executable. It's safest to provide the
    /// canonicalized path instead of just the name of the executable, since path resolution is not
    /// guaranteed to work on all platforms.
    fn bin_path<T>(&mut self, bin_path: T) -> &mut Self
    where
        T: Into<PathBuf>,
    {
        self.bin_path = Some(bin_path.into());
        self
    }

    /// Set the temporary directory.
    ///
    /// Per default an individual temporary directory is created for every [`UCommand`]. If not
    /// specified otherwise with [`UCommand::current_dir`] the working directory is set to this
    /// temporary directory.
    fn temp_dir(&mut self, temp_dir: Rc<TempDir>) -> &mut Self {
        self.tmpd = Some(temp_dir);
        self
    }

    /// Set the working directory for this [`UCommand`]
    ///
    /// Per default the working directory is set to the [`UCommands`] temporary directory.
    pub fn current_dir<T>(&mut self, current_dir: T) -> &mut Self
    where
        T: Into<PathBuf>,
    {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn set_stdin<T: Into<Stdio>>(&mut self, stdin: T) -> &mut Self {
        self.stdin = Some(stdin.into());
        self
    }

    pub fn set_stdout<T: Into<Stdio>>(&mut self, stdout: T) -> &mut Self {
        self.stdout = Some(stdout.into());
        self
    }

    pub fn set_stderr<T: Into<Stdio>>(&mut self, stderr: T) -> &mut Self {
        self.stderr = Some(stderr.into());
        self
    }

    pub fn stderr_to_stdout(&mut self) -> &mut Self {
        self.stderr_to_stdout = true;
        self
    }

    /// Add a parameter to the invocation. Path arguments are treated relative
    /// to the test environment directory.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.args.push_back(arg.as_ref().into());
        self
    }

    /// Add multiple parameters to the invocation. Path arguments are treated relative
    /// to the test environment directory.
    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut Self {
        self.args.extend(args.iter().map(|s| s.as_ref().into()));
        self
    }

    /// provides standard input to feed in to the command when spawned
    pub fn pipe_in<T: Into<Vec<u8>>>(&mut self, input: T) -> &mut Self {
        assert!(
            self.bytes_into_stdin.is_none(),
            "{}",
            MULTIPLE_STDIN_MEANINGLESS
        );
        self.set_stdin(Stdio::piped());
        self.bytes_into_stdin = Some(input.into());
        self
    }

    /// like `pipe_in()`, but uses the contents of the file at the provided relative path as the piped in data
    pub fn pipe_in_fixture<S: AsRef<OsStr>>(&mut self, file_rel_path: S) -> &mut Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.pipe_in(contents)
    }

    /// Ignores error caused by feeding stdin to the command.
    /// This is typically useful to test non-standard workflows
    /// like feeding something to a command that does not read it
    pub fn ignore_stdin_write_error(&mut self) -> &mut Self {
        self.ignore_stdin_write_error = true;
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env_vars
            .push((key.as_ref().into(), val.as_ref().into()));
        self
    }

    pub fn envs<I, K, V>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in iter {
            self.env(k, v);
        }
        self
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn limit(
        &mut self,
        resource: rlimit::Resource,
        soft_limit: u64,
        hard_limit: u64,
    ) -> &mut Self {
        self.limits.push((resource, soft_limit, hard_limit));
        self
    }

    /// Set the timeout for [`UCommand::run`] and similar methods in [`UCommand`].
    ///
    /// After the timeout elapsed these `run` methods (besides [`UCommand::run_no_wait`]) will
    /// panic. When [`UCommand::run_no_wait`] is used, this timeout is applied to
    /// [`UChild::wait_with_output`] including all other waiting methods in [`UChild`] implicitly
    /// using `wait_with_output()` and additionally [`UChild::kill`]. The default timeout of `kill`
    /// will be overwritten by this `timeout`.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the `std::process::Command` and apply the defaults on fields which were not specified
    /// by the user.
    ///
    /// These __defaults__ are:
    /// * `bin_path`: Depending on the platform and os, the native shell (unix -> `/bin/sh` etc.).
    /// This default also requires to set the first argument to `-c` on unix (`/C` on windows) if
    /// this argument wasn't specified explicitly by the user.
    /// * `util_name`: `None`. If neither `bin_path` nor `util_name` were given the arguments are
    /// run in a shell (See `bin_path` above).
    /// * `temp_dir`: If `current_dir` was not set, a new temporary directory will be created in
    /// which this command will be run and `current_dir` will be set to this `temp_dir`.
    /// * `current_dir`: The temporary directory given by `temp_dir`.
    /// * `timeout`: `30 seconds`
    /// * `stdin`: `Stdio::null()`
    /// * `ignore_stdin_write_error`: `false`
    /// * `stdout`, `stderr`: If not specified the output will be captured with [`CapturedOutput`]
    /// * `stderr_to_stdout`: `false`
    /// * `bytes_into_stdin`: `None`
    /// * `limits`: `None`.
    fn build(&mut self) -> (Command, Option<CapturedOutput>, Option<CapturedOutput>) {
        if self.bin_path.is_some() {
            if let Some(util_name) = &self.util_name {
                self.args.push_front(util_name.into());
            }
        } else if let Some(util_name) = &self.util_name {
            self.bin_path = Some(PathBuf::from(TESTS_BINARY));
            self.args.push_front(util_name.into());
        // neither `bin_path` nor `util_name` was set so we apply the default to run the arguments
        // in a platform specific shell
        } else if cfg!(unix) {
            #[cfg(target_os = "android")]
            let bin_path = PathBuf::from("/system/bin/sh");
            #[cfg(not(target_os = "android"))]
            let bin_path = PathBuf::from("/bin/sh");

            self.bin_path = Some(bin_path);
            let c_arg = OsString::from("-c");
            if !self.args.contains(&c_arg) {
                self.args.push_front(c_arg);
            }
        } else {
            self.bin_path = Some(PathBuf::from("cmd"));
            let c_arg = OsString::from("/C");
            let k_arg = OsString::from("/K");
            if !self
                .args
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&c_arg) || s.eq_ignore_ascii_case(&k_arg))
            {
                self.args.push_front(c_arg);
            }
        };

        // unwrap is safe here because we have set `self.bin_path` before
        let mut command = Command::new(self.bin_path.as_ref().unwrap());
        command.args(&self.args);

        // We use a temporary directory as working directory if not specified otherwise with
        // `current_dir()`. If neither `current_dir` nor a temporary directory is available, then we
        // create our own.
        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        } else if let Some(temp_dir) = &self.tmpd {
            command.current_dir(temp_dir.path());
        } else {
            let temp_dir = tempfile::tempdir().unwrap();
            self.current_dir = Some(temp_dir.path().into());
            command.current_dir(temp_dir.path());
            self.tmpd = Some(Rc::new(temp_dir));
        }

        command.env_clear();
        if cfg!(windows) {
            // spell-checker:ignore (dll) rsaenh
            // %SYSTEMROOT% is required on Windows to initialize crypto provider
            // ... and crypto provider is required for std::rand
            // From `procmon`: RegQueryValue HKLM\SOFTWARE\Microsoft\Cryptography\Defaults\Provider\Microsoft Strong Cryptographic Provider\Image Path
            // SUCCESS  Type: REG_SZ, Length: 66, Data: %SystemRoot%\system32\rsaenh.dll"
            if let Some(systemroot) = env::var_os("SYSTEMROOT") {
                command.env("SYSTEMROOT", systemroot);
            }
        } else {
            // if someone is setting LD_PRELOAD, there's probably a good reason for it
            if let Some(ld_preload) = env::var_os("LD_PRELOAD") {
                command.env("LD_PRELOAD", ld_preload);
            }
        }

        command
            .envs(DEFAULT_ENV)
            .envs(self.env_vars.iter().cloned());

        if self.timeout.is_none() {
            self.timeout = Some(Duration::from_secs(30));
        }

        let mut captured_stdout = None;
        let mut captured_stderr = None;
        if self.stderr_to_stdout {
            let mut output = CapturedOutput::default();

            command
                .stdin(self.stdin.take().unwrap_or_else(Stdio::null))
                .stdout(Stdio::from(output.try_clone().unwrap()))
                .stderr(Stdio::from(output.try_clone().unwrap()));
            captured_stdout = Some(output);
        } else {
            let stdout = if self.stdout.is_some() {
                self.stdout.take().unwrap()
            } else {
                let mut stdout = CapturedOutput::default();
                let stdio = Stdio::from(stdout.try_clone().unwrap());
                captured_stdout = Some(stdout);
                stdio
            };

            let stderr = if self.stderr.is_some() {
                self.stderr.take().unwrap()
            } else {
                let mut stderr = CapturedOutput::default();
                let stdio = Stdio::from(stderr.try_clone().unwrap());
                captured_stderr = Some(stderr);
                stdio
            };

            command
                .stdin(self.stdin.take().unwrap_or_else(Stdio::null))
                .stdout(stdout)
                .stderr(stderr);
        };

        (command, captured_stdout, captured_stderr)
    }

    /// Spawns the command, feeds the stdin if any, and returns the
    /// child process immediately.
    pub fn run_no_wait(&mut self) -> UChild {
        assert!(!self.has_run, "{}", ALREADY_RUN);
        self.has_run = true;

        let (mut command, captured_stdout, captured_stderr) = self.build();
        log_info("run", self.to_string());

        let child = command.spawn().unwrap();

        #[cfg(any(target_os = "linux", target_os = "android"))]
        for &(resource, soft_limit, hard_limit) in &self.limits {
            prlimit(
                child.id() as i32,
                resource,
                Some((soft_limit, hard_limit)),
                None,
            )
            .unwrap();
        }

        let mut child = UChild::from(self, child, captured_stdout, captured_stderr);

        if let Some(input) = self.bytes_into_stdin.take() {
            child.pipe_in(input);
        }

        child
    }

    /// Spawns the command, feeds the stdin if any, waits for the result
    /// and returns a command result.
    /// It is recommended that you instead use succeeds() or fails()
    pub fn run(&mut self) -> CmdResult {
        self.run_no_wait().wait().unwrap()
    }

    /// Spawns the command, feeding the passed in stdin, waits for the result
    /// and returns a command result.
    /// It is recommended that, instead of this, you use a combination of `pipe_in()`
    /// with succeeds() or fails()
    pub fn run_piped_stdin<T: Into<Vec<u8>>>(&mut self, input: T) -> CmdResult {
        self.pipe_in(input).run()
    }

    /// Spawns the command, feeds the stdin if any, waits for the result,
    /// asserts success, and returns a command result.
    #[track_caller]
    pub fn succeeds(&mut self) -> CmdResult {
        let cmd_result = self.run();
        cmd_result.success();
        cmd_result
    }

    /// Spawns the command, feeds the stdin if any, waits for the result,
    /// asserts failure, and returns a command result.
    #[track_caller]
    pub fn fails(&mut self) -> CmdResult {
        let cmd_result = self.run();
        cmd_result.failure();
        cmd_result
    }

    pub fn get_full_fixture_path(&self, file_rel_path: &str) -> String {
        let tmpdir_path = self.tmpd.as_ref().unwrap().path();
        format!("{}/{file_rel_path}", tmpdir_path.to_str().unwrap())
    }
}

impl std::fmt::Display for UCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut comm_string: Vec<String> = vec![self
            .bin_path
            .as_ref()
            .map_or(String::new(), |p| p.display().to_string())];
        comm_string.extend(self.args.iter().map(|s| s.to_string_lossy().to_string()));
        f.write_str(&comm_string.join(" "))
    }
}

/// Stored the captured output in a temporary file. The file is deleted as soon as
/// [`CapturedOutput`] is dropped.
#[derive(Debug)]
struct CapturedOutput {
    current_file: File,
    output: tempfile::NamedTempFile, // drop last
}

impl CapturedOutput {
    /// Creates a new instance of `CapturedOutput`
    fn new(output: tempfile::NamedTempFile) -> Self {
        Self {
            current_file: output.reopen().unwrap(),
            output,
        }
    }

    /// Try to clone the file pointer.
    fn try_clone(&mut self) -> io::Result<File> {
        self.output.as_file().try_clone()
    }

    /// Return the captured output as [`String`].
    ///
    /// Subsequent calls to any of the other output methods will operate on the subsequent output.
    fn output(&mut self) -> String {
        String::from_utf8(self.output_bytes()).unwrap()
    }

    /// Return the exact amount of bytes as `String`.
    ///
    /// Subsequent calls to any of the other output methods will operate on the subsequent output.
    ///
    /// # Important
    ///
    /// This method blocks indefinitely if the amount of bytes given by `size` cannot be read
    fn output_exact(&mut self, size: usize) -> String {
        String::from_utf8(self.output_exact_bytes(size)).unwrap()
    }

    /// Return the captured output as bytes.
    ///
    /// Subsequent calls to any of the other output methods will operate on the subsequent output.
    fn output_bytes(&mut self) -> Vec<u8> {
        let mut buffer = Vec::<u8>::new();
        self.current_file.read_to_end(&mut buffer).unwrap();
        buffer
    }

    /// Return all captured output, so far.
    ///
    /// Subsequent calls to any of the other output methods will operate on the subsequent output.
    fn output_all_bytes(&mut self) -> Vec<u8> {
        let mut buffer = Vec::<u8>::new();
        let mut file = self.output.reopen().unwrap();

        file.read_to_end(&mut buffer).unwrap();
        self.current_file = file;

        buffer
    }

    /// Return the exact amount of bytes.
    ///
    /// Subsequent calls to any of the other output methods will operate on the subsequent output.
    ///
    /// # Important
    ///
    /// This method blocks indefinitely if the amount of bytes given by `size` cannot be read
    fn output_exact_bytes(&mut self, size: usize) -> Vec<u8> {
        let mut buffer = vec![0; size];
        self.current_file.read_exact(&mut buffer).unwrap();
        buffer
    }
}

impl Default for CapturedOutput {
    fn default() -> Self {
        let mut retries = 10;
        let file = loop {
            let file = Builder::new().rand_bytes(10).suffix(".out").tempfile();
            if file.is_ok() || retries <= 0 {
                break file.unwrap();
            }
            sleep(Duration::from_millis(100));
            retries -= 1;
        };
        Self {
            current_file: file.reopen().unwrap(),
            output: file,
        }
    }
}

impl Drop for CapturedOutput {
    fn drop(&mut self) {
        let _ = remove_file(self.output.path());
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AssertionMode {
    All,
    Current,
    Exact(usize, usize),
}
pub struct UChildAssertion<'a> {
    uchild: &'a mut UChild,
}

impl<'a> UChildAssertion<'a> {
    pub fn new(uchild: &'a mut UChild) -> Self {
        Self { uchild }
    }

    fn with_output(&mut self, mode: AssertionMode) -> CmdResult {
        let exit_status = if self.uchild.is_alive() {
            None
        } else {
            Some(self.uchild.raw.wait().unwrap())
        };
        let (stdout, stderr) = match mode {
            AssertionMode::All => (
                self.uchild.stdout_all_bytes(),
                self.uchild.stderr_all_bytes(),
            ),
            AssertionMode::Current => (self.uchild.stdout_bytes(), self.uchild.stderr_bytes()),
            AssertionMode::Exact(expected_stdout_size, expected_stderr_size) => (
                self.uchild.stdout_exact_bytes(expected_stdout_size),
                self.uchild.stderr_exact_bytes(expected_stderr_size),
            ),
        };
        CmdResult::new(
            self.uchild.bin_path.clone(),
            self.uchild.util_name.clone(),
            self.uchild.tmpd.clone(),
            exit_status,
            stdout,
            stderr,
        )
    }

    // Make assertions of [`CmdResult`] with all output from start of the process until now.
    //
    // This method runs [`UChild::stdout_all_bytes`] and [`UChild::stderr_all_bytes`] under the
    // hood. See there for side effects
    pub fn with_all_output(&mut self) -> CmdResult {
        self.with_output(AssertionMode::All)
    }

    // Make assertions of [`CmdResult`] with the current output.
    //
    // This method runs [`UChild::stdout_bytes`] and [`UChild::stderr_bytes`] under the hood. See
    // there for side effects
    pub fn with_current_output(&mut self) -> CmdResult {
        self.with_output(AssertionMode::Current)
    }

    // Make assertions of [`CmdResult`] with the exact output.
    //
    // This method runs [`UChild::stdout_exact_bytes`] and [`UChild::stderr_exact_bytes`] under the
    // hood. See there for side effects
    pub fn with_exact_output(
        &mut self,
        expected_stdout_size: usize,
        expected_stderr_size: usize,
    ) -> CmdResult {
        self.with_output(AssertionMode::Exact(
            expected_stdout_size,
            expected_stderr_size,
        ))
    }

    // Assert that the child process is alive
    #[track_caller]
    pub fn is_alive(&mut self) -> &mut Self {
        match self
            .uchild
            .raw
            .try_wait()
        {
            Ok(Some(status)) => panic!(
                "Assertion failed. Expected '{}' to be running but exited with status={}.\nstdout: {}\nstderr: {}",
                uucore::util_name(),
                status,
                self.uchild.stdout_all(),
                self.uchild.stderr_all()
            ),
            Ok(None) => {}
            Err(error) => panic!("Assertion failed with error '{error:?}'"),
        }

        self
    }

    // Assert that the child process has exited
    #[track_caller]
    pub fn is_not_alive(&mut self) -> &mut Self {
        match self
            .uchild
            .raw
            .try_wait()
        {
            Ok(None) => panic!(
                "Assertion failed. Expected '{}' to be not running but was alive.\nstdout: {}\nstderr: {}",
                uucore::util_name(),
                self.uchild.stdout_all(),
                self.uchild.stderr_all()),
            Ok(_) =>  {},
            Err(error) => panic!("Assertion failed with error '{error:?}'"),
        }

        self
    }
}

/// Abstraction for a [`std::process::Child`] to handle the child process.
pub struct UChild {
    raw: Child,
    bin_path: PathBuf,
    util_name: Option<String>,
    captured_stdout: Option<CapturedOutput>,
    captured_stderr: Option<CapturedOutput>,
    ignore_stdin_write_error: bool,
    stderr_to_stdout: bool,
    join_handle: Option<JoinHandle<io::Result<()>>>,
    timeout: Option<Duration>,
    tmpd: Option<Rc<TempDir>>, // drop last
}

impl UChild {
    fn from(
        ucommand: &UCommand,
        child: Child,
        captured_stdout: Option<CapturedOutput>,
        captured_stderr: Option<CapturedOutput>,
    ) -> Self {
        Self {
            raw: child,
            bin_path: ucommand.bin_path.clone().unwrap(),
            util_name: ucommand.util_name.clone(),
            captured_stdout,
            captured_stderr,
            ignore_stdin_write_error: ucommand.ignore_stdin_write_error,
            stderr_to_stdout: ucommand.stderr_to_stdout,
            join_handle: None,
            timeout: ucommand.timeout,
            tmpd: ucommand.tmpd.clone(),
        }
    }

    /// Convenience method for `sleep(Duration::from_millis(millis))`
    pub fn delay(&mut self, millis: u64) -> &mut Self {
        sleep(Duration::from_millis(millis));
        self
    }

    /// Return the pid of the child process, similar to [`Child::id`].
    pub fn id(&self) -> u32 {
        self.raw.id()
    }

    /// Return true if the child process is still alive and false otherwise.
    pub fn is_alive(&mut self) -> bool {
        self.raw.try_wait().unwrap().is_none()
    }

    /// Return true if the child process is exited and false otherwise.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_not_alive(&mut self) -> bool {
        !self.is_alive()
    }

    /// Return a [`UChildAssertion`]
    pub fn make_assertion(&mut self) -> UChildAssertion {
        UChildAssertion::new(self)
    }

    /// Convenience function for calling [`UChild::delay`] and then [`UChild::make_assertion`]
    pub fn make_assertion_with_delay(&mut self, millis: u64) -> UChildAssertion {
        self.delay(millis).make_assertion()
    }

    /// Try to kill the child process and wait for it's termination.
    ///
    /// This method blocks until the child process is killed, but returns an error if `self.timeout`
    /// or the default of 60s was reached. If no such error happened, the process resources are
    /// released, so there is usually no need to call `wait` or alike on unix systems although it's
    /// still possible to do so.
    ///
    /// # Platform specific behavior
    ///
    /// On unix systems the child process resources will be released like a call to [`Child::wait`]
    /// or alike would do.
    ///
    /// # Error
    ///
    /// If [`Child::kill`] returned an error or if the child process could not be terminated within
    /// `self.timeout` or the default of 60s.
    pub fn try_kill(&mut self) -> io::Result<()> {
        let start = Instant::now();
        self.raw.kill()?;

        let timeout = self.timeout.unwrap_or(Duration::from_secs(60));
        // As a side effect, we're cleaning up the killed child process with the implicit call to
        // `Child::try_wait` in `self.is_alive`, which reaps the process id on unix systems. We
        // always fail with error on timeout if `self.timeout` is set to zero.
        while self.is_alive() || timeout == Duration::ZERO {
            if start.elapsed() < timeout {
                self.delay(10);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("kill: Timeout of '{}s' reached", timeout.as_secs_f64()),
                ));
            }
            hint::spin_loop();
        }

        Ok(())
    }

    /// Terminate the child process unconditionally and wait for the termination.
    ///
    /// Ignores any errors happening during [`Child::kill`] (i.e. child process already exited) but
    /// still panics on timeout.
    ///
    /// # Panics
    /// If the child process could not be terminated within `self.timeout` or the default of 60s.
    pub fn kill(&mut self) -> &mut Self {
        self.try_kill()
            .or_else(|error| {
                // We still throw the error on timeout in the `try_kill` function
                if error.kind() == io::ErrorKind::Other {
                    Err(error)
                } else {
                    Ok(())
                }
            })
            .unwrap();
        self
    }

    /// Wait for the child process to terminate and return a [`CmdResult`].
    ///
    /// See [`UChild::wait_with_output`] for details on timeouts etc. This method can also be run if
    /// the child process was killed with [`UChild::kill`].
    ///
    /// # Errors
    ///
    /// Returns the error from the call to [`UChild::wait_with_output`] if any
    pub fn wait(self) -> io::Result<CmdResult> {
        let (bin_path, util_name, tmpd) = (
            self.bin_path.clone(),
            self.util_name.clone(),
            self.tmpd.clone(),
        );

        #[allow(deprecated)]
        let output = self.wait_with_output()?;

        Ok(CmdResult {
            bin_path,
            util_name,
            tmpd,
            exit_status: Some(output.status),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    /// Wait for the child process to terminate and return an instance of [`Output`].
    ///
    /// If `self.timeout` is reached while waiting, a [`io::ErrorKind::Other`] representing a
    /// timeout error is returned. If no errors happened, we join with the thread created by
    /// [`UChild::pipe_in`] if any.
    ///
    /// # Error
    ///
    /// If `self.timeout` is reached while waiting or [`Child::wait_with_output`] returned an
    /// error.
    #[deprecated = "Please use wait() -> io::Result<CmdResult> instead."]
    pub fn wait_with_output(mut self) -> io::Result<Output> {
        let output = if let Some(timeout) = self.timeout {
            let child = self.raw;

            let (sender, receiver) = mpsc::channel();
            let handle = thread::spawn(move || sender.send(child.wait_with_output()));

            match receiver.recv_timeout(timeout) {
                Ok(result) => {
                    // unwraps are safe here because we got a result from the sender and there was no panic
                    // causing a disconnect.
                    handle.join().unwrap().unwrap();
                    result
                }
                Err(RecvTimeoutError::Timeout) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("wait: Timeout of '{}s' reached", timeout.as_secs_f64()),
                )),
                Err(RecvTimeoutError::Disconnected) => {
                    handle.join().expect("Panic caused disconnect").unwrap();
                    panic!("Error receiving from waiting thread because of unexpected disconnect");
                }
            }
        } else {
            self.raw.wait_with_output()
        };

        let mut output = output?;

        if let Some(join_handle) = self.join_handle.take() {
            join_handle
                .join()
                .expect("Error joining with the piping stdin thread")
                .unwrap();
        };

        if let Some(stdout) = self.captured_stdout.as_mut() {
            output.stdout = stdout.output_bytes();
        }
        if let Some(stderr) = self.captured_stderr.as_mut() {
            output.stderr = stderr.output_bytes();
        }

        Ok(output)
    }

    /// Read, consume and return the output as [`String`] from [`Child`]'s stdout.
    ///
    /// See also [`UChild::stdout_bytes`] for side effects.
    pub fn stdout(&mut self) -> String {
        String::from_utf8(self.stdout_bytes()).unwrap()
    }

    /// Read and return all child's output in stdout as String.
    ///
    /// Note, that a subsequent call of any of these functions
    ///
    /// * [`UChild::stdout`]
    /// * [`UChild::stdout_bytes`]
    /// * [`UChild::stdout_exact_bytes`]
    ///
    /// will operate on the subsequent output of the child process.
    pub fn stdout_all(&mut self) -> String {
        String::from_utf8(self.stdout_all_bytes()).unwrap()
    }

    /// Read, consume and return the output as bytes from [`Child`]'s stdout.
    ///
    /// Each subsequent call to any of the functions below will operate on the subsequent output of
    /// the child process:
    ///
    /// * [`UChild::stdout`]
    /// * [`UChild::stdout_exact_bytes`]
    /// * and the call to itself [`UChild::stdout_bytes`]
    pub fn stdout_bytes(&mut self) -> Vec<u8> {
        match self.captured_stdout.as_mut() {
            Some(output) => output.output_bytes(),
            None if self.raw.stdout.is_some() => {
                let mut buffer: Vec<u8> = vec![];
                let stdout = self.raw.stdout.as_mut().unwrap();
                stdout.read_to_end(&mut buffer).unwrap();
                buffer
            }
            None => vec![],
        }
    }

    /// Read and return all output from start of the child process until now.
    ///
    /// Each subsequent call of any of the methods below will operate on the subsequent output of
    /// the child process. This method will panic if the output wasn't captured (for example if
    /// [`UCommand::set_stdout`] was used).
    ///
    /// * [`UChild::stdout`]
    /// * [`UChild::stdout_bytes`]
    /// * [`UChild::stdout_exact_bytes`]
    pub fn stdout_all_bytes(&mut self) -> Vec<u8> {
        match self.captured_stdout.as_mut() {
            Some(output) => output.output_all_bytes(),
            None => {
                panic!("Usage error: This method cannot be used if the output wasn't captured.")
            }
        }
    }

    /// Read, consume and return the exact amount of bytes from `stdout`.
    ///
    /// This method may block indefinitely if the `size` amount of bytes exceeds the amount of bytes
    /// that can be read. See also [`UChild::stdout_bytes`] for side effects.
    pub fn stdout_exact_bytes(&mut self, size: usize) -> Vec<u8> {
        match self.captured_stdout.as_mut() {
            Some(output) => output.output_exact_bytes(size),
            None if self.raw.stdout.is_some() => {
                let mut buffer = vec![0; size];
                let stdout = self.raw.stdout.as_mut().unwrap();
                stdout.read_exact(&mut buffer).unwrap();
                buffer
            }
            None => vec![],
        }
    }

    /// Read, consume and return the child's stderr as String.
    ///
    /// See also [`UChild::stdout_bytes`] for side effects. If stderr is redirected to stdout with
    /// [`UCommand::stderr_to_stdout`] then always an empty string will be returned.
    pub fn stderr(&mut self) -> String {
        String::from_utf8(self.stderr_bytes()).unwrap()
    }

    /// Read and return all child's output in stderr as String.
    ///
    /// Note, that a subsequent call of any of these functions
    ///
    /// * [`UChild::stderr`]
    /// * [`UChild::stderr_bytes`]
    /// * [`UChild::stderr_exact_bytes`]
    ///
    /// will operate on the subsequent output of the child process. If stderr is redirected to
    /// stdout with [`UCommand::stderr_to_stdout`] then always an empty string will be returned.
    pub fn stderr_all(&mut self) -> String {
        String::from_utf8(self.stderr_all_bytes()).unwrap()
    }

    /// Read, consume and return the currently available bytes from child's stderr.
    ///
    /// If stderr is redirected to stdout with [`UCommand::stderr_to_stdout`] then always zero bytes
    /// are returned. See also [`UChild::stdout_bytes`] for side effects.
    pub fn stderr_bytes(&mut self) -> Vec<u8> {
        match self.captured_stderr.as_mut() {
            Some(output) => output.output_bytes(),
            None if self.raw.stderr.is_some() => {
                let mut buffer: Vec<u8> = vec![];
                let stderr = self.raw.stderr.as_mut().unwrap();
                stderr.read_to_end(&mut buffer).unwrap();
                buffer
            }
            None => vec![],
        }
    }

    /// Read and return all output from start of the child process until now.
    ///
    /// Each subsequent call of any of the methods below will operate on the subsequent output of
    /// the child process. This method will panic if the output wasn't captured (for example if
    /// [`UCommand::set_stderr`] was used). If [`UCommand::stderr_to_stdout`] was used always zero
    /// bytes are returned.
    ///
    /// * [`UChild::stderr`]
    /// * [`UChild::stderr_bytes`]
    /// * [`UChild::stderr_exact_bytes`]
    pub fn stderr_all_bytes(&mut self) -> Vec<u8> {
        match self.captured_stderr.as_mut() {
            Some(output) => output.output_all_bytes(),
            None if self.stderr_to_stdout => vec![],
            None => {
                panic!("Usage error: This method cannot be used if the output wasn't captured.")
            }
        }
    }

    /// Read, consume and return the exact amount of bytes from stderr.
    ///
    /// If stderr is redirect to stdout with [`UCommand::stderr_to_stdout`] then always zero bytes
    /// are returned.
    ///
    /// # Important
    /// This method blocks indefinitely if the `size` amount of bytes cannot be read.
    pub fn stderr_exact_bytes(&mut self, size: usize) -> Vec<u8> {
        match self.captured_stderr.as_mut() {
            Some(output) => output.output_exact_bytes(size),
            None if self.raw.stderr.is_some() => {
                let stderr = self.raw.stderr.as_mut().unwrap();
                let mut buffer = vec![0; size];
                stderr.read_exact(&mut buffer).unwrap();
                buffer
            }
            None => vec![],
        }
    }

    /// Pipe data into [`Child`] stdin in a separate thread to avoid deadlocks.
    ///
    /// In contrast to [`UChild::write_in`], this method is designed to simulate a pipe on the
    /// command line and can be used only once or else panics. Note, that [`UCommand::set_stdin`]
    /// must be used together with [`Stdio::piped`] or else this method doesn't work as expected.
    /// `Stdio::piped` is the current default when using [`UCommand::run_no_wait`]) without calling
    /// `set_stdin`. This method stores a [`JoinHandle`] of the thread in which the writing to the
    /// child processes' stdin is running. The associated thread is joined with the main process in
    /// the methods below when exiting the child process.
    ///
    /// * [`UChild::wait`]
    /// * [`UChild::wait_with_output`]
    /// * [`UChild::pipe_in_and_wait`]
    /// * [`UChild::pipe_in_and_wait_with_output`]
    ///
    /// Usually, there's no need to join manually but if needed, the [`UChild::join`] method can be
    /// used .
    ///
    /// [`JoinHandle`]: std::thread::JoinHandle
    pub fn pipe_in<T: Into<Vec<u8>>>(&mut self, content: T) -> &mut Self {
        let ignore_stdin_write_error = self.ignore_stdin_write_error;
        let content = content.into();
        let stdin = self
            .raw
            .stdin
            .take()
            .expect("Could not pipe into child process. Was it set to Stdio::null()?");

        let join_handle = thread::spawn(move || {
            let mut writer = BufWriter::new(stdin);

            match writer.write_all(&content).and_then(|()| writer.flush()) {
                Err(error) if !ignore_stdin_write_error => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("failed to write to stdin of child: {error}"),
                )),
                Ok(()) | Err(_) => Ok(()),
            }
        });

        self.join_handle = Some(join_handle);
        self
    }

    /// Call join on the thread created by [`UChild::pipe_in`] and if the thread is still running.
    ///
    /// This method can be called multiple times but is a noop if already joined.
    pub fn join(&mut self) -> &mut Self {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle
                .join()
                .expect("Error joining with the piping stdin thread")
                .unwrap();
        }
        self
    }

    /// Convenience method for [`UChild::pipe_in`] and then [`UChild::wait`]
    pub fn pipe_in_and_wait<T: Into<Vec<u8>>>(mut self, content: T) -> CmdResult {
        self.pipe_in(content);
        self.wait().unwrap()
    }

    /// Convenience method for [`UChild::pipe_in`] and then [`UChild::wait_with_output`]
    #[deprecated = "Please use pipe_in_and_wait() -> CmdResult instead."]
    pub fn pipe_in_and_wait_with_output<T: Into<Vec<u8>>>(mut self, content: T) -> Output {
        self.pipe_in(content);

        #[allow(deprecated)]
        self.wait_with_output().unwrap()
    }

    /// Write some bytes to the child process stdin.
    ///
    /// This function is meant for small data and faking user input like typing a `yes` or `no`.
    /// This function blocks until all data is written but can be used multiple times in contrast to
    /// [`UChild::pipe_in`].
    ///
    /// # Errors
    /// If [`ChildStdin::write_all`] or [`ChildStdin::flush`] returned an error
    pub fn try_write_in<T: Into<Vec<u8>>>(&mut self, data: T) -> io::Result<()> {
        let stdin = self.raw.stdin.as_mut().unwrap();

        match stdin.write_all(&data.into()).and_then(|()| stdin.flush()) {
            Err(error) if !self.ignore_stdin_write_error => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("failed to write to stdin of child: {error}"),
            )),
            Ok(()) | Err(_) => Ok(()),
        }
    }

    /// Convenience function for [`UChild::try_write_in`] and a following `unwrap`.
    pub fn write_in<T: Into<Vec<u8>>>(&mut self, data: T) -> &mut Self {
        self.try_write_in(data).unwrap();
        self
    }

    /// Close the child process stdout.
    ///
    /// Note this will have no effect if the output was captured with [`CapturedOutput`] which is the
    /// default if [`UCommand::set_stdout`] wasn't called.
    pub fn close_stdout(&mut self) -> &mut Self {
        self.raw.stdout.take();
        self
    }

    /// Close the child process stderr.
    ///
    /// Note this will have no effect if the output was captured with [`CapturedOutput`] which is the
    /// default if [`UCommand::set_stderr`] wasn't called.
    pub fn close_stderr(&mut self) -> &mut Self {
        self.raw.stderr.take();
        self
    }

    /// Close the child process stdin.
    ///
    /// Note, this does not have any effect if using the [`UChild::pipe_in`] method.
    pub fn close_stdin(&mut self) -> &mut Self {
        self.raw.stdin.take();
        self
    }
}

pub fn vec_of_size(n: usize) -> Vec<u8> {
    let result = vec![b'a'; n];
    assert_eq!(result.len(), n);
    result
}

pub fn whoami() -> String {
    // Apparently some CI environments have configuration issues, e.g. with 'whoami' and 'id'.
    //
    // From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
    //    whoami: cannot find name for user ID 1001
    // id --name: cannot find name for user ID 1001
    // id --name: cannot find name for group ID 116
    //
    // However, when running "id" from within "/bin/bash" it looks fine:
    // id: "uid=1001(runner) gid=118(docker) groups=118(docker),4(adm),101(systemd-journal)"
    // whoami: "runner"

    // Use environment variable to get current user instead of
    // invoking `whoami` and fall back to user "nobody" on error.
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|e| {
            println!("{UUTILS_WARNING}: {e}, using \"nobody\" instead");
            "nobody".to_string()
        })
}

/// Add prefix 'g' for `util_name` if not on linux
#[cfg(unix)]
pub fn host_name_for(util_name: &str) -> Cow<str> {
    // In some environments, e.g. macOS/freebsd, the GNU coreutils are prefixed with "g"
    // to not interfere with the BSD counterparts already in `$PATH`.
    #[cfg(not(target_os = "linux"))]
    {
        // make call to `host_name_for` idempotent
        if util_name.starts_with('g') && util_name != "groups" {
            util_name.into()
        } else {
            format!("g{util_name}").into()
        }
    }
    #[cfg(target_os = "linux")]
    util_name.into()
}

// GNU coreutils version 8.32 is the reference version since it is the latest version and the
// GNU test suite in "coreutils/.github/workflows/GnuTests.yml" runs against it.
// However, here 8.30 was chosen because right now there's no ubuntu image for the github actions
// CICD available with a higher version than 8.30.
// GNU coreutils versions from the CICD images for comparison:
// ubuntu-2004: 8.30 (latest)
// ubuntu-1804: 8.28
// macos-latest: 8.32
const VERSION_MIN: &str = "8.30"; // minimum Version for the reference `coreutil` in `$PATH`

const UUTILS_WARNING: &str = "uutils-tests-warning";
const UUTILS_INFO: &str = "uutils-tests-info";

/// Run `util_name --version` and return Ok if the version is >= `version_expected`.
/// Returns an error if
///     * `util_name` cannot run
///     * the version cannot be parsed
///     * the version is too low
///
/// This is used by `expected_result` to check if the coreutils version is >= `VERSION_MIN`.
/// It makes sense to use this manually in a test if a feature
/// is tested that was introduced after `VERSION_MIN`
///
/// Example:
///
/// ```no_run
/// use crate::common::util::*;
/// const VERSION_MIN_MULTIPLE_USERS: &str = "8.31";
///
/// #[test]
/// fn test_xyz() {
///     unwrap_or_return!(check_coreutil_version(
///         util_name!(),
///         VERSION_MIN_MULTIPLE_USERS
///     ));
///     // proceed with the test...
/// }
/// ```
#[cfg(unix)]
pub fn check_coreutil_version(
    util_name: &str,
    version_expected: &str,
) -> std::result::Result<String, String> {
    // example:
    // $ id --version | head -n 1
    // id (GNU coreutils) 8.32.162-4eda

    let util_name = &host_name_for(util_name);
    log_info("run", format!("{util_name} --version"));
    let version_check = match Command::new(util_name.as_ref())
        .env("LC_ALL", "C")
        .arg("--version")
        .output()
    {
        Ok(s) => s,
        Err(e) => return Err(format!("{UUTILS_WARNING}: '{util_name}' {e}")),
    };
    std::str::from_utf8(&version_check.stdout).unwrap()
        .split('\n')
        .collect::<Vec<_>>()
        .first()
        .map_or_else(
            || Err(format!("{UUTILS_WARNING}: unexpected output format for reference coreutil: '{util_name} --version'")),
            |s| {
                if s.contains(&format!("(GNU coreutils) {version_expected}")) {
                    Ok(format!("{UUTILS_INFO}: {s}"))
                } else if s.contains("(GNU coreutils)") {
                    let version_found = parse_coreutil_version(s);
                    let version_expected = version_expected.parse::<f32>().unwrap_or_default();
                    if version_found > version_expected {
                    Ok(format!("{UUTILS_INFO}: version for the reference coreutil '{util_name}' is higher than expected; expected: {version_expected}, found: {version_found}"))
                    } else {
                    Err(format!("{UUTILS_WARNING}: version for the reference coreutil '{util_name}' does not match; expected: {version_expected}, found: {version_found}")) }
                } else {
                    Err(format!("{UUTILS_WARNING}: no coreutils version string found for reference coreutils '{util_name} --version'"))
                }
            },
        )
}

// simple heuristic to parse the coreutils SemVer string, e.g. "id (GNU coreutils) 8.32.263-0475"
fn parse_coreutil_version(version_string: &str) -> f32 {
    version_string
        .split_whitespace()
        .last()
        .unwrap()
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".")
        .parse::<f32>()
        .unwrap_or_default()
}

/// This runs the GNU coreutils `util_name` binary in `$PATH` in order to
/// dynamically gather reference values on the system.
/// If the `util_name` in `$PATH` doesn't include a coreutils version string,
/// or the version is too low, this returns an error and the test should be skipped.
///
/// Example:
///
/// ```no_run
/// use crate::common::util::*;
/// #[test]
/// fn test_xyz() {
///     let ts = TestScenario::new(util_name!());
///     let result = ts.ucmd().run();
///     let exp_result = unwrap_or_return!(expected_result(&ts, &[]));
///     result
///         .stdout_is(exp_result.stdout_str())
///         .stderr_is(exp_result.stderr_str())
///         .code_is(exp_result.code());
/// }
///```
#[cfg(unix)]
pub fn expected_result(ts: &TestScenario, args: &[&str]) -> std::result::Result<CmdResult, String> {
    let util_name = ts.util_name.as_str();
    println!("{}", check_coreutil_version(util_name, VERSION_MIN)?);
    let util_name = host_name_for(util_name);

    let result = ts
        .cmd(util_name.as_ref())
        .env("PATH", PATH)
        .envs(DEFAULT_ENV)
        .args(args)
        .run();

    let (stdout, stderr): (String, String) = if cfg!(target_os = "linux") {
        (
            result.stdout_str().to_string(),
            result.stderr_str().to_string(),
        )
    } else {
        // `host_name_for` added prefix, strip 'g' prefix from results:
        let from = util_name.to_string() + ":";
        let to = &from[1..];
        (
            result.stdout_str().replace(&from, to),
            result.stderr_str().replace(&from, to),
        )
    };

    Ok(CmdResult::new(
        ts.bin_path.as_os_str().to_str().unwrap().to_string(),
        Some(ts.util_name.clone()),
        Some(result.tmpd()),
        result.exit_status,
        stdout.as_bytes(),
        stderr.as_bytes(),
    ))
}

/// This is a convenience wrapper to run a ucmd with root permissions.
/// It can be used to test programs when being root is needed
/// This runs `sudo -E --non-interactive target/debug/coreutils util_name args`
/// This is primarily designed to run in an environment where whoami is in $path
/// and where non-interactive sudo is possible.
/// To check if i) non-interactive sudo is possible and ii) if sudo works, this runs:
/// `sudo -E --non-interactive whoami` first.
///
/// This return an `Err()` if run inside CICD because there's no 'sudo'.
///
/// Example:
///
/// ```no_run
/// use crate::common::util::*;
/// #[test]
/// fn test_xyz() {
///    let ts = TestScenario::new("whoami");
///    let expected = "root\n".to_string();
///    if let Ok(result) = run_ucmd_as_root(&ts, &[]) {
///        result.stdout_is(expected);
///    } else {
///        println!("TEST SKIPPED");
///    }
/// }
///```
#[cfg(unix)]
pub fn run_ucmd_as_root(
    ts: &TestScenario,
    args: &[&str],
) -> std::result::Result<CmdResult, String> {
    run_ucmd_as_root_with_stdin_stdout(ts, args, None, None)
}

#[cfg(unix)]
pub fn run_ucmd_as_root_with_stdin_stdout(
    ts: &TestScenario,
    args: &[&str],
    stdin: Option<&str>,
    stdout: Option<&str>,
) -> std::result::Result<CmdResult, String> {
    if is_ci() {
        Err(format!("{UUTILS_INFO}: {}", "cannot run inside CI"))
    } else {
        // check if we can run 'sudo'
        log_info("run", "sudo -E --non-interactive whoami");
        match Command::new("sudo")
            .envs(DEFAULT_ENV)
            .args(["-E", "--non-interactive", "whoami"])
            .output()
        {
            Ok(output) if String::from_utf8_lossy(&output.stdout).eq("root\n") => {
                // we can run sudo and we're root
                // run ucmd as root:
                let mut cmd = ts.cmd("sudo");
                cmd.env("PATH", PATH)
                    .envs(DEFAULT_ENV)
                    .arg("-E")
                    .arg("--non-interactive")
                    .arg(&ts.bin_path)
                    .arg(&ts.util_name)
                    .args(args);
                if let Some(stdin) = stdin {
                    cmd.set_stdin(File::open(stdin).unwrap());
                }
                if let Some(stdout) = stdout {
                    cmd.set_stdout(File::open(stdout).unwrap());
                }
                Ok(cmd.run())
            }
            Ok(output)
                if String::from_utf8_lossy(&output.stderr).eq("sudo: a password is required\n") =>
            {
                Err("Cannot run non-interactive sudo".to_string())
            }
            Ok(_output) => Err("\"sudo whoami\" didn't return \"root\"".to_string()),
            Err(e) => Err(format!("{UUTILS_WARNING}: {e}")),
        }
    }
}

/// Sanity checks for test utils
#[cfg(test)]
mod tests {
    // spell-checker:ignore (tests) asdfsadfa
    use super::*;

    pub fn run_cmd<T: AsRef<OsStr>>(cmd: T) -> CmdResult {
        UCommand::new().arg(cmd).run()
    }

    #[test]
    fn test_command_result_when_no_output_with_exit_32() {
        let result = run_cmd("exit 32");

        if cfg!(windows) {
            std::assert!(result.bin_path.ends_with("cmd"));
        } else {
            std::assert!(result.bin_path.ends_with("sh"));
        }

        std::assert!(result.util_name.is_none());
        std::assert!(result.tmpd.is_some());

        assert!(result.exit_status.is_some());
        std::assert_eq!(result.code(), 32);
        result.code_is(32);
        assert!(!result.succeeded());
        result.failure();
        result.fails_silently();
        assert!(result.stderr.is_empty());
        assert!(result.stdout.is_empty());
        result.no_output();
        result.no_stderr();
        result.no_stdout();
    }

    #[test]
    #[should_panic]
    fn test_command_result_when_exit_32_then_success_panic() {
        run_cmd("exit 32").success();
    }

    #[test]
    fn test_command_result_when_no_output_with_exit_0() {
        let result = run_cmd("exit 0");

        assert!(result.exit_status.is_some());
        std::assert_eq!(result.code(), 0);
        result.code_is(0);
        assert!(result.succeeded());
        result.success();
        assert!(result.stderr.is_empty());
        assert!(result.stdout.is_empty());
        result.no_output();
        result.no_stderr();
        result.no_stdout();
    }

    #[test]
    #[should_panic]
    fn test_command_result_when_exit_0_then_failure_panics() {
        run_cmd("exit 0").failure();
    }

    #[test]
    #[should_panic]
    fn test_command_result_when_exit_0_then_silent_failure_panics() {
        run_cmd("exit 0").fails_silently();
    }

    #[test]
    fn test_command_result_when_stdout_with_exit_0() {
        #[cfg(windows)]
        let (result, vector, string) = (
            run_cmd("echo hello& exit 0"),
            vec![b'h', b'e', b'l', b'l', b'o', b'\r', b'\n'],
            "hello\r\n",
        );
        #[cfg(not(windows))]
        let (result, vector, string) = (
            run_cmd("echo hello; exit 0"),
            vec![b'h', b'e', b'l', b'l', b'o', b'\n'],
            "hello\n",
        );

        assert!(result.exit_status.is_some());
        std::assert_eq!(result.code(), 0);
        result.code_is(0);
        assert!(result.succeeded());
        result.success();
        assert!(result.stderr.is_empty());
        std::assert_eq!(result.stdout, vector);
        result.no_stderr();
        result.stdout_is(string);
        result.stdout_is_bytes(&vector);
        result.stdout_only(string);
        result.stdout_only_bytes(&vector);
    }

    #[test]
    fn test_command_result_when_stderr_with_exit_0() {
        #[cfg(windows)]
        let (result, vector, string) = (
            run_cmd("echo hello>&2& exit 0"),
            vec![b'h', b'e', b'l', b'l', b'o', b'\r', b'\n'],
            "hello\r\n",
        );
        #[cfg(not(windows))]
        let (result, vector, string) = (
            run_cmd("echo hello >&2; exit 0"),
            vec![b'h', b'e', b'l', b'l', b'o', b'\n'],
            "hello\n",
        );

        assert!(result.exit_status.is_some());
        std::assert_eq!(result.code(), 0);
        result.code_is(0);
        assert!(result.succeeded());
        result.success();
        assert!(result.stdout.is_empty());
        result.no_stdout();
        std::assert_eq!(result.stderr, vector);
        result.stderr_is(string);
        result.stderr_is_bytes(&vector);
        result.stderr_only(string);
        result.stderr_only_bytes(&vector);
    }

    #[test]
    fn test_std_does_not_contain() {
        #[cfg(windows)]
        let res = run_cmd(
            "(echo This is a likely error message& echo This is a likely error message>&2) & exit 0",
        );
        #[cfg(not(windows))]
        let res = run_cmd(
            "echo This is a likely error message; echo This is a likely error message >&2; exit 0",
        );
        res.stdout_does_not_contain("unlikely");
        res.stderr_does_not_contain("unlikely");
    }

    #[test]
    #[should_panic]
    fn test_stdout_does_not_contain_fail() {
        #[cfg(windows)]
        let res = run_cmd("echo This is a likely error message& exit 0");
        #[cfg(not(windows))]
        let res = run_cmd("echo This is a likely error message; exit 0");

        res.stdout_does_not_contain("likely");
    }

    #[test]
    #[should_panic]
    fn test_stderr_does_not_contain_fail() {
        #[cfg(windows)]
        let res = run_cmd("echo This is a likely error message>&2 & exit 0");
        #[cfg(not(windows))]
        let res = run_cmd("echo This is a likely error message >&2; exit 0");

        res.stderr_does_not_contain("likely");
    }

    #[test]
    fn test_stdout_matches() {
        #[cfg(windows)]
        let res = run_cmd(
            "(echo This is a likely error message& echo This is a likely error message>&2 ) & exit 0",
        );
        #[cfg(not(windows))]
        let res = run_cmd(
            "echo This is a likely error message; echo This is a likely error message >&2; exit 0",
        );

        let positive = regex::Regex::new(".*likely.*").unwrap();
        let negative = regex::Regex::new(".*unlikely.*").unwrap();
        res.stdout_matches(&positive);
        res.stdout_does_not_match(&negative);
    }

    #[test]
    #[should_panic]
    fn test_stdout_matches_fail() {
        #[cfg(windows)]
        let res = run_cmd(
            "(echo This is a likely error message& echo This is a likely error message>&2) & exit 0",
        );
        #[cfg(not(windows))]
        let res = run_cmd(
            "echo This is a likely error message; echo This is a likely error message >&2; exit 0",
        );

        let negative = regex::Regex::new(".*unlikely.*").unwrap();
        res.stdout_matches(&negative);
    }

    #[test]
    #[should_panic]
    fn test_stdout_not_matches_fail() {
        #[cfg(windows)]
        let res = run_cmd(
            "(echo This is a likely error message& echo This is a likely error message>&2) & exit 0",
        );
        #[cfg(not(windows))]
        let res = run_cmd(
            "echo This is a likely error message; echo This is a likely error message >&2; exit 0",
        );

        let positive = regex::Regex::new(".*likely.*").unwrap();
        res.stdout_does_not_match(&positive);
    }

    #[cfg(feature = "echo")]
    #[test]
    fn test_normalized_newlines_stdout_is() {
        let ts = TestScenario::new("echo");
        let res = ts.ucmd().args(&["-ne", "A\r\nB\nC"]).run();

        res.normalized_newlines_stdout_is("A\r\nB\nC");
        res.normalized_newlines_stdout_is("A\nB\nC");
        res.normalized_newlines_stdout_is("A\nB\r\nC");
    }

    #[cfg(feature = "echo")]
    #[test]
    #[should_panic]
    fn test_normalized_newlines_stdout_is_fail() {
        let ts = TestScenario::new("echo");
        let res = ts.ucmd().args(&["-ne", "A\r\nB\nC"]).run();

        res.normalized_newlines_stdout_is("A\r\nB\nC\n");
    }

    #[cfg(feature = "echo")]
    #[test]
    fn test_cmd_result_stdout_check_and_stdout_str_check() {
        let result = TestScenario::new("echo").ucmd().arg("Hello world").run();

        result.stdout_str_check(|stdout| stdout.ends_with("world\n"));
        result.stdout_check(|stdout| stdout.get(0..2).unwrap().eq(&[b'H', b'e']));
        result.no_stderr();
    }

    #[cfg(feature = "echo")]
    #[test]
    fn test_cmd_result_stderr_check_and_stderr_str_check() {
        let ts = TestScenario::new("echo");
        let result = run_cmd(format!(
            "{} {} Hello world >&2",
            ts.bin_path.display(),
            ts.util_name
        ));

        result.stderr_str_check(|stderr| stderr.ends_with("world\n"));
        result.stderr_check(|stderr| stderr.get(0..2).unwrap().eq(&[b'H', b'e']));
        result.no_stdout();
    }

    #[cfg(feature = "echo")]
    #[test]
    #[should_panic]
    fn test_cmd_result_stdout_str_check_when_false_then_panics() {
        let result = TestScenario::new("echo").ucmd().arg("Hello world").run();
        result.stdout_str_check(str::is_empty);
    }

    #[cfg(feature = "echo")]
    #[test]
    #[should_panic]
    fn test_cmd_result_stdout_check_when_false_then_panics() {
        let result = TestScenario::new("echo").ucmd().arg("Hello world").run();
        result.stdout_check(<[u8]>::is_empty);
    }

    #[cfg(feature = "echo")]
    #[test]
    #[should_panic]
    fn test_cmd_result_stderr_str_check_when_false_then_panics() {
        let result = TestScenario::new("echo").ucmd().arg("Hello world").run();
        result.stderr_str_check(|s| !s.is_empty());
    }

    #[cfg(feature = "echo")]
    #[test]
    #[should_panic]
    fn test_cmd_result_stderr_check_when_false_then_panics() {
        let result = TestScenario::new("echo").ucmd().arg("Hello world").run();
        result.stderr_check(|s| !s.is_empty());
    }

    #[cfg(feature = "echo")]
    #[test]
    #[should_panic]
    fn test_cmd_result_stdout_check_when_predicate_panics_then_panic() {
        let result = TestScenario::new("echo").ucmd().run();
        result.stdout_str_check(|_| panic!("Just testing"));
    }

    #[cfg(feature = "echo")]
    #[cfg(unix)]
    #[test]
    fn test_cmd_result_signal_when_normal_exit_then_no_signal() {
        let result = TestScenario::new("echo").ucmd().run();
        assert!(result.signal().is_none());
    }

    #[cfg(feature = "sleep")]
    #[cfg(unix)]
    #[test]
    #[should_panic = "Program must be run first or has not finished"]
    fn test_cmd_result_signal_when_still_running_then_panic() {
        let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();

        child
            .make_assertion()
            .is_alive()
            .with_current_output()
            .signal();
    }

    #[cfg(feature = "sleep")]
    #[cfg(unix)]
    #[test]
    fn test_cmd_result_signal_when_kill_then_signal() {
        let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();

        child.kill();
        child
            .make_assertion()
            .is_not_alive()
            .with_current_output()
            .signal_is(9)
            .signal_name_is("SIGKILL")
            .signal_name_is("KILL")
            .signal_name_is("9")
            .signal()
            .expect("Signal was none");

        let result = child.wait().unwrap();
        result
            .signal_is(9)
            .signal_name_is("SIGKILL")
            .signal_name_is("KILL")
            .signal_name_is("9")
            .signal()
            .expect("Signal was none");
    }

    #[cfg(feature = "sleep")]
    #[cfg(unix)]
    #[rstest]
    #[case::signal_full_name_lower_case("sigkill")]
    #[case::signal_short_name_lower_case("kill")]
    #[case::signal_only_part_of_name("IGKILL")] // spell-checker: disable-line
    #[case::signal_just_sig("SIG")]
    #[case::signal_value_too_high("100")]
    #[case::signal_value_negative("-1")]
    #[should_panic = "Invalid signal name or value"]
    fn test_cmd_result_signal_when_invalid_signal_name_then_panic(#[case] signal_name: &str) {
        let mut child = TestScenario::new("sleep").ucmd().arg("60").run_no_wait();
        child.kill();
        let result = child.wait().unwrap();
        result.signal_name_is(signal_name);
    }

    #[test]
    #[cfg(unix)]
    fn test_parse_coreutil_version() {
        use std::assert_eq;
        assert_eq!(
            parse_coreutil_version("id (GNU coreutils) 9.0.123-0123").to_string(),
            "9"
        );
        assert_eq!(
            parse_coreutil_version("id (GNU coreutils) 8.32.263-0475").to_string(),
            "8.32"
        );
        assert_eq!(
            parse_coreutil_version("id (GNU coreutils) 8.25.123-0123").to_string(),
            "8.25"
        );
        assert_eq!(
            parse_coreutil_version("id (GNU coreutils) 9.0").to_string(),
            "9"
        );
        assert_eq!(
            parse_coreutil_version("id (GNU coreutils) 8.32").to_string(),
            "8.32"
        );
        assert_eq!(
            parse_coreutil_version("id (GNU coreutils) 8.25").to_string(),
            "8.25"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_check_coreutil_version() {
        match check_coreutil_version("id", VERSION_MIN) {
            Ok(s) => assert!(s.starts_with("uutils-tests-")),
            Err(s) => assert!(s.starts_with("uutils-tests-warning")),
        };
        #[cfg(target_os = "linux")]
        std::assert_eq!(
            check_coreutil_version("no test name", VERSION_MIN),
            Err("uutils-tests-warning: 'no test name' \
            No such file or directory (os error 2)"
                .to_string())
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_expected_result() {
        let ts = TestScenario::new("id");
        // assert!(expected_result(&ts, &[]).is_ok());
        match expected_result(&ts, &[]) {
            Ok(r) => assert!(r.succeeded()),
            Err(s) => assert!(s.starts_with("uutils-tests-warning")),
        }
        let ts = TestScenario::new("no test name");
        assert!(expected_result(&ts, &[]).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_host_name_for() {
        #[cfg(target_os = "linux")]
        {
            std::assert_eq!(host_name_for("id"), "id");
            std::assert_eq!(host_name_for("groups"), "groups");
            std::assert_eq!(host_name_for("who"), "who");
        }
        #[cfg(not(target_os = "linux"))]
        {
            // spell-checker:ignore (strings) ggroups gwho
            std::assert_eq!(host_name_for("id"), "gid");
            std::assert_eq!(host_name_for("groups"), "ggroups");
            std::assert_eq!(host_name_for("who"), "gwho");
            std::assert_eq!(host_name_for("gid"), "gid");
            std::assert_eq!(host_name_for("ggroups"), "ggroups");
            std::assert_eq!(host_name_for("gwho"), "gwho");
        }
    }

    #[test]
    #[cfg(unix)]
    #[cfg(feature = "whoami")]
    fn test_run_ucmd_as_root() {
        if is_ci() {
            println!("TEST SKIPPED (cannot run inside CI)");
        } else {
            // Skip test if we can't guarantee non-interactive `sudo`, or if we're not "root"
            if let Ok(output) = Command::new("sudo")
                .env("LC_ALL", "C")
                .args(["-E", "--non-interactive", "whoami"])
                .output()
            {
                if output.status.success() && String::from_utf8_lossy(&output.stdout).eq("root\n") {
                    let ts = TestScenario::new("whoami");
                    std::assert_eq!(
                        run_ucmd_as_root(&ts, &[]).unwrap().stdout_str().trim(),
                        "root"
                    );
                } else {
                    println!("TEST SKIPPED (we're not root)");
                }
            } else {
                println!("TEST SKIPPED (cannot run sudo)");
            }
        }
    }

    // This error was first detected when running tail so tail is used here but
    // should fail with any command that takes piped input.
    // See also https://github.com/uutils/coreutils/issues/3895
    #[cfg(feature = "tail")]
    #[test]
    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    fn test_when_piped_input_then_no_broken_pipe() {
        let ts = TestScenario::new("tail");
        for i in 0..10000 {
            dbg!(i);
            let test_string = "a\nb\n";
            ts.ucmd()
                .args(&["-n", "0"])
                .pipe_in(test_string)
                .succeeds()
                .no_stdout()
                .no_stderr();
        }
    }

    #[cfg(feature = "echo")]
    #[test]
    fn test_uchild_when_run_with_a_non_blocking_util() {
        let ts = TestScenario::new("echo");
        ts.ucmd()
            .arg("hello world")
            .run()
            .success()
            .stdout_only("hello world\n");
    }

    // Test basically that most of the methods of UChild are working
    #[cfg(feature = "echo")]
    #[test]
    fn test_uchild_when_run_no_wait_with_a_non_blocking_util() {
        let ts = TestScenario::new("echo");
        let mut child = ts.ucmd().arg("hello world").run_no_wait();

        // check `child.is_alive()` and `child.delay()` is working
        let mut trials = 10;
        while child.is_alive() {
            assert!(
                trials > 0,
                "Assertion failed: child process is still alive."
            );

            child.delay(500);
            trials -= 1;
        }

        assert!(!child.is_alive());

        // check `child.is_not_alive()` is working
        assert!(child.is_not_alive());

        // check the current output is correct
        std::assert_eq!(child.stdout(), "hello world\n");
        assert!(child.stderr().is_empty());

        // check the current output of echo is empty. We already called `child.stdout()` and `echo`
        // exited so there's no additional output after the first call of `child.stdout()`
        assert!(child.stdout().is_empty());
        assert!(child.stderr().is_empty());

        // check that we're still able to access all output of the child process, even after exit
        // and call to `child.stdout()`
        std::assert_eq!(child.stdout_all(), "hello world\n");
        assert!(child.stderr_all().is_empty());

        // we should be able to call kill without panics, even if the process already exited
        child.make_assertion().is_not_alive();
        child.kill();

        // we should be able to call wait without panics and apply some assertions
        child.wait().unwrap().code_is(0).no_stdout().no_stderr();
    }

    #[cfg(feature = "cat")]
    #[test]
    fn test_uchild_when_pipe_in() {
        let ts = TestScenario::new("cat");
        let mut child = ts.ucmd().set_stdin(Stdio::piped()).run_no_wait();
        child.pipe_in("content");
        child.wait().unwrap().stdout_only("content").success();

        ts.ucmd().pipe_in("content").run().stdout_is("content");
    }

    #[cfg(feature = "rm")]
    #[test]
    fn test_uchild_when_run_no_wait_with_a_blocking_command() {
        let ts = TestScenario::new("rm");
        let at = &ts.fixtures;

        at.mkdir("a");
        at.touch("a/empty");

        #[cfg(target_vendor = "apple")]
        let delay: u64 = 2000;
        #[cfg(not(target_vendor = "apple"))]
        let delay: u64 = 1000;

        let yes = if cfg!(windows) { "y\r\n" } else { "y\n" };

        let mut child = ts
            .ucmd()
            .set_stdin(Stdio::piped())
            .stderr_to_stdout()
            .args(&["-riv", "a"])
            .run_no_wait();
        child
            .make_assertion_with_delay(delay)
            .is_alive()
            .with_current_output()
            .stdout_is("rm: descend into directory 'a'? ");

        #[cfg(windows)]
        let expected = "rm: descend into directory 'a'? \
                        rm: remove regular empty file 'a\\empty'? ";
        #[cfg(unix)]
        let expected = "rm: descend into directory 'a'? \
                              rm: remove regular empty file 'a/empty'? ";
        child.write_in(yes);
        child
            .make_assertion_with_delay(delay)
            .is_alive()
            .with_all_output()
            .stdout_is(expected);

        #[cfg(windows)]
        let expected = "removed 'a\\empty'\nrm: remove directory 'a'? ";
        #[cfg(unix)]
        let expected = "removed 'a/empty'\nrm: remove directory 'a'? ";

        child
            .write_in(yes)
            .make_assertion_with_delay(delay)
            .is_alive()
            .with_exact_output(44, 0)
            .stdout_only(expected);

        let expected = "removed directory 'a'\n";

        child.write_in(yes);
        child.wait().unwrap().stdout_only(expected).success();
    }

    #[cfg(feature = "tail")]
    #[test]
    fn test_uchild_when_run_with_stderr_to_stdout() {
        let ts = TestScenario::new("tail");
        let at = &ts.fixtures;

        at.write("data", "file data\n");

        let expected_stdout = "==> data <==\n\
                                    file data\n\
                                    tail: cannot open 'missing' for reading: No such file or directory\n";
        ts.ucmd()
            .args(&["data", "missing"])
            .stderr_to_stdout()
            .fails()
            .stdout_only(expected_stdout);
    }

    #[cfg(feature = "cat")]
    #[cfg(unix)]
    #[test]
    fn test_uchild_when_no_capture_reading_from_infinite_source() {
        use regex::Regex;

        let ts = TestScenario::new("cat");

        let expected_stdout = b"\0".repeat(12345);
        let mut child = ts
            .ucmd()
            .set_stdin(Stdio::from(File::open("/dev/zero").unwrap()))
            .set_stdout(Stdio::piped())
            .run_no_wait();

        child
            .make_assertion()
            .with_exact_output(12345, 0)
            .stdout_only_bytes(expected_stdout);

        child
            .kill()
            .make_assertion()
            .with_current_output()
            .stdout_matches(&Regex::new("[\0].*").unwrap())
            .no_stderr();
    }

    #[cfg(feature = "sleep")]
    #[test]
    fn test_uchild_when_wait_and_timeout_is_reached_then_timeout_error() {
        let ts = TestScenario::new("sleep");
        let child = ts
            .ucmd()
            .timeout(Duration::from_secs(1))
            .arg("10.0")
            .run_no_wait();

        match child.wait() {
            Err(error) if error.kind() == io::ErrorKind::Other => {
                std::assert_eq!(error.to_string(), "wait: Timeout of '1s' reached");
            }
            Err(error) => panic!("Assertion failed: Expected error with timeout but was: {error}"),
            Ok(_) => panic!("Assertion failed: Expected timeout of `wait`."),
        }
    }

    #[cfg(feature = "sleep")]
    #[rstest]
    #[timeout(Duration::from_secs(5))]
    fn test_uchild_when_kill_and_timeout_higher_than_kill_time_then_no_panic() {
        let ts = TestScenario::new("sleep");
        let mut child = ts
            .ucmd()
            .timeout(Duration::from_secs(60))
            .arg("20.0")
            .run_no_wait();

        child.kill().make_assertion().is_not_alive();
    }

    #[cfg(feature = "sleep")]
    #[test]
    fn test_uchild_when_try_kill_and_timeout_is_reached_then_error() {
        let ts = TestScenario::new("sleep");
        let mut child = ts.ucmd().timeout(Duration::ZERO).arg("10.0").run_no_wait();

        match child.try_kill() {
            Err(error) if error.kind() == io::ErrorKind::Other => {
                std::assert_eq!(error.to_string(), "kill: Timeout of '0s' reached");
            }
            Err(error) => panic!("Assertion failed: Expected error with timeout but was: {error}"),
            Ok(()) => panic!("Assertion failed: Expected timeout of `try_kill`."),
        }
    }

    #[cfg(feature = "sleep")]
    #[test]
    #[should_panic = "kill: Timeout of '0s' reached"]
    fn test_uchild_when_kill_with_timeout_and_timeout_is_reached_then_panic() {
        let ts = TestScenario::new("sleep");
        let mut child = ts.ucmd().timeout(Duration::ZERO).arg("10.0").run_no_wait();

        child.kill();
        panic!("Assertion failed: Expected timeout of `kill`.");
    }

    #[cfg(feature = "sleep")]
    #[test]
    #[should_panic(expected = "wait: Timeout of '1.1s' reached")]
    fn test_ucommand_when_run_with_timeout_and_timeout_is_reached_then_panic() {
        let ts = TestScenario::new("sleep");
        ts.ucmd()
            .timeout(Duration::from_millis(1100))
            .arg("10.0")
            .run();

        panic!("Assertion failed: Expected timeout of `run`.")
    }

    #[cfg(feature = "sleep")]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn test_ucommand_when_run_with_timeout_higher_then_execution_time_then_no_panic() {
        let ts = TestScenario::new("sleep");
        ts.ucmd().timeout(Duration::from_secs(60)).arg("1.0").run();
    }

    #[cfg(feature = "echo")]
    #[test]
    fn test_ucommand_when_default() {
        let shell_cmd = format!("{TESTS_BINARY} echo -n hello");

        let mut command = UCommand::new();
        command.arg(&shell_cmd).succeeds().stdout_is("hello");

        #[cfg(target_os = "android")]
        let (expected_bin, expected_arg) = (PathBuf::from("/system/bin/sh"), OsString::from("-c"));
        #[cfg(all(unix, not(target_os = "android")))]
        let (expected_bin, expected_arg) = (PathBuf::from("/bin/sh"), OsString::from("-c"));
        #[cfg(windows)]
        let (expected_bin, expected_arg) = (PathBuf::from("cmd"), OsString::from("/C"));

        std::assert_eq!(&expected_bin, command.bin_path.as_ref().unwrap());
        assert!(command.util_name.is_none());
        std::assert_eq!(command.args, &[expected_arg, OsString::from(&shell_cmd)]);
        assert!(command.tmpd.is_some());
    }

    #[cfg(feature = "echo")]
    #[test]
    fn test_ucommand_with_util() {
        let tmpd = tempfile::tempdir().unwrap();
        let mut command = UCommand::with_util("echo", Rc::new(tmpd));

        command
            .args(&["-n", "hello"])
            .succeeds()
            .stdout_only("hello");

        std::assert_eq!(
            &PathBuf::from(TESTS_BINARY),
            command.bin_path.as_ref().unwrap()
        );
        std::assert_eq!("echo", &command.util_name.unwrap());
        std::assert_eq!(
            &[
                OsString::from("echo"),
                OsString::from("-n"),
                OsString::from("hello")
            ],
            command.args.make_contiguous()
        );
        assert!(command.tmpd.is_some());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn test_compare_xattrs() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path1 = temp_dir.path().join("test_file1.txt");
        let file_path2 = temp_dir.path().join("test_file2.txt");

        File::create(&file_path1).unwrap();
        File::create(&file_path2).unwrap();

        let test_attr = "user.test_attr";
        let test_value = b"test value";
        xattr::set(&file_path1, test_attr, test_value).unwrap();

        assert!(!compare_xattrs(&file_path1, &file_path2));

        xattr::set(&file_path2, test_attr, test_value).unwrap();
        assert!(compare_xattrs(&file_path1, &file_path2));
    }
}
