#![allow(dead_code)]
extern crate tempdir;

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write, Result};
#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, Child};
use std::str::from_utf8;
use std::ffi::OsStr;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;
use self::tempdir::TempDir;

#[cfg(windows)]
static PROGNAME: &'static str = "uutils.exe";
#[cfg(not(windows))]
static PROGNAME: &'static str = "uutils";

static TESTS_DIR: &'static str = "tests";
static FIXTURES_DIR: &'static str = "fixtures";

static ALREADY_RUN: &'static str = " you have already run this UCommand, if you want to run \
                                    another command in the same test, use TestScenario::new instead of \
                                    testing();";
static MULTIPLE_STDIN_MEANINGLESS: &'static str = "Ucommand is designed around a typical use case of: provide args and input stream -> spawn process -> block until completion -> return output streams. For verifying that a particular section of the input stream is what causes a particular behavior, use the Command type directly.";

fn read_scenario_fixture<S: AsRef<OsStr>>(tmpd: &Option<Rc<TempDir>>, file_rel_path: S) -> String {
    let tmpdir_path = tmpd.as_ref().unwrap().as_ref().path();
    AtPath::new(tmpdir_path).read(file_rel_path.as_ref().to_str().unwrap())
}

pub fn repeat_str(s: &str, n: u32) -> String {
    let mut repeated = String::new();
    for _ in 0..n {
        repeated.push_str(s);
    }
    repeated
}

