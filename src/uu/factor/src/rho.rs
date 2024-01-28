// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::{thread_rng, SeedableRng};
use std::cmp::{max, min};

use crate::numeric::*;

pub(crate) fn find_divisor<A: Arithmetic>(input: A) -> u64 {
    let mut rand = {
        let range = Uniform::new(1, input.modulus());
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
        move || input.to_mod(range.sample(&mut rng))
    };

    let quadratic = |a, b| move |x| input.add(input.mul(a, input.mul(x, x)), b);

    loop {
        let f = quadratic(rand(), rand());
        let mut x = rand();
        let mut y = x;

        loop {
            x = f(x);
            y = f(f(y));
            let d = {
                let _x = input.to_u64(x);
                let _y = input.to_u64(y);
                gcd(input.modulus(), max(_x, _y) - min(_x, _y))
            };
            if d == input.modulus() {
                // Failure, retry with a different quadratic
                break;
            } else if d > 1 {
                return d;
            }
        }
    }
}
