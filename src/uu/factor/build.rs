// * This file is part of the uutils coreutils package.
// *
// * (c) kwantam <kwantam@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

//! Generate a table of the multiplicative inverses of p_i mod 2^64
//! for the first 1027 odd primes (all 13 bit and smaller primes).
//! You can supply a command line argument to override the default
//! value of 1027 for the number of entries in the table.
//!
//! 2 has no multiplicative inverse mode 2^64 because 2 | 2^64,
//! and in any case divisibility by two is trivial by checking the LSB.

// spell-checker:ignore (ToDO) invs newr newrp newtp outstr

#![cfg_attr(test, allow(dead_code))]

use std::env::{self, args};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use self::sieve::Sieve;

#[cfg(test)]
use miller_rabin::is_prime;

#[path = "src/numeric.rs"]
mod numeric;
use numeric::inv_mod_u64;

mod sieve;

#[cfg_attr(test, allow(dead_code))]
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&out_dir).join("prime_table.rs")).unwrap();

    // By default, we print the multiplicative inverses mod 2^64 of the first 1k primes
    const DEFAULT_SIZE: usize = 320;
    let n = args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(DEFAULT_SIZE);

    write!(file, "{}", PREAMBLE).unwrap();
    let mut cols = 3;

    // we want a total of n + 1 values
    let mut primes = Sieve::odd_primes().take(n + 1);

    // in each iteration of the for loop, we use the value yielded
    // by the previous iteration. This leaves one value left at the
    // end, which we call NEXT_PRIME.
    let mut x = primes.next().unwrap();
    for next in primes {
        // format the table
        let outstr = format!("({}, {}, {}),", x, inv_mod_u64(x), std::u64::MAX / x);
        if cols + outstr.len() > MAX_WIDTH {
            write!(file, "\n    {}", outstr).unwrap();
            cols = 4 + outstr.len();
        } else {
            write!(file, " {}", outstr).unwrap();
            cols += 1 + outstr.len();
        }

        x = next;
    }

    write!(
        file,
        "\n];\n\n#[allow(dead_code)]\npub const NEXT_PRIME: u64 = {};\n",
        x
    )
    .unwrap();
}

#[test]
fn test_generator_isprime() {
    assert_eq!(Sieve::odd_primes.take(10_000).all(is_prime));
}

#[test]
fn test_generator_10001() {
    let prime_10001 = Sieve::primes().skip(10_000).next();
    assert_eq!(prime_10001, Some(104_743));
}

const MAX_WIDTH: usize = 102;
const PREAMBLE: &str = r##"/*
* This file is part of the uutils coreutils package.
*
* (c) kwantam <kwantam@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

// *** NOTE: this file was automatically generated.
// Please do not edit by hand. Instead, modify and
// re-run src/factor/gen_tables.rs.

#[allow(clippy::unreadable_literal)]
pub const P_INVS_U64: &[(u64, u64, u64)] = &[
   "##;
