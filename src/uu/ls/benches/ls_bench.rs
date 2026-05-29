// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fs;
use tempfile::TempDir;
use uu_ls::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

/// Helper to run ls with given arguments on a directory
fn bench_ls_with_args(bencher: Bencher, temp_dir: &TempDir, args: &[&str]) {
    let temp_path_str = temp_dir.path().to_str().unwrap();
    let mut full_args = vec!["-R"];
    full_args.extend_from_slice(args);
    full_args.push(temp_path_str);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &full_args));
    });
}

/// Benchmark ls -R on balanced directory tree
#[divan::bench(args = [(6, 4, 15)])]
fn ls_recursive_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on balanced directory tree (tests PR #8728 optimization)
#[divan::bench(args = [(6, 4, 15)])]
fn ls_recursive_long_all_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on wide directory structures
#[divan::bench(args = [(10000, 1000)])]
fn ls_recursive_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on wide directory structures
#[divan::bench(args = [(15000, 1500)])]
fn ls_recursive_long_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on deep directory structures
#[divan::bench(args = [(200, 2)])]
fn ls_recursive_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on deep directory structures
#[divan::bench(args = [(100, 4)])]
fn ls_recursive_long_all_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on mixed file types (comprehensive real-world test)
#[divan::bench]
fn ls_recursive_mixed_tree(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_mixed_tree(temp_dir.path());

    for i in 0..10 {
        let subdir = temp_dir.path().join(format!("mixed_branch_{i}"));
        fs::create_dir(&subdir).unwrap();
        fs_tree::create_mixed_tree(&subdir);
    }

    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on mixed file types (most comprehensive test)
#[divan::bench]
fn ls_recursive_long_all_mixed_tree(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_mixed_tree(temp_dir.path());

    for i in 0..10 {
        let subdir = temp_dir.path().join(format!("mixed_branch_{i}"));
        fs::create_dir(&subdir).unwrap();
        fs_tree::create_mixed_tree(&subdir);
    }

    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

fn main() {
    divan::main();
}
