// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_true::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark the common case: true with no arguments
#[divan::bench]
fn true_no_args(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &[]));
    });
}

/// Benchmark multiple consecutive invocations (throughput test)
#[divan::bench]
fn true_consecutive_calls(bencher: Bencher) {
    bencher.bench(|| {
        for _ in 0..100 {
            black_box(run_util_function(uumain, &[]));
        }
    });
}

fn main() {
    divan::main();
}
