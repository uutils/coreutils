/*
* This file is part of the uutils coreutils package.
*
* (c) Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

// computes (a + b) % m using the russian peasant algorithm
pub fn multiply(mut a: u64, mut b: u64, m: u64) -> u64 {
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
    for wit in witnesses.iter() {
        if witness(*wit % num, exponent, num) {
            return false;
        }
    }
    true
}
