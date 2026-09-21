// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use hashbrown::HashTable;
use rustc_hash::FxHasher;
use std::hash::Hasher;

pub type Sym = usize;

/// Interns arbitrary byte strings.
///
/// During interning:
///
///     &[u8] -> Sym
///     Sym    -> &[u8]
///
/// Once `finish_interning()` is called, the hash table used for
/// `&[u8] -> Sym` lookups is dropped. `Sym -> &[u8]` resolution remains
/// available for the lifetime of the interner.
pub struct ByteInterner {
    /// Needed only while new values are being interned.
    table: Option<HashTable<Sym>>,

    /// All interned byte strings packed contiguously.
    bytes: Vec<u8>,

    /// Boundaries into `bytes`.
    ///
    /// Symbol `n` corresponds to:
    ///
    ///     bytes[offsets[n]..offsets[n + 1]]
    ///
    /// The initial zero is a sentinel, so the number of symbols is
    /// `offsets.len() - 1`.
    offsets: Vec<usize>,
}

impl Default for ByteInterner {
    fn default() -> Self {
        Self {
            table: Some(HashTable::new()),
            bytes: Vec::new(),
            offsets: vec![0],
        }
    }
}

impl ByteInterner {
    #[inline]
    fn hash(value: &[u8]) -> u64 {
        let mut hasher = FxHasher::default();
        hasher.write(value);
        hasher.finish()
    }

    #[inline]
    pub fn get_or_intern(&mut self, value: &[u8]) -> Sym {
        let hash = Self::hash(value);

        let Self {
            table,
            bytes,
            offsets,
        } = self;

        let table = table
            .as_mut()
            .expect("cannot intern values after finish_interning()");

        // Check whether this byte sequence is already interned.
        if let Some(&sym) = table.find(hash, |&sym| {
            let start = offsets[sym];
            let end = offsets[sym + 1];

            &bytes[start..end] == value
        }) {
            return sym;
        }

        // Allocate a new symbol and append the bytes directly to the arena.
        let sym = offsets.len() - 1;

        bytes.extend_from_slice(value);
        offsets.push(bytes.len());

        // HashTable stores only the symbol. If it needs to resize, hashes
        // for existing entries are reconstructed from the byte arena.
        table.insert_unique(hash, sym, |&sym| {
            let start = offsets[sym];
            let end = offsets[sym + 1];

            Self::hash(&bytes[start..end])
        });

        sym
    }

    #[inline]
    pub fn resolve(&self, sym: Sym) -> Option<&[u8]> {
        let start = *self.offsets.get(sym)?;
        let end = *self.offsets.get(sym + 1)?;

        Some(&self.bytes[start..end])
    }

    /// Drop the `bytes -> Sym` lookup structure.
    ///
    /// Call this after all input has been parsed and no more values will
    /// be interned.
    pub fn finish_interning(&mut self) {
        drop(self.table.take());
    }
}
