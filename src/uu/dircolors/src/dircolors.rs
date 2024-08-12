// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) clrtoeol dircolors eightbit endcode fnmatch leftcode multihardlink rightcode setenv sgid suid colorterm

use std::borrow::Borrow;
use std::env;
use std::fs::File;
//use std::io::IsTerminal;
use std::io::{BufRead, BufReader};
use std::path::Path;

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::colors::{FILE_ATTRIBUTE_CODES, FILE_COLORS, FILE_TYPES, TERMS};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{help_about, help_section, help_usage};

mod options {
    pub const BOURNE_SHELL: &str = "bourne-shell";
    pub const C_SHELL: &str = "c-shell";
    pub const PRINT_DATABASE: &str = "print-database";
    pub const PRINT_LS_COLORS: &str = "print-ls-colors";
    pub const FILE: &str = "FILE";
}

const USAGE: &str = help_usage!("dircolors.md");
const ABOUT: &str = help_about!("dircolors.md");
const AFTER_HELP: &str = help_section!("after help", "dircolors.md");

#[derive(PartialEq, Eq, Debug)]
pub enum OutputFmt {
    Shell,
    CShell,
    Display,
    Unknown,
}

pub fn guess_syntax() -> OutputFmt {
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

fn get_colors_format_strings(fmt: &OutputFmt) -> (String, String) {
    let prefix = match fmt {
        OutputFmt::Shell => "LS_COLORS='".to_string(),
        OutputFmt::CShell => "setenv LS_COLORS '".to_string(),
        OutputFmt::Display => String::new(),
        OutputFmt::Unknown => unreachable!(),
    };

    let suffix = match fmt {
        OutputFmt::Shell => "';\nexport LS_COLORS".to_string(),
        OutputFmt::CShell => "'".to_string(),
        OutputFmt::Display => String::new(),
        OutputFmt::Unknown => unreachable!(),
    };

    (prefix, suffix)
}

pub fn generate_type_output(fmt: &OutputFmt) -> String {
    match fmt {
        OutputFmt::Display => FILE_TYPES
            .iter()
            .map(|&(_, key, val)| format!("\x1b[{}m{}\t{}\x1b[0m", val, key, val))
            .collect::<Vec<String>>()
            .join("\n"),
        _ => {
            // Existing logic for other formats
            FILE_TYPES
                .iter()
                .map(|&(_, v1, v2)| format!("{}={}", v1, v2))
                .collect::<Vec<String>>()
                .join(":")
        }
    }
}

fn generate_ls_colors(fmt: &OutputFmt, sep: &str) -> String {
    match fmt {
        OutputFmt::Display => {
            let mut display_parts = vec![];
            let type_output = generate_type_output(fmt);
            display_parts.push(type_output);
            for &(extension, code) in FILE_COLORS {
                let prefix = if extension.starts_with('*') { "" } else { "*" };
                let formatted_extension =
                    format!("\x1b[{}m{}{}\t{}\x1b[0m", code, prefix, extension, code);
                display_parts.push(formatted_extension);
            }
            display_parts.join("\n")
        }
        _ => {
            // existing logic for other formats
            let mut parts = vec![];
            for &(extension, code) in FILE_COLORS {
                let prefix = if extension.starts_with('*') { "" } else { "*" };
                let formatted_extension = format!("{}{}", prefix, extension);
                parts.push(format!("{}={}", formatted_extension, code));
            }
            let (prefix, suffix) = get_colors_format_strings(fmt);
            let ls_colors = parts.join(sep);
            format!(
                "{}{}:{}:{}",
                prefix,
                generate_type_output(fmt),
                ls_colors,
                suffix
            )
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files = matches
        .get_many::<String>(options::FILE)
        .map_or(vec![], |file_values| file_values.collect());

    // clap provides .conflicts_with / .conflicts_with_all, but we want to
    // manually handle conflicts so we can match the output of GNU coreutils
    if (matches.get_flag(options::C_SHELL) || matches.get_flag(options::BOURNE_SHELL))
        && (matches.get_flag(options::PRINT_DATABASE) || matches.get_flag(options::PRINT_LS_COLORS))
    {
        return Err(UUsageError::new(
            1,
            "the options to output non shell syntax,\n\
             and to select a shell syntax are mutually exclusive",
        ));
    }

    if matches.get_flag(options::PRINT_DATABASE) && matches.get_flag(options::PRINT_LS_COLORS) {
        return Err(UUsageError::new(
            1,
            "options --print-database and --print-ls-colors are mutually exclusive",
        ));
    }

    if matches.get_flag(options::PRINT_DATABASE) {
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

        println!("{}", generate_dircolors_config());
        return Ok(());
    }

    let mut out_format = if matches.get_flag(options::C_SHELL) {
        OutputFmt::CShell
    } else if matches.get_flag(options::BOURNE_SHELL) {
        OutputFmt::Shell
    } else if matches.get_flag(options::PRINT_LS_COLORS) {
        OutputFmt::Display
    } else {
        OutputFmt::Unknown
    };

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
        println!("{}", generate_ls_colors(&out_format, ":"));
        return Ok(());
        /*
        // Check if data is being piped into the program
        if std::io::stdin().is_terminal() {
            // No data piped, use default behavior
            println!("{}", generate_ls_colors(&out_format, ":"));
            return Ok(());
        } else {
            // Data is piped, process the input from stdin
            let fin = BufReader::new(std::io::stdin());
            result = parse(fin.lines().map_while(Result::ok), &out_format, "-");
        }
         */
    } else if files.len() > 1 {
        return Err(UUsageError::new(
            1,
            format!("extra operand {}", files[1].quote()),
        ));
    } else if files[0].eq("-") {
        let fin = BufReader::new(std::io::stdin());
        // For example, for echo "owt 40;33"|dircolors -b -
        result = parse(fin.lines().map_while(Result::ok), &out_format, files[0]);
    } else {
        let path = Path::new(files[0]);
        if path.is_dir() {
            return Err(USimpleError::new(
                2,
                format!("expected file, got directory {}", path.quote()),
            ));
        }
        match File::open(path) {
            Ok(f) => {
                let fin = BufReader::new(f);
                result = parse(
                    fin.lines().map_while(Result::ok),
                    &out_format,
                    &path.to_string_lossy(),
                );
            }
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    format!("{}: {}", path.maybe_quote(), e),
                ));
            }
        }
    }

    match result {
        Ok(s) => {
            println!("{s}");
            Ok(())
        }
        Err(s) => Err(USimpleError::new(1, s)),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BOURNE_SHELL)
                .long("sh")
                .short('b')
                .visible_alias("bourne-shell")
                .overrides_with(options::C_SHELL)
                .help("output Bourne shell code to set LS_COLORS")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::C_SHELL)
                .long("csh")
                .short('c')
                .visible_alias("c-shell")
                .overrides_with(options::BOURNE_SHELL)
                .help("output C shell code to set LS_COLORS")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRINT_DATABASE)
                .long("print-database")
                .short('p')
                .help("print the byte counts")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRINT_LS_COLORS)
                .long("print-ls-colors")
                .help("output fully escaped colors for display")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath)
                .action(ArgAction::Append),
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
        parse_glob::from_str(pat).unwrap().matches(self)
    }
}

