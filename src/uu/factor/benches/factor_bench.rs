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

/// Many numbers in a single invocation, so that parsing, factoring and
/// writing the results outweigh the startup cost.
mod batch {
    use super::{Bencher, black_box, run_util_function, uumain};

    fn factor_all<T: ToString>(bencher: Bencher, numbers: impl Iterator<Item = T>) {
        let numbers: Vec<String> = numbers.map(|n| n.to_string()).collect();
        let args: Vec<&str> = numbers.iter().map(String::as_str).collect();
        bencher.bench(|| black_box(run_util_function(uumain, &args)));
    }

    /// Every number from 2 to 20001: trial division alone factors them.
    #[divan::bench]
    fn factor_batch_of_small_numbers(bencher: Bencher) {
        factor_all(bencher, 2..20_002);
    }

    /// The 2000 largest `u64` values, most of which have a large cofactor
    /// left once their small factors are removed.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_batch_of_largest_u64s(bencher: Bencher) {
        factor_all(bencher, u64::MAX - 1999..=u64::MAX);
    }

    /// The 2000 values that follow 2^64, the first ones too large for a
    /// `u64`. Their second largest prime factor is below 2^33.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_batch_of_smallest_u128s(bencher: Bencher) {
        factor_all(bencher, (1u128 << 64)..(1u128 << 64) + 2000);
    }

    /// 500 products of two primes just above 2^31, where all the time goes
    /// into Pollard's rho.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_batch_of_semiprimes(bencher: Bencher) {
        // The first primes above 2^31, 2^31 + 2^20, 2^31 + 2^21, ...
        let primes: Vec<u64> = (0..32)
            .map(|i| {
                let start = (1 << 31) + (i << 20) + 1;
                (start..)
                    .step_by(2)
                    .find(|&n| {
                        (3..)
                            .step_by(2)
                            .take_while(|d| d * d <= n)
                            .all(|d| n % d != 0)
                    })
                    .unwrap()
            })
            .collect();
        let products = primes
            .iter()
            .flat_map(|&p| primes.iter().map(move |&q| p * q))
            .take(500);
        factor_all(bencher, products);
    }
}

/// Products of several primes of similar, moderate size.
///
/// These are the numbers factorization regressions show up in first: trial
/// division finds nothing, Pollard's rho has to run for a while before it
/// splits anything, and any O(n^(1/4)) fallback never finishes. Each of them
/// is an input GNU factor handles in well under a second.
mod hard {
    use super::{Bencher, black_box, run_util_function, uumain};

    fn factor(bencher: Bencher, number: &str) {
        bencher.bench(|| black_box(run_util_function(uumain, &[number])));
    }

    /// 529341446939 * 529341447079 * 529341447139
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_three_39_bit_primes(bencher: Bencher) {
        factor(bencher, "148322726715648124896087586879631159");
    }

    /// Five primes near 2^38, just past the u128 range.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_five_38_bit_primes(bencher: Bencher) {
        factor(
            bencher,
            "1569275491456096801522790424087360918295323588350447935207",
        );
    }

    /// Thirteen primes near 2^39.
    #[divan::bench(sample_count = 3, sample_size = 1)]
    fn factor_thirteen_39_bit_primes(bencher: Bencher) {
        factor(
            bencher,
            "256192672085272469290287843204387360975152374284235599731951768269391636066386517575760286247162035537155995319918598846421204855240141082924971355328149",
        );
    }

    /// 34359738421^7. Pollard's rho cannot split a prime power at all, so this
    /// one is fast only as long as perfect powers are peeled off beforehand.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_a_prime_to_the_seventh(bencher: Bencher) {
        factor(
            bencher,
            "56539106683390492137844827055225747632151249167945695848217966183182073341",
        );
    }

    /// 2^70 * 3^5 * 5 * 340282366920938463463374607431768211507: a few tiny
    /// factors in front of one large prime, which should never reach rho.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_small_factors_and_a_large_prime(bencher: Bencher) {
        factor(
            bencher,
            "488107430943668296195870985548628140589274519139277715139461120",
        );
    }

    /// The Mersenne prime 2^1279 - 1, which is all primality testing and no
    /// factoring.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn factor_a_1279_bit_prime(bencher: Bencher) {
        factor(
            bencher,
            "10407932194664399081925240327364085538615262247266704805319112350403608059673360298012239441732324184842421613954281007791383566248323464908139906605677320762924129509389220345773183349661583550472959420547689811211693677147548478866962501384438260291732348885311160828538416585028255604666224831890918801847068222203140521026698435488732958028878050869736186900714720710555703168729087",
        );
    }
}

fn main() {
    divan::main();
}
