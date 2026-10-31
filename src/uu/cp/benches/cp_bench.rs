// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use uu_cp::uumain;
use uucore::benchmark::{binary_data, fs_tree, get_bench_args};

fn bench_cp_directory<F>(bencher: Bencher, args: &[&str], setup_source: F)
where
    F: Fn(&Path),
{
    let source_dir = TempDir::new().unwrap();
    let source = source_dir.path().join("source");
    fs::create_dir(&source).unwrap();
    setup_source(&source);

    let mut base_args = get_bench_args(&[]);
    base_args.extend(args.iter().map(|arg| (*arg).into()));
    base_args.push(source.into_os_string());

    bencher
        .with_inputs(|| {
            let dest_dir = TempDir::new_in(source_dir.path()).unwrap();
            let dest = dest_dir.path().join("dest");
            let mut args = base_args.clone();
            args.push(dest.into_os_string());
            (dest_dir, args.into_iter())
        })
        .bench_values(|(dest_dir, args)| {
            black_box(uumain(args));
            dest_dir
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
    bencher
        .with_inputs(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = temp_dir.path().join("source.bin");
            binary_data::create_file(&source, size_mb, b'x');
            // Use unique destination name to avoid filesystem allocation variance
            let dest = temp_dir
                .path()
                .join(format!("dest_{}.bin", (&raw const temp_dir).addr()));
            get_bench_args(&[&source, &dest]).into_iter()
        })
        .counter(divan::counter::BytesCount::new(size_mb * 1024 * 1024))
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
