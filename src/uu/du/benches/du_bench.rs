// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_du::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

/// Helper to run du with given arguments on a directory
fn bench_du_with_args(bencher: Bencher, temp_dir: &TempDir, args: &[&str]) {
    let temp_path_str = temp_dir.path().to_str().unwrap();
    let mut full_args = args.to_vec();
    full_args.push(temp_path_str);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &full_args));
    });
}

/* too much variance
/// Benchmark default du on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &[]);
}
*/

/* too much variance
/// Benchmark du -a (all files) on balanced tree
#[divan::bench(args = [(4, 3, 10)])]
fn du_all_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-a"]);
}
*/

/* too much variance
/// Benchmark du -h (human readable) on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_human_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-h"]);
}
*/

/// Benchmark du on wide directory structures (many files/dirs, shallow)
#[divan::bench(args = [(5000, 500)])]
fn du_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_du_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark du -a on wide directory structures
#[divan::bench(args = [(5000, 500)])]
fn du_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_du_with_args(bencher, &temp_dir, &["-a"]);
}

/// Benchmark du on deep directory structures
#[divan::bench(args = [(100, 3)])]
fn du_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
    bench_du_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark du -s (summarize) on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_summarize_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-s"]);
}

/// Benchmark du with --max-depth
#[divan::bench(args = [(6, 4, 10)])]
fn du_max_depth_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["--max-depth=2"]);
}

fn main() {
    divan::main();
}
