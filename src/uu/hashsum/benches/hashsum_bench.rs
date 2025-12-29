// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::io::Write;
use tempfile::NamedTempFile;
use uu_hashsum::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark MD5 hashing
#[divan::bench]
fn hashsum_md5(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--md5", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark SHA1 hashing
#[divan::bench]
fn hashsum_sha1(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--sha1", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark SHA256 hashing
#[divan::bench]
fn hashsum_sha256(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--sha256", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark SHA512 hashing
#[divan::bench]
fn hashsum_sha512(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--sha512", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark MD5 checksum verification
#[divan::bench]
fn hashsum_md5_check(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            // Create test file
            let data = text_data::generate_by_size(10, 80);
            let test_file = setup_test_file(&data);

            // Create checksum file - keep it alive by returning it
            let checksum_file = NamedTempFile::new().unwrap();
            let checksum_path = checksum_file.path().to_str().unwrap().to_string();

            // Write checksum content
            {
                let mut file = std::fs::File::create(&checksum_path).unwrap();
                writeln!(
                    file,
                    "d41d8cd98f00b204e9800998ecf8427e  {}",
                    test_file.to_str().unwrap()
                )
                .unwrap();
            }

            (checksum_file, checksum_path)
        })
        .bench_values(|(_checksum_file, checksum_path)| {
            black_box(run_util_function(
                uumain,
                &["--md5", "--check", &checksum_path],
            ));
        });
}

/// Benchmark SHA256 checksum verification
#[divan::bench]
fn hashsum_sha256_check(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            // Create test file
            let data = text_data::generate_by_size(10, 80);
            let test_file = setup_test_file(&data);

            // Create checksum file - keep it alive by returning it
            let checksum_file = NamedTempFile::new().unwrap();
            let checksum_path = checksum_file.path().to_str().unwrap().to_string();

            // Write checksum content
            {
                let mut file = std::fs::File::create(&checksum_path).unwrap();
                writeln!(
                    file,
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  {}",
                    test_file.to_str().unwrap()
                )
                .unwrap();
            }

            (checksum_file, checksum_path)
        })
        .bench_values(|(_checksum_file, checksum_path)| {
            black_box(run_util_function(
                uumain,
                &["--sha256", "--check", &checksum_path],
            ));
        });
}

fn main() {
    divan::main();
}
