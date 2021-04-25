use std::clone::Clone;
use std::cmp::Ordering::Less;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::SeekFrom::Start;
use std::io::{BufRead, BufReader, BufWriter, Seek, Write};
use std::marker::PhantomData;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use tempdir::TempDir;

use super::{GlobalSettings, Line};

/// Trait for types that can be used by
/// [ExternalSorter](struct.ExternalSorter.html). Must be sortable, cloneable,
/// serializeable, and able to report on it's size
pub trait ExternallySortable: Clone + Serialize + DeserializeOwned {
    /// Get the size, in bytes, of this object (used to constrain the buffer
    /// used in the external sort).
    fn get_size(&self) -> u64;
}

/// Iterator that provides sorted `T`s
pub struct ExtSortedIterator<Line> {
    buffers: Vec<VecDeque<Line>>,
    chunk_offsets: Vec<u64>,
    max_per_chunk: u64,
    chunks: u64,
    tmp_dir: TempDir,
    settings: GlobalSettings,
    failed: bool,
}

impl Iterator for ExtSortedIterator<Line>
where
    Line: ExternallySortable,
{
    type Item = Result<Line, Box<dyn Error>>;

    /// # Errors
    ///
    /// This method can fail due to issues reading intermediate sorted chunks
    /// from disk, or due to serde deserialization issues
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        // fill up any empty buffers
        let mut empty = true;
        for chunk_num in 0..self.chunks {
            if self.buffers[chunk_num as usize].is_empty() {
                let mut f = match File::open(self.tmp_dir.path().join(chunk_num.to_string())) {
                    Ok(f) => f,
                    Err(e) => {
                        self.failed = true;
                        return Some(Err(Box::new(e)));
                    }
                };
                match f.seek(Start(self.chunk_offsets[chunk_num as usize])) {
                    Ok(_) => (),
                    Err(e) => {
                        self.failed = true;
                        return Some(Err(Box::new(e)));
                    }
                }
                let bytes_read =
                    match fill_buff(&mut self.buffers[chunk_num as usize], f, self.max_per_chunk) {
                        Ok(bytes_read) => bytes_read,
                        Err(e) => {
                            self.failed = true;
                            return Some(Err(e));
                        }
                    };
                self.chunk_offsets[chunk_num as usize] += bytes_read;
                if !self.buffers[chunk_num as usize].is_empty() {
                    empty = false;
                }
            } else {
                empty = false;
            }
        }
        if empty {
            return None;
        }

        // find the next record to write
        // check is_empty() before unwrap()ing
        let mut idx = 0;
        for chunk_num in 0..self.chunks as usize {
            if !self.buffers[chunk_num].is_empty() {
                if self.buffers[idx].is_empty()
                    || (super::compare_by)(
                        self.buffers[chunk_num].front().unwrap(),
                        self.buffers[idx].front().unwrap(),
                        &self.settings,
                    ) == Less
                {
                    idx = chunk_num;
                }
            }
        }

        // unwrap due to checks above
        let r = self.buffers[idx].pop_front().unwrap();
        Some(Ok(r))
    }
}

/// Perform an external sort on an unsorted stream of incoming data
pub struct ExternalSorter<Line>
where
    Line: ExternallySortable,
{
    tmp_dir: Option<PathBuf>,
    buffer_bytes: u64,
    phantom: PhantomData<Line>,
    settings: GlobalSettings,
}

