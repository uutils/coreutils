use crate::Factors;
use std::num::Wrapping;

include!(concat!(env!("OUT_DIR"), "/prime_table.rs"));

pub(crate) fn factor(mut num: u64) -> (Factors, u64) {
    let mut factors = Factors::new();
    for &(prime, inv, ceil) in P_INVS_U64 {
        if num == 1 {
            break;
        }

        // inv = prime^-1 mod 2^64
        // ceil = floor((2^64-1) / prime)
        // if (num * inv) mod 2^64 <= ceil, then prime divides num
        // See https://math.stackexchange.com/questions/1251327/
        // for a nice explanation.
        loop {
            let Wrapping(x) = Wrapping(num) * Wrapping(inv);

            // While prime divides num
            if x <= ceil {
                num = x;
                factors.push(prime);
            } else {
                break;
            }
        }
    }

    // do we still have more factoring to do?
    // Decide whether to use Pollard Rho or slow divisibility based on
    // number's size:
    //if num >= 1 << 63 {
    // number is too big to use rho pollard without overflowing
    //trial_division_slow(num, factors);
    //} else if num > 1 {
    // number is still greater than 1, but not so big that we have to worry
    (factors, num)
    //}
}
