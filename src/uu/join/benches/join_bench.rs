// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::{fs::File, io::Write};
use tempfile::TempDir;
use uu_join::uumain;
use uucore::benchmark::run_util_function;

/// Create two sorted files with matching keys for join benchmarking
fn create_join_files(temp_dir: &TempDir, num_lines: usize) -> (String, String) {
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = temp_dir.path().join("file2.txt");

    let mut file1 = File::create(&file1_path).unwrap();
    let mut file2 = File::create(&file2_path).unwrap();

    for i in 0..num_lines {
        writeln!(file1, "{i:08} field1_{i} field2_{i}").unwrap();
        writeln!(file2, "{i:08} data1_{i} data2_{i}").unwrap();
    }

    (
        file1_path.to_str().unwrap().to_string(),
        file2_path.to_str().unwrap().to_string(),
    )
}

/// Create two files with partial overlap for join benchmarking
fn create_partial_overlap_files(
    temp_dir: &TempDir,
    num_lines: usize,
    overlap_ratio: f64,
) -> (String, String) {
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = temp_dir.path().join("file2.txt");

    let mut file1 = File::create(&file1_path).unwrap();
    let mut file2 = File::create(&file2_path).unwrap();

    let overlap_count = (num_lines as f64 * overlap_ratio) as usize;

    // File 1: keys 0 to num_lines-1
    for i in 0..num_lines {
        writeln!(file1, "{i:08} f1_data_{i}").unwrap();
    }

    // File 2: keys (num_lines - overlap_count) to (2*num_lines - overlap_count - 1)
    let start = num_lines - overlap_count;
    for i in 0..num_lines {
        writeln!(file2, "{:08} f2_data_{}", start + i, i).unwrap();
    }

    (
        file1_path.to_str().unwrap().to_string(),
        file2_path.to_str().unwrap().to_string(),
    )
}

/// Benchmark basic join with fully matching keys
#[divan::bench]
fn join_full_match(bencher: Bencher) {
    let num_lines = 10000;
    let temp_dir = TempDir::new().unwrap();
    let (file1, file2) = create_join_files(&temp_dir, num_lines);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[&file1, &file2]));
    });
}

/// Benchmark join with partial overlap (50%)
#[divan::bench]
fn join_partial_overlap(bencher: Bencher) {
    let num_lines = 10000;
    let temp_dir = TempDir::new().unwrap();
    let (file1, file2) = create_partial_overlap_files(&temp_dir, num_lines, 0.5);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[&file1, &file2]));
    });
}

/// Benchmark join with custom field separator
#[divan::bench]
fn join_custom_separator(bencher: Bencher) {
    let num_lines = 10000;
    let temp_dir = TempDir::new().unwrap();
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = temp_dir.path().join("file2.txt");

    let mut file1 = File::create(&file1_path).unwrap();
    let mut file2 = File::create(&file2_path).unwrap();

    for i in 0..num_lines {
        writeln!(file1, "{i:08}\tfield1_{i}\tfield2_{i}").unwrap();
        writeln!(file2, "{i:08}\tdata1_{i}\tdata2_{i}").unwrap();
    }

    let file1_str = file1_path.to_str().unwrap();
    let file2_str = file2_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-t", "\t", file1_str, file2_str],
        ));
    });
}

/// Benchmark join with French locale (fr_FR.UTF-8)
#[divan::bench]
fn join_french_locale(bencher: Bencher) {
    let num_lines = 10000;
    let temp_dir = TempDir::new().unwrap();
    let (file1, file2) = create_join_files(&temp_dir, num_lines);

    bencher
        .with_inputs(|| unsafe {
            std::env::set_var("LC_ALL", "fr_FR.UTF-8");
        })
        .bench_values(|_| {
            black_box(run_util_function(uumain, &[&file1, &file2]));
        });
}

fn main() {
    divan::main();
}