impl ExternalSorter<Line>
where
    Line: ExternallySortable,
{
    /// Create a new `ExternalSorter` with a specified memory buffer and
    /// temporary directory
    pub fn new(
        buffer_bytes: u64,
        tmp_dir: Option<PathBuf>,
        settings: GlobalSettings,
    ) -> ExternalSorter<Line> {
        ExternalSorter {
            buffer_bytes,
            tmp_dir,
            phantom: PhantomData,
            settings,
        }
    }

    /// Sort (based on `compare`) the `T`s provided by `unsorted` and return an
    /// iterator
    ///
    /// # Errors
    ///
    /// This method can fail due to issues writing intermediate sorted chunks
    /// to disk, or due to serde serialization issues
    pub fn sort_by<I>(
        &self,
        unsorted: I,
        settings: GlobalSettings,
    ) -> Result<ExtSortedIterator<Line>, Box<dyn Error>>
    where
        I: Iterator<Item = Line>,
    {
        let tmp_dir = match self.tmp_dir {
            Some(ref p) => TempDir::new_in(p, "uutils_sort")?,
            None => TempDir::new("uutils_sort")?,
        };
        // creating the thing we need to return first due to the face that we need to
        // borrow tmp_dir and move it out
        let mut iter = ExtSortedIterator {
            buffers: Vec::new(),
            chunk_offsets: Vec::new(),
            max_per_chunk: 0,
            chunks: 0,
            tmp_dir,
            settings,
            failed: false,
        };

        {
            let mut total_read = 0;
            let mut chunk = Vec::new();
            // Initial buffer is specified by user
            let mut adjusted_buffer_size = self.buffer_bytes;

            // make the initial chunks on disk
            for seq in unsorted {
                let seq_size = seq.get_size();
                total_read += seq_size;
                // Grow buffer size for a Line larger than buffer
                adjusted_buffer_size = 
                    if adjusted_buffer_size < seq_size {
                        seq_size
                    } else {
                        adjusted_buffer_size
                    };
                chunk.push(seq);

                if total_read >= adjusted_buffer_size {
                    super::sort_by(&mut chunk, &self.settings);
                    self.write_chunk(
                        &iter.tmp_dir.path().join(iter.chunks.to_string()),
                        &mut chunk,
                    )?;
                    chunk.clear();
                    total_read = 0;
                    iter.chunks += 1;
                }
            }
            // write the last chunk
            if chunk.len() > 0 {
                super::sort_by(&mut chunk, &self.settings);
                self.write_chunk(
                    &iter.tmp_dir.path().join(iter.chunks.to_string()),
                    &mut chunk,
                )?;
                iter.chunks += 1;
            }

            // initialize buffers for each chunk
            iter.max_per_chunk = self
                .buffer_bytes
                .checked_div(iter.chunks)
                .unwrap_or(self.buffer_bytes);
            iter.buffers = vec![VecDeque::new(); iter.chunks as usize];
            iter.chunk_offsets = vec![0 as u64; iter.chunks as usize];
            for chunk_num in 0..iter.chunks {
                let offset = fill_buff(
                    &mut iter.buffers[chunk_num as usize],
                    File::open(iter.tmp_dir.path().join(chunk_num.to_string()))?,
                    iter.max_per_chunk,
                )?;
                iter.chunk_offsets[chunk_num as usize] = offset;
            }
        }

        Ok(iter)
    }

    fn write_chunk(&self, file: &PathBuf, chunk: &mut Vec<Line>) -> Result<(), Box<dyn Error>> {
        let new_file = OpenOptions::new().create(true).append(true).open(file)?;
        let mut buf_write = Box::new(BufWriter::new(new_file)) as Box<dyn Write>;
        for s in chunk {
            let mut serialized = serde_json::to_string(&s).expect("JSON write error: ");
            serialized.push_str("\n");
            buf_write.write(serialized.as_bytes())?;
        }
        buf_write.flush()?;

        Ok(())
    }
}

fn fill_buff<Line>(
    vec: &mut VecDeque<Line>,
    file: File,
    max_bytes: u64,
) -> Result<u64, Box<dyn Error>>
where
    Line: ExternallySortable,
{
    let mut total_read = 0;
    let mut bytes_read = 0;
    for line in BufReader::new(file).lines() {
        let line_s = line?;
        bytes_read += line_s.len() + 1;
        // This is where the bad stuff happens usually
        let deserialized: Line = serde_json::from_str(&line_s).expect("JSON read error: ");
        total_read += deserialized.get_size();
        vec.push_back(deserialized);
        if total_read > max_bytes {
            break;
        }
    }

    Ok(bytes_read as u64)
}
