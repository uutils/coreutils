// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::{thread_rng, SeedableRng};
use std::cmp::{max, min};

use crate::numeric::*;

pub(crate) fn find_divisor<A: Arithmetic>(n: A) -> u64 {
    #![allow(clippy::many_single_char_names)]
    let mut rand = {
        let range = Uniform::new(1, n.modulus());
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
        move || n.to_mod(range.sample(&mut rng))
    };

    let quadratic = |a, b| move |x| n.add(n.mul(a, n.mul(x, x)), b);

    loop {
        let f = quadratic(rand(), rand());
        let mut x = rand();
        let mut y = x;

        loop {
            x = f(x);
            y = f(f(y));
            let d = {
                let _x = n.to_u64(x);
                let _y = n.to_u64(y);
                gcd(n.modulus(), max(_x, _y) - min(_x, _y))
            };
            if d == n.modulus() {
                // Failure, retry with a different quadratic
                break;
            } else if d > 1 {
                return d;
            }
        }
    }
}
