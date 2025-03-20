// spell-checker:ignore nonrepeating

use std::{collections::HashSet, ops::RangeInclusive};

use rand::{Rng, seq::SliceRandom};

use crate::WrappedRng;

enum NumberSet {
    AlreadyListed(HashSet<usize>),
    Remaining(Vec<usize>),
}

pub(crate) struct NonrepeatingIterator<'a> {
    range: RangeInclusive<usize>,
    rng: &'a mut WrappedRng,
    remaining_count: usize,
    buf: NumberSet,
}

impl<'a> NonrepeatingIterator<'a> {
    pub(crate) fn new(
        range: RangeInclusive<usize>,
        rng: &'a mut WrappedRng,
        amount: usize,
    ) -> Self {
        let capped_amount = if range.start() > range.end() {
            0
        } else if range == (0..=usize::MAX) {
            amount
        } else {
            amount.min(range.end() - range.start() + 1)
        };
        NonrepeatingIterator {
            range,
            rng,
            remaining_count: capped_amount,
            buf: NumberSet::AlreadyListed(HashSet::default()),
        }
    }

    fn produce(&mut self) -> usize {
        debug_assert!(self.range.start() <= self.range.end());
        match &mut self.buf {
            NumberSet::AlreadyListed(already_listed) => {
                let chosen = loop {
                    let guess = self.rng.random_range(self.range.clone());
                    let newly_inserted = already_listed.insert(guess);
                    if newly_inserted {
                        break guess;
                    }
                };
                // Once a significant fraction of the interval has already been enumerated,
                // the number of attempts to find a number that hasn't been chosen yet increases.
                // Therefore, we need to switch at some point from "set of already returned values" to "list of remaining values".
                let range_size = (self.range.end() - self.range.start()).saturating_add(1);
                if number_set_should_list_remaining(already_listed.len(), range_size) {
                    let mut remaining = self
                        .range
                        .clone()
                        .filter(|n| !already_listed.contains(n))
                        .collect::<Vec<_>>();
                    assert!(remaining.len() >= self.remaining_count);
                    remaining.partial_shuffle(&mut self.rng, self.remaining_count);
                    remaining.truncate(self.remaining_count);
                    self.buf = NumberSet::Remaining(remaining);
                }
                chosen
            }
            NumberSet::Remaining(remaining_numbers) => {
                debug_assert!(!remaining_numbers.is_empty());
                // We only enter produce() when there is at least one actual element remaining, so popping must always return an element.
                remaining_numbers.pop().unwrap()
            }
        }
    }
}

impl Iterator for NonrepeatingIterator<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.range.is_empty() || self.remaining_count == 0 {
            return None;
        }
        self.remaining_count -= 1;
        Some(self.produce())
    }
}

// This could be a method, but it is much easier to test as a stand-alone function.
fn number_set_should_list_remaining(listed_count: usize, range_size: usize) -> bool {
    // Arbitrarily determine the switchover point to be around 25%. This is because:
    // - HashSet has a large space overhead for the hash table load factor.
    // - This means that somewhere between 25-40%, the memory required for a "positive" HashSet and a "negative" Vec should be the same.
    // - HashSet has a small but non-negligible overhead for each lookup, so we have a slight preference for Vec anyway.
    // - At 25%, on average 1.33 attempts are needed to find a number that hasn't been taken yet.
    // - Finally, "24%" is computationally the simplest:
    listed_count >= range_size / 4
}

#[cfg(test)]
// Since the computed value is a bool, it is more readable to write the expected value out:
#[allow(clippy::bool_assert_comparison)]
mod test_number_set_decision {
    use super::number_set_should_list_remaining;

    #[test]
    fn test_stay_positive_large_remaining_first() {
        assert_eq!(false, number_set_should_list_remaining(0, usize::MAX));
    }

    #[test]
    fn test_stay_positive_large_remaining_second() {
        assert_eq!(false, number_set_should_list_remaining(1, usize::MAX));
    }

    #[test]
    fn test_stay_positive_large_remaining_tenth() {
        assert_eq!(false, number_set_should_list_remaining(9, usize::MAX));
    }

    #[test]
    fn test_stay_positive_smallish_range_first() {
        assert_eq!(false, number_set_should_list_remaining(0, 12345));
    }

    #[test]
    fn test_stay_positive_smallish_range_second() {
        assert_eq!(false, number_set_should_list_remaining(1, 12345));
    }

    #[test]
    fn test_stay_positive_smallish_range_tenth() {
        assert_eq!(false, number_set_should_list_remaining(9, 12345));
    }

    #[test]
    fn test_stay_positive_small_range_not_too_early() {
        assert_eq!(false, number_set_should_list_remaining(1, 10));
    }

    // Don't want to test close to the border, in case we decide to change the threshold.
    // However, at 50% coverage, we absolutely should switch:
    #[test]
    fn test_switch_half() {
        assert_eq!(true, number_set_should_list_remaining(1234, 2468));
    }

    // Ensure that the decision is monotonous:
    #[test]
    fn test_switch_late1() {
        assert_eq!(true, number_set_should_list_remaining(12340, 12345));
    }

    #[test]
    fn test_switch_late2() {
        assert_eq!(true, number_set_should_list_remaining(12344, 12345));
    }

    // Ensure that we are overflow-free:
    #[test]
    fn test_no_crash_exceed_max_size1() {
        assert_eq!(false, number_set_should_list_remaining(12345, usize::MAX));
    }

    #[test]
    fn test_no_crash_exceed_max_size2() {
        assert_eq!(
            true,
            number_set_should_list_remaining(usize::MAX - 1, usize::MAX)
        );
    }

    #[test]
    fn test_no_crash_exceed_max_size3() {
        assert_eq!(
            true,
            number_set_should_list_remaining(usize::MAX, usize::MAX)
        );
    }
}
