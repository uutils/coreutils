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

pub(crate) fn factor(mut num: u64) -> Factors {
    let mut factors = Factors::new();
    if num == 1 {
        return factors;
    }

    match if num < 1 << 63 {
        miller_rabin::test::<Small>(num)
    } else {
        miller_rabin::test::<Big>(num)
    } {
        Prime => {
            factors.push(num);
            return factors;
        }

        Composite(d) => {
            num /= d;
            factors *= factor(d);
        }

        Pseudoprime => {}
    };

    let divisor = if num < 1 << 63 {
        find_divisor::<Small>(num)
    } else {
        find_divisor::<Big>(num)
    };
    factors *= factor(divisor);
    factors *= factor(num / divisor);
    factors
}
