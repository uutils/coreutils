#![crate_name = "expr"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Roman Gafiyatullin <r.gafiyatullin@me.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[path="../common/util.rs"]
#[macro_use]
mod util;
mod tokens;
mod syntax_tree;

use std::io::{Write};

static NAME: &'static str = "expr";
// // FIXME: the following line is going to be needed once we can use getopts with this utility.
// static VERSION: &'static str = "0.0.1";

pub fn uumain(args: Vec<String>) -> i32 {
	// For expr utility we do not want getopts.
	// The following usage should work without escaping hyphens: `expr -15 = 1 +  2 \* \( 3 - -4 \)`

	let token_strings = args[1..].to_vec();

	match process_expr( &token_strings ) {
		Ok( expr_result ) => print_expr_ok( &expr_result ),
		Err( expr_error ) => print_expr_error( &expr_error )
	}
}

fn process_expr( token_strings: &Vec<String> ) -> Result< String, String > {
	let tokens = tokens::string_vec_to_tokens( &token_strings );
	let ast = syntax_tree::tokens_to_ast( &tokens );
	Result::Ok( ast.str_value() )
}

fn print_expr_ok( expr_result: &String ) -> i32 {
	println!("{}", expr_result);
	0
}

fn print_expr_error( expr_error: &String ) -> ! {
	crash!(2, "expression evaluation error\n\t{}", expr_error)
}
