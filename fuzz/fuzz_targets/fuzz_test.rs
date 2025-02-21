// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore STRINGSTRING INTEGERINTEGER FILEFILE

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_test::uumain;

use rand::prelude::IndexedRandom;
use rand::Rng;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::CommandResult;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Debug, Clone)]
enum ArgType {
    STRING,
    STRINGSTRING,
    INTEGER,
    INTEGERINTEGER,
    FILE,
    FILEFILE,
    // Add any other types as needed
}

static CMD_PATH: &str = "test";

#[derive(Debug, Clone)]
struct TestArg {
    arg: String,
    arg_type: ArgType,
}

fn generate_random_path(rng: &mut dyn rand::RngCore) -> &'static str {
    match rng.random_range(0..=3) {
        0 => "/dev/null",
        1 => "/dev/random",
        2 => "/tmp",
        _ => "/dev/urandom",
    }
}

fn generate_test_args() -> Vec<TestArg> {
    vec![
        TestArg {
            arg: "-z".to_string(),
            arg_type: ArgType::STRING,
        },
        TestArg {
            arg: "-n".to_string(),
            arg_type: ArgType::STRING,
        },
        TestArg {
            arg: "=".to_string(),
            arg_type: ArgType::STRINGSTRING,
        },
        TestArg {
            arg: "!=".to_string(),
            arg_type: ArgType::STRINGSTRING,
        },
        TestArg {
            arg: ">".to_string(),
            arg_type: ArgType::STRINGSTRING,
        },
        TestArg {
            arg: "<".to_string(),
            arg_type: ArgType::STRINGSTRING,
        },
        TestArg {
            arg: "-eq".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-ne".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-gt".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-ge".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-lt".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-le".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-f".to_string(),
            arg_type: ArgType::FILE,
        },
        TestArg {
            arg: "-d".to_string(),
            arg_type: ArgType::FILE,
        },
        TestArg {
            arg: "-e".to_string(),
            arg_type: ArgType::FILE,
        },
        TestArg {
            arg: "-ef".to_string(),
            arg_type: ArgType::FILEFILE,
        },
        TestArg {
            arg: "-nt".to_string(),
            arg_type: ArgType::FILEFILE,
        },
    ]
}

fn generate_test_arg() -> String {
    let mut rng = rand::rng();
    let test_args = generate_test_args();
    let mut arg = String::new();

    let choice = rng.random_range(0..=5);

    match choice {
        0 => {
            arg.push_str(&rng.random_range(-100..=100).to_string());
        }
        1..=3 => {
            let test_arg = test_args
                .choose(&mut rng)
                .expect("Failed to choose a random test argument");
            if test_arg.arg_type == ArgType::INTEGER {
                arg.push_str(&format!(
                    "{} {} {}",
                    &rng.random_range(-100..=100).to_string(),
                    test_arg.arg,
                    &rng.random_range(-100..=100).to_string()
                ));
            } else if test_arg.arg_type == ArgType::STRINGSTRING {
                let random_str = generate_random_string(rng.random_range(1..=10));
                let random_str2 = generate_random_string(rng.random_range(1..=10));

                arg.push_str(&format!(
                    "{} {} {}",
                    &random_str, test_arg.arg, &random_str2
                ));
            } else if test_arg.arg_type == ArgType::STRING {
                let random_str = generate_random_string(rng.random_range(1..=10));
                arg.push_str(&format!("{} {}", test_arg.arg, &random_str));
            } else if test_arg.arg_type == ArgType::FILEFILE {
                let path = generate_random_path(&mut rng);
                let path2 = generate_random_path(&mut rng);
                arg.push_str(&format!("{} {} {}", path, test_arg.arg, path2));
            } else if test_arg.arg_type == ArgType::FILE {
                let path = generate_random_path(&mut rng);
                arg.push_str(&format!("{} {}", test_arg.arg, path));
            }
        }
        4 => {
            let random_str = generate_random_string(rng.random_range(1..=10));
            arg.push_str(&random_str);
        }
        _ => {
            let path = generate_random_path(&mut rng);

            let file_test_args: Vec<TestArg> = test_args
                .iter()
                .filter(|ta| ta.arg_type == ArgType::FILE)
                .cloned()
                .collect();

            if let Some(test_arg) = file_test_args.choose(&mut rng) {
                arg.push_str(&format!("{}{}", test_arg.arg, path));
            }
        }
    }

    arg
}

fuzz_target!(|_data: &[u8]| {
    let mut rng = rand::rng();
    let max_args = rng.random_range(1..=6);
    let mut args = vec![OsString::from("test")];

    for _ in 0..max_args {
        args.push(OsString::from(generate_test_arg()));
    }

    let rust_result = generate_and_run_uumain(&args, uumain, None);

    let gnu_result = match run_gnu_cmd(CMD_PATH, &args[1..], false, None) {
        Ok(result) => result,
        Err(error_result) => {
            eprintln!("Failed to run GNU command:");
            eprintln!("Stderr: {}", error_result.stderr);
            eprintln!("Exit Code: {}", error_result.exit_code);
            CommandResult {
                stdout: String::new(),
                stderr: error_result.stderr,
                exit_code: error_result.exit_code,
            }
        }
    };

    compare_result(
        "test",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        false, // Set to true if you want to fail on stderr diff
    );
});
