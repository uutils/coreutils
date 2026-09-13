// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_csplit::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark splitting by line number
#[divan::bench]
fn csplit_line_number(bencher: Bencher) {
    let data = text_data::generate_by_lines(100_000, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| {
            let output_dir = TempDir::new().unwrap();
            let prefix = output_dir.path().join("xx");
            (output_dir, prefix.to_str().unwrap().to_string())
        })
        .bench_values(|(output_dir, prefix)| {
            black_box(run_util_function(
                uumain,
                &[
                    "-f",
                    &prefix,
                    file_path.to_str().unwrap(),
                    "10000",
                    "50000",
                    "90000",
                ],
            ));
            drop(output_dir);
        });
}

/// Benchmark splitting by regex pattern
#[divan::bench]
fn csplit_regex_pattern(bencher: Bencher) {
    // Generate data with periodic marker lines that we can split on
    let mut data = Vec::new();
    for i in 0..100_000 {
        if i % 10_000 == 0 && i > 0 {
            data.extend_from_slice(format!("SECTION {i}\n").as_bytes());
        } else {
            data.extend_from_slice(format!("line {i} with some content to process\n").as_bytes());
        }
    }
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| {
            let output_dir = TempDir::new().unwrap();
            let prefix = output_dir.path().join("xx");
            (output_dir, prefix.to_str().unwrap().to_string())
        })
        .bench_values(|(output_dir, prefix)| {
            black_box(run_util_function(
                uumain,
                &[
                    "-f",
                    &prefix,
                    file_path.to_str().unwrap(),
                    "/^SECTION/",
                    "{*}",
                ],
            ));
            drop(output_dir);
        });
}

fn main() {
    divan::main();
}
