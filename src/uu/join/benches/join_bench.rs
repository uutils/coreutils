// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_join::uumain;
use uucore::benchmark::{run_util_function, setup_test_file};

/// Benchmark basic join with sorted data
#[divan::bench]
fn join_basic_sorted(bencher: Bencher) {
    bencher.bench(|| {
        let num_lines = 10000;
        let mut file1_data = Vec::new();
        let mut file2_data = Vec::new();

        // Generate sorted test data with 50% overlap
        for i in 0..num_lines {
            let key = if i < num_lines / 2 {
                format!("key_{i:06}")
            } else {
                format!("unique1_{:06}", i - num_lines / 2)
            };

            file1_data.extend_from_slice(
                format!("{key}\tfile1_field1_{i}\tfile1_field2_{i}\n").as_bytes(),
            );

            let key2 = if i < num_lines / 2 {
                format!("key_{i:06}")
            } else {
                format!("unique2_{:06}", i - num_lines / 2)
            };

            file2_data.extend_from_slice(
                format!("{key2}\tfile2_field1_{i}\tfile2_field2_{i}\n").as_bytes(),
            );
        }

        // Sort the data
        let file1_lines: Vec<&str> = std::str::from_utf8(&file1_data).unwrap().lines().collect();
        let file2_lines: Vec<&str> = std::str::from_utf8(&file2_data).unwrap().lines().collect();

        let mut sorted_file1: Vec<_> = file1_lines.clone();
        let mut sorted_file2: Vec<_> = file2_lines.clone();

        sorted_file1.sort_unstable();
        sorted_file2.sort_unstable();

        let sorted_file1_data = (sorted_file1.join("\n") + "\n").into_bytes();
        let sorted_file2_data = (sorted_file2.join("\n") + "\n").into_bytes();

        let file1_path = setup_test_file(&sorted_file1_data);
        let file2_path = setup_test_file(&sorted_file2_data);

        black_box(run_util_function(
            uumain,
            &[file1_path.to_str().unwrap(), file2_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark join with custom delimiter
#[divan::bench]
fn join_custom_delimiter(bencher: Bencher) {
    bencher.bench(|| {
        let num_lines = 10000;
        let mut file1_data = Vec::new();
        let mut file2_data = Vec::new();

        // Generate CSV-style data
        for i in 0..num_lines {
            let key = format!("key_{i:06}");
            file1_data.extend_from_slice(format!("{key},value1_{i},data1_{i}\n").as_bytes());
            file2_data.extend_from_slice(format!("{key},value2_{i},data2_{i}\n").as_bytes());
        }

        let file1_path = setup_test_file(&file1_data);
        let file2_path = setup_test_file(&file2_data);

        black_box(run_util_function(
            uumain,
            &[
                "-t",
                ",",
                file1_path.to_str().unwrap(),
                file2_path.to_str().unwrap(),
            ],
        ));
    });
}

/// Benchmark join with no order checking (unsorted data)
#[divan::bench]
fn join_nocheck_order(bencher: Bencher) {
    bencher.bench(|| {
        let num_lines = 10000;
        let mut file1_data = Vec::new();
        let mut file2_data = Vec::new();

        // Generate unsorted test data with 50% overlap
        for i in 0..num_lines {
            let key = if i < num_lines / 2 {
                format!("key_{i:06}")
            } else {
                format!("unique1_{:06}", i - num_lines / 2)
            };

            file1_data.extend_from_slice(
                format!("{key}\tfile1_field1_{i}\tfile1_field2_{i}\n").as_bytes(),
            );

            let key2 = if i < num_lines / 2 {
                format!("key_{i:06}")
            } else {
                format!("unique2_{:06}", i - num_lines / 2)
            };

            file2_data.extend_from_slice(
                format!("{key2}\tfile2_field1_{i}\tfile2_field2_{i}\n").as_bytes(),
            );
        }

        let file1_path = setup_test_file(&file1_data);
        let file2_path = setup_test_file(&file2_data);

        black_box(run_util_function(
            uumain,
            &[
                "--nocheck-order",
                file1_path.to_str().unwrap(),
                file2_path.to_str().unwrap(),
            ],
        ));
    });
}

fn main() {
    divan::main();
}
