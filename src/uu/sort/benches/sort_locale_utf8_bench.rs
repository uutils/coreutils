// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Benchmarks for sort with UTF-8 locale (locale-aware collation).
//!
//! Note: The locale is set in main() BEFORE any benchmark runs because
//! the locale is cached on first access via OnceLock and cannot be changed afterwards.

use divan::{Bencher, black_box};
use tempfile::NamedTempFile;
use uu_sort::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark ASCII-only data sorting with UTF-8 locale
#[divan::bench]
fn sort_ascii_utf8_locale(bencher: Bencher) {
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

/// Benchmark mixed ASCII/Unicode data with UTF-8 locale
#[divan::bench]
fn sort_mixed_utf8_locale(bencher: Bencher) {
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

/// Benchmark numeric sorting with UTF-8 locale
#[divan::bench]
fn sort_numeric_utf8_locale(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..50_000 {
        let line = format!("{}\n", 50_000 - i);
        data.extend_from_slice(line.as_bytes());
    }
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-n", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark reverse sorting with UTF-8 locale
#[divan::bench]
fn sort_reverse_utf8_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-r", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark unique sorting with UTF-8 locale
#[divan::bench]
fn sort_unique_utf8_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-u", file_path.to_str().unwrap()],
        ));
    });
}

fn main() {
    // Set UTF-8 locale BEFORE any benchmarks run.
    // This must happen before divan::main() because the locale is cached
    // on first access via OnceLock and cannot be changed afterwards.
    unsafe {
        std::env::set_var("LC_ALL", "en_US.UTF-8");
    }
    divan::main();
}
