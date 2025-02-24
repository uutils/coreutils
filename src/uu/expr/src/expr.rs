// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use syntax_tree::{is_truthy, AstNode};
use thiserror::Error;
use uucore::{
    display::Quotable,
    error::{UError, UResult},
    format_usage, help_about, help_section, help_usage,
};

mod syntax_tree;

mod options {
    pub const VERSION: &str = "version";
    pub const HELP: &str = "help";
    pub const EXPRESSION: &str = "expression";
}

pub type ExprResult<T> = Result<T, ExprError>;

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum ExprError {
    #[error("syntax error: unexpected argument {}", .0.quote())]
    UnexpectedArgument(String),
    #[error("syntax error: missing argument after {}", .0.quote())]
    MissingArgument(String),
    #[error("non-integer argument")]
    NonIntegerArgument,
    #[error("missing operand")]
    MissingOperand,
    #[error("division by zero")]
    DivisionByZero,
    #[error("Invalid regex expression")]
    InvalidRegexExpression,
    #[error("syntax error: expecting ')' after {}", .0.quote())]
    ExpectedClosingBraceAfter(String),
    #[error("syntax error: expecting ')' instead of {}", .0.quote())]
    ExpectedClosingBraceInsteadOf(String),
    #[error("Unmatched ( or \\(")]
    UnmatchedOpeningParenthesis,
    #[error("Unmatched ) or \\)")]
    UnmatchedClosingParenthesis,
    #[error("Unmatched \\{{")]
    UnmatchedOpeningBrace,
    #[error("Unmatched ) or \\}}")]
    UnmatchedClosingBrace,
    #[error("Invalid content of {0}")]
    InvalidContent(String),
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
    let args: Vec<String> = args
        .skip(1) // Skip binary name
        .map(|a| a.to_string_lossy().to_string())
        .collect();

    if args.len() == 1 && args[0] == "--help" {
        let _ = uu_app().print_help();
    } else if args.len() == 1 && args[0] == "--version" {
        println!("{} {}", uucore::util_name(), crate_version!())
    } else {
        // The first argument may be "--" and should be be ignored.
        let args = if !args.is_empty() && args[0] == "--" {
            &args[1..]
        } else {
            &args
        };

        let res: String = AstNode::parse(args)?.eval()?.eval_as_string();
        println!("{res}");
        if !is_truthy(&res.into()) {
            return Err(1.into());
        }
    }

    Ok(())
}
