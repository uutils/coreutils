/*
* This file is part of the uutils coreutils package.
*
* (c) kwantam <kwantam@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

extern crate libc;
extern crate rand;

use rand::{weak_rng, Rng};
use sieve::Sieve;
use std::io::Write;
use std::process::{Command, Stdio};

#[path="../src/factor/sieve.rs"]
mod sieve;

const NUM_PRIMES: usize = 10000;
const LOG_PRIMES: f64 = 14.0;   // ceil(log2(NUM_PRIMES))

const NUM_TESTS: usize = 1000;
const PROGNAME: &'static str = "./factor";

#[test]
fn test_random() {

    let mut primes = Sieve::new().take(NUM_PRIMES - 1).collect::<Vec<u64>>();
    primes.push(2);
    let primes = primes;

    let mut rng = weak_rng();
    let mut rand_gt = move |min: u64| {
        let mut product = 1u64;
        let mut factors = Vec::new();
        while product < min {
            // log distribution---higher probability for lower numbers
            let mut factor;
            loop {
                let next = rng.gen_range(0f64, LOG_PRIMES).exp2().floor() as usize;
                if next < NUM_PRIMES {
                    factor = primes[next];
                    break;
                }
            }
            let factor = factor;

            match product.checked_mul(factor) {
                Some(p) => {
                    product = p;
                    factors.push(factor);
                },
                None => break,
            };
        }

        factors.sort();
        (product, factors)
    };

    // build an input and expected output string from factor
    let mut instring = String::new();
    let mut outstring = String::new();
    for _ in 0..NUM_TESTS {
        let (product, factors) = rand_gt(1 << 63);
        instring.push_str(&(format!("{} ", product))[..]);

        outstring.push_str(&(format!("{}:", product))[..]);
        for factor in factors.iter() {
            outstring.push_str(&(format!(" {}", factor))[..]);
        }
        outstring.push_str("\n");
    }

    // now run factor
    let mut process = Command::new(PROGNAME)
                                   .stdin(Stdio::piped())
                                   .stdout(Stdio::piped())
                                   .spawn()
                                   .unwrap_or_else(|e| panic!("{}", e));

    process.stdin.take().unwrap_or_else(|| panic!("Could not take child process stdin"))
        .write_all(instring.as_bytes()).unwrap_or_else(|e| panic!("{}", e));

    let output = process.wait_with_output().unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(&output.stdout[..], outstring.as_bytes());
}
