#![allow(dead_code)]

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::from_utf8;

#[macro_export]
macro_rules! assert_empty_stderr(
    ($cond:expr) => (
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

pub fn run(cmd: &mut Command) -> CmdResult {
    let prog = cmd.output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stdout: from_utf8(&prog.stdout).unwrap().to_string(),
        stderr: from_utf8(&prog.stderr).unwrap().to_string(),
    }
}

pub fn run_piped_stdin<T: AsRef<[u8]>>(cmd: &mut Command, input: T)-> CmdResult {
    let mut command = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    command.stdin
        .take()
        .unwrap_or_else(|| panic!("Could not take child process stdin"))
        .write_all(input.as_ref())
        .unwrap_or_else(|e| panic!("{}", e));

    let prog = command.wait_with_output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stdout: from_utf8(&prog.stdout).unwrap().to_string(),
        stderr: from_utf8(&prog.stderr).unwrap().to_string(),
    }
}

pub fn get_file_contents(name: &str) -> String {
    let mut f = File::open(Path::new(name)).unwrap();
    let mut contents = String::new();
    let _ = f.read_to_string(&mut contents);
    contents
}

pub fn mkdir(dir: &str) {
    fs::create_dir(Path::new(dir)).unwrap();
}

pub fn make_file(name: &str) -> File {
    match File::create(Path::new(name)) {
        Ok(f) => f,
        Err(e) => panic!("{}", e)
    }
}

pub fn touch(file: &str) {
    File::create(Path::new(file)).unwrap();
}

pub fn symlink(src: &str, dst: &str) {
    symlink_file(src, dst).unwrap();
}

pub fn is_symlink(path: &str) -> bool {
    match fs::symlink_metadata(path) {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false
    }
}

pub fn resolve_link(path: &str) -> String {
    match fs::read_link(path) {
        Ok(p) => p.to_str().unwrap().to_owned(),
        Err(_) => "".to_string()
    }
}

pub fn metadata(path: &str) -> fs::Metadata {
    match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => panic!("{}", e)
    }
}

pub fn file_exists(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(m) => m.is_file(),
        Err(_) => false
    }
}

pub fn dir_exists(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(m) => m.is_dir(),
        Err(_) => false
    }
}

pub fn cleanup(path: &'static str) {
    let p = Path::new(path);
    match fs::metadata(p) {
        Ok(m) => if m.is_file() {
            fs::remove_file(&p).unwrap();
        } else {
            fs::remove_dir(&p).unwrap();
        },
        Err(_) => {}
    }
}

pub fn current_directory() -> String {
    env::current_dir().unwrap().into_os_string().into_string().unwrap()
}

pub fn repeat_str(s: &str, n: u32) -> String {
    let mut repeated = String::new();
    for _ in 0 .. n {
        repeated.push_str(s);
    }
    repeated
}
