// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::ffi::OsString;
use uu_base64::uumain;
use uucore::benchmark::{create_test_file, run_util_function, text_data};

fn create_tmp_file(size_mb: usize) -> String {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = text_data::generate_by_size(size_mb, 80);
    let file_path = create_test_file(&data, temp_dir.path());
    String::from(file_path.to_str().unwrap())
}

/// Benchmark for base64 encoding
#[divan::bench()]
fn b64_encode_synthetic(bencher: Bencher) {
    let file_path_str = &create_tmp_file(5_000);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

// Benchmark for base64 decoding
#[divan::bench()]
fn b64_decode_synthetic(bencher: Bencher) {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path_str = &create_tmp_file(5_000);
    let in_file = create_test_file(b"", temp_dir.path());
    let in_file_str = in_file.to_str().unwrap();
    uumain(
        [
            OsString::from(file_path_str),
            OsString::from(format!(">{in_file_str}")),
        ]
        .iter()
        .map(|x| (*x).clone()),
    );

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-d", in_file_str]));
    });
}

// Benchmark different file sizes for base64 decoding ignoring garbage characters
#[divan::bench()]
fn b64_decode_ignore_garbage_synthetic(bencher: Bencher) {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path_str = &create_tmp_file(5_000);
    let in_file = create_test_file(b"", temp_dir.path());
    let in_file_str = in_file.to_str().unwrap();
    uumain(
        [
            OsString::from(file_path_str),
            OsString::from(format!(">{in_file_str}")),
        ]
        .iter()
        .map(|x| (*x).clone()),
    );

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-d", "-i", in_file_str]));
    });
}

fn main() {
    divan::main();
}
