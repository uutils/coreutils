// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use uu_wc::uumain;
use uucore::benchmark::{bench_util, text_data};

/// Benchmark different file sizes for byte counting
#[divan::bench(args = [500])]
fn wc_bytes_synthetic(bencher: Bencher, size_mb: usize) {
    let data = text_data::generate_by_size(size_mb, 80);
    bench_util(bencher, data, &["-c"], uumain);
}

#[divan::bench(args = [2_000])]
fn wc_words_synthetic(bencher: Bencher, size_mb: usize) {
    let data = text_data::generate_by_size(size_mb, 80);
    bench_util(bencher, data, &["-w"], uumain);
}

/// Benchmark combined byte+line counting
#[divan::bench(args = [2_000])]
fn wc_bytes_lines_synthetic(bencher: Bencher, size_mb: usize) {
    let data = text_data::generate_by_size(size_mb, 80);
    bench_util(bencher, data, &["-cl"], uumain);
}

/// Test different line lengths impact on performance
#[divan::bench(args = [(50, 500)])]
fn wc_lines_variable_length(bencher: Bencher, (size_mb, avg_line_len): (usize, usize)) {
    let data = text_data::generate_by_size(size_mb, avg_line_len);
    bench_util(bencher, data, &["-l"], uumain);
}

/// Benchmark large files by line count - up to 500K lines!
#[divan::bench(args = [500_000])]
fn wc_lines_large_line_count(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_util(bencher, data, &["-l"], uumain);
}

/// Benchmark character counting on large line counts
#[divan::bench(args = [100_000])]
fn wc_chars_large_line_count(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_util(bencher, data, &["-m"], uumain);
}

/// Benchmark word counting on large line counts
#[divan::bench(args = [100_000])]
fn wc_words_large_line_count(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_util(bencher, data, &["-w"], uumain);
}

/// Benchmark default wc (lines, words, bytes) on large line counts
#[divan::bench(args = [100_000])]
fn wc_default_large_line_count(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_util(bencher, data, &["-lwc"], uumain);
}

/// Benchmark very short vs very long lines with 100K lines
#[divan::bench(args = [(100_000, 200)])]
fn wc_lines_extreme_line_lengths(bencher: Bencher, (num_lines, line_len): (usize, usize)) {
    let data = text_data::generate_by_lines(num_lines, line_len);
    bench_util(bencher, data, &["-l"], uumain);
}

fn main() {
    divan::main();
}
