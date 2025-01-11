// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use syntax_tree::AstNode;
use thiserror::Error;
use uucore::{
    error::{UError, UResult},
    format_usage, help_about, help_section, help_usage,
};

use crate::syntax_tree::is_truthy;

mod syntax_tree;

mod options {
    pub const VERSION: &str = "version";
    pub const HELP: &str = "help";
    pub const EXPRESSION: &str = "expression";
}

pub type ExprResult<T> = Result<T, ExprError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExprError {
    #[error("syntax error: unexpected argument '{0}'")]
    UnexpectedArgument(String),

    #[error("syntax error: missing argument after '{0}'")]
    MissingArgument(String),

    #[error("non-integer argument")]
    NonIntegerArgument,

    #[error("missing operand")]
    MissingOperand,

    #[error("division by zero")]
    DivisionByZero,

    #[error("Invalid regex expression")]
    InvalidRegexExpression,

    #[error("syntax error: expecting ')' after '{0}'")]
    ExpectedClosingBraceAfter(String),

    #[error("syntax error: expecting ')' instead of '{0}'")]
    ExpectedClosingBrace(String),

    #[error("{0}")]
    UnmatchedBrace(#[from] BraceType),

    #[error("Invalid content of \\{{\\}}")]
    InvalidBraceContent,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BraceType {
    #[error("Unmatched ( or \\(")]
    OpenParen,

    #[error("Unmatched ) or \\)")]
    CloseParen,

    #[error("Unmatched \\{{")]
    OpenCurly,

    #[error("Unmatched \\}}")]
    CloseCurly,
}

#[derive(Debug, PartialEq)]
enum BraceContent {
    Valid,
    Invalid,
    Unmatched(BraceType),
}

impl UError for ExprError {
    fn code(&self) -> i32 {
        2
    }

    fn usage(&self) -> bool {
        *self == Self::MissingOperand
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(help_about!("expr.md"))
        .override_usage(format_usage(help_usage!("expr.md")))
        .after_help(help_section!("after help", "expr.md"))
        .infer_long_args(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new(options::VERSION)
                .long(options::VERSION)
                .help("output version information and exit")
                .action(ArgAction::Version),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("display this help and exit")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::EXPRESSION)
                .action(ArgAction::Append)
                .allow_hyphen_values(true),
        )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // For expr utility we do not want getopts.
    // The following usage should work without escaping hyphens: `expr -15 = 1 +  2 \* \( 3 - -4 \)`
    let matches = uu_app().try_get_matches_from(args)?;
    let token_strings: Vec<&str> = matches
        .get_many::<String>(options::EXPRESSION)
        .map(|v| v.into_iter().map(|s| s.as_ref()).collect::<Vec<_>>())
        .unwrap_or_default();

    let res: String = AstNode::parse(&token_strings)?.eval()?.eval_as_string();
    println!("{res}");
    if !is_truthy(&res.into()) {
        return Err(1.into());
    }
    Ok(())
}
