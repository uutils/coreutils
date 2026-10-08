// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_mv::uumain;
use uucore::benchmark::{fs_tree, get_bench_args};

/// Benchmark moving a single file (repeated to reach 100ms)
#[divan::bench]
fn mv_single_file(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 1000, 0);
            let args: Vec<_> = (0..1000)
                .map(|i| {
                    let src = temp_dir.path().join(format!("f{i}"));
                    let dst = temp_dir.path().join(format!("moved_{i}"));
                    get_bench_args(&[&src, &dst]).into_iter()
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
            let args = get_bench_args(&[&src_dir, &dst_dir]).into_iter();
            (temp_dir, args)
        })
        .bench_values(|(temp_dir, args)| {
            black_box(uumain(args));
            temp_dir
        });
}

/// Benchmark force overwrite
#[divan::bench]
fn mv_force_overwrite(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            fs_tree::create_wide_tree(temp_dir.path(), 2000, 0);
            let args: Vec<_> = (0..1000)
                .map(|i| {
                    let src = temp_dir.path().join(format!("f{i}"));
                    let dst = temp_dir.path().join(format!("f{}", i + 1000));
                    get_bench_args(&[&"-f", &src, &dst]).into_iter()
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

fn main() {
    divan::main();
}
