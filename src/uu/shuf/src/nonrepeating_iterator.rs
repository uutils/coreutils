// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use rustc_hash::FxHashMap;
use std::ops::RangeInclusive;

use uucore::error::UResult;

use crate::WrappedRng;

/// An iterator that samples from an integer range without repetition.
///
/// This is based on Fisher-Yates, and it's required for backward compatibility
/// that it behaves exactly like Fisher-Yates if --random-source or --random-seed
/// is used. But we have a few tricks:
///
/// - In the beginning we use a hash table instead of an array. This way we lazily
///   keep track of swaps without allocating the entire range upfront.
///
/// - When the hash table starts to get big relative to the remaining items
///   we switch over to an array.
///
/// - We store the array backwards so that we can shrink it as we go and free excess
///   memory every now and then.
///
/// Both the hash table and the array give the same output.
///
/// There's room for optimization:
///
/// - Switching over from the hash table to the array is costly. If we happen to know
///   (through --head-count) that only few draws remain then it would be better not
///   to switch.
///
/// - If the entire range gets used then we might as well allocate an array to start
///   with. But if the user e.g. pipes through `head` rather than using --head-count
///   we can't know whether that's the case, so there's a tradeoff.
///
///   GNU decides the other way: --head-count is noticeably faster than | head.
pub(crate) struct NonrepeatingIterator<'a> {
    rng: &'a mut WrappedRng,
    values: Values,
}

enum Values {
    Full(Vec<u64>),
    Sparse(RangeInclusive<u64>, FxHashMap<u64, u64>),
}

impl<'a> NonrepeatingIterator<'a> {
    pub(crate) fn new(range: RangeInclusive<u64>, rng: &'a mut WrappedRng) -> Self {
        const MAX_CAPACITY: usize = 128; // todo: optimize this
        let capacity = (range.size_hint().0).min(MAX_CAPACITY);
        let values = Values::Sparse(
            range,
            FxHashMap::with_capacity_and_hasher(capacity, rustc_hash::FxBuildHasher),
        );
        NonrepeatingIterator { rng, values }
    }

    fn produce(&mut self) -> UResult<u64> {
        match &mut self.values {
            Values::Full(items) => {
                let this_idx = items.len() - 1;

                let other_idx = self.rng.choose_from_range(0..=items.len() as u64 - 1)? as usize;
                // Flip the index to pretend we're going left-to-right
                let other_idx = items.len() - other_idx - 1;

                items.swap(this_idx, other_idx);

                let val = items.pop().unwrap();
                if items.len().is_power_of_two() && items.len() >= 512 {
                    items.shrink_to_fit();
                }
                Ok(val)
            }
            Values::Sparse(range, items) => {
                let this_idx = *range.start();
                let this_val = items.remove(&this_idx).unwrap_or(this_idx);

                let other_idx = self.rng.choose_from_range(range.clone())?;

                let val = if this_idx == other_idx {
                    this_val
                } else {
                    items.insert(other_idx, this_val).unwrap_or(other_idx)
                };
                *range = *range.start() + 1..=*range.end();

                Ok(val)
            }
        }
    }
}

impl Iterator for NonrepeatingIterator<'_> {
    type Item = UResult<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.values {
            Values::Full(items) if items.is_empty() => return None,
            Values::Full(_) => (),
            Values::Sparse(range, _) if range.is_empty() => return None,
            Values::Sparse(range, items) => {
                if items.len() >= items.capacity() {
                    self.values = Values::Full(hashmap_to_vec(range.clone(), items));
                }
            }
        }

        Some(self.produce())
    }
}

fn hashmap_to_vec(range: RangeInclusive<u64>, map: &FxHashMap<u64, u64>) -> Vec<u64> {
    let lookup = |idx| *map.get(&idx).unwrap_or(&idx);
    range.rev().map(lookup).collect()
}
