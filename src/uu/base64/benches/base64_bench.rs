// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uucore::bench_utils::shared::*;

/// Generate already base64-encoded data for decoding benchmarks
fn generate_base64_data(size_mb: usize) -> Vec<u8> {
    use uucore::encoding::for_cksum::BASE64;

    // Generate binary data first
    let binary_data = generate_binary_data(size_mb);

    // Encode it to base64
    BASE64.encode(&binary_data).into_bytes()
}

/// Benchmark base64 encoding with different file sizes - binary data
#[divan::bench(args = [1, 5, 10, 25, 50])]
fn base64_encode_binary(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_binary_data(size_mb);
    let file_path = create_test_file(&data, &temp_dir, "_binary");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("base64", &[file_path_str]));
    });
}

/// Benchmark base64 encoding with text data
#[divan::bench(args = [1, 5, 10, 25])]
fn base64_encode_text(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "_text");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("base64", &[file_path_str]));
    });
}

/// Benchmark base64 decoding with different file sizes
#[divan::bench(args = [1, 5, 10, 25])]
fn base64_decode_binary(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_base64_data(size_mb);
    let file_path = create_test_file(&data, &temp_dir, "_b64");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("base64", &["-d", file_path_str]));
    });
}

/// Benchmark base64 encoding with wrap option (different line lengths)
#[divan::bench(args = [(5, 64), (5, 76), (5, 0)])]
fn base64_encode_wrap(bencher: Bencher, (size_mb, wrap_cols): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_binary_data(size_mb);
    let file_path = create_test_file(&data, &temp_dir, "_wrap");
    let file_path_str = file_path.to_str().unwrap();

    let wrap_cols_str = wrap_cols.to_string();
    let wrap_arg = if wrap_cols == 0 {
        vec!["-w", "0", file_path_str]
    } else {
        vec!["-w", &wrap_cols_str, file_path_str]
    };

    bencher.bench(|| {
        black_box(run_uutils_command("base64", &wrap_arg));
    });
}

/// Benchmark base64 encoding from stdin (pipe simulation)
#[divan::bench(args = [1, 5, 10])]
fn base64_encode_stdin(bencher: Bencher, size_mb: usize) {
    use std::process::{Command, Stdio};

    let data = generate_binary_data(size_mb);

    bencher.bench(|| {
        let mut child = Command::new("../../../target/release/coreutils")
            .args(["base64"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start base64");

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(&data).unwrap();
        }

        black_box(child.wait().unwrap());
    });
}

/// Benchmark base64 decoding from stdin
#[divan::bench(args = [1, 5, 10])]
fn base64_decode_stdin(bencher: Bencher, size_mb: usize) {
    use std::process::{Command, Stdio};

    let data = generate_base64_data(size_mb);

    bencher.bench(|| {
        let mut child = Command::new("../../../target/release/coreutils")
            .args(["base64", "-d"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start base64");

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(&data).unwrap();
        }

        black_box(child.wait().unwrap());
    });
}

/// Benchmark base64 URL-safe encoding
#[divan::bench(args = [1, 5, 10])]
fn base64_encode_url_safe(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_binary_data(size_mb);
    let file_path = create_test_file(&data, &temp_dir, "_url");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("base64", &["-u", file_path_str]));
    });
}

/// Benchmark different data patterns (highly compressible vs random)
#[divan::bench(args = [(5, "zeros"), (5, "pattern"), (5, "random")])]
fn base64_encode_data_patterns(bencher: Bencher, (size_mb, pattern): (usize, &str)) {
    let temp_dir = TempDir::new().unwrap();

    let data = match pattern {
        "zeros" => generate_data_pattern(size_mb, DataPattern::Zeros),
        "pattern" => generate_data_pattern(size_mb, DataPattern::RepeatingPattern),
        "random" => generate_data_pattern(size_mb, DataPattern::Random),
        _ => unreachable!(),
    };

    let file_path = create_test_file(&data, &temp_dir, &format!("_{pattern}"));
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("base64", &[file_path_str]));
    });
}

fn main() {
    divan::main();
}
