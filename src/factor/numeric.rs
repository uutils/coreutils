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

use std::u64::MAX as MAX_U64;
use std::num::Wrapping;

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
fn pow(mut a: u64, mut b: u64, m: u64, mul: fn(u64, u64, u64) -> u64) -> u64 {
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

fn witness(mut a: u64, exponent: u64, m: u64) -> bool {
    if a == 0 {
        return false;
    }

    let mul = if m < 1 << 63 {
        sm_mul as fn(u64, u64, u64) -> u64
    } else {
        big_mul as fn(u64, u64, u64) -> u64
    };

    if pow(a, m-1, m, mul) != 1 {
        return true;
    }
    a = pow(a, exponent, m, mul);
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
        a = mul(a, a, m);
    }
}

// uses deterministic (i.e., fixed witness set) Miller-Rabin test
pub fn is_prime(num: u64) -> bool {
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

    // These witnesses detect all composites up to at least 2^64.
    // Discovered by Jim Sinclair, according to http://miller-rabin.appspot.com
    let witnesses = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];
    ! witnesses.iter().any(|&wit| witness(wit % num, exponent, num))
}
