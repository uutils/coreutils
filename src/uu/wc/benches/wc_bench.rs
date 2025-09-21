// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufWriter, Write};
use tempfile::TempDir;

/// Generate test data with different characteristics
fn generate_test_data(size_mb: usize, avg_line_length: usize) -> Vec<u8> {
    let total_size = size_mb * 1024 * 1024;
    let mut data = Vec::with_capacity(total_size);

    let mut current_size = 0;
    let mut line_chars = 0;

    while current_size < total_size {
        if line_chars >= avg_line_length {
            data.push(b'\n');
            line_chars = 0;
        } else {
            // Use various ASCII characters to make it realistic
            data.push(b'a' + (current_size % 26) as u8);
            line_chars += 1;
        }
        current_size += 1;
    }

    // Ensure we end with a newline
    if data.last() != Some(&b'\n') {
        data.push(b'\n');
    }

    data
}

/// Generate test data by line count instead of size
fn generate_test_data_by_lines(num_lines: usize, avg_line_length: usize) -> Vec<u8> {
    let mut data = Vec::new();

    for line_num in 0..num_lines {
        // Vary line length slightly for realism
        let line_length = avg_line_length + (line_num % 40).saturating_sub(20);

        for char_pos in 0..line_length {
            // Create more realistic text with spaces
            if char_pos > 0 && char_pos % 8 == 0 {
                data.push(b' '); // Add spaces every 8 characters
            } else {
                // Cycle through letters with some variation
                let char_offset = (line_num + char_pos) % 26;
                data.push(b'a' + char_offset as u8);
            }
        }
        data.push(b'\n');
    }

    data
}

/// Create a temporary file with test data
fn create_test_file(data: &[u8], temp_dir: &TempDir) -> std::path::PathBuf {
    let file_path = temp_dir.path().join("test_data.txt");
    let file = File::create(&file_path).unwrap();
    let mut writer = BufWriter::new(file);
    writer.write_all(data).unwrap();
    writer.flush().unwrap();
    file_path
}

/// Run uutils wc directly using internal uumain function (better for `CodSpeed`)
/// Since uumain returns i32 and expects an iterator, we need this wrapper
fn run_uutils_wc_internal(args: &[&str]) -> i32 {
    let mut full_args = vec![OsString::from("wc")];
    full_args.extend(args.iter().map(OsString::from));

    // Call the uumain function directly using the signature it expects
    // This doesn't capture output, so it won't be suitable for validation
    // but it will be instrumentable by CodSpeed
    std::panic::catch_unwind(|| uu_wc::uumain(full_args.into_iter())).unwrap_or(1)
}

/// Simple internal benchmark to validate `CodSpeed` integration
#[divan::bench]
fn simple_addition() {
    black_box(1 + 1);
}

/// Internal benchmark testing data generation performance
#[divan::bench(args = [1, 5, 10])]
fn data_generation_benchmark(bencher: Bencher, size_mb: usize) {
    bencher.bench(|| {
        black_box(generate_test_data(size_mb, 80));
    });
}

/// Internal benchmark for line counting logic (if we can access it)
#[divan::bench]
fn line_counting_internal() {
    let data = generate_test_data(1, 80);
    let line_count = bytecount::count(&data, b'\n');
    black_box(line_count);
}

/// Internal benchmark for line counting using direct function calls
#[divan::bench(args = [1, 5, 10])]
fn wc_lines_internal(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-l", file_path_str]));
    });
}

/// Internal benchmark for byte counting using direct function calls
#[divan::bench(args = [1, 5, 10])]
fn wc_bytes_internal(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-c", file_path_str]));
    });
}

/// Internal benchmark for word counting using direct function calls
#[divan::bench(args = [1, 5, 10])]
fn wc_words_internal(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-w", file_path_str]));
    });
}

/// Internal benchmark for character counting using direct function calls
#[divan::bench(args = [1, 5, 10, 25])]
fn wc_chars_internal(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-m", file_path_str]));
    });
}

/// Internal benchmark for combined byte+line counting
#[divan::bench(args = [1, 5, 10, 50])]
fn wc_bytes_lines_internal(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-cl", file_path_str]));
    });
}

/// Internal benchmark for default wc behavior (bytes, lines, words)
#[divan::bench(args = [1, 5, 10])]
fn wc_default_internal(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&[file_path_str]));
    });
}

/// Internal benchmark for different line lengths impact on performance
#[divan::bench(args = [(5, 50), (5, 100), (5, 200), (5, 500)])]
fn wc_lines_variable_length_internal(bencher: Bencher, (size_mb, avg_line_len): (usize, usize)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, avg_line_len);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-l", file_path_str]));
    });
}

/// Internal benchmark for large files by line count
#[divan::bench(args = [10_000, 50_000, 100_000, 500_000])]
fn wc_lines_large_line_count_internal(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-l", file_path_str]));
    });
}

/// Internal benchmark for character counting on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_chars_large_line_count_internal(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-m", file_path_str]));
    });
}

/// Internal benchmark for word counting on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_words_large_line_count_internal(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-w", file_path_str]));
    });
}

/// Internal benchmark for default wc on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_default_large_line_count_internal(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&[file_path_str]));
    });
}

/// Internal benchmark for extreme line lengths
#[divan::bench(args = [(100_000, 10), (100_000, 200), (100_000, 1000)])]
fn wc_lines_extreme_line_lengths_internal(bencher: Bencher, (num_lines, line_len): (usize, usize)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, line_len);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc_internal(&["-l", file_path_str]));
    });
}

fn main() {
    divan::main();
}
