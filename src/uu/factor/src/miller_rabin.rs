use crate::numeric::*;

// Small set of bases for the Miller-Rabin prime test, valid for all 64b integers;
//  discovered by Jim Sinclair on 2011-04-20, see miller-rabin.appspot.com
const BASIS: [u64; 7] = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];

// Deterministic Miller-Rabin primality-checking algorithm, adapted to extract
// (some) dividers; it will fail to factor strong pseudoprimes.
pub(crate) fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }

    let d = (n - 1).trailing_zeros();
    let r = (n - 1) >> d;

    let mul = if n < 1 << 63 {
        sm_mul as fn(u64, u64, u64) -> u64
    } else {
        big_mul as fn(u64, u64, u64) -> u64
    };

    for a in BASIS.iter() {
        let mut x = a % n;
        if x == 0 {
            break;
        }

        if pow(x, n - 1, n, mul) != 1 {
            return false;
        }
        x = pow(x, r, n, mul);
        if x == 1 || x == n - 1 {
            break;
        }

        loop {
            let y = mul(x, x, n);
            if y == 1 {
                return false;
            }
            if y == n - 1 {
                // This basis element is not a witness of `n` being composite.
                // Keep looking.
                break;
            }
            x = y;
        }
    }

    true
}
