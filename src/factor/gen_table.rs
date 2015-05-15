/*
* This file is part of the uutils coreutils package.
*
* (c) kwantam <kwantam@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

//! Generate a table of the multiplicative inverses of p_i mod 2^64
//! for the first 1027 odd primes (all 13 bit and smaller primes).
//! You can supply a commandline argument to override the default
//! value of 1027 for the number of entries in the table.
//!
//! 2 has no multiplicative inverse mode 2^64 because 2 | 2^64,
//! and in any case divisibility by two is trivial by checking the LSB.

#![cfg_attr(test, allow(dead_code))]

use sieve::Sieve;
use std::env::args;
use std::num::Wrapping;
use std::u64::MAX as MAX_U64;

#[cfg(test)]
use numeric::is_prime;

#[cfg(test)]
mod numeric;

mod sieve;

// extended Euclid algorithm
// precondition: a does not divide 2^64
fn inv_mod_u64(a: u64) -> Option<u64> {
    let mut t = 0u64;
    let mut newt = 1u64;
    let mut r = 0u64;
    let mut newr = a;

    while newr != 0 {
        let quot = if r == 0 {
            // special case when we're just starting out
            // This works because we know that
            // a does not divide 2^64, so floor(2^64 / a) == floor((2^64-1) / a);
            MAX_U64
        } else {
            r
        } / newr;

        let (tp, Wrapping(newtp)) =
            (newt, Wrapping(t) - (Wrapping(quot) * Wrapping(newt)));
        t = tp;
        newt = newtp;

        let (rp, Wrapping(newrp)) =
            (newr, Wrapping(r) - (Wrapping(quot) * Wrapping(newr)));
        r = rp;
        newr = newrp;
    }

    if r > 1 {      // not invertible
        return None;
    }

    Some(t)
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    // By default, we print the multiplicative inverses mod 2^64 of the first 1k primes
    let n = args().skip(1).next().unwrap_or("1027".to_string()).parse::<usize>().ok().unwrap_or(1027);

    print!("{}", PREAMBLE);
    let mut cols = 3;

    // we want a total of n + 1 values
    let mut primes = Sieve::odd_primes().take(n + 1);

    // in each iteration of the for loop, we use the value yielded
    // by the previous iteration. This leaves one value left at the
    // end, which we call NEXT_PRIME.
    let mut x = primes.next().unwrap();
    for next in primes {
        // format the table
        let outstr = format!("({}, {}, {}),", x, inv_mod_u64(x).unwrap(), MAX_U64 / x);
        if cols + outstr.len() > MAX_WIDTH {
            print!("\n    {}", outstr);
            cols = 4 + outstr.len();
        } else {
            print!(" {}", outstr);
            cols += 1 + outstr.len();
        }

        x = next;
    }

    print!("\n];\n\n#[allow(dead_code)]\npub const NEXT_PRIME: u64 = {};\n", x);
}

#[test]
fn test_inverter() {
    let num = 10000;

    let invs = Sieve::odd_primes().map(|x| inv_mod_u64(x).unwrap());
    assert!(Sieve::odd_primes().zip(invs).take(num).all(|(x, y)| {
        let Wrapping(z) = Wrapping(x) * Wrapping(y);
        is_prime(x) && z == 1
    }));
}

#[test]
fn test_generator() {
    let prime_10001 = Sieve::primes().skip(10000).next();
    assert_eq!(prime_10001, Some(104743));
}

const MAX_WIDTH: usize = 102;
const PREAMBLE: &'static str =
r##"/*
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

pub const P_INVS_U64: &'static [(u64, u64, u64)] = &[
   "##;
