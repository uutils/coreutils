// spell-checker: ignore (ToDO) INVS

use std::num::Wrapping;

use crate::Factors;

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

    (factors, num)
}
