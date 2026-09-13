// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::error::AllocationError;
use hashbrown::{HashTable, hash_table::Entry};
use rustc_hash::FxHasher;
use std::hash::Hasher;

pub type Sym = usize;

/// Mutable byte-string interning phase.
///
/// During construction:
///
///     &[u8] -> Sym
///
/// Call [`finish`](Self::finish) after all values have been interned to drop
/// the lookup table and obtain a [`ByteInterner`] for `Sym -> &[u8]`
/// resolution.
#[derive(Default)]
pub struct ByteInternerBuilder {
    /// Needed only while new values are being interned.
    table: HashTable<Sym>,

    /// All interned byte strings packed contiguously.
    bytes: Vec<u8>,

    /// End offsets into `bytes.
    ///
    /// Symbol `n` corresponds to:
    ///
    ///     bytes[ends[n-1]..ends[n]]
    ///
    ends: Vec<usize>,
}

/// Finished byte-string interner.
pub struct ByteInterner {
    bytes: Vec<u8>,
    ends: Vec<usize>,
}

impl ByteInternerBuilder {
    #[inline]
    fn hash(value: &[u8]) -> u64 {
        let mut hasher = FxHasher::default();
        hasher.write(value);
        hasher.finish()
    }

    #[inline]
    fn bounds(ends: &[usize], sym: Sym) -> (usize, usize) {
        let end = ends[sym];
        let start = if sym == 0 { 0 } else { ends[sym - 1] };
        (start, end)
    }

    #[inline]
    pub fn get_or_intern(&mut self, value: &[u8]) -> Result<Sym, AllocationError> {
        let hash = Self::hash(value);

        let Self { table, bytes, ends } = self;

        match table.entry(
            hash,
            |&sym| Self::value_for(bytes, ends, sym) == value,
            |&sym| Self::hash(Self::value_for(bytes, ends, sym)),
        ) {
            Entry::Occupied(entry) => Ok(*entry.get()),
            Entry::Vacant(entry) => {
                bytes.try_reserve(value.len())?;
                ends.try_reserve(1)?;

                let sym = ends.len();
                bytes.extend_from_slice(value);
                ends.push(bytes.len());
                entry.insert(sym);

                Ok(sym)
            }
        }
    }

    #[inline]
    pub fn finish(self) -> ByteInterner {
        ByteInterner {
            bytes: self.bytes,
            ends: self.ends,
        }
    }

    fn value_for<'a>(bytes: &'a [u8], ends: &[usize], sym: usize) -> &'a [u8] {
        let (start, end) = Self::bounds(ends, sym);
        &bytes[start..end]
    }
}

impl ByteInterner {
    #[inline]
    pub fn resolve(&self, sym: Sym) -> Option<&[u8]> {
        let end = *self.ends.get(sym)?;
        let start = if sym == 0 {
            0
        } else {
            *self.ends.get(sym - 1)?
        };

        Some(&self.bytes[start..end])
    }
}
