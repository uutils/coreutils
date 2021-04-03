#![allow(dead_code)]

#[cfg(not(windows))]
use libc;
use std::env;
#[cfg(not(windows))]
use std::ffi::CString;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Result, Write};
#[cfg(unix)]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::str::from_utf8;
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

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

/// Test if the program is running under CI
pub fn is_ci() -> bool {
    std::env::var("CI")
        .unwrap_or(String::from("false"))
        .eq_ignore_ascii_case("true")
}

/// Test if the program is running under WSL
// ref: <https://github.com/microsoft/WSL/issues/4555> @@ <https://archive.is/dP0bz>
// ToDO: test on WSL2 which likely doesn't need special handling; plan change to `is_wsl_1()` if WSL2 is less needy
pub fn is_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(b) = std::fs::read("/proc/sys/kernel/osrelease") {
            if let Ok(s) = std::str::from_utf8(&b) {
                let a = s.to_ascii_lowercase();
                return a.contains("microsoft") || a.contains("wsl");
            }
        }
    }
    false
}

/// Read a test scenario fixture, returning its bytes
fn read_scenario_fixture<S: AsRef<OsStr>>(tmpd: &Option<Rc<TempDir>>, file_rel_path: S) -> Vec<u8> {
    let tmpdir_path = tmpd.as_ref().unwrap().as_ref().path();
    AtPath::new(tmpdir_path).read_bytes(file_rel_path.as_ref().to_str().unwrap())
}

/// A command result is the outputs of a command (streams and status code)
/// within a struct which has convenience assertion functions about those outputs
#[derive(Debug)]
pub struct CmdResult {
    //tmpd is used for convenience functions for asserts against fixtures
    tmpd: Option<Rc<TempDir>>,
    /// exit status for command (if there is one)
    pub code: Option<i32>,
    /// zero-exit from running the Command?
    /// see [`success`]
    pub success: bool,
    /// captured standard output after running the Command
    pub stdout: String,
    /// captured standard error after running the Command
    pub stderr: String,
}

impl CmdResult {
    /// Returns a reference to the program's standard output as a slice of bytes
    pub fn stdout(&self) -> &[u8] {
        &self.stdout.as_bytes()
    }

    /// Returns the program's standard output as a string slice
    pub fn stdout_str(&self) -> &str {
        &self.stdout
    }

    /// Returns the program's standard output as a string
    /// consumes self
    pub fn stdout_move_str(self) -> String {
        self.stdout
    }

    /// Returns the program's standard output as a vec of bytes
    /// consumes self
    pub fn stdout_move_bytes(self) -> Vec<u8> {
        Vec::from(self.stdout)
    }

    /// Returns a reference to the program's standard error as a slice of bytes
    pub fn stderr(&self) -> &[u8] {
        &self.stderr.as_bytes()
    }

    /// Returns the program's standard error as a string slice
    pub fn stderr_str(&self) -> &str {
        &self.stderr
    }

    /// Returns the program's standard error as a string
    /// consumes self
    pub fn stderr_move_str(self) -> String {
        self.stderr
    }

    /// Returns the program's standard error as a vec of bytes
    /// consumes self
    pub fn stderr_move_bytes(self) -> Vec<u8> {
        Vec::from(self.stderr)
    }

