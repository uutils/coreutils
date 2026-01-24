// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Benchmarks for sort with C locale (fast byte-wise comparison).
//!
//! Note: The locale is set in main() BEFORE any benchmark runs because
//! the locale is cached on first access via OnceLock and cannot be changed afterwards.

use divan::{Bencher, black_box};
use tempfile::NamedTempFile;
use uu_sort::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark ASCII-only data sorting with C locale (byte comparison)
#[divan::bench]
fn sort_ascii_c_locale(bencher: Bencher) {
    let data = text_data::generate_ascii_data_simple(100_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", &output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark mixed ASCII/Unicode data with C locale (byte comparison)
#[divan::bench]
fn sort_mixed_c_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", &output_path, file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark German locale-specific data with C locale (byte comparison)
#[divan::bench]
fn sort_german_c_locale(bencher: Bencher) {
    let data = text_data::generate_german_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-o", &output_path, file_path.to_str().unwrap()],
        ));
    });
}

fn main() {
    // Set C locale BEFORE any benchmarks run.
    // This must happen before divan::main() because the locale is cached
    // on first access via OnceLock and cannot be changed afterwards.
    unsafe {
        std::env::set_var("LC_ALL", "C");
    }
    divan::main();
}
