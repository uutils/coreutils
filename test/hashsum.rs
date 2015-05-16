use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::str::from_utf8;

static PROGNAME: &'static str = "./hashsum";

struct CmdResult {
    success: bool,
    stdout: String,
    stderr: String,
}

fn run(cmd: &mut Command) -> CmdResult {
    let prog = cmd.output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stdout: from_utf8(&prog.stdout).unwrap().to_string(),
        stderr: from_utf8(&prog.stderr).unwrap().to_string(),
    }
}

fn run_piped_stdin(cmd: &mut Command, input: &[u8])-> CmdResult {
    let mut command = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    command.stdin
        .take()
        .unwrap_or_else(|| panic!("Could not take child process stdin"))
        .write_all(input)
        .unwrap_or_else(|e| panic!("{}", e));

    let prog = command.wait_with_output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stdout: from_utf8(&prog.stdout).unwrap().to_string(),
        stderr: from_utf8(&prog.stderr).unwrap().to_string(),
    }
}

fn get_file_contents(name: &str) -> String {
    let mut f = File::open(name).unwrap();
    let mut contents = String::new();
    let _ = f.read_to_string(&mut contents);
    contents
}

macro_rules! assert_empty_stderr(
    ($cond:expr) => (
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);

macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($t:ident)*) => ($(

    mod $t {
        use std::process::Command;

        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static EXPECTED_FILE: &'static str = concat!(stringify!($t), ".expected");

        #[test]
        fn test_single_file() {
            let mut cmd = Command::new(::PROGNAME);
            let result = ::run(&mut cmd.arg(DIGEST_ARG).arg("input.txt"));

            assert_empty_stderr!(result);
            assert!(result.success);
            assert_eq!(get_hash!(result.stdout), ::get_file_contents(EXPECTED_FILE));
        }

        #[test]
        fn test_stdin() {
            let input = ::get_file_contents("input.txt");
            let mut cmd = Command::new(::PROGNAME);
            let result = ::run_piped_stdin(&mut cmd.arg(DIGEST_ARG), input.as_bytes());

            assert_empty_stderr!(result);
            assert!(result.success);
            assert_eq!(get_hash!(result.stdout), ::get_file_contents(EXPECTED_FILE));
        }
    }
    )*)
}

test_digest! { md5 sha1 sha224 sha256 sha384 sha512 }
