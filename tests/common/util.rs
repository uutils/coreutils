//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

//spell-checker: ignore (linux) rlimit prlimit coreutil ggroups

#![allow(dead_code)]

use pretty_assertions::assert_eq;
#[cfg(target_os = "linux")]
use rlimit::prlimit;
#[cfg(unix)]
use std::borrow::Cow;
use std::env;
#[cfg(not(windows))]
use std::ffi::CString;
use std::ffi::OsStr;
use std::fs::{self, hard_link, File, OpenOptions};
use std::io::{Read, Result, Write};
#[cfg(unix)]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;
use uucore::{Args, InvalidEncodingHandling};

#[cfg(windows)]
static PROGNAME: &str = concat!(env!("CARGO_PKG_NAME"), ".exe");
#[cfg(not(windows))]
static PROGNAME: &str = env!("CARGO_PKG_NAME");

static TESTS_DIR: &str = "tests";
static FIXTURES_DIR: &str = "fixtures";

static ALREADY_RUN: &str = " you have already run this UCommand, if you want to run \
                            another command in the same test, use TestScenario::new instead of \
                            testing();";
static MULTIPLE_STDIN_MEANINGLESS: &str = "Ucommand is designed around a typical use case of: provide args and input stream -> spawn process -> block until completion -> return output streams. For verifying that a particular section of the input stream is what causes a particular behavior, use the Command type directly.";

static NO_STDIN_MEANINGLESS: &str = "Setting this flag has no effect if there is no stdin";

/// Test if the program is running under CI
pub fn is_ci() -> bool {
    std::env::var("CI")
        .unwrap_or_else(|_| String::from("false"))
        .eq_ignore_ascii_case("true")
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
    bin_path: String,
    /// util_name provided by `TestScenario` or `UCommand`
    util_name: Option<String>,
    //tmpd is used for convenience functions for asserts against fixtures
    tmpd: Option<Rc<TempDir>>,
    /// exit status for command (if there is one)
    code: Option<i32>,
    /// zero-exit from running the Command?
    /// see [`success`]
    success: bool,
    /// captured standard output after running the Command
    stdout: Vec<u8>,
    /// captured standard error after running the Command
    stderr: Vec<u8>,
}

