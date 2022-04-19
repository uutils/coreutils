// * This file is part of the uutils coreutils package.
// *
// * (c) kwantam <kwantam@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore (ToDO) filts, minidx, minkey paridx

use std::iter::{Chain, Copied, Cycle};
use std::slice::Iter;

/// A lazy Sieve of Eratosthenes.
///
/// This is a reasonably efficient implementation based on
/// O'Neill, M. E. "[The Genuine Sieve of Eratosthenes.](http://dx.doi.org/10.1017%2FS0956796808007004)"
/// Journal of Functional Programming, Volume 19, Issue 1, 2009, pp.  95--106.
#[derive(Default)]
pub struct Sieve {
    inner: Wheel,
    filts: PrimeHeap,
}

impl Iterator for Sieve {
    type Item = u64;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn next(&mut self) -> Option<u64> {
        for n in &mut self.inner {
            let mut prime = true;
            while let Some((next, inc)) = self.filts.peek() {
                // need to keep checking the min element of the heap
                // until we've found an element that's greater than n
                if next > n {
                    break; // next heap element is bigger than n
                }

                if next == n {
                    // n == next, and is composite.
                    prime = false;
                }
                // Increment the element in the prime heap.
                self.filts.replace((next + inc, inc));
            }

            if prime {
                // this is a prime; add it to the heap
                self.filts.insert(n);
                return Some(n);
            }
        }
        None
    }
}

impl Sieve {
    fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    #[inline]
    pub fn primes() -> PrimeSieve {
        INIT_PRIMES.iter().copied().chain(Self::new())
    }

    #[allow(dead_code)]
    #[inline]
    pub fn odd_primes() -> PrimeSieve {
        INIT_PRIMES[1..].iter().copied().chain(Self::new())
    }
}

pub type PrimeSieve = Chain<Copied<Iter<'static, u64>>, Sieve>;

/// An iterator that generates an infinite list of numbers that are
/// not divisible by any of 2, 3, 5, or 7.
struct Wheel {
    next: u64,
    increment: Cycle<Iter<'static, u64>>,
}

impl Iterator for Wheel {
    type Item = u64;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, None)
    }

    #[inline]
    fn next(&mut self) -> Option<u64> {
        let increment = self.increment.next().unwrap(); // infinite iterator, no check necessary
        let ret = self.next;
        self.next = ret + increment;
        Some(ret)
    }
}

impl Wheel {
    #[inline]
    fn new() -> Self {
        Self {
            next: 11u64,
            increment: WHEEL_INCS.iter().cycle(),
        }
    }
}

impl Default for Wheel {
    fn default() -> Self {
        Self::new()
    }
}

/// The increments of a wheel of circumference 210
/// (i.e., a wheel that skips all multiples of 2, 3, 5, 7)
const WHEEL_INCS: &[u64] = &[
    2, 4, 2, 4, 6, 2, 6, 4, 2, 4, 6, 6, 2, 6, 4, 2, 6, 4, 6, 8, 4, 2, 4, 2, 4, 8, 6, 4, 6, 2, 4, 6,
    2, 6, 6, 4, 2, 4, 6, 2, 6, 4, 2, 4, 2, 10, 2, 10,
];
const INIT_PRIMES: &[u64] = &[2, 3, 5, 7];

/// A min-heap of "infinite lists" of prime multiples, where a list is
/// represented as (head, increment).
#[derive(Debug, Default)]
struct PrimeHeap {
    data: Vec<(u64, u64)>,
}

impl PrimeHeap {
    fn peek(&self) -> Option<(u64, u64)> {
        if let Some(&(x, y)) = self.data.get(0) {
            Some((x, y))
        } else {
            None
        }
    }

    fn insert(&mut self, next: u64) {
        let mut idx = self.data.len();
        let key = next * next;

        let item = (key, next);
        self.data.push(item);
        loop {
            // break if we've bubbled to the top
            if idx == 0 {
                break;
            }

            let paridx = (idx - 1) / 2;
            let (k, _) = self.data[paridx];
            if key < k {
                // bubble up, found a smaller key
                self.data.swap(idx, paridx);
                idx = paridx;
            } else {
                // otherwise, parent is smaller, so we're done
                break;
            }
        }
    }

    fn remove(&mut self) -> (u64, u64) {
        let ret = self.data.swap_remove(0);

        let mut idx = 0;
        let len = self.data.len();
        let (key, _) = self.data[0];
        loop {
            let child1 = 2 * idx + 1;
            let child2 = 2 * idx + 2;

            // no more children
            if child1 >= len {
                break;
            }

            // find lesser child
            let (c1key, _) = self.data[child1];
            let (minidx, minkey) = if child2 >= len {
                (child1, c1key)
            } else {
                let (c2key, _) = self.data[child2];
                if c1key < c2key {
                    (child1, c1key)
                } else {
                    (child2, c2key)
                }
            };

            if minkey < key {
                self.data.swap(minidx, idx);
                idx = minidx;
                continue;
            }

            // smaller than both children, so done
            break;
        }

        ret
    }

    /// More efficient than inserting and removing in two steps
    /// because we save one traversal of the heap.
    fn replace(&mut self, next: (u64, u64)) -> (u64, u64) {
        self.data.push(next);
        self.remove()
    }
}
