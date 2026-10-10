// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
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
                    get_bench_args(&[&path]).into_iter()
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
            let path_refs: Vec<&dyn AsRef<std::ffi::OsStr>> = paths
                .iter()
                .map(|path| path as &dyn AsRef<std::ffi::OsStr>)
                .collect();
            (temp_dir, get_bench_args(&path_refs).into_iter())
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
            let args = get_bench_args(&[&"-r", &test_dir]).into_iter();
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
            let mut args = vec![String::from("-f")];
            args.extend(paths);
            let arg_refs: Vec<&dyn AsRef<std::ffi::OsStr>> = args
                .iter()
                .map(|arg| arg as &dyn AsRef<std::ffi::OsStr>)
                .collect();
            (temp_dir, get_bench_args(&arg_refs).into_iter())
        })
        .bench_values(|(temp_dir, args)| {
            black_box(uumain(args));
            temp_dir
        });
}

fn main() {
    divan::main();
}
