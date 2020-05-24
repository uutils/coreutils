#![crate_name = "uu_factor"]

/*
* This file is part of the uutils coreutils package.
*
* (c) T. Jameson Little <t.jameson.little@gmail.com>
* (c) Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
*     20150223 added Pollard rho method implementation
* (c) kwantam <kwantam@gmail.com>
*     20150429 sped up trial division by adding table of prime inverses
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

extern crate rand;

#[macro_use]
extern crate uucore;

use numeric::*;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::{thread_rng, SeedableRng};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::fmt;
use std::io::{stdin, BufRead};
use std::mem::swap;
use std::num::Wrapping;
use std::ops;

mod numeric;

include!(concat!(env!("OUT_DIR"), "/prime_table.rs"));

static SYNTAX: &str = "[OPTION] [NUMBER]...";
static SUMMARY: &str = "Print the prime factors of the given number(s).
 If none are specified, read from standard input.";
static LONG_HELP: &str = "";

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b > 0 {
        a %= b;
        swap(&mut a, &mut b);
    }
    a
}

struct Factors {
    f: HashMap<u64, u8>,
}

impl Factors {
    fn new() -> Factors {
        Factors { f: HashMap::new() }
    }

    fn add(&mut self, prime: u64, exp: u8) {
        assert!(exp > 0);
        self.f.insert(prime, exp + self.f.get(&prime).unwrap_or(&0));
    }

    fn push(&mut self, prime: u64) {
        self.add(prime, 1)
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


fn rho_pollard_pseudorandom_function(x: u64, a: u64, b: u64, num: u64) -> u64 {
    if num < 1 << 63 {
        (sm_mul(a, sm_mul(x, x, num), num) + b) % num
    } else {
        big_add(big_mul(a, big_mul(x, x, num), num), b, num)
    }
}

fn rho_pollard_find_divisor(num: u64) -> u64 {
    #![allow(clippy::many_single_char_names)]
    let range = Uniform::new(1, num);
    let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
    let mut x = range.sample(&mut rng);
    let mut y = x;
    let mut a = range.sample(&mut rng);
    let mut b = range.sample(&mut rng);

    loop {
        x = rho_pollard_pseudorandom_function(x, a, b, num);
        y = rho_pollard_pseudorandom_function(y, a, b, num);
        y = rho_pollard_pseudorandom_function(y, a, b, num);
        let d = gcd(num, max(x, y) - min(x, y));
        if d == num {
            // Failure, retry with different function
            x = range.sample(&mut rng);
            y = x;
            a = range.sample(&mut rng);
            b = range.sample(&mut rng);
        } else if d > 1 {
            return d;
        }
    }
}

fn rho_pollard_factor(num: u64, factors: &mut Factors) {
    if is_prime(num) {
        factors.push(num);
        return;
    }

    let divisor = rho_pollard_find_divisor(num);
    rho_pollard_factor(divisor, factors);
    rho_pollard_factor(num / divisor, factors);
}

fn table_division(mut num: u64) -> Factors {
    let mut factors = Factors::new();

    if num < 2 {
        factors.push(num);
        return factors
    }

    while num % 2 == 0 {
        num /= 2;
        factors.push(2);
    }

    if num == 1 {
        return factors;
    }

    if is_prime(num) {
        factors.push(num);
        return factors;
    }

    for &(prime, inv, ceil) in P_INVS_U64 {
        if num == 1 {
            break;
        }

        // inv = prime^-1 mod 2^64
        // ceil = floor((2^64-1) / prime)
        // if (num * inv) mod 2^64 <= ceil, then prime divides num
        // See http://math.stackexchange.com/questions/1251327/
        // for a nice explanation.
        loop {
            let Wrapping(x) = Wrapping(num) * Wrapping(inv); // x = num * inv mod 2^64
            if x <= ceil {
                num = x;
                factors.push(prime);
                if is_prime(num) {
                    factors.push(num);
                    return factors;
                }
            } else {
                break;
            }
        }
    }

    // do we still have more factoring to do?
    // Decide whether to use Pollard Rho or slow divisibility based on
    // number's size:
    //if num >= 1 << 63 {
    // number is too big to use rho pollard without overflowing
    //trial_division_slow(num, factors);
    //} else if num > 1 {
    // number is still greater than 1, but not so big that we have to worry
    rho_pollard_factor(num, &mut factors);
    factors
    //}
}

fn print_factors(num: u64) {
    print!("{}:{}", num, table_division(num));
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
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP).parse(args);

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
