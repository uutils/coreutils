use crate::numeric::*;

fn witness(mut a: u64, exponent: u64, m: u64) -> bool {
    if a == 0 {
        return false;
    }

    let mul = if m < 1 << 63 {
        sm_mul as fn(u64, u64, u64) -> u64
    } else {
        big_mul as fn(u64, u64, u64) -> u64
    };

    if pow(a, m - 1, m, mul) != 1 {
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
        if a == m - 1 {
            return false;
        }
        a = mul(a, a, m);
    }
}

// Deterministic Miller-Rabin primality-checking algorithm, adapted to extract
// (some) dividers; it will fail to factor strong pseudoprimes.
pub(crate) fn is_prime(num: u64) -> bool {
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
    let witnesses = [2, 325, 9_375, 28_178, 450_775, 9_780_504, 1_795_265_022];
    !witnesses
        .iter()
        .any(|&wit| witness(wit % num, exponent, num))
}
