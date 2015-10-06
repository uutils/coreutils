#![crate_name = "factor"]

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

extern crate getopts;
extern crate libc;
extern crate rand;

use numeric::*;
use prime_table::P_INVS_U64;
use rand::weak_rng;
use rand::distributions::{Range, IndependentSample};
use std::cmp::{max, min};
use std::io::{stdin, BufRead, BufReader, Write};
use std::num::Wrapping;
use std::mem::swap;

#[path="../common/util.rs"]
#[macro_use]
mod util;
mod numeric;
mod prime_table;

static NAME: &'static str = "factor";
static VERSION: &'static str = "1.0.0";

fn rho_pollard_pseudorandom_function(x: u64, a: u64, b: u64, num: u64) -> u64 {
    if num < 1 << 63 {
        (sm_mul(a, sm_mul(x, x, num), num) + b) % num
    } else {
        big_add(big_mul(a, big_mul(x, x, num), num), b, num)
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b > 0 {
        a %= b;
        swap(&mut a, &mut b);
    }
    a
}

fn rho_pollard_find_divisor(num: u64) -> u64 {
    let range = Range::new(1, num);
    let mut rng = rand::weak_rng();
    let mut x = range.ind_sample(&mut rng);
    let mut y = x;
    let mut a = range.ind_sample(&mut rng);
    let mut b = range.ind_sample(&mut rng);

    loop {
        x = rho_pollard_pseudorandom_function(x, a, b, num);
        y = rho_pollard_pseudorandom_function(y, a, b, num);
        y = rho_pollard_pseudorandom_function(y, a, b, num);
        let d = gcd(num, max(x, y) - min(x, y));
        if d == num {
            // Failure, retry with diffrent function
            x = range.ind_sample(&mut rng);
            y = x;
            a = range.ind_sample(&mut rng);
            b = range.ind_sample(&mut rng);
        } else if d > 1 {
            return d;
        }
    }
}

fn rho_pollard_factor(num: u64, factors: &mut Vec<u64>) {
    if is_prime(num) {
        factors.push(num);
        return;
    }
    let divisor = rho_pollard_find_divisor(num);
    rho_pollard_factor(divisor, factors);
    rho_pollard_factor(num / divisor, factors);
}

fn table_division(mut num: u64, factors: &mut Vec<u64>) {
    if num < 2 {
        return;
    }
    while num % 2 == 0 {
        num /= 2;
        factors.push(2);
    }
    if num == 1 {
        return;
    }
    if is_prime(num) {
        factors.push(num);
        return;
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
            let Wrapping(x) = Wrapping(num) * Wrapping(inv);    // x = num * inv mod 2^64
            if x <= ceil {
                num = x;
                factors.push(prime);
                if is_prime(num) {
                    factors.push(num);
                    return;
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
    rho_pollard_factor(num, factors);
    //}
}

fn print_factors(num: u64) {
    print!("{}:", num);

    let mut factors = Vec::new();
    // we always start with table division, and go from there
    table_division(num, &mut factors);
    factors.sort();

    for fac in factors.iter() {
        print!(" {}", fac);
    }
    println!("");
}

fn print_factors_str(num_str: &str) {
    if let Err(e) = num_str.parse::<u64>().and_then(|x| Ok(print_factors(x))) {
        show_warning!("{}: {}", num_str, e);
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "show this help message");
    opts.optflag("v", "version", "print the version and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f),
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
\t{0} [NUMBER]...
\t{0} [OPTION]

Print the prime factors of the given number(s). If none are specified,
read from standard input.",
                          NAME,
                          VERSION);

        print!("{}", opts.usage(&msg));
        return 1;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.free.is_empty() {
        for line in BufReader::new(stdin()).lines() {
            for number in line.unwrap().split_whitespace() {
                print_factors_str(number);
            }
        }
    } else {
        for num_str in matches.free.iter() {
            print_factors_str(num_str);
        }
    }
    0
}
