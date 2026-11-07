// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use rustc_hash::{FxBuildHasher, FxHashMap};
use std::ops::RangeInclusive;

use uucore::error::{UResult, USimpleError};
use uucore::translate;

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
/// - We store the array backwards so that we can remove selected values with `pop()` and
///   retain the allocation while consuming it instead of repeatedly reallocating it.
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
    /// Materialized permutation used for relatively small ranges.
    ///
    /// Values are stored in reverse order so that Fisher-Yates can efficiently
    /// remove the selected value with `pop()`.
    Full(Vec<u64>),

    /// Lazily materialized permutation used for large ranges.
    ///
    /// `items` contains only positions whose value differs from the identity
    /// permutation.
    Sparse {
        next: Option<u64>,
        end: u64,
        items: FxHashMap<u64, u64>,
    },
}

impl<'a> NonrepeatingIterator<'a> {
    pub(crate) fn new(
        range: RangeInclusive<u64>,
        rng: &'a mut WrappedRng,
        head_count: Option<usize>,
    ) -> UResult<Self> {
        // Avoid allocating enormous vectors for ranges that are unlikely
        // to be consumed completely.
        const TOO_LARGE_VEC_SIZE: usize = 16_777_216;

        // For callers without a consumption bound, keep the hash map's initial
        // allocation small. The shuf range path passes usize::MAX when no
        // --head-count was requested, so that path intentionally reserves the
        // full range and fails cleanly for impossible full permutations.
        const MAX_SPARSE_CAPACITY: usize = 128;
        // Lower bound on the length of the range.
        let range_len = range.size_hint().0;
        let full_range_requested = head_count.is_some_and(|count| count >= range_len);

        // For reasonably sized ranges, or when the whole range is requested,
        // use a normal Fisher-Yates shuffle. The sparse representation avoids
        // allocation only when it can stop before materializing the range.
        if range_len < TOO_LARGE_VEC_SIZE || full_range_requested {
            let mut items = Vec::new();

            if items.try_reserve_exact(range_len).is_ok() {
                // Preserve the backwards representation used by the
                // Fisher-Yates implementation.
                items.extend(range.rev());

                return Ok(Self {
                    rng,
                    values: Values::Full(items),
                });
            }
        }

        // Sparse representation:
        // reserve approximately the number of values we expect to consume.
        //
        let capacity = head_count.unwrap_or(MAX_SPARSE_CAPACITY).min(range_len);

        let mut items = FxHashMap::with_hasher(FxBuildHasher);

        items
            .try_reserve(capacity)
            .map_err(|_| USimpleError::new(1, translate!("shuf-error-memory-exhausted")))?;

        Ok(Self {
            rng,
            values: Values::Sparse {
                next: Some(*range.start()),
                end: *range.end(),
                items,
            },
        })
    }

    #[inline]
    fn produce(&mut self) -> Option<UResult<u64>> {
        match &mut self.values {
            Values::Full(items) => {
                let len = items.len();
                let last = len.checked_sub(1)?;

                // Fisher-Yates: choose an element from [0, len -1].
                let selected = match self.rng.choose_from_range(0..=(last as u64)) {
                    Ok(selected) => selected as usize,
                    Err(error) => return Some(Err(error)),
                };

                // The vector is stored backwards, so convert the selected
                // index to the corresponding index in the reversed vector.
                let selected = last - selected;

                items.swap(selected, last);

                Some(Ok(items.pop()?))
            }

            Values::Sparse { next, end, items } => {
                let current = (*next)?;

                // Remove the lazily materialized value at the current
                // position. If none exists, the identity permutation applies.
                let current_value = items.remove(&current).unwrap_or(current);

                // Select uniformly from the remaining range.
                let selected = match self.rng.choose_from_bounds(current, *end) {
                    Ok(selected) => selected,
                    Err(error) => return Some(Err(error)),
                };

                let value = if selected == current {
                    current_value
                } else {
                    // Move the value at `current` to `selected`.
                    //
                    // If `selected` was already materialized, its previous
                    // value is returned. Otherwise, its identity value is
                    // returned.
                    items.insert(selected, current_value).unwrap_or(selected)
                };

                *next = current.checked_add(1);

                Some(Ok(value))
            }
        }
    }
}

impl Iterator for NonrepeatingIterator<'_> {
    type Item = UResult<u64>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Values::Full(items) = &self.values {
            if items.is_empty() {
                return None;
            }
        } else if let Values::Sparse { next, end, items } = &self.values {
            let next_value = (*next)?;

            // Except for a range ending at u64::MAX, the next value is one
            // past the end after the last draw. Do not sample that empty range.
            if next_value > *end {
                return None;
            }

            // Once the sparse table is full, materialize the remaining
            // permutation.
            if items.len() >= items.capacity() {
                let values = hashmap_to_vec(next_value..=*end, items);

                self.values = Values::Full(values);
            }
        }

        self.produce()
    }
}

#[inline]
fn hashmap_to_vec(range: RangeInclusive<u64>, map: &FxHashMap<u64, u64>) -> Vec<u64> {
    let len = range.size_hint().0;
    let mut values = Vec::with_capacity(len);

    for idx in range.rev() {
        values.push(map.get(&idx).copied().unwrap_or(idx));
    }

    values
}
