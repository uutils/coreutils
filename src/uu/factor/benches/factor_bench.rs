// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore funcs semiprimes

use divan::{Bencher, black_box};
use num_bigint::BigUint;
use uu_factor::{factorize, uumain};
use uucore::benchmark::run_util_function;

// ============================================================================
// INTERNAL ALGORITHM BENCHMARKS (no I/O overhead - for CodSpeed accuracy)
// ============================================================================

/// Benchmark direct factorize() for small u64 numbers (2 to 100)
/// Tests trial division performance without CLI/stdout overhead
#[divan::bench]
fn factorize_small_u64(bencher: Bencher) {
    bencher.bench(|| {
        for n in 2u64..=100 {
            black_box(factorize(&BigUint::from(n)));
        }
    });
}

/// Benchmark direct factorize() for 32-bit semiprimes
/// Tests Pollard-Rho for 16-bit factors
#[divan::bench]
fn factorize_32bit_semiprime(bencher: Bencher) {
    let n = BigUint::from(4295098369u64); // 65537 × 65537
    bencher.bench(|| black_box(factorize(&n)));
}

/// Benchmark direct factorize() for 64-bit semiprime
/// Key performance target: product of two 32-bit primes
#[divan::bench]
fn factorize_64bit_semiprime(bencher: Bencher) {
    let n = BigUint::from(18446743979220271189u64); // 4294967279 × 4294967291
    bencher.bench(|| black_box(factorize(&n)));
}

/// Benchmark direct factorize() for large u64 prime
/// Tests primality checking for 64-bit numbers
#[divan::bench]
fn factorize_large_prime(bencher: Bencher) {
    let n = BigUint::from(18446744073709551557u64); // prime near 2^64
    bencher.bench(|| black_box(factorize(&n)));
}

/// Benchmark direct factorize() for Fermat-friendly (close factors)
#[divan::bench]
fn factorize_close_factors(bencher: Bencher) {
    let n = BigUint::from(4294049777u64); // 65521 × 65537 (close 16-bit primes)
    bencher.bench(|| black_box(factorize(&n)));
}

/// Benchmark direct factorize() for 96-bit number (Pollard-Rho + trial)
/// Uses product of 32-bit and 64-bit primes to stay within <128-bit fast path
#[divan::bench]
fn factorize_96bit_composite(bencher: Bencher) {
    // ~96-bit: 4294967291 (32-bit prime) × 4611686018427387847 (~62-bit prime)
    // This stays in the fast path (< 128 bits)
    let n = BigUint::parse_bytes(b"19807040619342712411247977", 10).unwrap();
    bencher.bench(|| black_box(factorize(&n)));
}

/// Benchmark direct factorize() for 120-bit composite with mixed factor sizes
/// Tests realistic factorization: 37 × 211 × 10781 × 18661380293 × 846276908707591607
/// GNU factor: 0.03s, this tests our algorithm selection efficiency
#[divan::bench]
fn factorize_120bit_mixed(bencher: Bencher) {
    // 120-bit: 1329227995784915872903807060280344217
    // Factors: 37 × 211 × 10781 × 18661380293 × 846276908707591607
    // - Small primes: 37, 211, 10781 (trial division)
    // - 34-bit: 18661380293 (Pollard-Rho)
    // - 60-bit: 846276908707591607 (Pollard-Rho or ECM)
    let n = BigUint::parse_bytes(b"1329227995784915872903807060280344217", 10).unwrap();
    bencher.bench(|| black_box(factorize(&n)));
}

// ============================================================================
// END-TO-END BENCHMARKS (includes CLI parsing + stdout)
// ============================================================================

/// Benchmark small u64 numbers (2 to 502) via CLI
/// Tests trial division and small factor performance
#[divan::bench]
fn factor_small_u64(bencher: Bencher) {
    bencher
        .with_inputs(|| (2u64, 502u64))
        .bench_values(|(start, end)| {
            for n in start..=end {
                black_box(run_util_function(uumain, &[&n.to_string()]));
            }
        });
}

/// Benchmark medium u64 numbers (32-bit semiprimes)
/// Tests Pollard-Rho for 16-bit factors
#[divan::bench]
fn factor_medium_u64(bencher: Bencher) {
    // 32-bit semiprimes: products of two ~16-bit primes
    let test_numbers: Vec<String> = vec![
        "4295098369".to_string(), // 65537 × 65537
        "3215031751".to_string(), // 56713 × 56687
        "2147483647".to_string(), // Mersenne prime M31
    ];

    bencher
        .with_inputs(|| test_numbers.clone())
        .bench_values(|numbers| {
            for n in &numbers {
                black_box(run_util_function(uumain, &[n]));
            }
        });
}

/// Benchmark large u64 primes
/// Tests primality checking for 64-bit numbers
#[divan::bench]
fn factor_large_u64_prime(bencher: Bencher) {
    // Large primes - should be fast (just primality test)
    let test_numbers: Vec<String> = vec![
        "18446744073709551557".to_string(), // prime near 2^64
        "9223372036854775783".to_string(),  // prime near 2^63
    ];

    bencher
        .with_inputs(|| test_numbers.clone())
        .bench_values(|numbers| {
            for n in &numbers {
                black_box(run_util_function(uumain, &[n]));
            }
        });
}

/// Benchmark 64-bit semiprime (product of two 32-bit primes)
/// This is a key performance target
#[divan::bench]
fn factor_64bit_semiprime(bencher: Bencher) {
    // 64-bit semiprime: 4294967291 × 4294967279 = 18446743979220271189
    let n = "18446743979220271189";

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[n]));
    });
}

/// Benchmark multiple numbers in sequence (realistic usage)
/// Tests throughput for batch factorization
#[divan::bench]
fn factor_batch_mixed(bencher: Bencher) {
    let test_numbers: Vec<String> = vec![
        "2".to_string(),
        "1000000007".to_string(), // prime
        "4295098369".to_string(), // 32-bit semiprime (65537²)
    ];

    bencher
        .with_inputs(|| test_numbers.clone())
        .bench_values(|numbers| {
            for n in &numbers {
                black_box(run_util_function(uumain, &[n]));
            }
        });
}

fn main() {
    divan::main();
}
