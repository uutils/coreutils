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
    let data = text_data::generate_ascii_data_simple(1_500_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = ["-o", &output_path, file_path.to_str().unwrap()];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark mixed ASCII/Unicode data with UTF-8 locale
#[divan::bench]
fn sort_mixed_utf8_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = ["-o", &output_path, file_path.to_str().unwrap()];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
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
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = ["-n", "-o", &output_path, file_path.to_str().unwrap()];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark reverse sorting with UTF-8 locale
#[divan::bench]
fn sort_reverse_utf8_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = ["-r", "-o", &output_path, file_path.to_str().unwrap()];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark unique sorting with UTF-8 locale
#[divan::bench]
fn sort_unique_utf8_locale(bencher: Bencher) {
    let data = text_data::generate_mixed_locale_data(50_000);
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = ["-u", "-o", &output_path, file_path.to_str().unwrap()];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark sorting very long lines (single repeated character per line) with UTF-8 locale.
/// This reproduces the pathological case from issue #12138 where computing full collation
/// sort keys for multi-megabyte lines caused a 40x slowdown vs GNU sort.
/// We use 1 MB lines (26 lines, one per letter) to keep the benchmark fast while still
/// exercising the prefix-based sort key optimization.
#[divan::bench]
fn sort_very_long_lines_utf8_locale(bencher: Bencher) {
    let mut data = Vec::new();
    // Create 26 lines of 1 MB each, each line is a single repeated letter
    let letters: Vec<u8> = (b'a'..=b'z').collect();
    for &ch in &letters {
        data.extend(std::iter::repeat_n(ch, 1_000_000));
        data.push(b'\n');
    }
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = [
        "--parallel",
        "1",
        "-o",
        &output_path,
        file_path.to_str().unwrap(),
    ];
    // Warm up
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark sorting lines that share a long common prefix but differ after 8 KB,
/// exercising the fallback from prefix sort keys to full locale comparison.
#[divan::bench]
fn sort_long_common_prefix_utf8_locale(bencher: Bencher) {
    let mut data = Vec::new();
    let prefix_len = 16 * 1024; // 16 KB common prefix (exceeds the 8 KB sort key limit)
    let prefix: Vec<u8> = std::iter::repeat_n(b'x', prefix_len).collect();
    // 26 lines that share the prefix but differ in the suffix
    for ch in b'a'..=b'z' {
        data.extend_from_slice(&prefix);
        data.extend(std::iter::repeat_n(ch, 100));
        data.push(b'\n');
    }
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = [
        "--parallel",
        "1",
        "-o",
        &output_path,
        file_path.to_str().unwrap(),
    ];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark merging a single pre-sorted file (`sort -m FILE`) with a UTF-8 locale.
///
/// This is the worst case for the old implementation: a full ICU collation sort key was
/// computed for every line even though merging a single file performs no comparisons at
/// all. The merge path now compares lazily with the collator, so this should be close to
/// a plain copy regardless of locale.
#[divan::bench]
fn merge_single_file_utf8_locale(bencher: Bencher) {
    // `-m` expects already-sorted input, so sort the generated lines first.
    let raw = text_data::generate_ascii_data_simple(500_000);
    let mut lines: Vec<&[u8]> = raw
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    lines.sort();
    let mut data = Vec::with_capacity(raw.len());
    for line in lines {
        data.extend_from_slice(line);
        data.push(b'\n');
    }
    let file_path = setup_test_file(&data);
    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();

    let args = ["-m", "-o", &output_path, file_path.to_str().unwrap()];
    black_box(run_util_function(uumain, &args));
    bencher.bench(|| {
        black_box(run_util_function(uumain, &args));
    });
}

/// Benchmark merging several pre-sorted files (`sort -m`) with a UTF-8 locale.
///
/// Mirrors `merge_pre_sorted_files` from `sort_bench_merge.rs`, but runs under a UTF-8
/// locale to exercise the locale-aware merge comparison.
#[divan::bench]
fn merge_pre_sorted_files_utf8_locale(bencher: Bencher) {
    const TOTAL_LINES: usize = 500_000;
    const NUM_FILES: usize = 8;

    let data = text_data::generate_ascii_data(TOTAL_LINES);
    let lines: Vec<&[u8]> = data
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    let lines_per_file = lines.len() / NUM_FILES;

    let mut file_paths = Vec::with_capacity(NUM_FILES);
    for i in 0..NUM_FILES {
        let start = i * lines_per_file;
        let end = if i == NUM_FILES - 1 {
            lines.len()
        } else {
            (i + 1) * lines_per_file
        };

        let mut chunk_lines = lines[start..end].to_vec();
        chunk_lines.sort();

        let mut chunk_data = Vec::new();
        for line in chunk_lines {
            chunk_data.extend_from_slice(line);
            chunk_data.push(b'\n');
        }
        file_paths.push(setup_test_file(&chunk_data));
    }

    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap().to_string();
    let file_args: Vec<&str> = file_paths.iter().map(|p| p.to_str().unwrap()).collect();

    bencher.bench(|| {
        let mut args = vec!["-m", "-o", output_path.as_str()];
        args.extend(&file_args);
        black_box(run_util_function(uumain, &args));
    });
}

fn main() {
    // Set UTF-8 locale BEFORE any benchmarks run.
    // This must happen before divan::main() because the locale is cached
    // on first access via OnceLock and cannot be changed afterwards.
    uucore::env::set_var("LC_ALL", "en_US.UTF-8");
    divan::main();
}
