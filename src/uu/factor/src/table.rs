// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 kwantam <kwantam@gmail.com>
// * (c) 2020 nicoo   <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker: ignore (ToDO) INVS

use std::num::Wrapping;

use crate::Factors;

include!(concat!(env!("OUT_DIR"), "/prime_table.rs"));

pub(crate) fn factor(mut num: u64) -> (Factors, u64) {
    let mut factors = Factors::one();
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
