// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
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

/// Run uutils wc with given arguments
fn run_uutils_wc(args: &[&str]) -> i32 {
    use std::process::{Command, Stdio};

    // Use the binary instead of calling uumain directly to avoid stdout issues
    let output = Command::new("../../../target/release/coreutils")
        .args(["wc"].iter().chain(args.iter()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to execute wc command");

    i32::from(!output.success())
}

/// Benchmark different file sizes for line counting
#[divan::bench(args = [1, 5, 10, 25, 50])]
fn wc_lines_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-l", file_path_str]));
    });
}

/// Benchmark different file sizes for character counting
#[divan::bench(args = [1, 5, 10, 25])]
fn wc_chars_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-m", file_path_str]));
    });
}

/// Benchmark different file sizes for byte counting
#[divan::bench(args = [1, 5, 10, 50, 100])]
fn wc_bytes_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-c", file_path_str]));
    });
}

/// Benchmark word counting (should use traditional read path)
#[divan::bench(args = [1, 5, 10, 25])]
fn wc_words_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-w", file_path_str]));
    });
}

/// Benchmark combined byte+line counting
#[divan::bench(args = [1, 5, 10, 50])]
fn wc_bytes_lines_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-cl", file_path_str]));
    });
}

/// Benchmark default wc behavior (bytes, lines, words)
#[divan::bench(args = [1, 5, 10])]
fn wc_default_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&[file_path_str]));
    });
}

/// Test different line lengths impact on performance
#[divan::bench(args = [(5, 50), (5, 100), (5, 200), (5, 500)])]
fn wc_lines_variable_length(bencher: Bencher, (size_mb, avg_line_len): (usize, usize)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data(size_mb, avg_line_len);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-l", file_path_str]));
    });
}

/// Benchmark large files by line count - up to 500K lines!
#[divan::bench(args = [10_000, 50_000, 100_000, 500_000])]
fn wc_lines_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-l", file_path_str]));
    });
}

/// Benchmark character counting on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_chars_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-m", file_path_str]));
    });
}

/// Benchmark word counting on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_words_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-w", file_path_str]));
    });
}

/// Benchmark default wc (lines, words, bytes) on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_default_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&[file_path_str]));
    });
}

/// Benchmark very short vs very long lines with 100K lines
#[divan::bench(args = [(100_000, 10), (100_000, 200), (100_000, 1000)])]
fn wc_lines_extreme_line_lengths(bencher: Bencher, (num_lines, line_len): (usize, usize)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_test_data_by_lines(num_lines, line_len);
    let file_path = create_test_file(&data, &temp_dir);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_wc(&["-l", file_path_str]));
    });
}

fn main() {
    divan::main();
}
