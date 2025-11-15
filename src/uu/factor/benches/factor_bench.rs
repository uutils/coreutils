// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs

use divan::{Bencher, black_box};
use uu_factor::uumain;
use uucore::benchmark::run_util_function;

/// Benchmark multiple u64 digits
#[divan::bench(args = [(2)])]
fn factor_multiple_u64s(bencher: Bencher, start_num: u64) {
    bencher
        // this is a range of 5000 different u128 integers
        .with_inputs(|| (start_num, start_num + 2500))
        .bench_values(|(start_u64, end_u64)| {
            for u64_digit in start_u64..=end_u64 {
                black_box(run_util_function(uumain, &[&u64_digit.to_string()]));
            }
        });
}

/* Too much variance
/// Benchmark multiple u128 digits
#[divan::bench(args = [(18446744073709551616)])]
fn factor_multiple_u128s(bencher: Bencher, start_num: u128) {
    bencher
        .with_inputs(|| {
            // this is a range of 1000 different u128 integers
            (start_num, start_num + 1000)
        })
        .bench_values(|(start_u128, end_u128)| {
            for u128_digit in start_u128..=end_u128 {
                black_box(run_util_function(uumain, &[&u128_digit.to_string()]));
            }
        });
}
*/

/* Too much variance
/// Benchmark multiple > u128::MAX digits
#[divan::bench]
fn factor_multiple_big_uint(bencher: Bencher) {
    // max u128 value is 340_282_366_920_938_463_463_374_607_431_768_211_455
    bencher
        // this is a range of 3 different BigUints. The range is small due to
        // some BigUints being unable to be factorized into prime numbers properly
        .with_inputs(|| (768_211_459_u64, 768_211_461_u64))
        .bench_values(|(start_big_uint, end_big_uint)| {
            for digit in start_big_uint..=end_big_uint {
                let big_uint_str = format!("340282366920938463463374607431768211456{digit}");
                black_box(run_util_function(uumain, &[&big_uint_str]));
            }
        });
}
*/

#[divan::bench()]
fn factor_table(bencher: Bencher) {
    #[cfg(target_os = "linux")]
    check_personality();

    const INPUT_SIZE: usize = 128;

    let inputs = {
        // Deterministic RNG; use an explicitly-named RNG to guarantee stability
        use rand::{RngCore, SeedableRng};
        const SEED: u64 = 0xdead_bebe_ea75_cafe; // spell-checker:disable-line
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);

        std::iter::repeat_with(move || {
            let mut array = [0u64; INPUT_SIZE];
            for item in &mut array {
                *item = rng.next_u64();
            }
            array
        })
        .take(10)
        .collect::<Vec<_>>()
    };

    bencher.bench(|| {
        for a in &inputs {
            for n in a {
                divan::black_box(num_prime::nt_funcs::factors(*n, None));
            }
        }
    });
}

#[cfg(target_os = "linux")]
fn check_personality() {
    use std::fs;
    const ADDR_NO_RANDOMIZE: u64 = 0x0040000;
    const PERSONALITY_PATH: &str = "/proc/self/personality";

    let p_string = fs::read_to_string(PERSONALITY_PATH)
        .unwrap_or_else(|_| panic!("Couldn't read '{PERSONALITY_PATH}'"))
        .strip_suffix('\n')
        .unwrap()
        .to_owned();

    let personality = u64::from_str_radix(&p_string, 16)
        .unwrap_or_else(|_| panic!("Expected a hex value for personality, got '{p_string:?}'"));
    if personality & ADDR_NO_RANDOMIZE == 0 {
        eprintln!(
            "WARNING: Benchmarking with ASLR enabled (personality is {personality:x}), results might not be reproducible."
        );
    }
}

fn main() {
    divan::main();
}
