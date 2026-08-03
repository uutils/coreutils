// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_cut::uumain;
use uucore::benchmark::{get_bench_args, setup_test_file, text_data};

/// Benchmark cutting specific byte ranges
#[divan::bench]
fn cut_bytes(bencher: Bencher) {
    let data = text_data::generate_by_lines(100_000, 80);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&"-b", &"1-20", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark cutting specific character ranges
#[divan::bench]
fn cut_characters(bencher: Bencher) {
    let data = text_data::generate_mixed_data(100_000);
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&"-c", &"5-30", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark cutting a range that both starts and ends inside long multi-byte
/// lines. `cut_characters` uses short lines and a range running past their end,
/// so it measures the fixed per-line cost; here the character walk dominates.
#[divan::bench]
fn cut_characters_long_lines(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..40_000 {
        // Roughly 100 characters a line, a third of them multi-byte, so that
        // neither end of the range lands on a byte offset equal to its
        // character position.
        let line = format!("{}école-piñata-{i:05}-Straße-", "aéiöu".repeat(16));
        data.extend_from_slice(line.as_bytes());
        data.push(b'\n');
    }
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&"-c", &"20-70", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark cutting fields with tab delimiter
#[divan::bench]
fn cut_fields_tab(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..100_000 {
        let line = format!("field1\tfield2_{i}\tfield3\tfield4\tfield5\n");
        data.extend_from_slice(line.as_bytes());
    }
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&"-f", &"2,4", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark cutting fields with custom delimiter
#[divan::bench]
fn cut_fields_custom_delim(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..100_000 {
        let line = format!("apple,banana_{i},cherry,date,elderberry\n");
        data.extend_from_slice(line.as_bytes());
    }
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&"-d", &",", &"-f", &"1,3,5", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark cutting fields with newline delimiter
#[divan::bench]
fn cut_fields_newline_delim(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..100_000 {
        let line = format!("field_content_number_{i}\n");
        data.extend_from_slice(line.as_bytes());
    }
    let file_path = setup_test_file(&data);

    bencher
        .with_inputs(|| get_bench_args(&[&"-d", &"\n", &"-f", &"1,3,5", &file_path]).into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    divan::main();
}
