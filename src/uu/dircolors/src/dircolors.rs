// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
// (c) Mitchell Mebane <mitchell.mebane@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) clrtoeol dircolors eightbit endcode fnmatch leftcode multihardlink rightcode setenv sgid suid

use std::borrow::Borrow;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{crate_version, Arg, Command};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};

mod options {
    pub const BOURNE_SHELL: &str = "bourne-shell";
    pub const C_SHELL: &str = "c-shell";
    pub const PRINT_DATABASE: &str = "print-database";
    pub const FILE: &str = "FILE";
}

static USAGE: &str = "{} [OPTION]... [FILE]";
static SUMMARY: &str = "Output commands to set the LS_COLORS environment variable.";
static LONG_HELP: &str = "
 If FILE is specified, read it to determine which colors to use for which
 file types and extensions.  Otherwise, a precompiled database is used.
 For details on the format of these files, run 'dircolors --print-database'
";

mod colors;
use self::colors::INTERNAL_DB;

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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(&args);

    let files = matches
        .values_of(options::FILE)
        .map_or(vec![], |file_values| file_values.collect());

    // clap provides .conflicts_with / .conflicts_with_all, but we want to
    // manually handle conflicts so we can match the output of GNU coreutils
    if (matches.is_present(options::C_SHELL) || matches.is_present(options::BOURNE_SHELL))
        && matches.is_present(options::PRINT_DATABASE)
    {
        return Err(UUsageError::new(
            1,
            "the options to output dircolors' internal database and\nto select a shell \
             syntax are mutually exclusive",
        ));
    }

    if matches.is_present(options::PRINT_DATABASE) {
        if !files.is_empty() {
            return Err(UUsageError::new(
                1,
                format!(
                    "extra operand {}\nfile operands cannot be combined with \
                     --print-database (-p)",
                    files[0].quote()
                ),
            ));
        }
        println!("{}", INTERNAL_DB);
        return Ok(());
    }

    let mut out_format = OutputFmt::Unknown;
    if matches.is_present(options::C_SHELL) {
        out_format = OutputFmt::CShell;
    } else if matches.is_present(options::BOURNE_SHELL) {
        out_format = OutputFmt::Shell;
    }

    if out_format == OutputFmt::Unknown {
        match guess_syntax() {
            OutputFmt::Unknown => {
                return Err(USimpleError::new(
                    1,
                    "no SHELL environment variable, and no shell type option given",
                ));
            }
            fmt => out_format = fmt,
        }
    }

    let result;
    if files.is_empty() {
        result = parse(INTERNAL_DB.lines(), &out_format, "");
    } else {
        if files.len() > 1 {
            return Err(UUsageError::new(
                1,
                format!("extra operand {}", files[1].quote()),
            ));
        }
        match File::open(files[0]) {
            Ok(f) => {
                let fin = BufReader::new(f);
                result = parse(fin.lines().filter_map(Result::ok), &out_format, files[0]);
            }
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    format!("{}: {}", files[0].maybe_quote(), e),
                ));
            }
        }
    }

    match result {
        Ok(s) => {
            println!("{}", s);
            Ok(())
        }
        Err(s) => {
            return Err(USimpleError::new(1, s));
        }
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BOURNE_SHELL)
                .long("sh")
                .short('b')
                .visible_alias("bourne-shell")
                .help("output Bourne shell code to set LS_COLORS")
                .display_order(1),
        )
        .arg(
            Arg::new(options::C_SHELL)
                .long("csh")
                .short('c')
                .visible_alias("c-shell")
                .help("output C shell code to set LS_COLORS")
                .display_order(2),
        )
        .arg(
            Arg::new(options::PRINT_DATABASE)
                .long("print-database")
                .short('p')
                .help("print the byte counts")
                .display_order(3),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .multiple_occurrences(true),
        )
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
        for (n, _) in self
            .as_bytes()
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == b'#')
        {
            // Ignore the content after '#'
            // only if it is preceded by at least one whitespace
            match self[..n].chars().last() {
                Some(c) if c.is_whitespace() => {
                    line = &self[..n - c.len_utf8()];
                    break;
                }
                None => {
                    // n == 0
                    line = &self[..0];
                    break;
                }
                _ => (),
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
use uucore::{format_usage, InvalidEncodingHandling};

fn parse<T>(lines: T, fmt: &OutputFmt, fp: &str) -> Result<String, String>
where
    T: IntoIterator,
    T::Item: Borrow<str>,
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

    let term = env::var("TERM").unwrap_or_else(|_| "none".to_owned());
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
            return Err(format!(
                "{}:{}: invalid line;  missing second token",
                fp.maybe_quote(),
                num
            ));
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
                if key.starts_with('.') {
                    result.push_str(format!("*{}={}:", key, val).as_str());
                } else if key.starts_with('*') {
                    result.push_str(format!("{}={}:", key, val).as_str());
                } else if lower == "options" || lower == "color" || lower == "eightbit" {
                    // Slackware only. Ignore
                } else if let Some(s) = table.get(lower.as_str()) {
                    result.push_str(format!("{}={}:", s, val).as_str());
                } else {
                    return Err(format!(
                        "{}:{}: unrecognized keyword {}",
                        fp.maybe_quote(),
                        num,
                        key
                    ));
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
