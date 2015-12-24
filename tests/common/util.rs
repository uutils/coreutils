#![allow(dead_code)]

extern crate tempdir;

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write, Result};
#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::from_utf8;
use std::ffi::OsStr;
use self::tempdir::TempDir;
use std::rc::Rc;

#[cfg(windows)]
static PROGNAME: &'static str = "target\\debug\\uutils.exe";
#[cfg(windows)]
static FIXTURES_DIR: &'static str = "tests\\fixtures";
#[cfg(not(windows))]
static PROGNAME: &'static str = "target/debug/uutils";
#[cfg(not(windows))]
static FIXTURES_DIR: &'static str = "tests/fixtures";
static ALREADY_RUN: &'static str = " you have already run this UCommand, if you want to run \
                                    another command in the same test, use TestSet::new instead of \
                                    testing();";

#[macro_export]
macro_rules! assert_empty_stderr(
    ($cond:expr) => (
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);

#[macro_export]
macro_rules! assert_no_error(
    ($cond:expr) => (
        assert!($cond.success);
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);

pub struct CmdResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn log_info<T: AsRef<str>, U: AsRef<str>>(msg: T, par: U) {
    println!("{}: {}", msg.as_ref(), par.as_ref());
}


pub fn repeat_str(s: &str, n: u32) -> String {
    let mut repeated = String::new();
    for _ in 0..n {
        repeated.push_str(s);
    }
    repeated
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
        // relative_from is currently unstable
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
}

pub struct TestSet {
    bin_path: PathBuf,
    util_name: String,
    pub fixtures: AtPath,
    tmpd: Rc<TempDir>,
}
impl TestSet {
    pub fn new(util_name: &str) -> TestSet {
        let tmpd = Rc::new(TempDir::new("uutils").unwrap());
        let ts = TestSet {
            bin_path: {
                let mut bin_path_builder = env::current_dir().unwrap();
                bin_path_builder.push(PathBuf::from(PROGNAME));
                bin_path_builder
            },
            util_name: String::from(util_name),
            fixtures: AtPath::new(&tmpd.as_ref().path()),
            tmpd: tmpd,
        };
        let mut fixture_path_builder = env::current_dir().unwrap();
        fixture_path_builder.push(PathBuf::from(FIXTURES_DIR));
        fixture_path_builder.push(PathBuf::from(util_name));
        match fs::metadata(&fixture_path_builder) {
            Ok(m) => if m.is_dir() {
                recursive_copy(&fixture_path_builder, &ts.fixtures.subdir).unwrap();
            },
            Err(_) => {}
        }
        ts
    }
    pub fn util_cmd(&self) -> UCommand {
        let mut cmd = self.cmd(&self.bin_path);
        cmd.arg(&self.util_name);
        cmd
    }
    pub fn cmd<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp(bin, self.tmpd.clone(), true)
    }
    // different names are used rather than an argument
    // because the need to keep the environment is exceedingly rare.
    pub fn util_cmd_keepenv(&self) -> UCommand {
        let mut cmd = self.cmd_keepenv(&self.bin_path);
        cmd.arg(&self.util_name);
        cmd
    }
    pub fn cmd_keepenv<S: AsRef<OsStr>>(&self, bin: S) -> UCommand {
        UCommand::new_from_tmp(bin, self.tmpd.clone(), false)
    }
}

pub struct UCommand {
    pub raw: Command,
    comm_string: String,
    tmpd: Option<Rc<TempDir>>,
    has_run: bool,
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
                    cmd.env_clear();
                }
                cmd
            },
            comm_string: String::from(arg.as_ref().to_str().unwrap()),
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

    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> Box<&mut UCommand> {
        if self.has_run {
            panic!(ALREADY_RUN);
        }
        for s in args {
            self.comm_string.push_str(" ");
            self.comm_string.push_str(s.as_ref().to_str().unwrap());
        }

        self.raw.args(args.as_ref());
        Box::new(self)
    }
    pub fn run(&mut self) -> CmdResult {
        self.has_run = true;
        log_info("run", &self.comm_string);
        let prog = self.raw.output().unwrap();
        CmdResult {
            success: prog.status.success(),
            stdout: from_utf8(&prog.stdout).unwrap().to_string(),
            stderr: from_utf8(&prog.stderr).unwrap().to_string(),
        }
    }
    pub fn run_piped_stdin<T: AsRef<[u8]>>(&mut self, input: T) -> CmdResult {
        self.has_run = true;
        log_info("run_piped_stdin", &self.comm_string);
        let mut result = self.raw
                             .stdin(Stdio::piped())
                             .stdout(Stdio::piped())
                             .stderr(Stdio::piped())
                             .spawn()
                             .unwrap();

        result.stdin
              .take()
              .unwrap_or_else(|| panic!("Could not take child process stdin"))
              .write_all(input.as_ref())
              .unwrap_or_else(|e| panic!("{}", e));

        let prog = result.wait_with_output().unwrap();
        CmdResult {
            success: prog.status.success(),
            stdout: from_utf8(&prog.stdout).unwrap().to_string(),
            stderr: from_utf8(&prog.stderr).unwrap().to_string(),
        }
    }
}

// returns a testSet and a ucommand initialized to the utility binary
// operating in the fixtures directory with a cleared environment
pub fn testset_and_ucommand(utilname: &str) -> (TestSet, UCommand) {
    let ts = TestSet::new(utilname);
    let ucmd = ts.util_cmd();
    (ts, ucmd)
}
pub fn testing(utilname: &str) -> (AtPath, UCommand) {
    let ts = TestSet::new(utilname);
    let ucmd = ts.util_cmd();
    (ts.fixtures, ucmd)
}