#[derive(PartialEq)]
enum ParseState {
    Global,
    Matched,
    Continue,
    Pass,
}

use uucore::{format_usage, parse_glob};

#[allow(clippy::cognitive_complexity)]
fn parse<T>(user_input: T, fmt: &OutputFmt, fp: &str) -> Result<String, String>
where
    T: IntoIterator,
    T::Item: Borrow<str>,
{
    let mut result = String::with_capacity(1790);
    let (prefix, suffix) = get_colors_format_strings(fmt);

    result.push_str(&prefix);

    let term = env::var("TERM").unwrap_or_else(|_| "none".to_owned());
    let term = term.as_str();

    let mut state = ParseState::Global;

    for (num, line) in user_input.into_iter().enumerate() {
        let num = num + 1;
        let line = line.borrow().purify();
        if line.is_empty() {
            continue;
        }

        let line = escape(line);

        let (key, val) = line.split_two();
        if val.is_empty() {
            return Err(format!(
                // The double space is what GNU is doing
                "{}:{}: invalid line;  missing second token",
                fp.maybe_quote(),
                num
            ));
        }
        let lower = key.to_lowercase();
        if lower == "term" || lower == "colorterm" {
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
                let search_key = lower.as_str();

                if key.starts_with('.') {
                    if *fmt == OutputFmt::Display {
                        result.push_str(format!("\x1b[{val}m*{key}\t{val}\x1b[0m\n").as_str());
                    } else {
                        result.push_str(format!("*{key}={val}:").as_str());
                    }
                } else if key.starts_with('*') {
                    if *fmt == OutputFmt::Display {
                        result.push_str(format!("\x1b[{val}m{key}\t{val}\x1b[0m\n").as_str());
                    } else {
                        result.push_str(format!("{key}={val}:").as_str());
                    }
                } else if lower == "options" || lower == "color" || lower == "eightbit" {
                    // Slackware only. Ignore
                } else if let Some((_, s)) = FILE_ATTRIBUTE_CODES
                    .iter()
                    .find(|&&(key, _)| key == search_key)
                {
                    if *fmt == OutputFmt::Display {
                        result.push_str(format!("\x1b[{val}m{s}\t{val}\x1b[0m\n").as_str());
                    } else {
                        result.push_str(format!("{s}={val}:").as_str());
                    }
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

    if fmt == &OutputFmt::Display {
        // remove latest "\n"
        result.pop();
    }
    result.push_str(&suffix);

    Ok(result)
}

/// Escape single quotes because they are not allowed between single quotes in shell code, and code
/// enclosed by single quotes is what is returned by `parse()`.
///
/// We also escape ":" to make the "quote" test pass in the GNU test suite:
/// <https://github.com/coreutils/coreutils/blob/master/tests/misc/dircolors.pl>
fn escape(s: &str) -> String {
    let mut result = String::new();
    let mut previous = ' ';

    for c in s.chars() {
        match c {
            '\'' => result.push_str("'\\''"),
            ':' if previous != '\\' => result.push_str("\\:"),
            _ => result.push(c),
        }
        previous = c;
    }

    result
}

pub fn generate_dircolors_config() -> String {
    let mut config = String::new();

    config.push_str(
        "\
         # Configuration file for dircolors, a utility to help you set the\n\
         # LS_COLORS environment variable used by GNU ls with the --color option.\n\
         # The keywords COLOR, OPTIONS, and EIGHTBIT (honored by the\n\
         # slackware version of dircolors) are recognized but ignored.\n\
         # Global config options can be specified before TERM or COLORTERM entries\n\
         # Below are TERM or COLORTERM entries, which can be glob patterns, which\n\
         # restrict following config to systems with matching environment variables.\n\
        ",
    );
    config.push_str("COLORTERM ?*\n");
    for term in TERMS {
        config.push_str(&format!("TERM {}\n", term));
    }

    config.push_str(
        "\
        # Below are the color init strings for the basic file types.\n\
        # One can use codes for 256 or more colors supported by modern terminals.\n\
        # The default color codes use the capabilities of an 8 color terminal\n\
        # with some additional attributes as per the following codes:\n\
        # Attribute codes:\n\
        # 00=none 01=bold 04=underscore 05=blink 07=reverse 08=concealed\n\
        # Text color codes:\n\
        # 30=black 31=red 32=green 33=yellow 34=blue 35=magenta 36=cyan 37=white\n\
        # Background color codes:\n\
        # 40=black 41=red 42=green 43=yellow 44=blue 45=magenta 46=cyan 47=white\n\
        #NORMAL 00 # no color code at all\n\
        #FILE 00 # regular file: use no color at all\n\
        ",
    );

    for (name, _, code) in FILE_TYPES {
        config.push_str(&format!("{} {}\n", name, code));
    }

    config.push_str("# List any file extensions like '.gz' or '.tar' that you would like ls\n");
    config.push_str("# to color below. Put the extension, a space, and the color init string.\n");

    for (ext, color) in FILE_COLORS {
        config.push_str(&format!("{} {}\n", ext, color));
    }
    config.push_str("# Subsequent TERM or COLORTERM entries, can be used to add / override\n");
    config.push_str("# config specific to those matching environment variables.");

    config
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn test_escape() {
        assert_eq!("", escape(""));
        assert_eq!("'\\''", escape("'"));
        assert_eq!("\\:", escape(":"));
        assert_eq!("\\:", escape("\\:"));
    }
}
