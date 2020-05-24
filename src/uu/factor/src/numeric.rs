/*
* This file is part of the uutils coreutils package.
*
* (c) Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
* (c) kwantam <kwantam@gmail.com>
*     20150507 added big_ routines to prevent overflow when num > 2^63
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

use std::mem::swap;
use std::num::Wrapping;
use std::u64::MAX as MAX_U64;

pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b > 0 {
        a %= b;
        swap(&mut a, &mut b);
    }
    a
}

pub fn big_add(a: u64, b: u64, m: u64) -> u64 {
    let Wrapping(msb_mod_m) = Wrapping(MAX_U64) - Wrapping(m) + Wrapping(1);
    let msb_mod_m = msb_mod_m % m;

    let Wrapping(res) = Wrapping(a) + Wrapping(b);
    if b <= MAX_U64 - a {
        res
    } else {
        (res + msb_mod_m) % m
    }
}

// computes (a + b) % m using the russian peasant algorithm
// CAUTION: Will overflow if m >= 2^63
pub fn sm_mul(mut a: u64, mut b: u64, m: u64) -> u64 {
    let mut result = 0;
    while b > 0 {
        if b & 1 != 0 {
            result = (result + a) % m;
        }
        a = (a << 1) % m;
        b >>= 1;
    }
    result
}

// computes (a + b) % m using the russian peasant algorithm
// Only necessary when m >= 2^63; otherwise, just wastes time.
pub fn big_mul(mut a: u64, mut b: u64, m: u64) -> u64 {
    // precompute 2^64 mod m, since we expect to wrap
    let Wrapping(msb_mod_m) = Wrapping(MAX_U64) - Wrapping(m) + Wrapping(1);
    let msb_mod_m = msb_mod_m % m;

    let mut result = 0;
    while b > 0 {
        if b & 1 != 0 {
            let Wrapping(next_res) = Wrapping(result) + Wrapping(a);
            let next_res = next_res % m;
            result = if result <= MAX_U64 - a {
                next_res
            } else {
                (next_res + msb_mod_m) % m
            };
        }
        let Wrapping(next_a) = Wrapping(a) << 1;
        let next_a = next_a % m;
        a = if a < 1 << 63 {
            next_a
        } else {
            (next_a + msb_mod_m) % m
        };
        b >>= 1;
    }
    result
}

// computes a.pow(b) % m
pub(crate) fn pow(mut a: u64, mut b: u64, m: u64, mul: fn(u64, u64, u64) -> u64) -> u64 {
    let mut result = 1;
    while b > 0 {
        if b & 1 != 0 {
            result = mul(result, a, m);
        }
        a = mul(a, a, m);
        b >>= 1;
    }
    result
}
