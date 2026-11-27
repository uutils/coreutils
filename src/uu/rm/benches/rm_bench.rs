// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![cfg(any(unix, windows, target_os = "wasi"))]

use divan::{Bencher, black_box};
use std::ffi::{OsStr, OsString};
use tempfile::TempDir;
use uu_rm::uumain;
use uucore::benchmark::{fs_tree, get_bench_args};

/// Benchmark removing a single file (repeated to reach 100ms)
#[divan::bench]
fn rm_single_file(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let args: Vec<_> = (0..1000)
                .map(|i| {
                    let path = temp_dir.path().join(format!("f{i}"));
                    get_bench_args([&path]).into_iter()
                })
                .collect();
            (temp_dir, args)
        })
        .bench_values(|(temp_dir, args)| {
            for args in args {
                black_box(uumain(args));
            }
            temp_dir
        });
}

/// Benchmark removing multiple files
#[divan::bench]
fn rm_multiple_files(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let paths = (0..1000).map(|i| temp_dir.path().join(format!("f{i}")));
            let args = get_bench_args(paths);
            (temp_dir, args.into_iter())
        })
        .bench_values(|(temp_dir, args)| {
            black_box(uumain(args));
            temp_dir
        });
}

/// Benchmark recursive directory removal
#[divan::bench]
fn rm_recursive_tree(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            let test_dir = temp_dir.path().join("test_tree");
            std::fs::create_dir(&test_dir).unwrap();
            // Increase depth and width for longer benchmark
            fs_tree::create_balanced_tree(&test_dir, 5, 5, 10);
            let args = get_bench_args([OsStr::new("-r"), test_dir.as_os_str()]).into_iter();
            (temp_dir, args)
        })
        .bench_values(|(temp_dir, args)| {
            black_box(uumain(args));
            temp_dir
        });
}

/// Benchmark force removal
#[divan::bench]
fn rm_force_files(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let paths = (0..1000).map(|i| temp_dir.path().join(format!("f{i}")).into_os_string());
            let args = get_bench_args(std::iter::once(OsString::from("-f")).chain(paths));
            (temp_dir, args.into_iter())
        })
        .bench_values(|(temp_dir, args)| {
            black_box(uumain(args));
            temp_dir
        });
}

fn main() {
    divan::main();
}
