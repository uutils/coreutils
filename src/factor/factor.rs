#![crate_name = "factor"]
#![feature(collections, core, io, rustc_private)]

/*
* This file is part of the uutils coreutils package.
*
* (c) T. Jameson Little <t.jameson.little@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

extern crate getopts;
extern crate libc;
extern crate rand;

use std::vec::Vec;
use std::old_io::BufferedReader;
use std::old_io::stdio::stdin_raw;
use std::cmp::{max,min};
use std::mem::swap;
use rand::distributions::{IndependentSample, Range};

#[path="../common/util.rs"]
#[macro_use]
mod util;

static VERSION: &'static str = "1.0.0";
static NAME: &'static str = "factor";

// computes (a + b) % m using the russian peasant algorithm
fn multiply(mut a: u64, mut b: u64, m: u64) -> u64 {
    let mut result = 0;
    while b > 0 {
        if b & 1 > 0 {
            result = (result + a) % m;
        }
        a = (a << 1) % m;
        b >>= 1;
    }
    result
}

// computes a.pow(b) % m
fn pow(mut a: u64, mut b: u64, m: u64) -> u64 {
    let mut result = 1;
    while b > 0 {
        if b & 1 > 0 {
            result = multiply(result, a, m);
        }
        a = multiply(a, a, m);
        b >>= 1;
    }
    result
}

fn witness(mut a: u64, exponent: u64, m: u64) -> bool {
    if a == 0 {
        return false;
    }
    if pow(a, m-1, m) != 1 {
        return true;
    }
    a = pow(a, exponent, m);
    if a == 1 {
        return false;
    }
    loop {
        if a == 1 {
            return true;
        }
        if a == m-1 {
            return false;
        }
        a = multiply(a, a, m);
    }
}

// uses the Miller-Rabin test
fn is_prime(num: u64) -> bool {
    if num < 2 {
        return false;
    }
    if num % 2 == 0 {
        return num == 2;
    }
    let mut exponent = num - 1;
    while exponent & 1 == 0 {
        exponent >>= 1;
    }
    let witnesses = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];
    for wit in witnesses.iter() {
        if witness(*wit % num, exponent, num) {
            return false;
        }
    }
    true
}

fn trial_division(mut num: u64) -> Vec<u64> {
    let mut ret = Vec::new();

    if num < 2 {
        return ret;
    }
    while num % 2 == 0 {
        num /= 2;
        ret.push(2);
    }
    let mut i = 3;
    while i * i <= num {
        while num % i == 0 {
            num /= i;
            ret.push(i);
        }
        i += 2;
    }
    if num > 1 {
        ret.push(num);
    }
    ret
}

fn rho_pollard_pseudorandom_function(x: u64, a: u64, b: u64, num: u64) -> u64 {
    (multiply(a, multiply(x, x, num), num) + b) % num
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
        let d = gcd(num, max(x,y) - min(x,y));
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

fn rho_pollard_factor(num: u64) -> Vec<u64> {
    let mut ret = Vec::new();
    if is_prime(num) {
        ret.push(num);
        return ret;
    }
    let divisor = rho_pollard_find_divisor(num);
    ret.push_all(rho_pollard_factor(divisor).as_slice());
    ret.push_all(rho_pollard_factor(num/divisor).as_slice());
    ret
}

fn print_factors(num: u64) {
    print!("{}:", num);

    // Rho-Pollard is slower for small numbers and may cause 64-bit overflows
    // for numbers bigger than 1 << 63, hence the constraints
    let mut factors = if num < 1 << 63 && num > 1 << 40 {
        rho_pollard_factor(num)
    } else {
        trial_division(num)
    };
    factors.sort();
    for fac in factors.iter() {
        print!(" {}", fac);
    }
    println!("");
}

fn print_factors_str(num_str: &str) {
    let num = match num_str.parse::<u64>() {
        Ok(x) => x,
        Err(e)=> { crash!(1, "{} not a number: {}", num_str, e); }
    };
    print_factors(num);
}

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
    let opts = [
        getopts::optflag("h", "help", "show this help message"),
        getopts::optflag("v", "version", "print the version and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        print!("{program} {version}\n\
                \n\
                Usage:\n\
                \t{program} [NUMBER]...\n\
                \t{program} [OPTION]\n\
                \n\
                {usage}", program = program, version = VERSION, usage = getopts::usage("Print the prime factors of the given number(s). \
                                        If none are specified, read from standard input.", &opts));
        return 1;
    }
    if matches.opt_present("version") {
        println!("{} {}", program, VERSION);
        return 0;
    }

    if matches.free.is_empty() {
        for line in BufferedReader::new(stdin_raw()).lines() {
            print_factors_str(line.unwrap().as_slice().trim());
        }
    } else {
        for num_str in matches.free.iter() {
            print_factors_str(num_str.as_slice());
        }
    }
    0
}
