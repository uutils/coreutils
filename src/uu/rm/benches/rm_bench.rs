// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_rm::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

/// Benchmark removing a single file (repeated to reach 100ms)
#[divan::bench]
fn rm_single_file(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let paths: Vec<String> = (0..1000)
                .map(|i| {
                    temp_dir
                        .path()
                        .join(format!("f{i}"))
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect();
            (temp_dir, paths)
        })
        .bench_values(|(temp_dir, paths)| {
            for path in &paths {
                black_box(run_util_function(uumain, &[path]));
            }
            drop(temp_dir);
        });
}

/// Benchmark removing multiple files
#[divan::bench]
fn rm_multiple_files(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let paths: Vec<String> = (0..1000)
                .map(|i| {
                    temp_dir
                        .path()
                        .join(format!("f{i}"))
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect();
            (temp_dir, paths)
        })
        .bench_values(|(temp_dir, paths)| {
            let args: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            black_box(run_util_function(uumain, &args));
            drop(temp_dir);
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
            (temp_dir, test_dir.to_str().unwrap().to_string())
        })
        .bench_values(|(temp_dir, path)| {
            black_box(run_util_function(uumain, &["-r", &path]));
            drop(temp_dir);
        });
}

/// Benchmark force removal
#[divan::bench]
fn rm_force_files(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let paths: Vec<String> = (0..1000)
                .map(|i| {
                    temp_dir
                        .path()
                        .join(format!("f{i}"))
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect();
            (temp_dir, paths)
        })
        .bench_values(|(temp_dir, paths)| {
            let mut args = vec!["-f"];
            let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            args.extend(path_refs);
            black_box(run_util_function(uumain, &args));
            drop(temp_dir);
        });
}

fn main() {
    divan::main();
}
