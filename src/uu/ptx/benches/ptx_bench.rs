// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Benchmarks for `ptx`.

use divan::{Bencher, black_box};
use uu_ptx::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

fn bench_ptx(bencher: Bencher, data: &[u8], args: &[&str]) {
    let file_path = setup_test_file(data);
    let file_path_str = file_path.to_str().unwrap();

    let mut full_args: Vec<&str> = args.to_vec();
    full_args.push(file_path_str);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &full_args));
    });
}

/// Build a fixed ~1 MiB input spread across num_lines lines.
fn fixed_size_data(num_lines: usize) -> Vec<u8> {
    let line_len = (1024 * 1024 / num_lines).max(1);
    text_data::generate_by_lines(num_lines, line_len)
}

/// Benchmark the common case of many short lines.
#[divan::bench(args = [100_000])]
fn ptx_short_lines(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_ptx(bencher, &data, &[]);
}

/// Benchmark a fixed ~1 MiB input spread across 100 lines.
#[divan::bench(args = [100])]
fn ptx_long_lines(bencher: Bencher, num_lines: usize) {
    bench_ptx(bencher, &fixed_size_data(num_lines), &[]);
}

/// Benchmark -r on many short lines.
#[divan::bench(args = [100_000])]
fn ptx_input_references_short_lines(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_ptx(bencher, &data, &["-r"]);
}

/// Benchmark -r on long lines
#[divan::bench(args = [1000])]
fn ptx_input_references_long_lines(bencher: Bencher, num_lines: usize) {
    bench_ptx(bencher, &fixed_size_data(num_lines), &["-r"]);
}

fn tex_special_data(num_lines: usize) -> Vec<u8> {
    let lines = [
        "the permuted index of \\alpha sorted around each keyword",
        "context $x$ lines paired with file_name and A&B output",
        "roughly 50% of {group} entries escaped as #1 or x_1 here",
        "each \\ref and $y$ carries a file_name style suffix too",
        "sorted A&B context around 90% of the {group} keywords",
        "index entries with x_1 and #1 spread across the \\alpha line",
    ];
    text_data::generate_data_from_words(&lines, num_lines)
}

/// Benchmark -T on many short lines.
#[divan::bench(args = [10000])]
fn ptx_tex(bencher: Bencher, num_lines: usize) {
    let data = tex_special_data(num_lines);
    bench_ptx(bencher, &data, &["-T"]);
}

fn main() {
    divan::main();
}
