use std::{collections::BTreeMap, path::PathBuf};

use fiemap::FiemapExtentFlags;
use uucore::display::Quotable;
use uucore::error::{UError, USimpleError};

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

#[derive(Default)]
pub struct SeenPhysicalExtents
{
    pub ranges: BTreeMap<u64, u64>,
    pub log_infos: bool,
}

impl SeenPhysicalExtents {

    pub fn get_overlapping_and_insert(&mut self, range: &Range) -> u64 {

        let mut same_or_before = self.ranges.range_mut(..range.start+1);

        let mut need_new_entry = true;
        let mut overlapping_sum: u64 = 0;
        if let Some((_start, end)) = same_or_before.next_back() {
            if *end >= range.end {
                return range.end - range.start; // fully covered, no new entry needed
            }
            if *end >= range.start {
                overlapping_sum += *end - range.start;
                *end = range.end;     // partially covered from begin.
                                      // Extend existing entry.
                need_new_entry = false;
            }
        }

        if need_new_entry {
            // element before doesn't exist or doesn't overlap, insert new
            self.ranges.insert(range.start, range.end);
        }

        let mut current_pos = range.start+1;
        loop {
            let mut after = self.ranges.range(current_pos..);

            if let Some((&start, end)) = after.next() {
                if start >= range.end {
                    return overlapping_sum; // fully outside, done
                }

                if *end > range.end {
                    overlapping_sum += range.end - start;
                    let new_start = range.end;
                    let new_end = *end;
                    self.ranges.insert(new_start, new_end);
                    self.ranges.remove(&start);
                    return overlapping_sum; // partially outside, adapt, done
                }

                overlapping_sum += *end - start; // fully inside, remove, continue
                current_pos = *end;
                self.ranges.remove(&start);
            }
            else {
                return overlapping_sum;
            }
        }
    }

    pub fn get_total_overlap_and_insert(&mut self, path: PathBuf) -> (u64, Vec<Box<dyn UError>>) {

        let mut errors = Vec::new();

        let extents =
            match fiemap::fiemap(path.clone()) {
                Ok(result) => result,
                Err(e) => {
                    errors.push(USimpleError::new(1,
                        format!("FIEMAP: cannot access {}, e: {}", path.quote(), e)));
                    return (0, errors);
                }
            };

        let mut total_overlapping: u64 = 0;

        for (i, extent_result) in extents.enumerate()
        {
            let extent = match extent_result {
                Err(e) => {
                    errors.push(USimpleError::new(1,
                                format!("FIEMAP: extent error {}, {}",
                                path.quote(), e)));
                    return (0, errors);
                }
                Ok(extent) => extent,
            };

            if !extent.fe_flags.contains(FiemapExtentFlags::UNKNOWN) && // if this bit is set, the record doesn't contain valid information (yet)
                extent.fe_flags.contains(FiemapExtentFlags::SHARED) // performance: only with this bit set, extents are relevant for us
            {
                let range = Range{
                    start: extent.fe_physical,
                    end: extent.fe_physical + extent.fe_length,
                };

                total_overlapping += self.get_overlapping_and_insert(&range);

                if self.log_infos {
                    errors.push(USimpleError::new(0,
                            format!("extent: {}, sum:{}, extents: {}, range:{}..{}, flags:{:#x}",
                                path.quote(), total_overlapping,
                                i,range.start,range.end,
                                extent.fe_flags.bits())));
                }
            }
        }

        return (total_overlapping, errors);
    }
}
