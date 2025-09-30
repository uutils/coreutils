// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fs::{self, File};
use std::path::Path;
use tempfile::TempDir;
use uu_du::uumain;
use uucore::benchmark::run_util_function;

/// Helper to run du with given arguments on a directory
fn bench_du_with_args(bencher: Bencher, temp_dir: &TempDir, args: &[&str]) {
    let temp_path_str = temp_dir.path().to_str().unwrap();
    let mut full_args = args.to_vec();
    full_args.push(temp_path_str);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &full_args));
    });
}

/// Create a balanced directory tree for benchmarking
fn create_directory_tree(
    base_dir: &Path,
    depth: usize,
    dirs_per_level: usize,
    files_per_dir: usize,
) {
    if depth == 0 {
        return;
    }

    // Create files in current directory
    for file_idx in 0..files_per_dir {
        let file_path = base_dir.join(format!("f{file_idx}"));
        File::create(&file_path).unwrap();
    }

    // Create subdirectories and recurse
    for dir_idx in 0..dirs_per_level {
        let dir_path = base_dir.join(format!("d{dir_idx}"));
        fs::create_dir(&dir_path).unwrap();
        create_directory_tree(&dir_path, depth - 1, dirs_per_level, files_per_dir);
    }
}

/// Create a wide directory tree (many files/dirs at shallow depth)
fn create_wide_tree(base_dir: &Path, total_files: usize, total_dirs: usize) {
    // Create many files in root
    for file_idx in 0..total_files {
        let file_path = base_dir.join(format!("f{file_idx}"));
        File::create(&file_path).unwrap();
    }

    // Create many directories with few files each
    for dir_idx in 0..total_dirs {
        let dir_path = base_dir.join(format!("d{dir_idx}"));
        fs::create_dir(&dir_path).unwrap();
        for file_idx in 0..5 {
            File::create(dir_path.join(format!("f{file_idx}"))).unwrap();
        }
    }
}

/// Create a deep directory tree (deep nesting)
fn create_deep_tree(base_dir: &Path, depth: usize, files_per_level: usize) {
    let mut current_dir = base_dir.to_path_buf();

    for level in 0..depth {
        // Create files at this level
        for file_idx in 0..files_per_level {
            File::create(current_dir.join(format!("f{file_idx}"))).unwrap();
        }

        // Create next level directory
        if level < depth - 1 {
            let next_dir = current_dir.join("d");
            fs::create_dir(&next_dir).unwrap();
            current_dir = next_dir;
        }
    }
}

/// Benchmark default du on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark du -a (all files) on balanced tree
#[divan::bench(args = [(4, 3, 10)])]
fn du_all_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-a"]);
}

/// Benchmark du -h (human readable) on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_human_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-h"]);
}

/// Benchmark du on wide directory structures (many files/dirs, shallow)
#[divan::bench(args = [(5000, 500)])]
fn du_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_du_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark du -a on wide directory structures
#[divan::bench(args = [(5000, 500)])]
fn du_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_du_with_args(bencher, &temp_dir, &["-a"]);
}

/// Benchmark du on deep directory structures
#[divan::bench(args = [(100, 3)])]
fn du_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_deep_tree(temp_dir.path(), depth, files_per_level);
    bench_du_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark du -s (summarize) on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_summarize_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-s"]);
}

/// Benchmark du with --max-depth
#[divan::bench(args = [(6, 4, 10)])]
fn du_max_depth_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["--max-depth=2"]);
}

fn main() {
    divan::main();
}
