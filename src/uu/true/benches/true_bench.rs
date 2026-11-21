// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_true::uumain;
use uucore::benchmark::get_bench_args;

/// Benchmark multiple consecutive invocations (avoid less than 1 ns)
#[divan::bench]
fn true_consecutive_calls(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            (0..100)
                .map(|_| get_bench_args(&[]).into_iter())
                .collect::<Vec<_>>()
        })
        .bench_values(|args| {
            for args in args {
                black_box(uumain(args));
            }
        });
}

fn main() {
    divan::main();
}
