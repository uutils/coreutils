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
use std::ffi::OsString;

use libc::{dup, dup2, STDOUT_FILENO};
use std::process::Command;
mod fuzz_common;
use crate::fuzz_common::is_gnu_cmd;

static CMD_PATH: &str = "expr";

fn run_gnu_expr(args: &[OsString]) -> Result<(String, i32), std::io::Error> {
    is_gnu_cmd(CMD_PATH)?; // Check if it's a GNU implementation

    let mut command = Command::new(CMD_PATH);
    for arg in args {
        command.arg(arg);
    }
    let output = command.output()?;
    let exit_code = output.status.code().unwrap_or(-1);
    if output.status.success() {
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code,
        ))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("GNU expr execution failed with exit code {}", exit_code),
        ))
    }
}

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

    // Save the original stdout file descriptor
    let original_stdout_fd = unsafe { dup(STDOUT_FILENO) };

    // Create a pipe to capture stdout
    let mut pipe_fds = [-1; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    let uumain_exit_code;
    {
        // Redirect stdout to the write end of the pipe
        unsafe { dup2(pipe_fds[1], STDOUT_FILENO) };

        // Run uumain with the provided arguments
        uumain_exit_code = uumain(args.clone().into_iter());

        // Restore original stdout
        unsafe { dup2(original_stdout_fd, STDOUT_FILENO) };
        unsafe { libc::close(original_stdout_fd) };
    }
    // Close the write end of the pipe
    unsafe { libc::close(pipe_fds[1]) };

    // Read captured output from the read end of the pipe
    let mut captured_output = Vec::new();
    let mut read_buffer = [0; 1024];
    loop {
        let bytes_read = unsafe {
            libc::read(
                pipe_fds[0],
                read_buffer.as_mut_ptr() as *mut libc::c_void,
                read_buffer.len(),
            )
        };
        if bytes_read <= 0 {
            break;
        }
        captured_output.extend_from_slice(&read_buffer[..bytes_read as usize]);
    }

    // Close the read end of the pipe
    unsafe { libc::close(pipe_fds[0]) };

    // Convert captured output to a string
    let rust_output = String::from_utf8_lossy(&captured_output)
        .to_string()
        .trim()
        .to_owned();

    // Run GNU expr with the provided arguments and compare the output
    match run_gnu_expr(&args[1..]) {
        Ok((gnu_output, gnu_exit_code)) => {
            let gnu_output = gnu_output.trim().to_owned();
            if uumain_exit_code != gnu_exit_code {
                println!("Expression: {}", expr);
                println!("Rust code: {}", uumain_exit_code);
                println!("GNU code: {}", gnu_exit_code);
                panic!("Different error codes");
            }
            if rust_output != gnu_output {
                println!("Expression: {}", expr);
                println!("Rust output: {}", rust_output);
                println!("GNU output: {}", gnu_output);
                panic!("Different output between Rust & GNU");
            } else {
                println!(
                    "Outputs matched for expression: {} => Result: {}",
                    expr, rust_output
                );
            }
        }
        Err(_) => {
            println!("GNU expr execution failed for expression: {}", expr);
        }
    }
});
