/*
* This file is part of the uutils coreutils package.
*
* (c) kwantam <kwantam@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

use std::iter::repeat;

// A lazy Sieve of Eratosthenes
// Not particularly efficient, but fine for generating a few thousand primes.
pub struct Sieve {
    inner: Box<Iterator<Item=u64>>,
    filts: Vec<u64>,
}

impl Iterator for Sieve {
    type Item = u64;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn next(&mut self) -> Option<u64> {
        while let Some(n) = self.inner.next() {
            if self.filts.iter().all(|&x| n % x != 0) {
                self.filts.push(n);
                return Some(n);
            }
        }
        None
    }
}

impl Sieve {
    #[inline]
    pub fn new() -> Sieve {
        fn next(s: &mut u64, t: u64) -> Option<u64> {
            let ret = Some(*s);
            *s = *s + t;
            ret
        }
        let next = next;

        let odds_by_3 = Box::new(repeat(2).scan(3, next)) as Box<Iterator<Item=u64>>;

        Sieve { inner: odds_by_3, filts: Vec::new() }
    }
}
