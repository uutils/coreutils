/*
* This file is part of the uutils coreutils package.
*
* (c) kwantam <kwantam@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

//! Generate a table of the multiplicative inverses of p_i mod 2^64
//! for the first 10000 odd primes.
//!
//! 2 has no multiplicative inverse mode 2^64 because 2 | 2^64,
//! and in any case divisibility by two is trivial by checking the LSB.

use std::env::args;
use std::iter::repeat;
use std::num::Wrapping;
use std::u64::MAX as MAX_U64;

#[cfg(test)]
use numeric::is_prime;

#[cfg(test)]
mod numeric;

// A lazy Sieve of Eratosthenes
// Not particularly efficient, but fine for generating a few thousand primes.
struct Sieve {
    inner: Box<Iterator<Item=u64>>,
    filts: Vec<u64>,
}

impl Iterator for Sieve {
    type Item = u64;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn next(&mut self) -> Option<u64> {
        while let Some(n) = self.inner.next() {
            if self.filts.iter().all(|&x| n % x != 0) {
                self.filts.push(n);
                return Some(n);
            }
        }
        None
    }
}

impl Sieve {
    #[inline]
    pub fn new() -> Sieve {
        fn next(s: &mut u64, t: u64) -> Option<u64> {
            let ret = Some(*s);
            *s = *s + t;
            ret
        }
        let next = next;

        let odds_by_3 = Box::new(repeat(2).scan(3, next)) as Box<Iterator<Item=u64>>;

        Sieve { inner: odds_by_3, filts: Vec::new() }
    }
}

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
    // By default, we print the multiplicative inverses mod 2^64 of the first 10k primes
    let n = args().skip(1).next().unwrap_or("10000".to_string()).parse::<usize>().ok().unwrap_or(10000);

    print!("{}", PREAMBLE);

    let m = n;
    Sieve::new()
        .scan((0, 3), move |st, x| {
            let (count, mut cols) = *st;
            if count < m {
                // format the table
                let outstr = format!("({}, {}, {}),", x, inv_mod_u64(x).unwrap(), MAX_U64 / x);
                if cols + outstr.len() > MAX_WIDTH {
                    print!("\n    {}", outstr);
                    cols = 4 + outstr.len();
                } else {
                    print!(" {}", outstr);
                    cols += 1 + outstr.len();
                }

                *st = (count + 1, cols);
                Some(1)
            } else if count == m {
                // now we're done formatting the table, print NEXT_PRIME
                print!("\n];\n\npub const NEXT_PRIME: u64 = {};\n", x);

                *st = (count + 1, cols);
                Some(1)
            } else {
                None
            }
        }).take(m + 1).count();
}

#[test]
fn test_generator_and_inverter() {
    let num = 10000;

    let invs = Sieve::new().map(|x| inv_mod_u64(x).unwrap());
    assert!(Sieve::new().zip(invs).take(num).all(|(x, y)| {
        let Wrapping(z) = Wrapping(x) * Wrapping(y);
        is_prime(x) && z == 1
    }));
}

const MAX_WIDTH: usize = 100;
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
