#![crate_name = "uu_stdbuf"]

/*
* This file is part of the uutils coreutils package.
*
* (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
*
* For the full copyright and license information, please view the LICENSE
* file that was distributed with this source code.
*/

extern crate getopts;

#[macro_use]
extern crate uucore;

use getopts::{Matches, Options};
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use uucore::fs::{canonicalize, CanonicalizeMode};

static NAME: &'static str = "stdbuf";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static LIBSTDBUF: &'static str = "libstdbuf"; 

enum BufferType {
    Default,
    Line,
    Size(u64)
}

struct ProgramOptions {
    stdin: BufferType,
    stdout: BufferType,
    stderr: BufferType,
}

enum ErrMsg {
    Retry,
    Fatal
}

enum OkMsg {
    Buffering,
    Help,
    Version
}

#[cfg(target_os = "linux")]
fn preload_strings() -> (&'static str, &'static str) { 
    ("LD_PRELOAD", ".so")
}

#[cfg(target_os = "macos")]
fn preload_strings() -> (&'static str, &'static str) { 
    ("DYLD_LIBRARY_PATH", ".dylib")
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn preload_strings() -> (&'static str, &'static str) { 
    crash!(1, "Command not supported for this operating system!")
}

fn print_version() {
    println!("{} {}", NAME, VERSION);
}

fn print_usage(opts: &Options) {
    let brief = 
        "Run COMMAND, with modified buffering operations for its standard streams\n \
        Mandatory arguments to long options are mandatory for short options too.";
    let explanation = 
        "If MODE is 'L' the corresponding stream will be line buffered.\n \
        This option is invalid with standard input.\n\n \
        If MODE is '0' the corresponding stream will be unbuffered.\n\n \
        Otherwise MODE is a number which may be followed by one of the following:\n\n \
        KB 1000, K 1024, MB 1000*1000, M 1024*1024, and so on for G, T, P, E, Z, Y.\n \
        In this case the corresponding stream will be fully buffered with the buffer size set to MODE bytes.\n\n \
        NOTE: If COMMAND adjusts the buffering of its standard streams ('tee' does for e.g.) then that will override \
        corresponding settings changed by 'stdbuf'.\n \
        Also some filters (like 'dd' and 'cat' etc.) don't use streams for I/O, \
        and are thus unaffected by 'stdbuf' settings.\n";
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage: stdbuf OPTION... COMMAND");
    println!("");
    println!("{}\n{}", opts.usage(brief), explanation);
}

fn parse_size(size: &str) -> Option<u64> {
    let ext = size.trim_left_matches(|c: char| c.is_digit(10));
    let num = size.trim_right_matches(|c: char| c.is_alphabetic());
    let mut recovered = num.to_owned();
    recovered.push_str(ext);
    if recovered != size {
        return None;
    }
    let buf_size: u64 = match num.parse().ok() {
        Some(m) => m,
        None => return None,
    };
    let (power, base): (u32, u64) = match ext {
        "" => (0, 0),
        "KB" => (1, 1024),
        "K" => (1, 1000),
        "MB" => (2, 1024),
        "M" => (2, 1000),
        "GB" => (3, 1024),
        "G" => (3, 1000),
        "TB" => (4, 1024),
        "T" => (4, 1000),
        "PB" => (5, 1024),
        "P" => (5, 1000),
        "EB" => (6, 1024),
        "E" => (6, 1000),
        "ZB" => (7, 1024),
        "Z" => (7, 1000),
        "YB" => (8, 1024),
        "Y" => (8, 1000),
        _ => return None,
    };
    Some(buf_size * base.pow(power))
}

fn check_option(matches: &Matches, name: &str, modified: &mut bool) -> Option<BufferType> {
    match matches.opt_str(name) {
        Some(value) => {
            *modified = true;
            match &value[..] {
                "L" => {
                    if name == "input" {
                        show_info!("line buffering stdin is meaningless");
                        None
                    } else {
                        Some(BufferType::Line)
                    }
                },
                x => {
                    let size = match parse_size(x) {
                        Some(m) => m,
                        None => { show_error!("Invalid mode {}", x); return None }
                    };
                    Some(BufferType::Size(size))
                },
            }
        },
        None => Some(BufferType::Default),
    }
}

fn parse_options(args: &[String], options: &mut ProgramOptions, optgrps: &Options) -> Result<OkMsg, ErrMsg> {
    let matches = match optgrps.parse(args) {
        Ok(m) => m,
        Err(_) => return Err(ErrMsg::Retry)
    };
    if matches.opt_present("help") {
        return Ok(OkMsg::Help);
    }
    if matches.opt_present("version") {
        return Ok(OkMsg::Version);
    }
    let mut modified = false;
    options.stdin = try!(check_option(&matches, "input", &mut modified).ok_or(ErrMsg::Fatal));
    options.stdout = try!(check_option(&matches, "output", &mut modified).ok_or(ErrMsg::Fatal));
    options.stderr = try!(check_option(&matches, "error", &mut modified).ok_or(ErrMsg::Fatal));

    if matches.free.len() != 1 {
        return Err(ErrMsg::Retry);
    }
    if !modified {
        show_error!("you must specify a buffering mode option");
        return Err(ErrMsg::Fatal);
    }
    Ok(OkMsg::Buffering)
}

fn set_command_env(command: &mut Command, buffer_name: &str, buffer_type: BufferType) {
    match buffer_type {
        BufferType::Size(m) => { command.env(buffer_name, m.to_string()); },
        BufferType::Line => { command.env(buffer_name, "L"); },
        BufferType::Default => {},
    }
}

fn exe_path() -> io::Result<PathBuf> {
    let exe_path = try!(std::env::current_exe());
    let absolute_path = try!(canonicalize(exe_path, CanonicalizeMode::Normal));
    Ok(match absolute_path.parent() {
        Some(p) => p.to_path_buf(),
        None => absolute_path.clone()
    })
}

fn get_preload_env() -> (String, String) {
    let (preload, extension) = preload_strings();
    let mut libstdbuf = LIBSTDBUF.to_owned();
    libstdbuf.push_str(extension);
    // First search for library in directory of executable.
    let mut path = exe_path().unwrap_or_else(|_| crash!(1, "Impossible to fetch the path of this executable."));
    path.push(libstdbuf.clone());
    if path.exists() {
        match path.as_os_str().to_str() {
            Some(s) => { return (preload.to_owned(), s.to_owned()); },
            None => crash!(1, "Error while converting path.")
        };
    }
    // We assume library is in LD_LIBRARY_PATH/ DYLD_LIBRARY_PATH.
    (preload.to_owned(), libstdbuf)
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optopt("i", "input", "adjust standard input stream buffering", "MODE");
    opts.optopt("o", "output", "adjust standard output stream buffering", "MODE");
    opts.optopt("e", "error", "adjust standard error stream buffering", "MODE");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let mut options = ProgramOptions {stdin: BufferType::Default, stdout: BufferType::Default, stderr: BufferType::Default};
    let mut command_idx: i32 = -1;
    for i in 1 .. args.len()+1 {
        match parse_options(&args[1 .. i], &mut options, &opts) {
            Ok(OkMsg::Buffering) => {
                command_idx = (i as i32) - 1;
                break;
            },
            Ok(OkMsg::Help) => {
                print_usage(&opts);
                return 0;
            },
            Ok(OkMsg::Version) => {
                print_version();
                return 0;
            },
            Err(ErrMsg::Fatal) => break,
            Err(ErrMsg::Retry) => continue,
        }
    };
    if command_idx == -1 {
        crash!(125, "Invalid options\nTry 'stdbuf --help' for more information.");
    }
    let command_name = &args[command_idx as usize];
    let mut command = Command::new(command_name);
    let (preload_env, libstdbuf) = get_preload_env();
    command.args(&args[(command_idx as usize) + 1 ..]).env(preload_env, libstdbuf);
    set_command_env(&mut command, "_STDBUF_I", options.stdin);
    set_command_env(&mut command, "_STDBUF_O", options.stdout);
    set_command_env(&mut command, "_STDBUF_E", options.stderr);
    let mut process = match command.spawn() {
        Ok(p) => p,
        Err(e) => crash!(1, "failed to execute process: {}", e)
    };
    match process.wait() {
        Ok(status) => {
            match status.code() {
                Some(i) => return i,
                None => crash!(1, "process killed by signal {}", status.signal().unwrap()),
            }
        },
        Err(e) => crash!(1, "{}", e)
    };
}
