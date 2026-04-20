// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parallel-or-sequential sort helpers and thread-pool initialization.
//!
//! On targets without thread support (`wasm32-wasip1` without atomics) these
//! fall back to the sequential `[T]::sort_*` methods and a no-op pool init.
//! On every other target they use rayon's parallel sorts.

#[cfg(not(wasi_no_threads))]
mod imp {
    use std::cmp::Ordering;
    use std::num::NonZero;

    use rayon::slice::ParallelSliceMut;

    pub fn sort_by<T: Send>(slice: &mut [T], cmp: impl Fn(&T, &T) -> Ordering + Sync) {
        slice.par_sort_by(cmp);
    }

    pub fn sort_unstable_by<T: Send>(slice: &mut [T], cmp: impl Fn(&T, &T) -> Ordering + Sync) {
        slice.par_sort_unstable_by(cmp);
    }

    pub fn init_thread_pool(num_threads: Option<u64>) {
        let num_threads = num_threads.map_or_else(
            || std::thread::available_parallelism().map_or(1, NonZero::get),
            |n| n as usize,
        );
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global();
    }
}

#[cfg(wasi_no_threads)]
mod imp {
    use std::cmp::Ordering;

    // The `Send`/`Sync` bounds mirror the parallel implementation so that call
    // sites compile identically on both targets. They are stricter than the
    // underlying `[T]::sort_*` methods require, but every caller in this crate
    // already satisfies them.
    pub fn sort_by<T: Send>(slice: &mut [T], cmp: impl Fn(&T, &T) -> Ordering + Sync) {
        slice.sort_by(cmp);
    }

    pub fn sort_unstable_by<T: Send>(slice: &mut [T], cmp: impl Fn(&T, &T) -> Ordering + Sync) {
        slice.sort_unstable_by(cmp);
    }

    pub fn init_thread_pool(_num_threads: Option<u64>) {
        // No-op: there is no thread pool on this target, so --parallel is ignored.
    }
}

pub use imp::{init_thread_pool, sort_by, sort_unstable_by};
