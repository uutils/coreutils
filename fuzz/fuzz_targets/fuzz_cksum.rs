// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore chdir

#![no_main]
use libfuzzer_sys::fuzz_target;
use rand::Rng;
use std::env::temp_dir;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use uu_cksum::uumain;
use uufuzz::{
    CommandResult, compare_result, generate_and_run_uumain, generate_random_file,
    generate_random_string,
    pretty_print::{print_or_empty, print_test_begin},
    replace_fuzz_binary_name, run_gnu_cmd,
};

static CMD_PATH: &str = "cksum";

fn generate_cksum_args() -> Vec<String> {
    let mut rng = rand::rng();
    let mut args = Vec::new();

    let digests = [
        "sysv", "bsd", "crc", "md5", "sha1", "sha224", "sha256", "sha384", "sha512", "blake2b",
        "sm3",
    ];
    let digest_opts = [
        "--base64",
        "--raw",
        "--tag",
        "--untagged",
        "--text",
        "--binary",
    ];

    if rng.random_bool(0.3) {
        args.push("-a".to_string());
        args.push(digests[rng.random_range(0..digests.len())].to_string());
    }

    if rng.random_bool(0.2) {
        args.push(digest_opts[rng.random_range(0..digest_opts.len())].to_string());
    }

    if rng.random_bool(0.15) {
        args.push("-l".to_string());
        args.push(rng.random_range(8..513).to_string());
    }

    if rng.random_bool(0.05) {
        for _ in 0..rng.random_range(0..3) {
            args.push(format!("file_{}", generate_random_string(5)));
        }
    } else {
        args.push("-c".to_string());
    }

    if rng.random_bool(0.25) {
        if let Ok(file_path) = generate_random_file() {
            args.push(file_path);
        }
    }

    if args.is_empty() || !args.iter().any(|arg| arg.starts_with("file_")) {
        args.push("-a".to_string());
        args.push(digests[rng.random_range(0..digests.len())].to_string());

        if let Ok(file_path) = generate_random_file() {
            args.push(file_path);
        }
    }

    args
}

fn generate_checksum_file(
    algo: &str,
    file_path: &str,
    digest_opts: &[&str],
) -> Result<String, std::io::Error> {
    let checksum_file_path = temp_dir().join("checksum_file");
    let mut cmd = Command::new(CMD_PATH);
    cmd.arg("-a").arg(algo);

    for opt in digest_opts {
        cmd.arg(opt);
    }

    cmd.arg(file_path);
    let output = cmd.output()?;

    let mut checksum_file = File::create(&checksum_file_path)?;
    checksum_file.write_all(&output.stdout)?;

    Ok(checksum_file_path.to_str().unwrap().to_string())
}

fn select_random_digest_opts<'a>(
    rng: &mut rand::rngs::ThreadRng,
    digest_opts: &'a [&'a str],
) -> Vec<&'a str> {
    digest_opts
        .iter()
        .filter(|_| rng.random_bool(0.5))
        .copied()
        .collect()
}

fuzz_target!(|_data: &[u8]| {
    let cksum_args = generate_cksum_args();
    let mut args = vec![OsString::from("cksum")];
    args.extend(cksum_args.iter().map(OsString::from));

    if let Ok(file_path) = generate_random_file() {
        let algo = cksum_args
            .iter()
            .position(|arg| arg == "-a")
            .map_or("md5", |index| &cksum_args[index + 1]);

        let all_digest_opts = ["--base64", "--raw", "--tag", "--untagged"];
        let mut rng = rand::rng();
        let selected_digest_opts = select_random_digest_opts(&mut rng, &all_digest_opts);

        if let Ok(checksum_file_path) =
            generate_checksum_file(algo, &file_path, &selected_digest_opts)
        {
            print_test_begin(format!("cksum {args:?}"));

            if let Ok(content) = fs::read_to_string(&checksum_file_path) {
                println!("File content ({checksum_file_path})");
                print_or_empty(&content);
            } else {
                eprintln!("Error reading the checksum file.");
            }
            let mut rust_result = generate_and_run_uumain(&args, uumain, None);

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

            // Lower the number of false positives caused by binary names
            replace_fuzz_binary_name("cksum", &mut rust_result);

            compare_result(
                "cksum",
                &format!("{:?}", &args[1..]),
                None,
                &rust_result,
                &gnu_result,
                false,
            );
        }
    }
});
