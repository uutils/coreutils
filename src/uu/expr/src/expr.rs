// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::io::{Write, stdout};
use syntax_tree::{AstNode, is_truthy};
use thiserror::Error;
use uucore::os_string_to_vec;
use uucore::translate;
use uucore::{
    display::Quotable,
    error::{UError, UResult},
    format_usage,
};

mod diagnostics;
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
    // The offending operand is carried for diagnostics only; GNU prints the bare
    // message, so it is deliberately absent from `Display`. Raw bytes, so that a
    // non-UTF-8 operand can still be matched back to its argument.
    #[error("{}", translate!("expr-error-non-integer-argument"))]
    NonIntegerArgument(Vec<u8>),
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
    Command::new("expr")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("expr"))
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

/// Where an expression failed, in terms a diagnostic can point at.
pub enum FailurePoint {
    /// The parser failed after consuming this many arguments.
    Parse(usize),
    /// Evaluation failed, at this argument when one is to blame.
    Eval(Option<usize>),
}

/// Parse and evaluate the expression.
fn evaluate(args: &[Vec<u8>]) -> Result<Vec<u8>, (ExprError, FailurePoint)> {
    let ast = AstNode::parse_located(args).map_err(|(e, at)| (e, FailurePoint::Parse(at)))?;
    let value = ast
        .eval_located()
        .map_err(|(e, at)| (e, FailurePoint::Eval(at)))?;
    Ok(value.eval_as_string())
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // For expr utility we do not want getopts.
    // The following usage should work without escaping hyphens: `expr -15 = 1 + 2 \* \( 3 - -4 \)`
    let args = args
        .skip(1) // Skip binary name
        .map(os_string_to_vec)
        .collect::<Result<Vec<_>, _>>()?;

    let mut args = &args[..];
    match args {
        [a] if a == b"--help" => uu_app().print_help()?,
        [a] if a == b"--version" => writeln!(stdout(), "expr {}", uucore::crate_version!())?,
        _ => {
            // ignore -- as the 1st argument
            if let [a, rest @ ..] = args
                && a == b"--"
            {
                args = rest;
            }

            let res = evaluate(args).map_err(|(e, at)| {
                let reported = uucore::diagnostics::enabled() && diagnostics::render(args, &e, &at);
                uucore::error::quiet_if_reported(reported, e)
            })?;
            stdout().write_all(&res)?;
            stdout().write_all(b"\n")?;
            if !is_truthy(&res.into()) {
                return Err(1.into());
            }
        }
    }

    Ok(())
}
