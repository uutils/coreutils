// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::io::Write;
use syntax_tree::{AstNode, is_truthy};
use thiserror::Error;
use uucore::os_string_to_vec;
use uucore::translate;
use uucore::{
    display::Quotable,
    error::{UError, UResult},
    format_usage,
};

mod locale_aware;
mod syntax_tree;

mod options {
    pub const VERSION: &str = "version";
    pub const HELP: &str = "help";
    pub const EXPRESSION: &str = "expression";
}

pub type ExprResult<T> = Result<T, ExprError>;

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum ExprError {
    #[error("{}", translate!("expr-error-unexpected-argument", "arg" => _0.quote()))]
    UnexpectedArgument(String),
    #[error("{}", translate!("expr-error-missing-argument", "arg" => _0.quote()))]
    MissingArgument(String),
    #[error("{}", translate!("expr-error-non-integer-argument"))]
    NonIntegerArgument,
    #[error("{}", translate!("expr-error-missing-operand"))]
    MissingOperand,
    #[error("{}", translate!("expr-error-division-by-zero"))]
    DivisionByZero,
    #[error("{}", translate!("expr-error-invalid-regex-expression"))]
    InvalidRegexExpression,
    #[error("{}", translate!("expr-error-expected-closing-brace-after", "arg" => _0.quote()))]
    ExpectedClosingBraceAfter(String),
    #[error("{}", translate!("expr-error-expected-closing-brace-instead-of", "arg" => _0.quote()))]
    ExpectedClosingBraceInsteadOf(String),
    #[error("{}", translate!("expr-error-unmatched-opening-parenthesis"))]
    UnmatchedOpeningParenthesis,
    #[error("{}", translate!("expr-error-unmatched-closing-parenthesis"))]
    UnmatchedClosingParenthesis,
    #[error("{}", translate!("expr-error-unmatched-opening-brace"))]
    UnmatchedOpeningBrace,
    #[error("{}", translate!("expr-error-invalid-bracket-content"))]
    InvalidBracketContent,
    #[error("{}", translate!("expr-error-trailing-backslash"))]
    TrailingBackslash,
    #[error("{}", translate!("expr-error-too-big-range-quantifier-index"))]
    TooBigRangeQuantifierIndex,
    #[error("{}", translate!("expr-error-match-utf8", "arg" => _0.quote()))]
    UnsupportedNonUtf8Match(String),
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
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("expr-about"))
        .override_usage(format_usage(&translate!("expr-usage")))
        .after_help(translate!("expr-after-help"))
        .infer_long_args(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new(options::VERSION)
                .long(options::VERSION)
                .help(translate!("expr-help-version"))
                .action(ArgAction::Version),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("expr-help-help"))
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
    let args = args
        .skip(1) // Skip binary name
        .map(os_string_to_vec)
        .collect::<Result<Vec<_>, _>>()?;

    if args.len() == 1 && args[0] == b"--help" {
        let _ = uu_app().print_help();
    } else if args.len() == 1 && args[0] == b"--version" {
        println!("{} {}", uucore::util_name(), uucore::crate_version!());
    } else {
        // The first argument may be "--" and should be be ignored.
        let args = if !args.is_empty() && args[0] == b"--" {
            &args[1..]
        } else {
            &args
        };

        let res = AstNode::parse(args)?.eval()?.eval_as_string();
        let _ = std::io::stdout().write_all(&res);
        let _ = std::io::stdout().write_all(b"\n");

        if !is_truthy(&res.into()) {
            return Err(1.into());
        }
    }

    Ok(())
}
