// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_seq::uumain;
use uucore::benchmark::get_bench_args;

/// Benchmark simple integer sequence
#[divan::bench]
fn seq_integers(bencher: Bencher) {
    bencher
        .with_inputs(|| get_bench_args(&[&"1", &"1000000"]))
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark large integer
#[divan::bench]
fn seq_large_integers(bencher: Bencher) {
    bencher
        .with_inputs(|| get_bench_args(&[&"4e10003", &"4e10003"]))
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark sequence with custom separator
#[divan::bench]
fn seq_custom_separator(bencher: Bencher) {
    bencher
        .with_inputs(|| get_bench_args(&[&"-s", &",", &"1", &"1000000"]))
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark sequence with step
#[divan::bench]
fn seq_with_step(bencher: Bencher) {
    bencher
        .with_inputs(|| get_bench_args(&[&"1", &"2", &"1000000"]))
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark formatted output
#[divan::bench]
fn seq_formatted(bencher: Bencher) {
    bencher
        .with_inputs(|| get_bench_args(&[&"-f", &"%.3f", &"1", &"0.1", &"10000"]))
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
