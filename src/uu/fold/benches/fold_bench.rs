// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use std::fmt::Write;
use uu_fold::uumain;
use uucore::benchmark::bench_util;

/// Benchmark folding many short lines
#[divan::bench(args = [100_000])]
fn fold_many_lines(bencher: Bencher, num_lines: usize) {
    // Create long lines that need folding
    let data = (0..num_lines)
        .fold(String::new(), |mut acc, i| {
            writeln!(&mut acc, "This is a very long line number {i} that definitely needs to be folded at the default width of 80 columns").unwrap();
            acc
        });
    bench_util(bencher, data, &[], uumain);
}

/// Benchmark folding with custom width
#[divan::bench(args = [50_000])]
fn fold_custom_width(bencher: Bencher, num_lines: usize) {
    let data = (0..num_lines).fold(String::new(), |mut acc, i| {
        writeln!(
            &mut acc,
            "Line {i} with enough text to exceed width 40 characters and require folding"
        )
        .unwrap();
        acc
    });
    bench_util(bencher, data, &["-w", "40"], uumain);
}

fn main() {
    divan::main();
}
