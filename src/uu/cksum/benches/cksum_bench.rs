// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_cksum::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

// Macro to generate benchmarks for each algorithm
macro_rules! bench_algorithm {
    ($algo_name:ident, $algo_str:expr) => {
        #[divan::bench]
        fn $algo_name(bencher: Bencher) {
            let data = text_data::generate_by_size(100, 80);
            let file_path = setup_test_file(&data);

            bencher.bench(|| {
                black_box(run_util_function(
                    uumain,
                    &["--algorithm", $algo_str, file_path.to_str().unwrap()],
                ));
            });
        }
    };
    ($algo_name:ident, $algo_str:expr, $length:expr) => {
        #[divan::bench]
        fn $algo_name(bencher: Bencher) {
            let data = text_data::generate_by_size(100, 80);
            let file_path = setup_test_file(&data);

            bencher.bench(|| {
                black_box(run_util_function(
                    uumain,
                    &[
                        "--algorithm",
                        $algo_str,
                        "--length",
                        $length,
                        file_path.to_str().unwrap(),
                    ],
                ));
            });
        }
    };
}

// Special macro for SHAKE algorithms that require length parameter
// Since SHAKE algorithms have fundamental --length parameter conflicts in cksum,
// we implement them using direct digest calculation for meaningful benchmarks
macro_rules! bench_shake_algorithm {
    ($algo_name:ident, $algo_str:expr, Shake128) => {
        #[divan::bench]
        fn $algo_name(bencher: Bencher) {
            use uucore::sum::{Digest, Shake128};

            let data = text_data::generate_by_size(100, 80);

            bencher.bench(|| {
                let mut shake = Shake128::new();
                shake.hash_update(&data);

                // SHAKE algorithms can output any length, use 256 bits (32 bytes) for meaningful comparison
                let mut output = [0u8; 32];
                shake.hash_finalize(&mut output);

                black_box(output);
            });
        }
    };
    ($algo_name:ident, $algo_str:expr, Shake256) => {
        #[divan::bench]
        fn $algo_name(bencher: Bencher) {
            use uucore::sum::{Digest, Shake256};

            let data = text_data::generate_by_size(100, 80);

            bencher.bench(|| {
                let mut shake = Shake256::new();
                shake.hash_update(&data);

                // SHAKE algorithms can output any length, use 256 bits (32 bytes) for meaningful comparison
                let mut output = [0u8; 32];
                shake.hash_finalize(&mut output);

                black_box(output);
            });
        }
    };
}

// Generate benchmarks for all supported algorithms
bench_algorithm!(cksum_sysv, "sysv");
bench_algorithm!(cksum_bsd, "bsd");
bench_algorithm!(cksum_crc, "crc");
bench_algorithm!(cksum_crc32b, "crc32b");
bench_algorithm!(cksum_md5, "md5");
bench_algorithm!(cksum_sha1, "sha1");
bench_algorithm!(cksum_sha2, "sha2", "256");
bench_algorithm!(cksum_sha3, "sha3", "256");
bench_algorithm!(cksum_blake2b, "blake2b");
bench_algorithm!(cksum_sm3, "sm3");
bench_algorithm!(cksum_sha224, "sha224");
bench_algorithm!(cksum_sha256, "sha256");
bench_algorithm!(cksum_sha384, "sha384");
bench_algorithm!(cksum_sha512, "sha512");
bench_algorithm!(cksum_blake3, "blake3");
bench_shake_algorithm!(cksum_shake128, "shake128", Shake128);
bench_shake_algorithm!(cksum_shake256, "shake256", Shake256);

/// Benchmark cksum with default CRC algorithm
#[divan::bench]
fn cksum_default(bencher: Bencher) {
    let data = text_data::generate_by_size(100, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark cksum with raw output format
#[divan::bench]
fn cksum_raw_output(bencher: Bencher) {
    let data = text_data::generate_by_size(100, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["--raw", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark cksum processing multiple files
#[divan::bench]
fn cksum_multiple_files(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let data1 = text_data::generate_by_size(50, 80);
            let data2 = text_data::generate_by_size(50, 80);
            let data3 = text_data::generate_by_size(50, 80);

            let file1 = setup_test_file(&data1);
            let file2 = setup_test_file(&data2);
            let file3 = setup_test_file(&data3);

            (file1, file2, file3)
        })
        .bench_values(|(file1, file2, file3)| {
            black_box(run_util_function(
                uumain,
                &[
                    file1.to_str().unwrap(),
                    file2.to_str().unwrap(),
                    file3.to_str().unwrap(),
                ],
            ));
        });
}

fn main() {
    divan::main();
}
