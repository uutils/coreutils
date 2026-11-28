// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore interner Interner

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
/// The lookup table is only needed while values are being interned.
pub struct ByteInterner {
    /// Needed only while new values are being interned.
    table: HashTable<Sym>,

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
            table: HashTable::new(),
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

    pub(crate) fn into_parts(self) -> (Vec<u8>, Vec<usize>) {
        let Self {
            table: _,
            bytes,
            offsets,
        } = self;

        (bytes, offsets)
    }
}
