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
    fn three_39_bit_primes(bencher: Bencher) {
        factor(bencher, "148322726715648124896087586879631159");
    }

    /// Five primes near 2^38, just past the u128 range.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn five_38_bit_primes(bencher: Bencher) {
        factor(
            bencher,
            "1569275491456096801522790424087360918295323588350447935207",
        );
    }

    /// Thirteen primes near 2^39.
    #[divan::bench(sample_count = 3, sample_size = 1)]
    fn thirteen_39_bit_primes(bencher: Bencher) {
        factor(
            bencher,
            "256192672085272469290287843204387360975152374284235599731951768269391636066386517575760286247162035537155995319918598846421204855240141082924971355328149",
        );
    }

    /// 34359738421^7. Pollard's rho cannot split a prime power at all, so this
    /// one is fast only as long as perfect powers are peeled off beforehand.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn a_prime_to_the_seventh(bencher: Bencher) {
        factor(
            bencher,
            "56539106683390492137844827055225747632151249167945695848217966183182073341",
        );
    }

    /// 2^70 * 3^5 * 5 * 340282366920938463463374607431768211507: a few tiny
    /// factors in front of one large prime, which should never reach rho.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn small_factors_and_a_large_prime(bencher: Bencher) {
        factor(
            bencher,
            "488107430943668296195870985548628140589274519139277715139461120",
        );
    }

    /// The Mersenne prime 2^1279 - 1, which is all primality testing and no
    /// factoring.
    #[divan::bench(sample_count = 10, sample_size = 1)]
    fn a_1279_bit_prime(bencher: Bencher) {
        factor(
            bencher,
            "10407932194664399081925240327364085538615262247266704805319112350403608059673360298012239441732324184842421613954281007791383566248323464908139906605677320762924129509389220345773183349661583550472959420547689811211693677147548478866962501384438260291732348885311160828538416585028255604666224831890918801847068222203140521026698435488732958028878050869736186900714720710555703168729087",
        );
    }
}

fn main() {
    divan::main();
}
