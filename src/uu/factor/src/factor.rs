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

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{self, stdin, stdout, BufRead, Write};
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
    f: BTreeMap<u64, u8>,
}

impl Factors {
    fn one() -> Factors {
        Factors { f: BTreeMap::new() }
    }

    fn prime(p: u64) -> Factors {
        debug_assert!(miller_rabin::is_prime(p));
        let mut f = Factors::one();
        f.push(p);
        f
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
        for (p, exp) in self.f.iter() {
            for _ in 0..*exp {
                write!(f, " {}", p)?
            }
        }

        Ok(())
    }
}

fn factor(mut n: u64) -> Factors {
    let mut factors = Factors::one();

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

fn print_factors_str(num_str: &str, w: &mut impl io::Write) -> Result<(), Box<dyn Error>> {
    num_str
        .parse::<u64>()
        .map_err(|e| e.into())
        .and_then(|x| writeln!(w, "{}:{}", x, factor(x)).map_err(|e| e.into()))
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = app!(SYNTAX, SUMMARY, LONG_HELP).parse(args.collect_str());
    let stdout = stdout();
    let mut w = io::BufWriter::new(stdout.lock());

    if matches.free.is_empty() {
        let stdin = stdin();

        for line in stdin.lock().lines() {
            for number in line.unwrap().split_whitespace() {
                if let Err(e) = print_factors_str(number, &mut w) {
                    show_warning!("{}: {}", number, e);
                }
            }
        }
    } else {
        for number in &matches.free {
            if let Err(e) = print_factors_str(number, &mut w) {
                show_warning!("{}: {}", number, e);
            }
        }
    }

    if let Err(e) = w.flush() {
        show_error!("{}", e);
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

    #[test]
    fn factor_recombines_strong_pseudoprime() {
        // This is a strong pseudoprime (wrt. miller_rabin::BASIS)
        //  and triggered a bug in rho::factor's codepath handling
        //  miller_rabbin::Result::Composite
        let pseudoprime = 17179869183;
        for _ in 0..20 {
            // Repeat the test 20 times, as it only fails some fraction
            // of the time.
            assert!(factor(pseudoprime).product() == pseudoprime);
        }
    }
}
