// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
// use std::ffi::OsString;
use uu_base64::uumain;
use uucore::benchmark::{create_test_file, get_bench_args, text_data};

fn create_tmp_file(size_mb: usize) -> std::path::PathBuf {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, 80);
    create_test_file(&data, temp_dir.path())
}

fn redirect_in(in_file: &std::path::Path) -> std::ffi::OsString {
    let in_file_str = in_file.as_os_str();
    let mut s = std::ffi::OsString::with_capacity(&in_file_str.len() + 1);
    s.push(">");
    s.push(in_file_str);
    s
}

// fn redirect_in(in_file: &std::path::PathBuf) -> std::ffi::OsString {
//     std::iter::once(std::ffi::OsString::from(">").as_os_str())
//         .chain(std::iter::once(in_file.as_os_str()))
//         .collect()
// }

/// Benchmark for base64 encoding
#[divan::bench()]
fn b64_encode_synthetic(bencher: Bencher) {
    let file_path = create_tmp_file(5_000);

    bencher
        .with_inputs(|| get_bench_args(&[&file_path]))
        .bench_values(|args| black_box(uumain(args)));
}

// Benchmark for base64 decoding
#[divan::bench()]
fn b64_decode_synthetic(bencher: Bencher) {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_tmp_file(5_000);
    let in_file = create_test_file(b"", temp_dir.path());

    uumain([file_path.into(), redirect_in(&in_file)].into_iter());

    bencher
        .with_inputs(|| get_bench_args(&[&"-d", &in_file]))
        .bench_values(|args| black_box(uumain(args)));
}

// Benchmark different file sizes for base64 decoding ignoring garbage characters
#[divan::bench()]
fn b64_decode_ignore_garbage_synthetic(bencher: Bencher) {
    let tempdir = tempfile::tempdir().unwrap();
    let temp_dir = tempdir;
    let file_path = create_tmp_file(5_000);
    let in_file = create_test_file(b"", temp_dir.path());

    uumain([file_path.into(), redirect_in(&in_file)].into_iter());

    bencher
        .with_inputs(|| get_bench_args(&[&"-d", &"-i", &in_file]))
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
