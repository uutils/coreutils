// Copyright 2018 Andre-Philippe Paquet
// Modifications copyright 2021 Robert Swinford
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// This file has been modified for use in the uutils' coreutils subproject, sort.

use rayon::prelude::*;
use std::{
    cmp::Ordering,
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Error, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

/// Exposes external sorting (i.e. on disk sorting) capability on arbitrarily
/// sized iterator, even if the generated content of the iterator doesn't fit in
/// memory.
///
/// It uses an in-memory buffer sorted and flushed to disk in segment files when
/// full. Once sorted, it returns a new sorted iterator with all items. In order
/// to remain efficient for all implementations, the crate doesn't handle
/// serialization, but leaves that to the user.
pub struct ExternalSorter {
    segment_size: usize,
    sort_dir: Option<PathBuf>,
    parallel: bool,
}

impl ExternalSorter {
    pub fn new() -> ExternalSorter {
        ExternalSorter {
            segment_size: 16000000000,
            sort_dir: None,
            parallel: false,
        }
    }

    /// Sets the maximum size of each segment in number of sorted items.
    ///
    /// This number of items needs to fit in memory. While sorting, a
    /// in-memory buffer is used to collect the items to be sorted. Once
    /// it reaches the maximum size, it is sorted and then written to disk.
    ///
    /// Using a higher segment size makes sorting faster by leveraging
    /// faster in-memory operations.
    pub fn with_segment_size(mut self, size: usize) -> Self {
        self.segment_size = size;
        self
    }

    /// Sets directory in which sorted segments will be written (if it doesn't
    /// fit in memory).
    pub fn with_sort_dir(mut self, path: PathBuf) -> Self {
        self.sort_dir = Some(path);
        self
    }

    /// Uses Rayon to sort the in-memory buffer.
    ///
    /// This may not be needed if the buffer isn't big enough for parallelism to
    /// be gainful over the overhead of multithreading.
    pub fn with_parallel_sort(mut self) -> Self {
        self.parallel = true;
        self
    }
    
    /// Sorts a given iterator with a comparator function, returning a new iterator with items
    pub fn sort_by<T, I, F>(&self, iterator: I, cmp: F) -> Result<SortedIterator<T, F>, Error>
    where
        T: Sortable,
        I: Iterator<Item = T>,
        F: Fn(&T, &T) -> Ordering + Send + Sync,
    {
        let mut tempdir: Option<tempfile::TempDir> = None;
        let mut sort_dir: Option<PathBuf> = None;

        let mut segments_file: Vec<File> = Vec::new();
        let size_of_items = std::mem::size_of::<T>();
        let mut buffer: Vec<T> = Vec::with_capacity(self.segment_size / size_of_items);
        for next_item in iterator {
            buffer.push(next_item);
            if buffer.len() > self.segment_size {
                let sort_dir = self.lazy_create_dir(&mut tempdir, &mut sort_dir)?;
                self.sort_and_write_segment(sort_dir, &mut segments_file, &mut buffer, &cmp)?;
            }
        }

        // Write any items left in buffer, but only if we had at least 1 segment
        // written. Otherwise we use the buffer itself to iterate from memory
        let pass_through_queue = if !buffer.is_empty() && !segments_file.is_empty() {
            let sort_dir = self.lazy_create_dir(&mut tempdir, &mut sort_dir)?;
            self.sort_and_write_segment(sort_dir, &mut segments_file, &mut buffer, &cmp)?;
            None
        } else {
            buffer.sort_by(&cmp);
            Some(VecDeque::from(buffer))
        };

        SortedIterator::new(tempdir, pass_through_queue, segments_file, cmp)
    }

    /// We only want to create directory if it's needed (i.e. if the dataset
    /// doesn't fit in memory) to prevent filesystem latency
    fn lazy_create_dir<'a>(
        &self,
        tempdir: &mut Option<tempfile::TempDir>,
        sort_dir: &'a mut Option<PathBuf>,
    ) -> Result<&'a Path, Error> {
        if let Some(sort_dir) = sort_dir {
            return Ok(sort_dir);
        }

        *sort_dir = if let Some(ref sort_dir) = self.sort_dir {
            Some(sort_dir.to_path_buf())
        } else {
            *tempdir = Some(tempfile::TempDir::new()?);
            Some(tempdir.as_ref().unwrap().path().to_path_buf())
        };

        Ok(sort_dir.as_ref().unwrap())
    }

    fn sort_and_write_segment<T, F>(
        &self,
        sort_dir: &Path,
        segments: &mut Vec<File>,
        buffer: &mut Vec<T>,
        cmp: F,
    ) -> Result<(), Error>
    where
        T: Sortable,
        F: Fn(&T, &T) -> Ordering + Send + Sync,
    {
        if self.parallel {
            buffer.par_sort_by(|a, b| cmp(a, b));
        } else {
            buffer.sort_by(|a, b| cmp(a, b));
        }

        let segment_path = sort_dir.join(format!("{}", segments.len()));
        let segment_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&segment_path)?;
        let mut buf_writer = BufWriter::new(segment_file);

        for item in buffer.drain(0..) {
            item.encode(&mut buf_writer);
        }

        let file = buf_writer.into_inner()?;
        segments.push(file);

        Ok(())
    }
}

impl Default for ExternalSorter {
    fn default() -> Self {
        ExternalSorter::new()
    }
}

pub trait Sortable: Sized + Send {
    fn encode<W: Write>(&self, writer: &mut W);
    fn decode<R: Read>(reader: &mut R) -> Option<Self>;
}

pub struct SortedIterator<T: Sortable, F> {
    _tempdir: Option<tempfile::TempDir>,
    pass_through_queue: Option<VecDeque<T>>,
    segments_file: Vec<BufReader<File>>,
    next_values: Vec<Option<T>>,
    cmp: F,
}

impl<T: Sortable, F: Fn(&T, &T) -> Ordering + Send + Sync> SortedIterator<T, F> {
    fn new(
        tempdir: Option<tempfile::TempDir>,
        pass_through_queue: Option<VecDeque<T>>,
        mut segments_file: Vec<File>,
        cmp: F,
    ) -> Result<SortedIterator<T, F>, Error> {
        for segment in &mut segments_file {
            segment.seek(SeekFrom::Start(0))?;
        }

        let next_values = segments_file
            .iter_mut()
            .map(|file| T::decode(file))
            .collect();

        let segments_file_buffered = segments_file.into_iter().map(BufReader::new).collect();

        Ok(SortedIterator {
            _tempdir: tempdir,
            pass_through_queue,
            segments_file: segments_file_buffered,
            next_values,
            cmp,
        })
    }
}

impl<T: Sortable, F: Fn(&T, &T) -> Ordering> Iterator for SortedIterator<T, F> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        // if we have a pass through, we dequeue from it directly
        if let Some(ptb) = self.pass_through_queue.as_mut() {
            return ptb.pop_front();
        }

        // otherwise, we iter from segments on disk
        let mut smallest_idx: Option<usize> = None;
        {
            let mut smallest: Option<&T> = None;
            for idx in 0..self.segments_file.len() {
                let next_value = self.next_values[idx].as_ref();
                if next_value.is_none() {
                    continue;
                }

                if smallest.is_none()
                    || (self.cmp)(next_value.unwrap(), smallest.unwrap()) == Ordering::Less
                {
                    smallest = Some(next_value.unwrap());
                    smallest_idx = Some(idx);
                }
            }
        }

        smallest_idx.map(|idx| {
            let file = &mut self.segments_file[idx];
            let value = self.next_values[idx].take().unwrap();
            self.next_values[idx] = T::decode(file);
            value
        })
    }
}