impl CmdResult {
    pub fn new(
        bin_path: String,
        util_name: Option<String>,
        tmpd: Option<Rc<TempDir>>,
        code: Option<i32>,
        success: bool,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Self {
        Self {
            bin_path,
            util_name,
            tmpd,
            code,
            success,
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
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
    /// Panics if not run
    pub fn code(&self) -> i32 {
        self.code.expect("Program must be run first")
    }

    pub fn code_is(&self, expected_code: i32) -> &Self {
        assert_eq!(self.code(), expected_code);
        self
    }

    /// Returns the program's TempDir
    /// Panics if not present
    pub fn tmpd(&self) -> Rc<TempDir> {
        match &self.tmpd {
            Some(ptr) => ptr.clone(),
            None => panic!("Command not associated with a TempDir"),
        }
    }

    /// Returns whether the program succeeded
    pub fn succeeded(&self) -> bool {
        self.success
    }

    /// asserts that the command resulted in a success (zero) status code
    pub fn success(&self) -> &Self {
        assert!(
            self.success,
            "Command was expected to succeed.\nstdout = {}\n stderr = {}",
            self.stdout_str(),
            self.stderr_str()
        );
        self
    }

    /// asserts that the command resulted in a failure (non-zero) status code
    pub fn failure(&self) -> &Self {
        assert!(
            !self.success,
            "Command was expected to fail.\nstdout = {}\n stderr = {}",
            self.stdout_str(),
            self.stderr_str()
        );
        self
    }

    /// asserts that the command's exit code is the same as the given one
    pub fn status_code(&self, code: i32) -> &Self {
        assert_eq!(self.code, Some(code));
        self
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// generally, it's better to use stdout_only() instead,
    /// but you might find yourself using this function if
    /// 1.  you can not know exactly what stdout will be or
    /// 2.  you know that stdout will also be empty
    pub fn no_stderr(&self) -> &Self {
        assert!(
            self.stderr.is_empty(),
            "Expected stderr to be empty, but it's:\n{}",
            self.stderr_str()
        );
        self
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// unless asserting there was neither stdout or stderr, stderr_only is usually a better choice
    /// generally, it's better to use stderr_only() instead,
    /// but you might find yourself using this function if
    /// 1.  you can not know exactly what stderr will be or
    /// 2.  you know that stderr will also be empty
    pub fn no_stdout(&self) -> &Self {
        assert!(
            self.stdout.is_empty(),
            "Expected stdout to be empty, but it's:\n{}",
            self.stdout_str()
        );
        self
    }

    /// asserts that the command resulted in stdout stream output that equals the
    /// passed in value, trailing whitespace are kept to force strict comparison (#1235)
    /// stdout_only is a better choice unless stderr may or will be non-empty
    pub fn stdout_is<T: AsRef<str>>(&self, msg: T) -> &Self {
        assert_eq!(self.stdout_str(), String::from(msg.as_ref()));
        self
    }

    /// like `stdout_is`, but succeeds if any elements of `expected` matches stdout.
    pub fn stdout_is_any<T: AsRef<str> + std::fmt::Debug>(&self, expected: &[T]) -> &Self {
        if !expected.iter().any(|msg| self.stdout_str() == msg.as_ref()) {
            panic!(
                "stdout was {}\nExpected any of {:#?}",
                self.stdout_str(),
                expected
            );
        }
        self
    }

    /// Like `stdout_is` but newlines are normalized to `\n`.
    pub fn normalized_newlines_stdout_is<T: AsRef<str>>(&self, msg: T) -> &Self {
        let msg = msg.as_ref().replace("\r\n", "\n");
        assert_eq!(self.stdout_str().replace("\r\n", "\n"), msg);
        self
    }

    /// asserts that the command resulted in stdout stream output,
    /// whose bytes equal those of the passed in slice
    pub fn stdout_is_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        assert_eq!(self.stdout, msg.as_ref());
        self
    }

    /// like stdout_is(...), but expects the contents of the file at the provided relative path
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
    pub fn stdout_is_fixture_bytes<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_is_bytes(contents)
    }

    /// like stdout_is_fixture(...), but replaces the data in fixture file based on values provided in template_vars
    /// command output
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
    pub fn stdout_is_templated_fixture_any<T: AsRef<OsStr>>(
        &self,
        file_rel_path: T,
        template_vars: &[Vec<(String, String)>],
    ) {
        let contents = String::from_utf8(read_scenario_fixture(&self.tmpd, file_rel_path)).unwrap();
        let possible_values = template_vars.iter().map(|vars| {
            let mut contents = contents.clone();
            for kv in vars.iter() {
                contents = contents.replace(&kv.0, &kv.1);
            }
            contents
        });
        self.stdout_is_any(&possible_values.collect::<Vec<_>>());
    }

    /// asserts that the command resulted in stderr stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// stderr_only is a better choice unless stdout may or will be non-empty
    pub fn stderr_is<T: AsRef<str>>(&self, msg: T) -> &Self {
        assert_eq!(
            self.stderr_str().trim_end(),
            String::from(msg.as_ref()).trim_end()
        );
        self
    }

    /// asserts that the command resulted in stderr stream output,
    /// whose bytes equal those of the passed in slice
    pub fn stderr_is_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        assert_eq!(self.stderr, msg.as_ref());
        self
    }

    /// Like stdout_is_fixture, but for stderr
    pub fn stderr_is_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stderr_is(String::from_utf8(contents).unwrap())
    }

    /// asserts that
    /// 1.  the command resulted in stdout stream output that equals the
    ///     passed in value
    /// 2.  the command resulted in empty (zero-length) stderr stream output
    pub fn stdout_only<T: AsRef<str>>(&self, msg: T) -> &Self {
        self.no_stderr().stdout_is(msg)
    }

    /// asserts that
    /// 1.  the command resulted in a stdout stream whose bytes
    ///     equal those of the passed in value
    /// 2.  the command resulted in an empty stderr stream
    pub fn stdout_only_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        self.no_stderr().stdout_is_bytes(msg)
    }

