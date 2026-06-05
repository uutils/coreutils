// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use tempfile::NamedTempFile;
use uu_sort::uumain;
use uucore::benchmark::{run_util_function, setup_test_file, text_data};

/// Benchmark merging pre-sorted files (`sort -m`).
///
/// Mirrors the "Merging" section from `BENCHMARKING.md`:
/// 1. Generate a shuffled wordlist (using the existing ASCII helper).
/// 2. Split it into slices.
/// 3. Sort each slice individually.
/// 4. Benchmark merging them back together with `sort -m`.
#[divan::bench]
fn merge_pre_sorted_files(bencher: Bencher) {
    const TOTAL_LINES: usize = 500_000;
    const NUM_FILES: usize = 8;

    // 1. Generate data (mimicking a shuffled wordlist)
    let data = text_data::generate_ascii_data(TOTAL_LINES);

    // 2. Split into chunks and 3. sort each chunk individually
    let lines: Vec<&[u8]> = data
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    let lines_per_file = lines.len() / NUM_FILES;

    let mut file_paths = Vec::with_capacity(NUM_FILES);
    for i in 0..NUM_FILES {
        let start = i * lines_per_file;
        let end = if i == NUM_FILES - 1 {
            lines.len()
        } else {
            (i + 1) * lines_per_file
        };

        let mut chunk_lines = lines[start..end].to_vec();
        chunk_lines.sort();

        let mut chunk_data = Vec::new();
        for line in chunk_lines {
            chunk_data.extend_from_slice(line);
            chunk_data.push(b'\n');
        }

        let path = setup_test_file(&chunk_data);
        file_paths.push(path);
    }

    let output_file = NamedTempFile::new().unwrap();
    let output_path = output_file.path().to_str().unwrap();

    let file_args: Vec<&str> = file_paths.iter().map(|p| p.to_str().unwrap()).collect();

    // 4. Benchmark the merge step
    bencher.bench(|| {
        let mut args = vec!["-m", "-o", output_path];
        args.extend(&file_args);
        black_box(run_util_function(uumain, &args));
    });
}

fn main() {
    divan::main();
}
