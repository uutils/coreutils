// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_cksum::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark cksum with default CRC algorithm
#[divan::bench]
fn cksum_default(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark cksum on small file
#[divan::bench]
fn cksum_small_file(bencher: Bencher) {
    let data = text_data::generate_by_size(1, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark cksum on large file
#[divan::bench]
fn cksum_large_file(bencher: Bencher) {
    let data = text_data::generate_by_size(50, 80);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark cksum with raw output format
#[divan::bench]
fn cksum_raw_output(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);
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
            let data1 = text_data::generate_by_size(5, 80);
            let data2 = text_data::generate_by_size(5, 80);
            let data3 = text_data::generate_by_size(5, 80);

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

/// Benchmark cksum reading from stdin
#[divan::bench]
fn cksum_stdin(bencher: Bencher) {
    let data = text_data::generate_by_size(10, 80);

    bencher
        .with_inputs(|| {
            // Create temporary file with test data
            setup_test_file(&data)
        })
        .bench_values(|_file_path| {
            black_box(run_util_function(
                uumain,
                &["-"], // Read from stdin
            ));
        });
}

fn main() {
    divan::main();
}
