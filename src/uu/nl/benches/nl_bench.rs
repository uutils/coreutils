// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use uu_nl::uumain;
use uucore::benchmark::{bench_util, text_data};

/// Benchmark numbering many lines (default mode - most common use case)
#[divan::bench(args = [100_000])]
fn nl_many_lines(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_by_lines(num_lines, 80);
    bench_util(bencher, data, &[], uumain);
}

/// Benchmark large file with -ba option (number all lines - most common argument)
#[divan::bench(args = [10])]
fn nl_large_file(bencher: Bencher, size_mb: usize) {
    let data = text_data::generate_by_size(size_mb, 80);
    bench_util(bencher, data, &["-ba"], uumain);
}

fn main() {
    divan::main();
}
