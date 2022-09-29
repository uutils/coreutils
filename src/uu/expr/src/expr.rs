//* This file is part of the uutils coreutils package.
//*
//* (c) Roman Gafiyatullin <r.gafiyatullin@me.com>
//*
//* For the full copyright and license information, please view the LICENSE
//* file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{
    error::{UResult, USimpleError},
    format_usage, help_section, help_usage,
};

mod syntax_tree;
mod tokens;

mod options {
    pub const VERSION: &str = "version";
    pub const HELP: &str = "help";
    pub const EXPRESSION: &str = "expression";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(help_section!("about", "expr.md"))
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
    let args = args.collect_lossy();

    // For expr utility we do not want getopts.
    // The following usage should work without escaping hyphens: `expr -15 = 1 +  2 \* \( 3 - -4 \)`
    let matches = uu_app().try_get_matches_from(args)?;
    let token_strings = matches
        .get_many::<String>(options::EXPRESSION)
        .map(|v| v.into_iter().map(|s| s.as_ref()).collect::<Vec<_>>())
        .unwrap_or_default();

    match process_expr(&token_strings[..]) {
        Ok(expr_result) => print_expr_ok(&expr_result),
        Err(expr_error) => Err(USimpleError::new(2, &expr_error)),
    }
}

fn process_expr(token_strings: &[&str]) -> Result<String, String> {
    let maybe_tokens = tokens::strings_to_tokens(token_strings);
    let maybe_ast = syntax_tree::tokens_to_ast(maybe_tokens);
    evaluate_ast(maybe_ast)
}

fn print_expr_ok(expr_result: &str) -> UResult<()> {
    println!("{}", expr_result);
    if expr_result == "0" || expr_result.is_empty() {
        Err(1.into())
    } else {
        Ok(())
    }
}

fn evaluate_ast(maybe_ast: Result<Box<syntax_tree::AstNode>, String>) -> Result<String, String> {
    maybe_ast.and_then(|ast| ast.evaluate())
}