    /// Returns the program's exit code
    /// Panics if not run
    pub fn code(&self) -> i32 {
        self.code.expect("Program must be run first")
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
    pub fn success(&self) -> &CmdResult {
        assert!(self.success);
        self
    }

    /// asserts that the command resulted in a failure (non-zero) status code
    pub fn failure(&self) -> &CmdResult {
        assert!(!self.success);
        self
    }

    /// asserts that the command's exit code is the same as the given one
    pub fn status_code(&self, code: i32) -> &CmdResult {
        assert!(self.code == Some(code));
        self
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// generally, it's better to use stdout_only() instead,
    /// but you might find yourself using this function if
    /// 1. you can not know exactly what stdout will be
    /// or 2. you know that stdout will also be empty
    pub fn no_stderr(&self) -> &CmdResult {
        assert!(self.stderr.is_empty());
        self
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// unless asserting there was neither stdout or stderr, stderr_only is usually a better choice
    /// generally, it's better to use stderr_only() instead,
    /// but you might find yourself using this function if
    /// 1. you can not know exactly what stderr will be
    /// or 2. you know that stderr will also be empty
    pub fn no_stdout(&self) -> &CmdResult {
        assert!(self.stdout.is_empty());
        self
    }

    /// asserts that the command resulted in stdout stream output that equals the
    /// passed in value, trailing whitespace are kept to force strict comparison (#1235)
    /// stdout_only is a better choice unless stderr may or will be non-empty
    pub fn stdout_is<T: AsRef<str>>(&self, msg: T) -> &CmdResult {
        assert_eq!(self.stdout, String::from(msg.as_ref()));
        self
    }

    /// asserts that the command resulted in stdout stream output,
    /// whose bytes equal those of the passed in slice
    pub fn stdout_is_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &CmdResult {
        assert_eq!(self.stdout.as_bytes(), msg.as_ref());
        self
    }

    /// like stdout_is(...), but expects the contents of the file at the provided relative path
    pub fn stdout_is_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &CmdResult {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_is_bytes(contents)
    }

    /// asserts that the command resulted in stderr stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// stderr_only is a better choice unless stdout may or will be non-empty
    pub fn stderr_is<T: AsRef<str>>(&self, msg: T) -> &CmdResult {
        assert_eq!(
            self.stderr.trim_end(),
            String::from(msg.as_ref()).trim_end()
        );
        self
    }

    /// asserts that the command resulted in stderr stream output,
    /// whose bytes equal those of the passed in slice
    pub fn stderr_is_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &CmdResult {
        assert_eq!(self.stderr.as_bytes(), msg.as_ref());
        self
    }

    /// asserts that
    /// 1. the command resulted in stdout stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// and 2. the command resulted in empty (zero-length) stderr stream output
    pub fn stdout_only<T: AsRef<str>>(&self, msg: T) -> &CmdResult {
        self.no_stderr().stdout_is(msg)
    }

    /// asserts that
    /// 1.  the command resulted in a stdout stream whose bytes
    ///     equal those of the passed in value
    /// 2.  the command resulted in an empty stderr stream
    pub fn stdout_only_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &CmdResult {
        self.no_stderr().stdout_is_bytes(msg)
    }

    /// like stdout_only(...), but expects the contents of the file at the provided relative path
    pub fn stdout_only_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> &CmdResult {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_only_bytes(contents)
    }

    /// asserts that
    /// 1. the command resulted in stderr stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// and 2. the command resulted in empty (zero-length) stdout stream output
    pub fn stderr_only<T: AsRef<str>>(&self, msg: T) -> &CmdResult {
        self.no_stdout().stderr_is(msg)
    }

    /// asserts that
    /// 1.  the command resulted in a stderr stream whose bytes equal the ones
    ///     of the passed value
    /// 2.  the command resulted in an empty stdout stream
    pub fn stderr_only_bytes<T: AsRef<[u8]>>(&self, msg: T) -> &CmdResult {
        self.no_stderr().stderr_is_bytes(msg)
    }

    pub fn fails_silently(&self) -> &CmdResult {
        assert!(!self.success);
        assert!(self.stderr.is_empty());
        self
    }

    pub fn stdout_contains<T: AsRef<str>>(&self, cmp: T) -> &CmdResult {
        assert!(self.stdout_str().contains(cmp.as_ref()));
        self
    }

    pub fn stderr_contains<T: AsRef<str>>(&self, cmp: &T) -> &CmdResult {
        assert!(self.stderr_str().contains(cmp.as_ref()));
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
    pub fn new(subdir: &Path) -> AtPath {
        AtPath {
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
        log_info("open(write)", self.plus_as_string(name));
        std::fs::write(self.plus(name), contents)
            .unwrap_or_else(|e| panic!("Couldn't write {}: {}", name, e));
    }

    pub fn write_bytes(&self, name: &str, contents: &[u8]) {
        log_info("open(write)", self.plus_as_string(name));
        std::fs::write(self.plus(name), contents)
            .unwrap_or_else(|e| panic!("Couldn't write {}: {}", name, e));
    }

    pub fn append(&self, name: &str, contents: &str) {
        log_info("open(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .open(self.plus(name))
            .unwrap();
        f.write(contents.as_bytes())
            .unwrap_or_else(|e| panic!("Couldn't write {}: {}", name, e));
    }

    pub fn append_bytes(&self, name: &str, contents: &[u8]) {
        log_info("open(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .open(self.plus(name))
            .unwrap();
        f.write_all(contents)
            .unwrap_or_else(|e| panic!("Couldn't append to {}: {}", name, e));
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
                libc::S_IFIFO & stat.st_mode != 0
            } else {
                false
            }
        }
    }

    pub fn symlink_file(&self, src: &str, dst: &str) {
        log_info(
            "symlink",
            &format!("{},{}", self.plus_as_string(src), self.plus_as_string(dst)),
        );
        symlink_file(&self.plus(src), &self.plus(dst)).unwrap();
    }

    pub fn symlink_dir(&self, src: &str, dst: &str) {
        log_info(
            "symlink",
            &format!("{},{}", self.plus_as_string(src), self.plus_as_string(dst)),
        );
        symlink_dir(&self.plus(src), &self.plus(dst)).unwrap();
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
        if s.starts_with(prefix) {
            String::from(&s[prefix.len()..])
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
    bin_path: PathBuf,
    util_name: String,
    pub fixtures: AtPath,
    tmpd: Rc<TempDir>,
}

impl TestScenario {
    pub fn new(util_name: &str) -> TestScenario {
        let tmpd = Rc::new(TempDir::new().unwrap());
        let ts = TestScenario {
            bin_path: {
                // Instead of hardcoding the path relative to the current
                // directory, use Cargo's OUT_DIR to find path to executable.
                // This allows tests to be run using profiles other than debug.
                let target_dir = path_concat!(env!("OUT_DIR"), "..", "..", "..", PROGNAME);
                PathBuf::from(AtPath::new(Path::new(&target_dir)).root_dir_resolved())
            },
            util_name: String::from(util_name),
            fixtures: AtPath::new(tmpd.as_ref().path()),
            tmpd: tmpd,
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
        let mut cmd = self.cmd(&self.bin_path);
        cmd.arg(&self.util_name);
        cmd
    }

    /// Returns builder for invoking any system command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    pub fn cmd<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp(bin, self.tmpd.clone(), true)
    }

    /// Returns builder for invoking any uutils command. Paths given are treated
    /// relative to the environment's unique temporary test directory.
    pub fn ccmd<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        let mut cmd = self.cmd(&self.bin_path);
        cmd.arg(bin);
        cmd
    }

    // different names are used rather than an argument
    // because the need to keep the environment is exceedingly rare.
    pub fn ucmd_keepenv(&self) -> UCommand {
        let mut cmd = self.cmd_keepenv(&self.bin_path);
        cmd.arg(&self.util_name);
        cmd
    }

    pub fn cmd_keepenv<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp(bin, self.tmpd.clone(), false)
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
    tmpd: Option<Rc<TempDir>>,
    has_run: bool,
    stdin: Option<Vec<u8>>,
}

impl UCommand {
    pub fn new<T: AsRef<OsStr>, U: AsRef<OsStr>>(arg: T, curdir: U, env_clear: bool) -> UCommand {
        UCommand {
            tmpd: None,
            has_run: false,
            raw: {
                let mut cmd = Command::new(arg.as_ref());
                cmd.current_dir(curdir.as_ref());
                if env_clear {
                    if cfg!(windows) {
                        // %SYSTEMROOT% is required on Windows to initialize crypto provider
                        // ... and crypto provider is required for std::rand
                        // From procmon: RegQueryValue HKLM\SOFTWARE\Microsoft\Cryptography\Defaults\Provider\Microsoft Strong Cryptographic Provider\Image Path
                        // SUCCESS  Type: REG_SZ, Length: 66, Data: %SystemRoot%\system32\rsaenh.dll"
                        for (key, _) in env::vars_os() {
                            if key.as_os_str() != "SYSTEMROOT" {
                                cmd.env_remove(key);
                            }
                        }
                    } else {
                        cmd.env_clear();
                    }
                }
                cmd
            },
            comm_string: String::from(arg.as_ref().to_str().unwrap()),
            stdin: None,
        }
    }

    pub fn new_from_tmp<T: AsRef<OsStr>>(arg: T, tmpd: Rc<TempDir>, env_clear: bool) -> UCommand {
        let tmpd_path_buf = String::from(&(*tmpd.as_ref().path().to_str().unwrap()));
        let mut ucmd: UCommand = UCommand::new(arg.as_ref(), tmpd_path_buf, env_clear);
        ucmd.tmpd = Some(tmpd);
        ucmd
    }

    /// Add a parameter to the invocation. Path arguments are treated relative
    /// to the test environment directory.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut UCommand {
        if self.has_run {
            panic!("{}", ALREADY_RUN);
        }
        self.comm_string.push_str(" ");
        self.comm_string.push_str(arg.as_ref().to_str().unwrap());
        self.raw.arg(arg.as_ref());
        self
    }

    /// Add multiple parameters to the invocation. Path arguments are treated relative
    /// to the test environment directory.
    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut UCommand {
        if self.has_run {
            panic!("{}", MULTIPLE_STDIN_MEANINGLESS);
        }
        for s in args {
            self.comm_string.push_str(" ");
            self.comm_string.push_str(s.as_ref().to_str().unwrap());
        }

        self.raw.args(args.as_ref());
        self
    }

    /// provides stdinput to feed in to the command when spawned
    pub fn pipe_in<T: Into<Vec<u8>>>(&mut self, input: T) -> &mut UCommand {
        if self.stdin.is_some() {
            panic!("{}", MULTIPLE_STDIN_MEANINGLESS);
        }
        self.stdin = Some(input.into());
        self
    }

    /// like pipe_in(...), but uses the contents of the file at the provided relative path as the piped in data
    pub fn pipe_in_fixture<S: AsRef<OsStr>>(&mut self, file_rel_path: S) -> &mut UCommand {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.pipe_in(contents)
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut UCommand
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        if self.has_run {
            panic!("{}", ALREADY_RUN);
        }
        self.raw.env(key, val);
        self
    }

    /// Spawns the command, feeds the stdin if any, and returns the
    /// child process immediately.
    pub fn run_no_wait(&mut self) -> Child {
        if self.has_run {
            panic!("{}", ALREADY_RUN);
        }
        self.has_run = true;
        log_info("run", &self.comm_string);
        let mut result = self
            .raw
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        if let Some(ref input) = self.stdin {
            result
                .stdin
                .take()
                .unwrap_or_else(|| panic!("Could not take child process stdin"))
                .write_all(input)
                .unwrap_or_else(|e| panic!("{}", e));
        }

        result
    }

    /// Spawns the command, feeds the stdin if any, waits for the result
    /// and returns a command result.
    /// It is recommended that you instead use succeeds() or fails()
    pub fn run(&mut self) -> CmdResult {
        let prog = self.run_no_wait().wait_with_output().unwrap();

        CmdResult {
            tmpd: self.tmpd.clone(),
            code: prog.status.code(),
            success: prog.status.success(),
            stdout: from_utf8(&prog.stdout).unwrap().to_string(),
            stderr: from_utf8(&prog.stderr).unwrap().to_string(),
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
}

pub fn read_size(child: &mut Child, size: usize) -> String {
    let mut output = Vec::new();
    output.resize(size, 0);
    sleep(Duration::from_secs(1));
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(output.as_mut_slice())
        .unwrap();
    String::from_utf8(output).unwrap()
}
