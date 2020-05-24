use crate::miller_rabin::Result::*;
use crate::{miller_rabin, Factors};
use numeric::*;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::{thread_rng, SeedableRng};
use std::cmp::{max, min};

fn find_divisor<A: Arithmetic>(n: u64) -> u64 {
    #![allow(clippy::many_single_char_names)]
    let mut rand = {
        let range = Uniform::new(1, n);
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
        move || range.sample(&mut rng)
    };

    let quadratic = |a, b| move |x| A::add(A::mul(a, A::mul(x, x, n), n), b, n);

    loop {
        let f = quadratic(rand(), rand());
        let mut x = rand();
        let mut y = x;

        loop {
            x = f(x);
            y = f(f(y));
            let d = gcd(n, max(x, y) - min(x, y));
            if d == n {
                // Failure, retry with a different quadratic
                break;
            } else if d > 1 {
                return d;
            }
        }
    }
}

fn _factor<A: Arithmetic>(mut num: u64) -> Factors {
    // Shadow the name, so the recursion automatically goes from “Big” arithmetic to small.
    let _factor = |n| {
        if n < 1 << 63 {
            _factor::<Small>(n)
        } else {
            _factor::<A>(n)
        }
    };

    let mut factors = Factors::new();
    if num == 1 {
        return factors;
    }

    match miller_rabin::test::<A>(num) {
        Prime => {
            factors.push(num);
            return factors;
        }

        Composite(d) => {
            num /= d;
            factors *= _factor(d)
        }

        Pseudoprime => {}
    };

    let divisor = find_divisor::<A>(num);
    factors *= _factor(divisor);
    factors *= _factor(num / divisor);
    factors
}

pub(crate) fn factor(n: u64) -> Factors {
    if n < 1 << 63 {
        _factor::<Small>(n)
    } else {
        _factor::<Big>(n)
    }
}
