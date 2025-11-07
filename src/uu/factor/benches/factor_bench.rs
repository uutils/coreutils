// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_factor::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark one u64 digit
#[divan::bench]
fn factor_single_u64(bencher: Bencher) {
    bencher
        .with_inputs(|| 1000000_u64)
        .bench_values(|single_u64| {
            black_box(run_util_function(uumain, &[&single_u64.to_string()]));
        });
}

/// Benchmark multiple u64 digits
#[divan::bench]
fn factor_multiple_u64s(bencher: Bencher) {
    bencher
        .with_inputs(|| (2_u64, 1000000_u64))
        .bench_values(|(start_u64, end_u64)| {
            for u64_digit in start_u64..=end_u64 {
                black_box(run_util_function(uumain, &[&u64_digit.to_string()]));
            }
        });
}

/// Benchmark one u128 digit
#[divan::bench]
fn factor_single_u128(bencher: Bencher) {
    bencher
        .with_inputs(|| 18446744073709551616_u128)
        .bench_values(|single_u128| {
            black_box(run_util_function(uumain, &[&single_u128.to_string()]));
        });
}

/// Benchmark multiple u128 digits
#[divan::bench]
fn factor_multiple_u128s(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            // this is a range of 1 million different u128 integers
            (18446744073709551616_u128, 18446744073710551616_u128)
        })
        .bench_values(|(start_u128, end_u128)| {
            for u128_digit in start_u128..=end_u128 {
                black_box(run_util_function(uumain, &[&u128_digit.to_string()]));
            }
        });
}

/// Benchmark single > u128::MAX digits
#[divan::bench]
fn factor_single_big_uint(bencher: Bencher) {
    // max u128 value is 340_282_366_920_938_463_463_374_607_431_768_211_455
    bencher
        .with_inputs(|| "340_282_366_920_938_463_463_374_607_431_768_211_456")
        .bench_values(|single_big_uint| {
            black_box(run_util_function(uumain, &[single_big_uint]));
        });
}

/// Benchmark multiple > u128::MAX digits
#[divan::bench]
fn factor_multiple_big_uint(bencher: Bencher) {
    // max u128 value is 340_282_366_920_938_463_463_374_607_431_768_211_455
    bencher
        .with_inputs(|| (768_211_456_u64, 769_211_456_u64))
        .bench_values(|(start_big_uint, end_big_uint)| {
            for digit in start_big_uint..=end_big_uint {
                let big_uint_str = format!("340282366920938463463374607431768211456{digit}");
                black_box(run_util_function(uumain, &[&big_uint_str]));
            }
        });
}

fn main() {
    divan::main();
}
