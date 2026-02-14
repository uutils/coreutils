// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::TempDir;
use uu_csplit::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark splitting by line count with many splits.
/// This exercises the buffer FIFO (push_back / pop_front) on every line.
#[divan::bench]
fn csplit_line_count_many_splits(bencher: Bencher) {
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
                    "1000",
                    "{99}",
                ],
            ));
            drop(output_dir);
        });
}

/// Benchmark splitting by regex with many matches.
/// Each regex match triggers buffer add/remove operations.
#[divan::bench]
fn csplit_regex_many_matches(bencher: Bencher) {
    // Generate data where every 100th line starts with "SPLIT"
    let mut data = Vec::new();
    for i in 0..100_000 {
        if i > 0 && i % 100 == 0 {
            data.extend_from_slice(format!("SPLIT marker line {i}\n").as_bytes());
        } else {
            data.extend_from_slice(format!("regular line {i}\n").as_bytes());
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
                    "/^SPLIT/",
                    "{*}",
                ],
            ));
            drop(output_dir);
        });
}

/// Benchmark splitting by regex with negative offset.
/// This exercises the rewind_buffer and shrink_buffer_to_size paths.
#[divan::bench]
fn csplit_regex_with_offset(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..100_000 {
        if i > 0 && i % 500 == 0 {
            data.extend_from_slice(format!("MARKER line {i}\n").as_bytes());
        } else {
            data.extend_from_slice(format!("content line {i}\n").as_bytes());
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
                    "/^MARKER/-2",
                    "{*}",
                ],
            ));
            drop(output_dir);
        });
}

fn main() {
    divan::main();
}