    /// like stdout_only(...), but expects the contents of the file at the provided relative path
    pub fn stdout_only_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_only_bytes(contents)
    }

    /// asserts that
    /// 1.  the command resulted in stderr stream output that equals the
    ///     passed in value, when both are trimmed of trailing whitespace
    /// 2.  the command resulted in empty (zero-length) stdout stream output
    pub fn stderr_only<T: AsRef<str>>(&self, msg: T) -> &Self {
        self.no_stdout().stderr_is(msg)
    }

    /// asserts that
    /// 1.  the command resulted in a stderr stream whose bytes equal the ones
    ///     of the passed value
    /// 2.  the command resulted in an empty stdout stream
    pub fn stderr_only_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &Self {
        self.no_stdout().stderr_is_bytes(msg)
    }

    pub fn fails_silently(&self) -> &Self {
        assert!(!self.success);
        assert!(self.stderr.is_empty());
        self
    }

    /// asserts that
    /// 1.  the command resulted in stderr stream output that equals the
    ///     the following format when both are trimmed of trailing whitespace
    ///     `"{util_name}: {msg}\nTry '{bin_path} {util_name} --help' for more information."`
    ///     This the expected format when a UUsageError is returned or when show_error! is called
    ///     `msg` should be the same as the one provided to UUsageError::new or show_error!
    ///
    /// 2.  the command resulted in empty (zero-length) stdout stream output
    pub fn usage_error<T: AsRef<str>>(&self, msg: T) -> &Self {
        self.stderr_only(format!(
            "{0}: {2}\nTry '{1} {0} --help' for more information.",
            self.util_name.as_ref().unwrap(), // This shouldn't be called using a normal command
            self.bin_path,
            msg.as_ref()
        ))
    }

    pub fn stdout_contains<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            self.stdout_str().contains(cmp.as_ref()),
            "'{}' does not contain '{}'",
            self.stdout_str(),
            cmp.as_ref()
        );
        self
    }

    pub fn stderr_contains<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            self.stderr_str().contains(cmp.as_ref()),
            "'{}' does not contain '{}'",
            self.stderr_str(),
            cmp.as_ref()
        );
        self
    }

    pub fn stdout_does_not_contain<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(
            !self.stdout_str().contains(cmp.as_ref()),
            "'{}' contains '{}' but should not",
            self.stdout_str(),
            cmp.as_ref(),
        );
        self
    }

    pub fn stderr_does_not_contain<T: AsRef<str>>(&self, cmp: T) -> &Self {
        assert!(!self.stderr_str().contains(cmp.as_ref()));
        self
    }

    pub fn stdout_matches(&self, regex: &regex::Regex) -> &Self {
        if !regex.is_match(self.stdout_str().trim()) {
            panic!("Stdout does not match regex:\n{}", self.stdout_str());
        }
        self
    }

    pub fn stdout_does_not_match(&self, regex: &regex::Regex) -> &Self {
        if regex.is_match(self.stdout_str().trim()) {
            panic!("Stdout matches regex:\n{}", self.stdout_str());
        }
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
                fs::copy(&entry.path(), new_dest)?;
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

    pub fn plus(&self, name: &str) -> PathBuf {
        let mut pathbuf = self.subdir.clone();
        pathbuf.push(name);
        pathbuf
    }

    pub fn plus_as_string(&self, name: &str) -> String {
        String::from(self.plus(name).to_str().unwrap())
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
            .unwrap_or_else(|e| panic!("Couldn't read {}: {}", name, e));
        contents
    }

    pub fn read_bytes(&self, name: &str) -> Vec<u8> {
        let mut f = self.open(name);
        let mut contents = Vec::new();
        f.read_to_end(&mut contents)
            .unwrap_or_else(|e| panic!("Couldn't read {}: {}", name, e));
        contents
    }

    pub fn write(&self, name: &str, contents: &str) {
        log_info("write(default)", self.plus_as_string(name));
        std::fs::write(self.plus(name), contents)
            .unwrap_or_else(|e| panic!("Couldn't write {}: {}", name, e));
    }

    pub fn write_bytes(&self, name: &str, contents: &[u8]) {
        log_info("write(default)", self.plus_as_string(name));
        std::fs::write(self.plus(name), contents)
            .unwrap_or_else(|e| panic!("Couldn't write {}: {}", name, e));
    }

    pub fn append(&self, name: &str, contents: &str) {
        log_info("write(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(self.plus(name))
            .unwrap();
        f.write_all(contents.as_bytes())
            .unwrap_or_else(|e| panic!("Couldn't write(append) {}: {}", name, e));
    }

    pub fn append_bytes(&self, name: &str, contents: &[u8]) {
        log_info("write(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(self.plus(name))
            .unwrap();
        f.write_all(contents)
            .unwrap_or_else(|e| panic!("Couldn't write(append) to {}: {}", name, e));
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
            .unwrap_or_else(|e| panic!("Couldn't write(truncate) {}: {}", name, e));
    }

    pub fn rename(&self, source: &str, target: &str) {
        let source = self.plus(source);
        let target = self.plus(target);
        log_info("rename", format!("{:?} {:?}", source, target));
        std::fs::rename(&source, &target)
            .unwrap_or_else(|e| panic!("Couldn't rename {:?} -> {:?}: {}", source, target, e));
    }

    pub fn remove(&self, source: &str) {
        let source = self.plus(source);
        log_info("remove", format!("{:?}", source));
        std::fs::remove_file(&source)
            .unwrap_or_else(|e| panic!("Couldn't remove {:?}: {}", source, e));
    }

    pub fn copy(&self, source: &str, target: &str) {
        let source = self.plus(source);
        let target = self.plus(target);
        log_info("copy", format!("{:?} {:?}", source, target));
        std::fs::copy(&source, &target)
            .unwrap_or_else(|e| panic!("Couldn't copy {:?} -> {:?}: {}", source, target, e));
    }

    pub fn rmdir(&self, dir: &str) {
        log_info("rmdir", self.plus_as_string(dir));
        fs::remove_dir(&self.plus(dir)).unwrap();
    }

    pub fn mkdir(&self, dir: &str) {
        log_info("mkdir", self.plus_as_string(dir));
        fs::create_dir(&self.plus(dir)).unwrap();
    }

    pub fn mkdir_all(&self, dir: &str) {
        log_info("mkdir_all", self.plus_as_string(dir));
        fs::create_dir_all(self.plus(dir)).unwrap();
    }

    pub fn make_file(&self, name: &str) -> File {
        match File::create(&self.plus(name)) {
            Ok(f) => f,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn touch(&self, file: &str) {
        log_info("touch", self.plus_as_string(file));
        File::create(&self.plus(file)).unwrap();
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
            &format!(
                "{},{}",
                self.plus_as_string(original),
                self.plus_as_string(link)
            ),
        );
        hard_link(&self.plus(original), &self.plus(link)).unwrap();
    }

    pub fn symlink_file(&self, original: &str, link: &str) {
        log_info(
            "symlink",
            &format!(
                "{},{}",
                self.plus_as_string(original),
                self.plus_as_string(link)
            ),
        );
        symlink_file(&self.plus(original), &self.plus(link)).unwrap();
    }

    pub fn relative_symlink_file(&self, original: &str, link: &str) {
        log_info("symlink", &format!("{},{}", original, link));
        symlink_file(original, link).unwrap();
    }

    pub fn symlink_dir(&self, original: &str, link: &str) {
        log_info(
            "symlink",
            &format!(
                "{},{}",
                self.plus_as_string(original),
                self.plus_as_string(link)
            ),
        );
        symlink_dir(&self.plus(original), &self.plus(link)).unwrap();
    }

    pub fn is_symlink(&self, path: &str) -> bool {
        log_info("is_symlink", self.plus_as_string(path));
        match fs::symlink_metadata(&self.plus(path)) {
            Ok(m) => m.file_type().is_symlink(),
            Err(_) => false,
        }
    }

    pub fn resolve_link(&self, path: &str) -> String {
        log_info("resolve_link", self.plus_as_string(path));
        match fs::read_link(&self.plus(path)) {
            Ok(p) => self.minus_as_string(p.to_str().unwrap()),
            Err(_) => "".to_string(),
        }
    }

    pub fn symlink_metadata(&self, path: &str) -> fs::Metadata {
        match fs::symlink_metadata(&self.plus(path)) {
            Ok(m) => m,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn metadata(&self, path: &str) -> fs::Metadata {
        match fs::metadata(&self.plus(path)) {
            Ok(m) => m,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn file_exists(&self, path: &str) -> bool {
        match fs::metadata(&self.plus(path)) {
            Ok(m) => m.is_file(),
            Err(_) => false,
        }
    }

    pub fn dir_exists(&self, path: &str) -> bool {
        match fs::metadata(&self.plus(path)) {
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
    pub fn new(util_name: &str) -> Self {
        let tmpd = Rc::new(TempDir::new().unwrap());
        let ts = Self {
            bin_path: {
                // Instead of hard coding the path relative to the current
                // directory, use Cargo's OUT_DIR to find path to executable.
                // This allows tests to be run using profiles other than debug.
                let target_dir = path_concat!(env!("OUT_DIR"), "..", "..", "..", PROGNAME);
                PathBuf::from(AtPath::new(Path::new(&target_dir)).root_dir_resolved())
            },
            util_name: String::from(util_name),
            fixtures: AtPath::new(tmpd.as_ref().path()),
            tmpd,
        };
        let mut fixture_path_builder = env::current_dir().unwrap();
        fixture_path_builder.push(TESTS_DIR);
        fixture_path_builder.push(FIXTURES_DIR);
        fixture_path_builder.push(util_name);
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
        self.composite_cmd(&self.bin_path, &self.util_name, true)
    }

    /// Returns builder for invoking the target uutils binary. Paths given are
    /// treated relative to the environment's unique temporary test directory.
    pub fn composite_cmd<S: AsRef<OsStr>, T: AsRef<OsStr>>(
        &self,
        bin: S,
        util_name: T,
        env_clear: bool,
    ) -> UCommand {
        UCommand::new_from_tmp(bin, &Some(util_name), self.tmpd.clone(), env_clear)
    }

    /// Returns builder for invoking any system command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    pub fn cmd<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp::<S, S>(bin, &None, self.tmpd.clone(), true)
    }

    /// Returns builder for invoking any uutils command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    pub fn ccmd<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        self.composite_cmd(&self.bin_path, bin, true)
    }

    // different names are used rather than an argument
    // because the need to keep the environment is exceedingly rare.
    pub fn ucmd_keepenv(&self) -> UCommand {
        self.composite_cmd(&self.bin_path, &self.util_name, false)
    }

    /// Returns builder for invoking any system command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    /// Differs from the builder returned by `cmd` in that `cmd_keepenv` does not call
    /// `Command::env_clear` (Clears the entire environment map for the child process.)
    pub fn cmd_keepenv<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp::<S, S>(bin, &None, self.tmpd.clone(), false)
    }
}

/// A `UCommand` is a wrapper around an individual Command that provides several additional features
/// 1. it has convenience functions that are more ergonomic to use for piping in stdin, spawning the command
///       and asserting on the results.
/// 2. it tracks arguments provided so that in test cases which may provide variations of an arg in loops
///     the test failure can display the exact call which preceded an assertion failure.
/// 3. it provides convenience construction arguments to set the Command working directory and/or clear its environment.
#[derive(Debug)]
pub struct UCommand {
    pub raw: Command,
    comm_string: String,
    bin_path: String,
    util_name: Option<String>,
    tmpd: Option<Rc<TempDir>>,
    has_run: bool,
    ignore_stdin_write_error: bool,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
    bytes_into_stdin: Option<Vec<u8>>,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    limits: Vec<(rlimit::Resource, u64, u64)>,
}

impl UCommand {
    pub fn new<T: AsRef<OsStr>, S: AsRef<OsStr>, U: AsRef<OsStr>>(
        bin_path: T,
        util_name: &Option<S>,
        curdir: U,
        env_clear: bool,
    ) -> Self {
        let bin_path = bin_path.as_ref();
        let util_name = util_name.as_ref().map(|un| un.as_ref());

        let mut ucmd = Self {
            tmpd: None,
            has_run: false,
            raw: {
                let mut cmd = Command::new(bin_path);
                cmd.current_dir(curdir.as_ref());
                if env_clear {
                    cmd.env_clear();
                    if cfg!(windows) {
                        // spell-checker:ignore (dll) rsaenh
                        // %SYSTEMROOT% is required on Windows to initialize crypto provider
                        // ... and crypto provider is required for std::rand
                        // From `procmon`: RegQueryValue HKLM\SOFTWARE\Microsoft\Cryptography\Defaults\Provider\Microsoft Strong Cryptographic Provider\Image Path
                        // SUCCESS  Type: REG_SZ, Length: 66, Data: %SystemRoot%\system32\rsaenh.dll"
                        if let Some(systemroot) = env::var_os("SYSTEMROOT") {
                            cmd.env("SYSTEMROOT", systemroot);
                        }
                    } else {
                        // if someone is setting LD_PRELOAD, there's probably a good reason for it
                        if let Some(ld_preload) = env::var_os("LD_PRELOAD") {
                            cmd.env("LD_PRELOAD", ld_preload);
                        }
                    }
                }
                cmd
            },
            comm_string: String::from(bin_path.to_str().unwrap()),
            bin_path: bin_path.to_str().unwrap().to_string(),
            util_name: util_name.map(|un| un.to_str().unwrap().to_string()),
            ignore_stdin_write_error: false,
            bytes_into_stdin: None,
            stdin: None,
            stdout: None,
            stderr: None,
            #[cfg(any(target_os = "linux", target_os = "android"))]
            limits: vec![],
        };

        if let Some(un) = util_name {
            ucmd.arg(un);
        }

        ucmd
    }

    pub fn new_from_tmp<T: AsRef<OsStr>, S: AsRef<OsStr>>(
        bin_path: T,
        util_name: &Option<S>,
        tmpd: Rc<TempDir>,
        env_clear: bool,
    ) -> Self {
        let tmpd_path_buf = String::from(&(*tmpd.as_ref().path().to_str().unwrap()));
        let mut ucmd: Self = Self::new(bin_path, util_name, tmpd_path_buf, env_clear);
        ucmd.tmpd = Some(tmpd);
        ucmd
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

    /// Add a parameter to the invocation. Path arguments are treated relative
    /// to the test environment directory.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        assert!(!self.has_run, "{}", ALREADY_RUN);
        self.comm_string.push(' ');
        self.comm_string
            .push_str(arg.as_ref().to_str().unwrap_or_default());
        self.raw.arg(arg.as_ref());
        self
    }

    /// Add multiple parameters to the invocation. Path arguments are treated relative
    /// to the test environment directory.
    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut Self {
        assert!(!self.has_run, "{}", MULTIPLE_STDIN_MEANINGLESS);
        let strings = args
            .iter()
            .map(|s| s.as_ref().to_os_string())
            .collect_str(InvalidEncodingHandling::Ignore)
            .accept_any();

        for s in strings {
            self.comm_string.push(' ');
            self.comm_string.push_str(&s);
        }

        self.raw.args(args.as_ref());
        self
    }

    /// provides standard input to feed in to the command when spawned
    pub fn pipe_in<T: Into<Vec<u8>>>(&mut self, input: T) -> &mut Self {
        assert!(
            self.bytes_into_stdin.is_none(),
            "{}",
            MULTIPLE_STDIN_MEANINGLESS
        );
        self.bytes_into_stdin = Some(input.into());
        self
    }

    /// like pipe_in(...), but uses the contents of the file at the provided relative path as the piped in data
    pub fn pipe_in_fixture<S: AsRef<OsStr>>(&mut self, file_rel_path: S) -> &mut Self {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.pipe_in(contents)
    }

    /// Ignores error caused by feeding stdin to the command.
    /// This is typically useful to test non-standard workflows
    /// like feeding something to a command that does not read it
    pub fn ignore_stdin_write_error(&mut self) -> &mut Self {
        assert!(self.bytes_into_stdin.is_some(), "{}", NO_STDIN_MEANINGLESS);
        self.ignore_stdin_write_error = true;
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        assert!(!self.has_run, "{}", ALREADY_RUN);
        self.raw.env(key, val);
        self
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn with_limit(
        &mut self,
        resource: rlimit::Resource,
        soft_limit: u64,
        hard_limit: u64,
    ) -> &mut Self {
        self.limits.push((resource, soft_limit, hard_limit));
        self
    }

    /// Spawns the command, feeds the stdin if any, and returns the
    /// child process immediately.
    pub fn run_no_wait(&mut self) -> Child {
        assert!(!self.has_run, "{}", ALREADY_RUN);
        self.has_run = true;
        log_info("run", &self.comm_string);
        let mut child = self
            .raw
            .stdin(self.stdin.take().unwrap_or_else(Stdio::piped))
            .stdout(self.stdout.take().unwrap_or_else(Stdio::piped))
            .stderr(self.stderr.take().unwrap_or_else(Stdio::piped))
            .spawn()
            .unwrap();

        #[cfg(target_os = "linux")]
        for &(resource, soft_limit, hard_limit) in &self.limits {
            prlimit(
                child.id() as i32,
                resource,
                Some((soft_limit, hard_limit)),
                None,
            )
            .unwrap();
        }

        if let Some(ref input) = self.bytes_into_stdin {
            let write_result = child
                .stdin
                .take()
                .unwrap_or_else(|| panic!("Could not take child process stdin"))
                .write_all(input);
            if !self.ignore_stdin_write_error {
                if let Err(e) = write_result {
                    panic!("failed to write to stdin of child: {}", e);
                }
            }
        }

        child
    }

    /// Spawns the command, feeds the stdin if any, waits for the result
    /// and returns a command result.
    /// It is recommended that you instead use succeeds() or fails()
    pub fn run(&mut self) -> CmdResult {
        let prog = self.run_no_wait().wait_with_output().unwrap();

        CmdResult {
            bin_path: self.bin_path.clone(),
            util_name: self.util_name.clone(),
            tmpd: self.tmpd.clone(),
            code: prog.status.code(),
            success: prog.status.success(),
            stdout: prog.stdout,
            stderr: prog.stderr,
        }
    }

    /// Spawns the command, feeding the passed in stdin, waits for the result
    /// and returns a command result.
    /// It is recommended that, instead of this, you use a combination of pipe_in()
    /// with succeeds() or fails()
    pub fn run_piped_stdin<T: Into<Vec<u8>>>(&mut self, input: T) -> CmdResult {
        self.pipe_in(input).run()
    }

    /// Spawns the command, feeds the stdin if any, waits for the result,
    /// asserts success, and returns a command result.
    pub fn succeeds(&mut self) -> CmdResult {
        let cmd_result = self.run();
        cmd_result.success();
        cmd_result
    }

    /// Spawns the command, feeds the stdin if any, waits for the result,
    /// asserts failure, and returns a command result.
    pub fn fails(&mut self) -> CmdResult {
        let cmd_result = self.run();
        cmd_result.failure();
        cmd_result
    }

    pub fn get_full_fixture_path(&self, file_rel_path: &str) -> String {
        let tmpdir_path = self.tmpd.as_ref().unwrap().path();
        format!("{}/{}", tmpdir_path.to_str().unwrap(), file_rel_path)
    }
}

/// Wrapper for `child.stdout.read_exact()`.
/// Careful, this blocks indefinitely if `size` bytes is never reached.
pub fn read_size(child: &mut Child, size: usize) -> String {
    String::from_utf8(read_size_bytes(child, size)).unwrap()
}

/// Read the specified number of bytes from the stdout of the child process.
///
/// Careful, this blocks indefinitely if `size` bytes is never reached.
pub fn read_size_bytes(child: &mut Child, size: usize) -> Vec<u8> {
    let mut output = Vec::new();
    output.resize(size, 0);
    sleep(Duration::from_secs(1));
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(output.as_mut_slice())
        .unwrap();
    output
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
            println!("{}: {}, using \"nobody\" instead", UUTILS_WARNING, e);
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
            format!("g{}", util_name).into()
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
    log_info("run", format!("{} --version", util_name));
    let version_check = match Command::new(util_name.as_ref())
        .env("LC_ALL", "C")
        .arg("--version")
        .output()
    {
        Ok(s) => s,
        Err(e) => return Err(format!("{}: '{}' {}", UUTILS_WARNING, util_name, e)),
    };
    std::str::from_utf8(&version_check.stdout).unwrap()
        .split('\n')
        .collect::<Vec<_>>()
        .get(0)
        .map_or_else(
            || Err(format!("{}: unexpected output format for reference coreutil: '{} --version'", UUTILS_WARNING, util_name)),
            |s| {
                if s.contains(&format!("(GNU coreutils) {}", version_expected)) {
                    Ok(format!("{}: {}", UUTILS_INFO, s))
                } else if s.contains("(GNU coreutils)") {
                    let version_found = parse_coreutil_version(s);
                    let version_expected = version_expected.parse::<f32>().unwrap_or_default();
                    if version_found > version_expected {
                    Ok(format!("{}: version for the reference coreutil '{}' is higher than expected; expected: {}, found: {}", UUTILS_INFO, util_name, version_expected, version_found))
                    } else {
                    Err(format!("{}: version for the reference coreutil '{}' does not match; expected: {}, found: {}", UUTILS_WARNING, util_name, version_expected, version_found)) }
                } else {
                    Err(format!("{}: no coreutils version string found for reference coreutils '{} --version'", UUTILS_WARNING, util_name))
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
    println!("{}", check_coreutil_version(&ts.util_name, VERSION_MIN)?);
    let util_name = &host_name_for(&ts.util_name);

    let result = ts
        .cmd_keepenv(util_name.as_ref())
        .env("LC_ALL", "C")
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
        Some(result.code()),
        result.succeeded(),
        stdout.as_bytes(),
        stderr.as_bytes(),
    ))
}

/// Sanity checks for test utils
#[cfg(test)]
mod tests {
    // spell-checker:ignore (tests) asdfsadfa
    use super::*;

    #[test]
    fn test_code_is() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: Some(32),
            success: false,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.code_is(32);
    }

    #[test]
    #[should_panic]
    fn test_code_is_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: Some(32),
            success: false,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.code_is(1);
    }

    #[test]
    fn test_failure() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: false,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.failure();
    }

    #[test]
    #[should_panic]
    fn test_failure_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.failure();
    }

    #[test]
    fn test_success() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.success();
    }

    #[test]
    #[should_panic]
    fn test_success_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: false,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.success();
    }

    #[test]
    fn test_no_stderr_output() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "".into(),
            stderr: "".into(),
        };
        res.no_stderr();
        res.no_stdout();
    }

    #[test]
    #[should_panic]
    fn test_no_stderr_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "".into(),
            stderr: "asdfsadfa".into(),
        };

        res.no_stderr();
    }

    #[test]
    #[should_panic]
    fn test_no_stdout_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "asdfsadfa".into(),
            stderr: "".into(),
        };

        res.no_stdout();
    }

    #[test]
    fn test_std_does_not_contain() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "This is a likely error message\n".into(),
            stderr: "This is a likely error message\n".into(),
        };
        res.stdout_does_not_contain("unlikely");
        res.stderr_does_not_contain("unlikely");
    }

    #[test]
    #[should_panic]
    fn test_stdout_does_not_contain_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "This is a likely error message\n".into(),
            stderr: "".into(),
        };

        res.stdout_does_not_contain("likely");
    }

    #[test]
    #[should_panic]
    fn test_stderr_does_not_contain_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "".into(),
            stderr: "This is a likely error message\n".into(),
        };

        res.stderr_does_not_contain("likely");
    }

    #[test]
    fn test_stdout_matches() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "This is a likely error message\n".into(),
            stderr: "This is a likely error message\n".into(),
        };
        let positive = regex::Regex::new(".*likely.*").unwrap();
        let negative = regex::Regex::new(".*unlikely.*").unwrap();
        res.stdout_matches(&positive);
        res.stdout_does_not_match(&negative);
    }

    #[test]
    #[should_panic]
    fn test_stdout_matches_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "This is a likely error message\n".into(),
            stderr: "This is a likely error message\n".into(),
        };
        let negative = regex::Regex::new(".*unlikely.*").unwrap();

        res.stdout_matches(&negative);
    }

    #[test]
    #[should_panic]
    fn test_stdout_not_matches_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "This is a likely error message\n".into(),
            stderr: "This is a likely error message\n".into(),
        };
        let positive = regex::Regex::new(".*likely.*").unwrap();

        res.stdout_does_not_match(&positive);
    }

    #[test]
    fn test_normalized_newlines_stdout_is() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "A\r\nB\nC".into(),
            stderr: "".into(),
        };

        res.normalized_newlines_stdout_is("A\r\nB\nC");
        res.normalized_newlines_stdout_is("A\nB\nC");
        res.normalized_newlines_stdout_is("A\nB\r\nC");
    }

    #[test]
    #[should_panic]
    fn test_normalized_newlines_stdout_is_fail() {
        let res = CmdResult {
            bin_path: "".into(),
            util_name: None,
            tmpd: None,
            code: None,
            success: true,
            stdout: "A\r\nB\nC".into(),
            stderr: "".into(),
        };

        res.normalized_newlines_stdout_is("A\r\nB\nC\n");
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
}
