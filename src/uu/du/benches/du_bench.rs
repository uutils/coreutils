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
/// MEASURES BUG #9146: stdout buffering inefficiency
///
/// CURRENT: Each of ~81 entries triggers 3 stdout writes = ~243 syscalls.
/// With BufWriter, this becomes ~3-5 buffered flushes total.
/// Performance should improve significantly after fix.
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
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
            temp_dir
        })
        .bench_values(|temp_dir| {
            let temp_path_str = temp_dir.path().to_str().unwrap();
            let args = vec![temp_path_str];
            black_box(run_util_function(uumain, &args));
        });
}

/// Benchmark du -a on wide directory structures
/// MEASURES BUG #9146: stdout buffering inefficiency
///
/// CURRENT: Each of ~5,500 entries (5,000 files + 500 dirs) triggers 3 stdout writes
/// = ~16,500 syscalls. With BufWriter, this becomes ~3-5 buffered flushes total.
/// Performance should improve dramatically after fix.
#[divan::bench(args = [(5000, 500)])]
fn du_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
            temp_dir
        })
        .bench_values(|temp_dir| {
            let temp_path_str = temp_dir.path().to_str().unwrap();
            let args = vec![temp_path_str, "-a"];
            black_box(run_util_function(uumain, &args));
        });
}

/// Benchmark du on deep directory structures
#[divan::bench(args = [(100, 3)])]
fn du_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
            temp_dir
        })
        .bench_values(|temp_dir| {
            let temp_path_str = temp_dir.path().to_str().unwrap();
            let args = vec![temp_path_str];
            black_box(run_util_function(uumain, &args));
        });
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

/// STRESS TEST: Benchmark du -a on large directory structure
/// MEASURES BUFWRITER SCALING: Tests if 64KiB buffer benefits scale linearly
///
/// EXPECTED: ~161 entries (dirs+files) with ~483 potential stdout writes
/// With BufWriter: Should complete with ~3-5 syscalls regardless of entry count
/// This test validates that the optimization scales to larger real-world directories
/// and that memory usage remains bounded (64KiB buffer).
#[divan::bench(args = [(3, 5, 6)])]
fn du_all_stress_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_du_with_args(bencher, &temp_dir, &["-a"]);
}

/// STRESS TEST: Benchmark du -a on extremely wide directory structure\
/// MEASURES BUFWRIDER UNDER EXTREME LOAD: Tests worst-case for stdout frequency
///
///    EXPECTED: ~2,500 entries = ~7,500 potential stdout writes without buffering
/// This is the scenario that most directly exposes the issue #9146 performance bottleneck.
/// Success is measured not just by time, but by consistent performance regardless of entry count.
#[divan::bench(args = [(2000, 500)])]
fn du_all_extreme_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_du_with_args(bencher, &temp_dir, &["-a"]);
}

fn main() {
    divan::main();
}
