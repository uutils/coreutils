// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_shuf::uumain;
use uucore::benchmark::{get_bench_args, setup_test_file, text_data};

/// Benchmark shuffling lines from a file
/// Tests the default mode with a large number of lines
#[divan::bench(args = [100_000])]
fn shuf_lines(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&file_path]))
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark shuffling a numeric range with -i
/// Tests the input-range mode which uses a different algorithm
#[divan::bench(args = [1_000_000])]
fn shuf_input_range(bencher: Bencher, range_size: usize) {
    let range_arg = format!("1-{range_size}");

    bencher
        .with_inputs(|| get_bench_args(&[&"-i", &range_arg]))
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark shuffling with repeat (sampling with replacement)
/// Tests the -r flag combined with -n to output a specific count
#[divan::bench(args = [50_000])]
fn shuf_repeat_sampling(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(10_000, 80);
    let file_path = setup_test_file(&data);
    let count = format!("{num_lines}");

    bencher
        .with_inputs(|| get_bench_args(&[&"-r", &"-n", &count, &file_path]))
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
