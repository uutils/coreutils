// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 kwantam <kwantam@gmail.com>
// * (c) 2020 nicoo   <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker: ignore (ToDO) INVS

use crate::Factors;

include!(concat!(env!("OUT_DIR"), "/prime_table.rs"));

pub fn factor(num: &mut u64, factors: &mut Factors) {
    for &(prime, inv, ceil) in PRIME_INVERSIONS_U64 {
        if *num == 1 {
            break;
        }

        // inv = prime^-1 mod 2^64
        // ceil = floor((2^64-1) / prime)
        // if (num * inv) mod 2^64 <= ceil, then prime divides num
        // See https://math.stackexchange.com/questions/1251327/
        // for a nice explanation.
        let mut k = 0;
        loop {
            let x = num.wrapping_mul(inv);

            // While prime divides num
            if x <= ceil {
                *num = x;
                k += 1;
                #[cfg(feature = "coz")]
                coz::progress!("factor found");
            } else {
                if k > 0 {
                    factors.add(prime, k);
                }
                break;
            }
        }
    }
}

pub const CHUNK_SIZE: usize = 8;
pub fn factor_chunk(n_s: &mut [u64; CHUNK_SIZE], f_s: &mut [Factors; CHUNK_SIZE]) {
    for &(prime, inv, ceil) in PRIME_INVERSIONS_U64 {
        if n_s[0] == 1 && n_s[1] == 1 && n_s[2] == 1 && n_s[3] == 1 {
            break;
        }

        for (num, factors) in n_s.iter_mut().zip(f_s.iter_mut()) {
            if *num == 1 {
                continue;
            }
            let mut k = 0;
            loop {
                let x = num.wrapping_mul(inv);

                // While prime divides num
                if x <= ceil {
                    *num = x;
                    k += 1;
                } else {
                    if k > 0 {
                        factors.add(prime, k);
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Factors;
    use quickcheck::quickcheck;
    use rand::{rngs::SmallRng, Rng, SeedableRng};

    quickcheck! {
        fn chunk_vs_iter(seed: u64) -> () {
            let mut rng = SmallRng::seed_from_u64(seed);
            let mut n_c: [u64; CHUNK_SIZE] = rng.gen();
            let mut f_c: [Factors; CHUNK_SIZE] = rng.gen();

            let mut n_i = n_c;
            let mut f_i = f_c.clone();
            for (n, f) in n_i.iter_mut().zip(f_i.iter_mut()) {
                factor(n, f);
            }

            factor_chunk(&mut n_c, &mut f_c);

            assert_eq!(n_i, n_c);
            assert_eq!(f_i, f_c);
        }
    }
}
