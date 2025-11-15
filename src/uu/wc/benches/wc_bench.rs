// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_wc::uumain;
use uucore::benchmark::{create_test_file, run_util_function, text_data};

/// Benchmark different file sizes for byte counting
#[divan::bench(args = [500])]
fn wc_bytes_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-c", file_path_str]));
    });
}

#[divan::bench(args = [2_000])]
fn wc_words_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-w", file_path_str]));
    });
}

/// Benchmark combined byte+line counting
#[divan::bench(args = [2_000])]
fn wc_bytes_lines_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-cl", file_path_str]));
    });
}

/// Test different line lengths impact on performance
#[divan::bench(args = [(50, 500)])]
fn wc_lines_variable_length(bencher: Bencher, (size_mb, avg_line_len): (usize, usize)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, avg_line_len);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-l", file_path_str]));
    });
}

/// Benchmark large files by line count - up to 500K lines!
#[divan::bench(args = [500_000])]
fn wc_lines_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-l", file_path_str]));
    });
}

/// Benchmark character counting on large line counts
#[divan::bench(args = [100_000])]
fn wc_chars_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-m", file_path_str]));
    });
}

/// Benchmark word counting on large line counts
#[divan::bench(args = [100_000])]
fn wc_words_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-w", file_path_str]));
    });
}

/// Benchmark default wc (lines, words, bytes) on large line counts
#[divan::bench(args = [100_000])]
fn wc_default_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-lwc", file_path_str]));
    });
}

/// Benchmark very short vs very long lines with 100K lines
#[divan::bench(args = [(100_000, 200)])]
fn wc_lines_extreme_line_lengths(bencher: Bencher, (num_lines, line_len): (usize, usize)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_lines(num_lines, line_len);
    let file_path = create_test_file(&data, temp_dir.path());
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-l", file_path_str]));
    });
}

fn main() {
    divan::main();
}
