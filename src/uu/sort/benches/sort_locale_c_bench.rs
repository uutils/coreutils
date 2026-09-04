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
use uucore::benchmark::{get_bench_args, setup_test_file, text_data};

/// Benchmark ASCII-only data sorting with C locale (byte comparison)
#[divan::bench]
fn sort_ascii_c_locale(bencher: Bencher) {
    let data = text_data::generate_ascii_data_simple(2_000_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path();

    bencher
        .with_inputs(|| get_bench_args(&[&"-o", &output_path, &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark mixed ASCII/Unicode data with C locale (byte comparison)
#[divan::bench]
fn sort_mixed_c_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path();

    bencher
        .with_inputs(|| get_bench_args(&[&"-o", &output_path, &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark German locale-specific data with C locale (byte comparison)
#[divan::bench]
fn sort_german_c_locale(bencher: Bencher) {
    let data = text_data::generate_german_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path();

    bencher
        .with_inputs(|| get_bench_args(&[&"-o", &output_path, &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark whole-line sorting of lines that all share a long prefix, such as
/// timestamped log lines or paths under one directory. The sorter has to find
/// and skip that prefix before it can key each line on the bytes that differ.
#[divan::bench]
fn sort_shared_prefix_c_locale(bencher: Bencher) {
    const PREFIX: &[u8] = b"/srv/data/2026/09/03/node-07/service/events/request-";
    let mut data = Vec::new();
    let mut state: u32 = 0x9e37_79b9;
    for _ in 0..500_000 {
        // Cheap deterministic pseudo-random suffix.
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        data.extend_from_slice(PREFIX);
        data.extend_from_slice(format!("{state:08x}.json\n").as_bytes());
    }
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path();

    bencher
        .with_inputs(|| {
            get_bench_args(&[&"--parallel=1", &"-o", &output_path, &file_path]).into_iter()
        })
        .bench_values(|args| black_box(uumain(args)));
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
