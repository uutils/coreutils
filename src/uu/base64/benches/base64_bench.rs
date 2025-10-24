// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::Bencher;
use std::ffi::OsString;
use uu_base64::uumain;
use uucore::benchmark::{bench_util, create_test_file, run_util_function, text_data};

/// Benchmark for base64 encoding
#[divan::bench()]
fn b64_encode_synthetic(bencher: Bencher) {
    let data = text_data::generate_by_size(5_000, 80);
    bench_util(bencher, data, &[], uumain);
}

// Benchmark for base64 decoding
#[divan::bench()]
fn b64_decode_synthetic(bencher: Bencher) {
    let temp_dir = tempfile::tempdir().unwrap();
    let source_data = text_data::generate_by_size(5_000, 80);
    let source_file = create_test_file(&source_data, temp_dir.path());
    let encoded_file = create_test_file(b"", temp_dir.path());
    let encoded_file_str = encoded_file.to_str().unwrap();

    // First encode the data to create the test input
    uumain(
        [
            OsString::from(source_file.to_str().unwrap()),
            OsString::from(format!(">{encoded_file_str}")),
        ]
        .iter()
        .map(|x| (*x).clone()),
    );

    bencher.bench(|| {
        divan::black_box(run_util_function(uumain, &["-d", encoded_file_str]));
    });
}

// Benchmark different file sizes for base64 decoding ignoring garbage characters
#[divan::bench()]
fn b64_decode_ignore_garbage_synthetic(bencher: Bencher) {
    let temp_dir = tempfile::tempdir().unwrap();
    let source_data = text_data::generate_by_size(5_000, 80);
    let source_file = create_test_file(&source_data, temp_dir.path());
    let encoded_file = create_test_file(b"", temp_dir.path());
    let encoded_file_str = encoded_file.to_str().unwrap();

    // First encode the data to create the test input
    uumain(
        [
            OsString::from(source_file.to_str().unwrap()),
            OsString::from(format!(">{encoded_file_str}")),
        ]
        .iter()
        .map(|x| (*x).clone()),
    );

    bencher.bench(|| {
        divan::black_box(run_util_function(uumain, &["-d", "-i", encoded_file_str]));
    });
}

fn main() {
    divan::main();
}
