use crate::Factors;
use numeric::*;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::{thread_rng, SeedableRng};
use std::cmp::{max, min};

fn pseudorandom_function(x: u64, a: u64, b: u64, num: u64) -> u64 {
    if num < 1 << 63 {
        (sm_mul(a, sm_mul(x, x, num), num) + b) % num
    } else {
        big_add(big_mul(a, big_mul(x, x, num), num), b, num)
    }
}

fn find_divisor(num: u64) -> u64 {
    #![allow(clippy::many_single_char_names)]
    let range = Uniform::new(1, num);
    let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
    let mut x = range.sample(&mut rng);
    let mut y = x;
    let mut a = range.sample(&mut rng);
    let mut b = range.sample(&mut rng);

    loop {
        x = pseudorandom_function(x, a, b, num);
        y = pseudorandom_function(y, a, b, num);
        y = pseudorandom_function(y, a, b, num);
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

pub(crate) fn factor(num: u64) -> Factors {
    let mut factors = Factors::new();
    if num == 1 { return factors; }
    if is_prime(num) {
        factors.push(num);
        return factors;
    }

    let divisor = find_divisor(num);
    factors *= factor(divisor);
    factors *= factor(num / divisor);
    factors
}
