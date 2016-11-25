#![crate_name = "uu_dircolors"]

// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

extern crate glob;

#[macro_use]
extern crate uucore;


use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::borrow::Borrow;
use std::env;

static SYNTAX: &'static str = "[OPTION]... [FILE]";
static SUMMARY: &'static str = "Output commands to set the LS_COLORS environment variable."; 
static LONG_HELP: &'static str = "
 If FILE is specified, read it to determine which colors to use for which
 file types and extensions.  Otherwise, a precompiled database is used.
 For details on the format of these files, run 'dircolors --print-database'
"; 

mod colors;
use colors::INTERNAL_DB;

#[derive(PartialEq, Debug)]
pub enum OutputFmt {
    Shell,
    CShell,
    Unknown,
}

pub fn guess_syntax() -> OutputFmt {
    use std::path::Path;
    match env::var("SHELL") {
        Ok(ref s) if !s.is_empty() => {
            let shell_path: &Path = s.as_ref();
            if let Some(name) = shell_path.file_name() {
                if name == "csh" || name == "tcsh" {
                    OutputFmt::CShell
                } else {
                    OutputFmt::Shell
                }
            } else {
                OutputFmt::Shell
            }
        }
        _ => OutputFmt::Unknown,
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("b", "sh", "output Bourne shell code to set LS_COLORS")
        .optflag("",
                 "bourne-shell",
                 "output Bourne shell code to set LS_COLORS")
        .optflag("c", "csh", "output C shell code to set LS_COLORS")
        .optflag("", "c-shell", "output C shell code to set LS_COLORS")
        .optflag("p", "print-database", "print the byte counts")
        .parse(args);

    if (matches.opt_present("csh") || matches.opt_present("c-shell") ||
        matches.opt_present("sh") || matches.opt_present("bourne-shell")) &&
       matches.opt_present("print-database") {
        disp_err!("the options to output dircolors' internal database and\nto select a shell \
                   syntax are mutually exclusive");
        return 1;
    }

    if matches.opt_present("print-database") {
        if !matches.free.is_empty() {
            disp_err!("extra operand ‘{}’\nfile operands cannot be combined with \
                      --print-database (-p)",
                      matches.free[0]);
            return 1;
        }
        println!("{}", INTERNAL_DB);
        return 0;
    }

    let mut out_format = OutputFmt::Unknown;
    if matches.opt_present("csh") || matches.opt_present("c-shell") {
        out_format = OutputFmt::CShell;
    } else if matches.opt_present("sh") || matches.opt_present("bourne-shell") {
        out_format = OutputFmt::Shell;
    }

    if out_format == OutputFmt::Unknown {
        match guess_syntax() {
            OutputFmt::Unknown => {
                show_info!("no SHELL environment variable, and no shell type option given");
                return 1;
            }
            fmt => out_format = fmt,
        }
    }

    let result;
    if matches.free.is_empty() {
        result = parse(INTERNAL_DB.lines(), out_format, "")
    } else {
        if matches.free.len() > 1 {
            disp_err!("extra operand ‘{}’", matches.free[1]);
            return 1;
        }
        match File::open(matches.free[0].as_str()) {
            Ok(f) => {
                let fin = BufReader::new(f);
                result = parse(fin.lines().filter_map(|l| l.ok()),
                               out_format,
                               matches.free[0].as_str())
            }
            Err(e) => {
                show_info!("{}: {}", matches.free[0], e);
                return 1;
            }
        }
    }
    match result {
        Ok(s) => {
            println!("{}", s);
            0
        }
        Err(s) => {
            show_info!("{}", s);
            1
        }
    }
}

pub trait StrUtils {
    /// Remove comments and trim whitespace
    fn purify(&self) -> &Self;
    /// Like split_whitespace() but only produce 2 components
    fn split_two(&self) -> (&str, &str);
    fn fnmatch(&self, pattern: &str) -> bool;
}

impl StrUtils for str {
    fn purify(&self) -> &Self {
        let mut line = self;
        for (n, c) in self.chars().enumerate() {
            if c != '#' {
                continue;
            }

            // Ignore if '#' is at the beginning of line
            if n == 0 {
                line = &self[..0];
                break;
            }

            // Ignore the content after '#'
            // only if it is preceded by at least one whitespace
            if self.chars().nth(n - 1).unwrap().is_whitespace() {
                line = &self[..n];
            }
        }
        line.trim()
    }

    fn split_two(&self) -> (&str, &str) {
        if let Some(b) = self.find(char::is_whitespace) {
            let key = &self[..b];
            if let Some(e) = self[b..].find(|c: char| !c.is_whitespace()) {
                (key, &self[b + e..])
            } else {
                (key, "")
            }
        } else {
            ("", "")
        }
    }

    fn fnmatch(&self, pat: &str) -> bool {
        pat.parse::<glob::Pattern>().unwrap().matches(self)
    }
}

#[derive(PartialEq)]
enum ParseState {
    Global,
    Matched,
    Continue,
    Pass,
}
use std::collections::HashMap;
fn parse<T>(lines: T, fmt: OutputFmt, fp: &str) -> Result<String, String>
    where T: IntoIterator,
          T::Item: Borrow<str>
{
    // 1440 > $(dircolors | wc -m)
    let mut result = String::with_capacity(1440);
    match fmt {
        OutputFmt::Shell => result.push_str("LS_COLORS='"),
        OutputFmt::CShell => result.push_str("setenv LS_COLORS '"),
        _ => unreachable!(),
    }

    let mut table: HashMap<&str, &str> = HashMap::with_capacity(48);
    table.insert("normal", "no");
    table.insert("norm", "no");
    table.insert("file", "fi");
    table.insert("reset", "rs");
    table.insert("dir", "di");
    table.insert("lnk", "ln");
    table.insert("link", "ln");
    table.insert("symlink", "ln");
    table.insert("orphan", "or");
    table.insert("missing", "mi");
    table.insert("fifo", "pi");
    table.insert("pipe", "pi");
    table.insert("sock", "so");
    table.insert("blk", "bd");
    table.insert("block", "bd");
    table.insert("chr", "cd");
    table.insert("char", "cd");
    table.insert("door", "do");
    table.insert("exec", "ex");
    table.insert("left", "lc");
    table.insert("leftcode", "lc");
    table.insert("right", "rc");
    table.insert("rightcode", "rc");
    table.insert("end", "ec");
    table.insert("endcode", "ec");
    table.insert("suid", "su");
    table.insert("setuid", "su");
    table.insert("sgid", "sg");
    table.insert("setgid", "sg");
    table.insert("sticky", "st");
    table.insert("other_writable", "ow");
    table.insert("owr", "ow");
    table.insert("sticky_other_writable", "tw");
    table.insert("owt", "tw");
    table.insert("capability", "ca");
    table.insert("multihardlink", "mh");
    table.insert("clrtoeol", "cl");

    let term = env::var("TERM").unwrap_or("none".to_owned());
    let term = term.as_str();

    let mut state = ParseState::Global;

    for (num, line) in lines.into_iter().enumerate() {
        let num = num + 1;
        let line = line.borrow().purify();
        if line.is_empty() {
            continue;
        }

        let (key, val) = line.split_two();
        if val.is_empty() {
            return Err(format!("{}:{}: invalid line;  missing second token", fp, num));
        }
        let lower = key.to_lowercase();

        if lower == "term" {
            if term.fnmatch(val) {
                state = ParseState::Matched;
            } else if state != ParseState::Matched {
                state = ParseState::Pass;
            }
        } else {
            if state == ParseState::Matched {
                // prevent subsequent mismatched TERM from
                // cancelling the input
                state = ParseState::Continue;
            }
            if state != ParseState::Pass {
                if key.starts_with(".") {
                    result.push_str(format!("*{}={}:", key, val).as_str());
                } else if key.starts_with("*") {
                    result.push_str(format!("{}={}:", key, val).as_str());
                } else if lower == "options" || lower == "color" || lower == "eightbit" {
                    // Slackware only. Ignore
                } else {
                    if let Some(s) = table.get(lower.as_str()) {
                        result.push_str(format!("{}={}:", s, val).as_str());
                    } else {
                        return Err(format!("{}:{}: unrecognized keyword {}", fp, num, key));
                    }
                }
            }
        }
    }

    match fmt {
        OutputFmt::Shell => result.push_str("';\nexport LS_COLORS"),
        OutputFmt::CShell => result.push('\''),
        _ => unreachable!(),
    }

    Ok(result)
}
