// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uucore::bench_utils::shared::*;

/// Benchmark different file sizes for line counting
#[divan::bench(args = [1, 5, 10, 25, 50])]
fn wc_lines_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-l", file_path_str]));
    });
}

/// Benchmark different file sizes for character counting
#[divan::bench(args = [1, 5, 10, 25])]
fn wc_chars_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-m", file_path_str]));
    });
}

/// Benchmark different file sizes for byte counting
#[divan::bench(args = [1, 5, 10, 50, 100])]
fn wc_bytes_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-c", file_path_str]));
    });
}

/// Benchmark word counting (should use traditional read path)
#[divan::bench(args = [1, 5, 10, 25])]
fn wc_words_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-w", file_path_str]));
    });
}

/// Benchmark combined byte+line counting
#[divan::bench(args = [1, 5, 10, 50])]
fn wc_bytes_lines_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-cl", file_path_str]));
    });
}

/// Benchmark default wc behavior (bytes, lines, words)
#[divan::bench(args = [1, 5, 10])]
fn wc_default_synthetic(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &[file_path_str]));
    });
}

/// Test different line lengths impact on performance
#[divan::bench(args = [(5, 50), (5, 100), (5, 200), (5, 500)])]
fn wc_lines_variable_length(bencher: Bencher, (size_mb, avg_line_len): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data(size_mb, avg_line_len);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-l", file_path_str]));
    });
}

/// Benchmark large files by line count - up to 500K lines!
#[divan::bench(args = [10_000, 50_000, 100_000, 500_000])]
fn wc_lines_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-l", file_path_str]));
    });
}

/// Benchmark character counting on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_chars_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-m", file_path_str]));
    });
}

/// Benchmark word counting on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_words_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-w", file_path_str]));
    });
}

/// Benchmark default wc (lines, words, bytes) on large line counts
#[divan::bench(args = [10_000, 50_000, 100_000])]
fn wc_default_large_line_count(bencher: Bencher, num_lines: usize) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data_by_lines(num_lines, 80);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &[file_path_str]));
    });
}

/// Benchmark very short vs very long lines with 100K lines
#[divan::bench(args = [(100_000, 10), (100_000, 200), (100_000, 1000)])]
fn wc_lines_extreme_line_lengths(bencher: Bencher, (num_lines, line_len): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let data = generate_text_data_by_lines(num_lines, line_len);
    let file_path = create_test_file(&data, &temp_dir, "");
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_uutils_command("wc", &["-l", file_path_str]));
    });
}

fn main() {
    divan::main();
}
