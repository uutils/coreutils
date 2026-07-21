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
#[divan::bench(args = [100])]
fn ptx_input_references_long_lines(bencher: Bencher, num_lines: usize) {
    bench_ptx(bencher, &fixed_size_data(num_lines), &["-r"]);
}

fn main() {
    divan::main();
}
