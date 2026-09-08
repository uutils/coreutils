use hashbrown::HashTable;
use rustc_hash::FxHasher;
use std::hash::Hasher;

pub type Sym = usize;

#[derive(Clone, Copy)]
struct Entry {
    hash: u64,
    sym: Sym,
}

#[derive(Default)]
pub struct ByteInterner {
    table: HashTable<Entry>,
    bytes: Vec<u8>,
    spans: Vec<(usize, usize)>,
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

        if let Some(entry) = self.table.find(hash, |entry| {
            let (start, len) = self.spans[entry.sym];
            self.bytes[start..start + len] == *value
        }) {
            return entry.sym;
        }

        let sym = self.spans.len();
        let start = self.bytes.len();

        self.bytes.extend_from_slice(value);
        self.spans.push((start, value.len()));

        self.table
            .insert_unique(hash, Entry { hash, sym }, |entry| entry.hash);

        sym
    }

    #[inline]
    pub fn resolve(&self, sym: Sym) -> Option<&[u8]> {
        let &(start, len) = self.spans.get(sym)?;
        Some(&self.bytes[start..start + len])
    }
}
