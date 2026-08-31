// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs semiprime

use divan::{Bencher, black_box};
use uu_factor::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark multiple u64 digits.
#[divan::bench(args = [(2)])]
fn factor_multiple_u64s(bencher: Bencher, start_num: u64) {
    bencher.bench(|| {
        for n in start_num..=start_num + 2500 {
            black_box(run_util_function(uumain, &[&n.to_string()]));
        }
    });
}

/// Benchmark a large u64 prime.
#[divan::bench]
fn factor_large_u64_prime(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["18446744073709551557"]));
    });
}

/// Benchmark a 64-bit semiprime made from two 32-bit primes.
#[divan::bench]
fn factor_64bit_semiprime(bencher: Bencher) {
    bencher.bench(|| {
        black_box(run_util_function(uumain, &["18446743979220271189"]));
    });
}

fn main() {
    divan::main();
}
