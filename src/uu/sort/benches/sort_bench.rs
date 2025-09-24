// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::path::PathBuf;
use uu_sort::uumain;
use uucore::benchmark::{create_test_file, run_util_function};

/// Helper function to generate test data from a list of words
fn generate_data_from_words(words: &[&str], num_lines: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..num_lines {
        let word = words[i % words.len()];
        let number = i % 1000;
        data.extend_from_slice(format!("{word}_{number:03}\n").as_bytes());
    }
    data
}

/// Helper function to generate test data from a list of words without number suffix
fn generate_data_from_words_simple(words: &[&str], num_lines: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..num_lines {
        let word = words[i % words.len()];
        data.extend_from_slice(format!("{word}\n").as_bytes());
    }
    data
}

/// Generate test data with ASCII-only text
fn generate_ascii_data(num_lines: usize) -> Vec<u8> {
    let words = [
        "apple",
        "banana",
        "cherry",
        "date",
        "elderberry",
        "fig",
        "grape",
        "honeydew",
        "kiwi",
        "lemon",
        "mango",
        "nectarine",
        "orange",
        "papaya",
        "quince",
        "raspberry",
        "strawberry",
        "tangerine",
        "ugli",
        "vanilla",
        "watermelon",
        "xigua",
        "yellow",
        "zucchini",
        "avocado",
    ];

    generate_data_from_words(&words, num_lines)
}

/// Generate test data with accented characters that require locale-aware sorting
fn generate_accented_data(num_lines: usize) -> Vec<u8> {
    let words = [
        // French words with accents
        "café",
        "naïve",
        "résumé",
        "fiancé",
        "crème",
        "déjà",
        "façade",
        "château",
        "élève",
        "côte",
        // German words with umlauts
        "über",
        "Müller",
        "schön",
        "Köln",
        "Düsseldorf",
        "Österreich",
        "Zürich",
        "Mädchen",
        "Bär",
        "größer",
        // Spanish words with tildes and accents
        "niño",
        "señor",
        "año",
        "mañana",
        "español",
        "corazón",
        "María",
        "José",
        "más",
        "también",
    ];

    generate_data_from_words(&words, num_lines)
}

/// Generate test data with mixed ASCII and non-ASCII characters
fn generate_mixed_data(num_lines: usize) -> Vec<u8> {
    let words = [
        // Mix of ASCII and accented words
        "apple",
        "café",
        "banana",
        "naïve",
        "cherry",
        "résumé",
        "date",
        "fiancé",
        "elderberry",
        "crème",
        "über",
        "grape",
        "Müller",
        "honeydew",
        "schön",
        "niño",
        "kiwi",
        "señor",
        "lemon",
        "año",
        "mango",
        "María",
        "orange",
        "José",
        "papaya",
    ];

    generate_data_from_words(&words, num_lines)
}

/// Generate test data with uppercase/lowercase variations
fn generate_case_sensitive_data(num_lines: usize) -> Vec<u8> {
    let base_words = [
        "apple", "Apple", "APPLE", "banana", "Banana", "BANANA", "café", "Café", "CAFÉ", "über",
        "Über", "ÜBER",
    ];

    generate_data_from_words_simple(&base_words, num_lines)
}

fn setup_test_file(data: &[u8]) -> PathBuf {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_test_file(data, temp_dir.path());
    // Keep temp_dir alive by leaking it - the OS will clean it up
    std::mem::forget(temp_dir);
    file_path
}

/// Benchmark sorting ASCII-only data
#[divan::bench(args = [100_000, 500_000])]
fn sort_ascii_only(bencher: Bencher, num_lines: usize) {
    let data = generate_ascii_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark sorting accented/non-ASCII data
#[divan::bench(args = [100_000, 500_000])]
fn sort_accented_data(bencher: Bencher, num_lines: usize) {
    let data = generate_accented_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark sorting mixed ASCII/non-ASCII data
#[divan::bench(args = [100_000, 500_000])]
fn sort_mixed_data(bencher: Bencher, num_lines: usize) {
    let data = generate_mixed_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark case-sensitive sorting with mixed case data
#[divan::bench(args = [100_000, 500_000])]
fn sort_case_sensitive(bencher: Bencher, num_lines: usize) {
    let data = generate_case_sensitive_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark case-insensitive sorting (fold case)
#[divan::bench(args = [100_000, 500_000])]
fn sort_case_insensitive(bencher: Bencher, num_lines: usize) {
    let data = generate_case_sensitive_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-f", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark dictionary order sorting (only blanks and alphanumeric)
#[divan::bench(args = [100_000, 500_000])]
fn sort_dictionary_order(bencher: Bencher, num_lines: usize) {
    let data = generate_mixed_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-d", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark numeric sorting with mixed data
#[divan::bench(args = [100_000, 500_000])]
fn sort_numeric(bencher: Bencher, num_lines: usize) {
    let mut data = Vec::new();

    // Generate numeric data with some text prefixes
    for i in 0..num_lines {
        let value = (i * 13) % 10000; // Pseudo-random numeric values
        data.extend_from_slice(format!("value_{value}\n").as_bytes());
    }

    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-n", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark reverse sorting with locale-aware data
#[divan::bench(args = [100_000, 500_000])]
fn sort_reverse_locale(bencher: Bencher, num_lines: usize) {
    let data = generate_accented_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-r", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark sorting with specific key field
#[divan::bench(args = [100_000, 500_000])]
fn sort_key_field(bencher: Bencher, num_lines: usize) {
    let mut data = Vec::new();

    // Generate data with multiple fields
    let words = ["café", "naïve", "apple", "über", "banana"];
    for i in 0..num_lines {
        let word = words[i % words.len()];
        let num1 = i % 100;
        let num2 = (i * 7) % 100;
        data.extend_from_slice(format!("{num1}\t{word}\t{num2}\n").as_bytes());
    }

    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        // Sort by second field
        black_box(run_util_function(
            uumain,
            &["-k", "2", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark unique sorting with locale-aware data
#[divan::bench(args = [100_000, 500_000])]
fn sort_unique_locale(bencher: Bencher, num_lines: usize) {
    let data = generate_accented_data(num_lines);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-u", file_path.to_str().unwrap()],
        ));
    });
}

fn main() {
    divan::main();
}
