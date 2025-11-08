// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;
use uu_dd::uumain;
use uucore::benchmark::run_util_function;

fn create_test_file(path: &Path, size_mb: usize) {
    let buffer = vec![b'x'; size_mb * 1024 * 1024];
    let mut file = File::create(path).unwrap();
    file.write_all(&buffer).unwrap();
    file.sync_all().unwrap();
}

fn remove_file(path: &Path) {
    if path.exists() {
        fs::remove_file(path).unwrap();
    }
}

/// Benchmark basic dd copy with default settings
#[divan::bench(args = [32])]
fn dd_copy_default(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd copy with 4KB block size (common page size)
#[divan::bench(args = [24])]
fn dd_copy_4k_blocks(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=4K",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd copy with 64KB block size
#[divan::bench(args = [64])]
fn dd_copy_64k_blocks(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=64K",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd copy with 1MB block size
#[divan::bench(args = [128])]
fn dd_copy_1m_blocks(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=1M",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd copy with separate input and output block sizes
#[divan::bench(args = [48])]
fn dd_copy_separate_blocks(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "ibs=8K",
                "obs=16K",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd with count limit (partial copy)
#[divan::bench(args = [32])]
fn dd_copy_partial(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=4K",
                "count=1024",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd with skip (seeking in input)
#[divan::bench(args = [48])]
fn dd_copy_with_skip(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=4K",
                "skip=256",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd with seek (seeking in output)
#[divan::bench(args = [48])]
fn dd_copy_with_seek(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=4K",
                "seek=256",
                "status=none",
            ],
        ));
    });
}

/// Benchmark dd with different block sizes for comparison
#[divan::bench(args = [32])]
fn dd_copy_8k_blocks(bencher: Bencher, size_mb: usize) {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("input.bin");
    let output = temp_dir.path().join("output.bin");

    create_test_file(&input, size_mb);

    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();

    bencher.bench(|| {
        remove_file(&output);
        black_box(run_util_function(
            uumain,
            &[
                &format!("if={input_str}"),
                &format!("of={output_str}"),
                "bs=8K",
                "status=none",
            ],
        ));
    });
}

fn main() {
    divan::main();
}
