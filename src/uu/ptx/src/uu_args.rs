// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const USAGE: &str = help_usage!("ptx.md");
const ABOUT: &str = help_about!("ptx.md");

pub mod options {
    pub static FILE: &str = "file";
    pub static AUTO_REFERENCE: &str = "auto-reference";
    pub static TRADITIONAL: &str = "traditional";
    pub static FLAG_TRUNCATION: &str = "flag-truncation";
    pub static MACRO_NAME: &str = "macro-name";
    pub static FORMAT_ROFF: &str = "format=roff";
    pub static RIGHT_SIDE_REFS: &str = "right-side-refs";
    pub static SENTENCE_REGEXP: &str = "sentence-regexp";
    pub static FORMAT_TEX: &str = "format=tex";
    pub static WORD_REGEXP: &str = "word-regexp";
    pub static BREAK_FILE: &str = "break-file";
    pub static IGNORE_CASE: &str = "ignore-case";
    pub static GAP_SIZE: &str = "gap-size";
    pub static IGNORE_FILE: &str = "ignore-file";
    pub static ONLY_FILE: &str = "only-file";
    pub static REFERENCES: &str = "references";
    pub static WIDTH: &str = "width";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::AUTO_REFERENCE)
                .short('A')
                .long(options::AUTO_REFERENCE)
                .help("output automatically generated references")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .short('G')
                .long(options::TRADITIONAL)
                .help("behave more like System V 'ptx'")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FLAG_TRUNCATION)
                .short('F')
                .long(options::FLAG_TRUNCATION)
                .help("use STRING for flagging line truncations")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::MACRO_NAME)
                .short('M')
                .long(options::MACRO_NAME)
                .help("macro name to use instead of 'xx'")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::FORMAT_ROFF)
                .short('O')
                .long(options::FORMAT_ROFF)
                .help("generate output as roff directives")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RIGHT_SIDE_REFS)
                .short('R')
                .long(options::RIGHT_SIDE_REFS)
                .help("put references at right, not counted in -w")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SENTENCE_REGEXP)
                .short('S')
                .long(options::SENTENCE_REGEXP)
                .help("for end of lines or end of sentences")
                .value_name("REGEXP"),
        )
        .arg(
            Arg::new(options::FORMAT_TEX)
                .short('T')
                .long(options::FORMAT_TEX)
                .help("generate output as TeX directives")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WORD_REGEXP)
                .short('W')
                .long(options::WORD_REGEXP)
                .help("use REGEXP to match each keyword")
                .value_name("REGEXP"),
        )
        .arg(
            Arg::new(options::BREAK_FILE)
                .short('b')
                .long(options::BREAK_FILE)
                .help("word break characters in this FILE")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('f')
                .long(options::IGNORE_CASE)
                .help("fold lower case to upper case for sorting")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::GAP_SIZE)
                .short('g')
                .long(options::GAP_SIZE)
                .help("gap size in columns between output fields")
                .value_name("NUMBER"),
        )
        .arg(
            Arg::new(options::IGNORE_FILE)
                .short('i')
                .long(options::IGNORE_FILE)
                .help("read ignore word list from FILE")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ONLY_FILE)
                .short('o')
                .long(options::ONLY_FILE)
                .help("read only word list from this FILE")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::REFERENCES)
                .short('r')
                .long(options::REFERENCES)
                .help("first field of each line is a reference")
                .value_name("FILE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help("output width in columns, reference excluded")
                .value_name("NUMBER"),
        )
}
