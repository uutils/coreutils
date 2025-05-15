// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore libcoreutils libloading

#[cfg(not(feature = "dynamic"))]
fn main() {
    coreutils::multicall_main();
}

#[cfg(feature = "dynamic")]
fn main() {
    use libloading::{Library, Symbol, library_filename};
    unsafe {
        let library = Library::new(library_filename("coreutils"))
            .unwrap_or_else(|e| panic!("Could not load libcoreutils: {}", e));
        let library_main: Symbol<fn()> = library
            .get(b"coreutils_multicall_main_wrapper")
            .unwrap_or_else(|e| panic!("Could not find main symbol: {}", e));
        library_main();
    }
}
