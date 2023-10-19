// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_expr::uumain;

use rand::seq::SliceRandom;
use rand::Rng;
use std::{env, ffi::OsString};

mod fuzz_common;
use crate::fuzz_common::{generate_and_run_uumain, run_gnu_cmd};

static CMD_PATH: &str = "expr";

fn generate_random_string(max_length: usize) -> String {
    let mut rng = rand::thread_rng();
    let valid_utf8: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        .chars()
        .collect();
    let invalid_utf8 = [0xC3, 0x28]; // Invalid UTF-8 sequence
    let mut result = String::new();

    for _ in 0..rng.gen_range(1..=max_length) {
        if rng.gen_bool(0.9) {
            let ch = valid_utf8.choose(&mut rng).unwrap();
            result.push(*ch);
        } else {
            let ch = invalid_utf8.choose(&mut rng).unwrap();
            if let Some(c) = char::from_u32(*ch as u32) {
                result.push(c);
            }
        }
    }

    result
}

fn generate_expr(max_depth: u32) -> String {
    let mut rng = rand::thread_rng();
    let ops = ["+", "-", "*", "/", "%", "<", ">", "=", "&", "|"];

    let mut expr = String::new();
    let mut depth = 0;
    let mut last_was_operator = false;

    while depth <= max_depth {
        if last_was_operator || depth == 0 {
            // Add a number
            expr.push_str(&rng.gen_range(1..=100).to_string());
            last_was_operator = false;
        } else {
            // 90% chance to add an operator followed by a number
            if rng.gen_bool(0.9) {
                let op = *ops.choose(&mut rng).unwrap();
                expr.push_str(&format!(" {} ", op));
                last_was_operator = true;
            }
            // 10% chance to add a random string (potentially invalid syntax)
            else {
                let random_str = generate_random_string(rng.gen_range(1..=10));
                expr.push_str(&random_str);
                last_was_operator = false;
            }
        }
        depth += 1;
    }

    // Ensure the expression ends with a number if it ended with an operator
    if last_was_operator {
        expr.push_str(&rng.gen_range(1..=100).to_string());
    }

    expr
}

fuzz_target!(|_data: &[u8]| {
    let mut rng = rand::thread_rng();
    let expr = generate_expr(rng.gen_range(0..=20));
    let mut args = vec![OsString::from("expr")];
    args.extend(expr.split_whitespace().map(OsString::from));

    let (rust_output, uumain_exit_code) = generate_and_run_uumain(&args, uumain);

    // Use C locale to avoid false positives, like in https://github.com/uutils/coreutils/issues/5378,
    // because uutils expr doesn't support localization yet
    // TODO remove once uutils expr supports localization
    env::set_var("LC_COLLATE", "C");

    // Run GNU expr with the provided arguments and compare the output
    match run_gnu_cmd(CMD_PATH, &args[1..], true) {
        Ok((gnu_output, gnu_exit_code)) => {
            let gnu_output = gnu_output.trim().to_owned();
            if uumain_exit_code != gnu_exit_code {
                println!("Expression: {}", expr);
                println!("Rust code: {}", uumain_exit_code);
                println!("GNU code: {}", gnu_exit_code);
                panic!("Different error codes");
            }
            if rust_output == gnu_output {
                println!(
                    "Outputs matched for expression: {} => Result: {}",
                    expr, rust_output
                );
            } else {
                println!("Expression: {}", expr);
                println!("Rust output: {}", rust_output);
                println!("GNU output: {}", gnu_output);
                panic!("Different output between Rust & GNU");
            }
        }
        Err(_) => {
            println!("GNU expr execution failed for expression: {}", expr);
        }
    }
});
