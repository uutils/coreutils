// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use uu_unexpand::uumain;
use uucore::benchmark::{bench_util, generate_indented_text};

/// Benchmark unexpanding many lines with leading spaces (most common use case)
#[divan::bench(args = [100_000])]
fn unexpand_many_lines(bencher: Bencher, num_lines: usize) {
    let data = generate_indented_text(num_lines);
    bench_util(bencher, data, &[], uumain);
}

/// Benchmark large file with spaces (tests performance on large files)
#[divan::bench(args = [10])]
fn unexpand_large_file(bencher: Bencher, size_mb: usize) {
    // Generate approximately size_mb worth of indented lines
    let line_size = 50; // approximate bytes per line
    let num_lines = (size_mb * 1024 * 1024) / line_size;
    let data = generate_indented_text(num_lines);
    bench_util(bencher, data, &[], uumain);
}

fn main() {
    divan::main();
}
