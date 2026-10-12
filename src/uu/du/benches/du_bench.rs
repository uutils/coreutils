// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_du::uumain;
use uucore::benchmark::{fs_tree, get_bench_args};

/* too much variance
/// Benchmark default du on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    fs_tree::create_balanced_tree(temp_path, depth, dirs_per_level, files_per_dir);

    bencher
        .with_inputs(|| get_bench_args(&[&temp_path]))
        .bench_values(|args| black_box(uumain(args)));
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
    let temp_path = temp_dir.path();
    fs_tree::create_balanced_tree(temp_path, depth, dirs_per_level, files_per_dir);

    bencher
        .with_inputs(|| get_bench_args(&[&"-a", &temp_path]))
        .bench_values(|args| black_box(uumain(args)));
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
    let temp_path = temp_dir.path();
    fs_tree::create_balanced_tree(temp_path, depth, dirs_per_level, files_per_dir);

    bencher
        .with_inputs(|| get_bench_args(&[&"-h", &temp_path]))
        .bench_values(|args| black_box(uumain(args)));
}
*/

/// Run a throwaway traversal to warm the OS page/inode cache before timing.
fn warm_cache(path: &std::path::Path) {
    let _ = black_box(uumain(get_bench_args(&[&path]).into_iter()));
}

/// Benchmark du on wide directory structures (many files/dirs, shallow).
#[divan::bench(args = [(5000, 500)])]
fn du_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    fs_tree::create_wide_tree(temp_path, total_files, total_dirs);
    warm_cache(temp_path);

    bencher
        .with_inputs(|| get_bench_args(&[&temp_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark du -a on wide directory structures
#[divan::bench(args = [(5000, 500)])]
fn du_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    fs_tree::create_wide_tree(temp_path, total_files, total_dirs);
    warm_cache(temp_path);

    bencher
        .with_inputs(|| get_bench_args(&[&"-a", &temp_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark du on deep directory structures
#[divan::bench(args = [(100, 3)])]
fn du_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    fs_tree::create_deep_tree(temp_path, depth, files_per_level);
    warm_cache(temp_path);

    bencher
        .with_inputs(|| get_bench_args(&[&temp_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark du -s (summarize) on balanced tree
#[divan::bench(args = [(5, 4, 10)])]
fn du_summarize_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    fs_tree::create_balanced_tree(temp_path, depth, dirs_per_level, files_per_dir);
    warm_cache(temp_path);

    bencher
        .with_inputs(|| get_bench_args(&[&"-s", &temp_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
