// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_mv::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

/// Benchmark moving a single file (repeated to reach 100ms)
#[divan::bench]
fn mv_single_file(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let files: Vec<(String, String)> = (0..1000)
                .map(|i| {
                    let src = temp_dir.path().join(format!("f{i}"));
                    let dst = temp_dir.path().join(format!("moved_{i}"));
                    (
                        src.to_str().unwrap().to_string(),
                        dst.to_str().unwrap().to_string(),
                    )
                })
                .collect();
            (temp_dir, files)
        })
        .bench_values(|(temp_dir, files)| {
            for (src, dst) in &files {
                black_box(run_util_function(uumain, &[src, dst]));
            }
            drop(temp_dir);
        });
}

/// Benchmark moving multiple files to directory
#[divan::bench]
fn mv_multiple_to_dir(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let dest_dir = temp_dir.path().join("dest");
            std::fs::create_dir(&dest_dir).unwrap();

            let mut args: Vec<String> = (0..1000)
                .map(|i| {
                    temp_dir
                        .path()
                        .join(format!("f{i}"))
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect();
            args.push(dest_dir.to_str().unwrap().to_string());
            (temp_dir, args)
        })
        .bench_values(|(temp_dir, args)| {
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            black_box(run_util_function(uumain, &arg_refs));
            drop(temp_dir);
        });
}

/// Benchmark moving directory recursively
#[divan::bench]
fn mv_directory(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            let src_dir = temp_dir.path().join("src_tree");
            std::fs::create_dir(&src_dir).unwrap();
            // Increase tree size for longer benchmark
            fs_tree::create_balanced_tree(&src_dir, 5, 5, 10);
            let dst_dir = temp_dir.path().join("dest_tree");
            (
                temp_dir,
                src_dir.to_str().unwrap().to_string(),
                dst_dir.to_str().unwrap().to_string(),
            )
        })
        .bench_values(|(temp_dir, src, dst)| {
            black_box(run_util_function(uumain, &[&src, &dst]));
            drop(temp_dir);
        });
}

/// Benchmark force overwrite
#[divan::bench]
fn mv_force_overwrite(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 2000, 0);
            let files: Vec<(String, String)> = (0..1000)
                .map(|i| {
                    let src = temp_dir.path().join(format!("f{i}"));
                    let dst = temp_dir.path().join(format!("f{}", i + 1000));
                    (
                        src.to_str().unwrap().to_string(),
                        dst.to_str().unwrap().to_string(),
                    )
                })
                .collect();
            (temp_dir, files)
        })
        .bench_values(|(temp_dir, files)| {
            for (src, dst) in &files {
                black_box(run_util_function(uumain, &["-f", src, dst]));
            }
            drop(temp_dir);
        });
}

fn main() {
    divan::main();
}
