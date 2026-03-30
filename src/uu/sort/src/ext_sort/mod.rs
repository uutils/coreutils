// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! External sort: sort large inputs that may not fit in memory.
//!
//! On most platforms this uses a multi-threaded chunked approach with
//! temporary files. On WASI without atomics, synchronous fallbacks are
//! used instead (selected via `cfg` guards inside the module).

mod threaded;
pub use threaded::ext_sort;
