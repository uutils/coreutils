// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_split::uumain;
use uucore::benchmark::{get_bench_args, setup_test_file, text_data};

/// Benchmark splitting by line count
#[divan::bench]
fn split_lines(bencher: Bencher) {
    let data = text_data::generate_by_lines(100_000, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| {
            let output_dir = TempDir::new().unwrap();
            let prefix = output_dir.path().join("x");
            let args = get_bench_args(&[&"-l", &"1000", &file_path, &prefix]);
            (output_dir, args)
        })
        .bench_values(|(output_dir, args)| {
            black_box(uumain(args));
            drop(output_dir);
        });
}

/// Benchmark splitting by byte size
#[divan::bench]
fn split_bytes(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| {
            let output_dir = TempDir::new().unwrap();
            let prefix = output_dir.path().join("x");
            let args = get_bench_args(&[&"-b", &"100K", &file_path, &prefix]);
            (output_dir, args)
        })
        .bench_values(|(output_dir, args)| {
            black_box(uumain(args));
            drop(output_dir);
        });
}

/// Benchmark splitting by number of chunks
#[divan::bench]
fn split_number_chunks(bencher: Bencher) {
    let data = text_data::generate_by_lines(100_000, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| {
            let output_dir = TempDir::new().unwrap();
            let prefix = output_dir.path().join("x");
            let args = get_bench_args(&[&"-n", &"10", &file_path, &prefix]);
            (output_dir, args)
        })
        .bench_values(|(output_dir, args)| {
            black_box(uumain(args));
            drop(output_dir);
        });
}

/// Benchmark splitting with numeric suffix
#[divan::bench]
fn split_numeric_suffix(bencher: Bencher) {
    let data = text_data::generate_by_lines(100_000, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| {
            let output_dir = TempDir::new().unwrap();
            let prefix = output_dir.path().join("x");
            let args = get_bench_args(&[&"-d", &"-l", &"500", &file_path, &prefix]);
            (output_dir, args)
        })
        .bench_values(|(output_dir, args)| {
            black_box(uumain(args));
            drop(output_dir);
        });
}

fn main() {
    divan::main();
}
