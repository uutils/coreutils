// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;
use uu_cp::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

fn remove_path(path: &Path) {
    if !path.exists() {
        return;
    }

    if path.is_dir() {
        fs::remove_dir_all(path).unwrap();
    } else {
        fs::remove_file(path).unwrap();
    }
}

fn bench_cp_directory<F>(bencher: Bencher, args: &[&str], setup_source: F)
where
    F: Fn(&Path),
{
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source");
    let dest = temp_dir.path().join("dest");

    fs::create_dir(&source).unwrap();
    setup_source(&source);

    let source_str = source.to_str().unwrap();
    let dest_str = dest.to_str().unwrap();

    bencher.bench(|| {
        remove_path(&dest);

        let mut full_args = Vec::with_capacity(args.len() + 2);
        full_args.extend_from_slice(args);
        full_args.push(source_str);
        full_args.push(dest_str);

        black_box(run_util_function(uumain, &full_args));
    });
}

#[divan::bench(args = [(5, 4, 10)])]
fn cp_recursive_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    bench_cp_directory(bencher, &["-R"], |source| {
        fs_tree::create_balanced_tree(source, depth, dirs_per_level, files_per_dir);
    });
}

#[divan::bench(args = [(5, 4, 10)])]
fn cp_archive_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    bench_cp_directory(bencher, &["-a"], |source| {
        fs_tree::create_balanced_tree(source, depth, dirs_per_level, files_per_dir);
    });
}

#[divan::bench(args = [(6000, 800)])]
fn cp_recursive_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    bench_cp_directory(bencher, &["-R"], |source| {
        fs_tree::create_wide_tree(source, total_files, total_dirs);
    });
}

#[divan::bench(args = [(120, 4)])]
fn cp_recursive_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    bench_cp_directory(bencher, &["-R"], |source| {
        fs_tree::create_deep_tree(source, depth, files_per_level);
    });
}

#[divan::bench(args = [(5, 4, 10)])]
fn cp_preserve_metadata(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    bench_cp_directory(bencher, &["-R", "--preserve=mode,timestamps"], |source| {
        fs_tree::create_balanced_tree(source, depth, dirs_per_level, files_per_dir);
    });
}

#[divan::bench(args = [16])]
fn cp_large_file(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.bin");
    let dest = temp_dir.path().join("dest.bin");

    let buffer = vec![b'x'; size_mb * 1024 * 1024];
    let mut file = File::create(&source).unwrap();
    file.write_all(&buffer).unwrap();
    file.sync_all().unwrap();

    let source_str = source.to_str().unwrap();
    let dest_str = dest.to_str().unwrap();

    bencher.bench(|| {
        remove_path(&dest);

        black_box(run_util_function(uumain, &[source_str, dest_str]));
    });
}

fn main() {
    divan::main();
}
