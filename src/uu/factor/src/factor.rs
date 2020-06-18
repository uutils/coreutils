// * This file is part of the uutils coreutils package.
// *
// * (c) 2014 T. Jameson Little <t.jameson.little@gmail.com>
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

extern crate rand;

#[macro_use]
extern crate uucore;

use std::collections::HashMap;
use std::fmt;
use std::io::{stdin, BufRead};
use std::ops;

mod miller_rabin;
mod numeric;
mod rho;
mod table;

static SYNTAX: &str = "[OPTION] [NUMBER]...";
static SUMMARY: &str = "Print the prime factors of the given number(s).
 If none are specified, read from standard input.";
static LONG_HELP: &str = "";

struct Factors {
    f: HashMap<u64, u8>,
}

impl Factors {
    fn new() -> Factors {
        Factors { f: HashMap::new() }
    }

    fn add(&mut self, prime: u64, exp: u8) {
        debug_assert!(exp > 0);
        let n = *self.f.get(&prime).unwrap_or(&0);
        self.f.insert(prime, exp + n);
    }

    fn push(&mut self, prime: u64) {
        self.add(prime, 1)
    }

    #[cfg(test)]
    fn product(&self) -> u64 {
        self.f
            .iter()
            .fold(1, |acc, (p, exp)| acc * p.pow(*exp as u32))
    }
}

impl ops::MulAssign<Factors> for Factors {
    fn mul_assign(&mut self, other: Factors) {
        for (prime, exp) in &other.f {
            self.add(*prime, *exp);
        }
    }
}

impl fmt::Display for Factors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Use a representation with efficient in-order iteration
        let mut primes: Vec<&u64> = self.f.keys().collect();
        primes.sort();

        for p in primes {
            for _ in 0..self.f[&p] {
                write!(f, " {}", p)?
            }
        }

        Ok(())
    }
}

fn factor(mut n: u64) -> Factors {
    let mut factors = Factors::new();

    if n < 2 {
        factors.push(n);
        return factors;
    }

    let z = n.trailing_zeros();
    if z > 0 {
        factors.add(2, z as u8);
        n >>= z;
    }

    if n == 1 {
        return factors;
    }

    let (f, n) = table::factor(n);
    factors *= f;

    if n >= table::NEXT_PRIME {
        factors *= rho::factor(n);
    }

    factors
}

fn print_factors(num: u64) {
    print!("{}:{}", num, factor(num));
    println!();
}

fn print_factors_str(num_str: &str) {
    if let Err(e) = num_str.parse::<u64>().and_then(|x| {
        print_factors(x);
        Ok(())
    }) {
        show_warning!("{}: {}", num_str, e);
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = app!(SYNTAX, SUMMARY, LONG_HELP).parse(args);

    if matches.free.is_empty() {
        let stdin = stdin();
        for line in stdin.lock().lines() {
            for number in line.unwrap().split_whitespace() {
                print_factors_str(number);
            }
        }
    } else {
        for num_str in &matches.free {
            print_factors_str(num_str);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::factor;

    #[test]
    fn factor_recombines_small() {
        assert!((1..10_000)
            .map(|i| 2 * i + 1)
            .all(|i| factor(i).product() == i));
    }

    #[test]
    fn factor_recombines_overflowing() {
        assert!((0..250)
            .map(|i| 2 * i + 2u64.pow(32) + 1)
            .all(|i| factor(i).product() == i));
    }
}
