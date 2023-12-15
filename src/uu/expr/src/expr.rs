// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fmt::Display;

use clap::{crate_version, Arg, ArgAction, Command};
use syntax_tree::AstNode;
use uucore::{
    display::Quotable,
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

#[derive(Debug, PartialEq, Eq)]
pub enum ExprError {
    UnexpectedArgument(String),
    MissingArgument(String),
    NonIntegerArgument,
    MissingOperand,
    DivisionByZero,
    InvalidRegexExpression,
    ExpectedClosingBraceAfter(String),
}

impl Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedArgument(s) => {
                write!(f, "syntax error: unexpected argument {}", s.quote())
            }
            Self::MissingArgument(s) => {
                write!(f, "syntax error: missing argument after {}", s.quote())
            }
            Self::NonIntegerArgument => write!(f, "non-integer argument"),
            Self::MissingOperand => write!(f, "missing operand"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::InvalidRegexExpression => write!(f, "Invalid regex expression"),
            Self::ExpectedClosingBraceAfter(s) => {
                write!(f, "expected ')' after {}", s.quote())
            }
        }
    }
}

impl std::error::Error for ExprError {}

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
