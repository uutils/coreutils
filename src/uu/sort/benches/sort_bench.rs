// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::NamedTempFile;
use uu_sort::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark sorting ASCII-only data
#[divan::bench(args = [500_000])]
fn sort_ascii_only(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_ascii_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark sorting accented/non-ASCII data
#[divan::bench(args = [500_000])]
fn sort_accented_data(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_accented_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark sorting mixed ASCII/non-ASCII data
#[divan::bench(args = [500_000])]
fn sort_mixed_data(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_mixed_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark case-sensitive sorting with mixed case data
#[divan::bench(args = [500_000])]
fn sort_case_sensitive(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_case_sensitive_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark case-insensitive sorting (fold case)
#[divan::bench(args = [500_000])]
fn sort_case_insensitive(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_case_sensitive_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-f", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark dictionary order sorting (only blanks and alphanumeric)
#[divan::bench(args = [500_000])]
fn sort_dictionary_order(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_mixed_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-d", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark numeric sorting with mixed data
#[divan::bench(args = [500_000])]
fn sort_numeric(bencher: Bencher, num_lines: usize) {
    let mut data = Vec::new();

    // Generate numeric data with some text prefixes
    for i in 0..num_lines {
        let value = (i * 13) % 10000; // Pseudo-random numeric values
        data.extend_from_slice(format!("value_{value}\n").as_bytes());
    }

    let file_path = setup_test_file(&data);

    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-n", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark general numeric sorting (-g) with decimal and exponent notation
#[divan::bench(args = [200_000])]
fn sort_general_numeric(bencher: Bencher, num_lines: usize) {
    let mut data = Vec::new();

    // Generate numeric data with decimal points and exponents
    for i in 0..num_lines {
        let int_part = (i * 13) % 100_000;
        let frac_part = (i * 7) % 1000;
        let exp = (i % 5) as i32 - 2; // -2..=2
        let sign = if i % 2 == 0 { "" } else { "-" };
        data.extend_from_slice(format!("{sign}{int_part}.{frac_part:03}e{exp:+}\n").as_bytes());
    }

    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-g", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark reverse sorting with locale-aware data
#[divan::bench(args = [500_000])]
fn sort_reverse_locale(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_accented_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-r", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark sorting with specific key field
#[divan::bench(args = [500_000])]
fn sort_key_field(bencher: Bencher, num_lines: usize) {
    let mut data = Vec::new();

    // Generate data with multiple fields
    let words = ["café", "naïve", "apple", "über", "banana"];
    for i in 0..num_lines {
        let word = words[i % words.len()];
        let num1 = i % 100;
        let num2 = (i * 7) % 100;
        data.extend_from_slice(format!("{num1}\t{word}\t{num2}\n").as_bytes());
    }

    let file_path = setup_test_file(&data);

    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        // Sort by second field
        black_box(run_util_function(
            uumain,
            &["-k", "2", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark unique sorting with locale-aware data
#[divan::bench(args = [500_000])]
fn sort_unique_locale(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_accented_data(num_lines);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-u", "-o", output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark sorting with very long lines exceeding START_BUFFER_SIZE (8000 bytes)
#[divan::bench(args = [10_000])]
fn sort_long_line(bencher: Bencher, line_size: usize) {
    // Create files with very long lines to test buffer handling
    let mut data_a = vec![b'b'; line_size];
    data_a.push(b'\n');

    let mut data_b = vec![b'a'; line_size];
    data_b.push(b'\n');

    let file_a = setup_test_file(&data_a);
    let file_b = setup_test_file(&data_b);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &[
                file_a.to_str().unwrap(),
                file_b.to_str().unwrap(),
                "-o",
                output_path,
            ],
        ));
    });
}

fn main() {
    divan::main();
}
