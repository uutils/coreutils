// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::collections::HashMap;
use syntax_tree::{AstNode, is_truthy};
use thiserror::Error;
use uucore::locale::{get_message, get_message_with_args};
use uucore::{
    display::Quotable,
    error::{UError, UResult},
    format_usage,
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
    #[error("{}", get_message_with_args("expr-error-unexpected-argument", HashMap::from([("arg".to_string(), _0.quote().to_string())])))]
    UnexpectedArgument(String),
    #[error("{}", get_message_with_args("expr-error-missing-argument", HashMap::from([("arg".to_string(), _0.quote().to_string())])))]
    MissingArgument(String),
    #[error("{}", get_message("expr-error-non-integer-argument"))]
    NonIntegerArgument,
    #[error("{}", get_message("expr-error-missing-operand"))]
    MissingOperand,
    #[error("{}", get_message("expr-error-division-by-zero"))]
    DivisionByZero,
    #[error("{}", get_message("expr-error-invalid-regex-expression"))]
    InvalidRegexExpression,
    #[error("{}", get_message_with_args("expr-error-expected-closing-brace-after", HashMap::from([("arg".to_string(), _0.quote().to_string())])))]
    ExpectedClosingBraceAfter(String),
    #[error("{}", get_message_with_args("expr-error-expected-closing-brace-instead-of", HashMap::from([("arg".to_string(), _0.quote().to_string())])))]
    ExpectedClosingBraceInsteadOf(String),
    #[error("{}", get_message("expr-error-unmatched-opening-parenthesis"))]
    UnmatchedOpeningParenthesis,
    #[error("{}", get_message("expr-error-unmatched-closing-parenthesis"))]
    UnmatchedClosingParenthesis,
    #[error("{}", get_message("expr-error-unmatched-opening-brace"))]
    UnmatchedOpeningBrace,
    #[error("{}", get_message("expr-error-invalid-bracket-content"))]
    InvalidBracketContent,
    #[error("{}", get_message("expr-error-trailing-backslash"))]
    TrailingBackslash,
    #[error("{}", get_message("expr-error-too-big-range-quantifier-index"))]
    TooBigRangeQuantifierIndex,
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
        .version(uucore::crate_version!())
        .about(get_message("expr-about"))
        .override_usage(format_usage(&get_message("expr-usage")))
        .after_help(get_message("expr-after-help"))
        .infer_long_args(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new(options::VERSION)
                .long(options::VERSION)
                .help(get_message("expr-help-version"))
                .action(ArgAction::Version),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(get_message("expr-help-help"))
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
    // The following usage should work without escaping hyphens: `expr -15 = 1 + 2 \* \( 3 - -4 \)`
    let args: Vec<String> = args
        .skip(1) // Skip binary name
        .map(|a| a.to_string_lossy().to_string())
        .collect();

    if args.len() == 1 && args[0] == "--help" {
        let _ = uu_app().print_help();
    } else if args.len() == 1 && args[0] == "--version" {
        println!("{} {}", uucore::util_name(), uucore::crate_version!());
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