/// A command result is the outputs of a command (streams and status code)
/// within a struct which has convenience assertion functions about those outputs
pub struct CmdResult {
    //tmpd is used for convenience functions for asserts against fixtures
    tmpd: Option<Rc<TempDir>>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl CmdResult {
    /// asserts that the command resulted in a success (zero) status code
    pub fn success(&self) -> Box<&CmdResult> {
        assert!(self.success);
        Box::new(self)
    }

    /// asserts that the command resulted in a failure (non-zero) status code
    pub fn failure(&self) -> Box<&CmdResult> {
        assert!(!self.success);
        Box::new(self)
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// generally, it's better to use stdout_only() instead,
    /// but you might find yourself using this function if
    /// 1. you can not know exactly what stdout will be
    /// or 2. you know that stdout will also be empty
    pub fn no_stderr(&self) -> Box<&CmdResult> {
        assert_eq!("", self.stderr);
        Box::new(self)
    }

    /// asserts that the command resulted in empty (zero-length) stderr stream output
    /// unless asserting there was neither stdout or stderr, stderr_only is usually a better choice
    /// generally, it's better to use stderr_only() instead,
    /// but you might find yourself using this function if
    /// 1. you can not know exactly what stderr will be
    /// or 2. you know that stderr will also be empty
    pub fn no_stdout(&self) -> Box<&CmdResult> {
        assert_eq!("", self.stdout);
        Box::new(self)
    }

    /// asserts that the command resulted in stdout stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// stdout_only is a better choice unless stderr may or will be non-empty
    pub fn stdout_is<T: AsRef<str>>(&self, msg: T) -> Box<&CmdResult> {
        assert_eq!(String::from(msg.as_ref()).trim_right(), self.stdout.trim_right());
        Box::new(self)
    }

    /// like stdout_is(...), but expects the contents of the file at the provided relative path
    pub fn stdout_is_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> Box<&CmdResult> {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_is(contents)
    }

    /// asserts that the command resulted in stderr stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// stderr_only is a better choice unless stdout may or will be non-empty
    pub fn stderr_is<T: AsRef<str>>(&self, msg: T) -> Box<&CmdResult> {
        assert_eq!(String::from(msg.as_ref()).trim_right(), self.stderr.trim_right());
        Box::new(self)
    }

    /// like stderr_is(...), but expects the contents of the file at the provided relative path
    pub fn stderr_is_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> Box<&CmdResult> {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stderr_is(contents)
    }

    /// asserts that
    /// 1. the command resulted in stdout stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// and 2. the command resulted in empty (zero-length) stderr stream output
    pub fn stdout_only<T: AsRef<str>>(&self, msg: T) -> Box<&CmdResult> {
        self.no_stderr().stdout_is(msg)
    }

    /// like stdout_only(...), but expects the contents of the file at the provided relative path
    pub fn stdout_only_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> Box<&CmdResult> {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stdout_only(contents)
    }

    /// asserts that
    /// 1. the command resulted in stderr stream output that equals the
    /// passed in value, when both are trimmed of trailing whitespace
    /// and 2. the command resulted in empty (zero-length) stdout stream output
    pub fn stderr_only<T: AsRef<str>>(&self, msg: T) -> Box<&CmdResult> {
        self.no_stdout().stderr_is(msg)
    }

    /// like stderr_only(...), but expects the contents of the file at the provided relative path
    pub fn stderr_only_fixture<T: AsRef<OsStr>>(&self, file_rel_path: T) -> Box<&CmdResult> {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.stderr_only(contents)
    }

    pub fn fails_silently(&self) -> Box<&CmdResult> {
        assert!(!self.success);
        assert_eq!("", self.stderr);
        Box::new(self)
    }
}

pub fn log_info<T: AsRef<str>, U: AsRef<str>>(msg: T, par: U) {
    println!("{}: {}", msg.as_ref(), par.as_ref());
}

pub fn recursive_copy(src: &Path, dest: &Path) -> Result<()> {
    if try!(fs::metadata(src)).is_dir() {
        for entry in try!(fs::read_dir(src)) {
            let entry = try!(entry);
            let mut new_dest = PathBuf::from(dest);
            new_dest.push(entry.file_name());
            if try!(fs::metadata(entry.path())).is_dir() {
                try!(fs::create_dir(&new_dest));
                try!(recursive_copy(&entry.path(), &new_dest));
            } else {
                try!(fs::copy(&entry.path(), new_dest));
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
        AtPath { subdir: PathBuf::from(subdir) }
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
            for component in prefixed.components()
                                     .skip(self.subdir.components().count()) {
                unprefixed.push(component.as_ref().to_str().unwrap());
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
        let _ = f.read_to_string(&mut contents);
        contents
    }

    pub fn write(&self, name: &str, contents: &str) {
        let mut f = self.open(name);
        let _ = f.write(contents.as_bytes());
    }

    pub fn append(&self, name: &str, contents: &str) {
        log_info("open(append)", self.plus_as_string(name));
        let mut f = OpenOptions::new().write(true).append(true).open(self.plus(name)).unwrap();
        let _ = f.write(contents.as_bytes());
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

    pub fn symlink(&self, src: &str, dst: &str) {
        log_info("symlink",
                 &format!("{},{}", self.plus_as_string(src), self.plus_as_string(dst)));
        symlink_file(&self.plus(src), &self.plus(dst)).unwrap();
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
            Ok(p) => {
                self.minus_as_string(p.to_str().unwrap())
            }
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

    pub fn cleanup(&self, path: &'static str) {
        let p = &self.plus(path);
        match fs::metadata(p) {
            Ok(m) => if m.is_file() {
                fs::remove_file(&p).unwrap();
            } else {
                fs::remove_dir(&p).unwrap();
            },
            Err(_) => {}
        }
    }

    pub fn root_dir(&self) -> String {
        log_info("current_directory", "");
        self.subdir.to_str().unwrap().to_owned()
    }

    pub fn root_dir_resolved(&self) -> String {
        log_info("current_directory_resolved", "");
        let s = self.subdir.canonicalize().unwrap().to_str().unwrap().to_owned();

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
/// 2. provides a temporary directory for the test case
/// 3. copies over fixtures for the utility to the temporary directory
pub struct TestScenario {
    bin_path: PathBuf,
    util_name: String,
    pub fixtures: AtPath,
    tmpd: Rc<TempDir>,
}

impl TestScenario {
    pub fn new(util_name: &str) -> TestScenario {
        let tmpd = Rc::new(TempDir::new("uutils").unwrap());
        let ts = TestScenario {
            bin_path: {
                // Instead of hardcoding the path relative to the current
                // directory, use Cargo's OUT_DIR to find path to executable.
                // This allows tests to be run using profiles other than debug.
                let target_dir = path_concat!(env::var("OUT_DIR").unwrap(), "..", "..", "..", PROGNAME);
                PathBuf::from(AtPath::new(&Path::new(&target_dir)).root_dir_resolved())
            },
            util_name: String::from(util_name),
            fixtures: AtPath::new(&tmpd.as_ref().path()),
            tmpd: tmpd,
        };
        let mut fixture_path_builder = env::current_dir().unwrap();
        fixture_path_builder.push(TESTS_DIR);
        fixture_path_builder.push(FIXTURES_DIR);
        fixture_path_builder.push(util_name);
        match fs::metadata(&fixture_path_builder) {
            Ok(m) => if m.is_dir() {
                recursive_copy(&fixture_path_builder, &ts.fixtures.subdir).unwrap();
            },
            Err(_) => {}
        }
        ts
    }

    pub fn ucmd(&self) -> UCommand {
        let mut cmd = self.cmd(&self.bin_path);
        cmd.arg(&self.util_name);
        cmd
    }

    pub fn cmd<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp(bin, self.tmpd.clone(), true)
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

/// A UCommand is a wrapper around an individual Command that provides several additional features
/// 1. it has convenience functions that are more ergonomic to use for piping in stdin, spawning the command
///       and asserting on the results.
/// 2. it tracks arguments provided so that in test cases which may provide variations of an arg in loops
///     the test failure can display the exact call which preceded an assertion failure.
/// 3. it provides convenience construction arguments to set the Command working directory and/or clear its environment.
pub struct UCommand {
    pub raw: Command,
    comm_string: String,
    tmpd: Option<Rc<TempDir>>,
    has_run: bool,
    stdin: Option<Vec<u8>>
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
            stdin: None
        }
    }

    pub fn new_from_tmp<T: AsRef<OsStr>>(arg: T, tmpd: Rc<TempDir>, env_clear: bool) -> UCommand {
        let tmpd_path_buf = String::from(&(*tmpd.as_ref().path().to_str().unwrap()));
        let mut ucmd: UCommand = UCommand::new(arg.as_ref(), tmpd_path_buf, env_clear);
        ucmd.tmpd = Some(tmpd);
        ucmd
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> Box<&mut UCommand> {
        if self.has_run {
            panic!(ALREADY_RUN);
        }
        self.comm_string.push_str(" ");
        self.comm_string.push_str(arg.as_ref().to_str().unwrap());
        self.raw.arg(arg.as_ref());
        Box::new(self)
    }

    /// like arg(...), but uses the contents of the file at the provided relative path as the argument
    pub fn arg_fixture<S: AsRef<OsStr>>(&mut self, file_rel_path: S) -> Box<&mut UCommand> {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.arg(contents)
    }

    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> Box<&mut UCommand> {
        if self.has_run {
            panic!(MULTIPLE_STDIN_MEANINGLESS);
        }
        for s in args {
            self.comm_string.push_str(" ");
            self.comm_string.push_str(s.as_ref().to_str().unwrap());
        }

        self.raw.args(args.as_ref());
        Box::new(self)
    }

    /// provides stdinput to feed in to the command when spawned
    pub fn pipe_in<T: Into<Vec<u8>>>(&mut self, input: T) -> Box<&mut UCommand> {
        if self.stdin.is_some() {
            panic!(MULTIPLE_STDIN_MEANINGLESS);
        }
        self.stdin = Some(input.into());
        Box::new(self)
    }

    /// like pipe_in(...), but uses the contents of the file at the provided relative path as the piped in data
    pub fn pipe_in_fixture<S: AsRef<OsStr>>(&mut self, file_rel_path: S) -> Box<&mut UCommand> {
        let contents = read_scenario_fixture(&self.tmpd, file_rel_path);
        self.pipe_in(contents)
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> Box<&mut UCommand> where K: AsRef<OsStr>, V: AsRef<OsStr> {
        if self.has_run {
            panic!(ALREADY_RUN);
        }
        self.raw.env(key, val);
        Box::new(self)
    }

    /// Spawns the command, feeds the stdin if any, and returns the
    /// child process immediately.
    pub fn run_no_wait(&mut self) -> Child {
        if self.has_run {
            panic!(ALREADY_RUN);
        }
        self.has_run = true;
        log_info("run", &self.comm_string);
        let mut result = self.raw
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        if let Some(ref input) = self.stdin {
            result.stdin
                .take()
                .unwrap_or_else(
                    || panic!(
                        "Could not take child process stdin"))
                .write_all(&input)
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
    /// asserts success, and returns a command result.
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
    child.stdout.as_mut().unwrap().read(output.as_mut_slice()).unwrap();
    String::from_utf8(output).unwrap()
}
