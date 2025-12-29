// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_seq::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark simple integer sequence
#[divan::bench]
fn seq_integers(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["1", "1000000"]));
    });
}

/// Benchmark large integer
#[divan::bench]
fn seq_large_integers(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["4e10003", "4e10003"]));
    });
}

/// Benchmark sequence with custom separator
#[divan::bench]
fn seq_custom_separator(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-s", ",", "1", "1000000"]));
    });
}

/// Benchmark sequence with step
#[divan::bench]
fn seq_with_step(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["1", "2", "1000000"]));
    });
}

/// Benchmark formatted output
#[divan::bench]
fn seq_formatted(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-f", "%.3f", "1", "0.1", "10000"],
        ));
    });
}

fn main() {
    divan::main();
}
