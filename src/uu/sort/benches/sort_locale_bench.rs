// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::env;
use std::path::PathBuf;
use uu_sort::uumain;
use uucore::benchmark::{create_test_file, run_util_function};

/// Helper function to generate test data from a list of words
fn generate_data_from_words_with_counter(words: &[&str], num_lines: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..num_lines {
        let word = words[i % words.len()];
        let line = format!("{word}{i:04}\n");
        data.extend_from_slice(line.as_bytes());
    }
    data
}

fn generate_ascii_data(num_lines: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..num_lines {
        let line = format!("line_{:06}\n", (num_lines - i - 1));
        data.extend_from_slice(line.as_bytes());
    }
    data
}

fn generate_mixed_locale_data(num_lines: usize) -> Vec<u8> {
    let mixed_strings = [
        "zebra", "äpfel", "banana", "öl", "cat", "über", "dog", "zürich", "elephant", "café",
        "fish", "naïve", "grape", "résumé", "house", "piñata",
    ];
    generate_data_from_words_with_counter(&mixed_strings, num_lines)
}

fn generate_german_locale_data(num_lines: usize) -> Vec<u8> {
    let german_words = [
        "Ärger", "Öffnung", "Über", "Zucker", "Bär", "Föhn", "Größe", "Höhe", "Käse", "Löwe",
        "Mädchen", "Nüsse", "Röntgen", "Schäfer", "Tür", "Würfel", "ä", "ö", "ü", "ß", "a", "o",
        "u", "s",
    ];
    generate_data_from_words_with_counter(&german_words, num_lines)
}

fn generate_random_strings(num_lines: usize, length: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let charset =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789äöüÄÖÜßéèêëàâîïôûç";
    let charset_bytes = charset.as_bytes();

    for i in 0..num_lines {
        let mut line = String::new();
        for j in 0..length {
            let idx = ((i * length + j) * 17 + 42) % charset_bytes.len();
            line.push(charset_bytes[idx] as char);
        }
        line.push('\n');
        data.extend_from_slice(line.as_bytes());
    }
    data
}

fn setup_test_file(data: &[u8]) -> PathBuf {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_test_file(data, temp_dir.path());
    // Keep temp_dir alive by leaking it - the OS will clean it up
    std::mem::forget(temp_dir);
    file_path
}

/// Benchmark ASCII-only data sorting with C locale (byte comparison)
#[divan::bench]
fn sort_ascii_c_locale(bencher: Bencher) {
    let temp_dir = tempfile::tempdir().unwrap();
    let data = generate_ascii_data(100_000);
    let file_path = create_test_file(&data, temp_dir.path());

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "C");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark ASCII-only data sorting with UTF-8 locale
#[divan::bench]
fn sort_ascii_utf8_locale(bencher: Bencher) {
    let data = generate_ascii_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "en_US.UTF-8");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark mixed ASCII/Unicode data with C locale
#[divan::bench]
fn sort_mixed_c_locale(bencher: Bencher) {
    let data = generate_mixed_locale_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "C");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark mixed ASCII/Unicode data with UTF-8 locale
#[divan::bench]
fn sort_mixed_utf8_locale(bencher: Bencher) {
    let data = generate_mixed_locale_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "en_US.UTF-8");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark German locale-specific data with C locale
#[divan::bench]
fn sort_german_c_locale(bencher: Bencher) {
    let data = generate_german_locale_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "C");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark German locale-specific data with German locale
#[divan::bench]
fn sort_german_locale(bencher: Bencher) {
    let data = generate_german_locale_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "de_DE.UTF-8");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark random strings of different lengths
#[divan::bench]
fn sort_random_strings(bencher: Bencher) {
    let data = generate_random_strings(10_000, 50);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "en_US.UTF-8");
        }
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

/// Benchmark numeric sorting performance
#[divan::bench]
fn sort_numeric(bencher: Bencher) {
    let mut data = Vec::new();
    for i in 0..10_000 {
        let line = format!("{}\n", 10_000 - i);
        data.extend_from_slice(line.as_bytes());
    }
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "en_US.UTF-8");
        }
        black_box(run_util_function(
            uumain,
            &["-n", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark reverse sorting
#[divan::bench]
fn sort_reverse_mixed(bencher: Bencher) {
    let data = generate_mixed_locale_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "en_US.UTF-8");
        }
        black_box(run_util_function(
            uumain,
            &["-r", file_path.to_str().unwrap()],
        ));
    });
}

/// Benchmark unique sorting
#[divan::bench]
fn sort_unique_mixed(bencher: Bencher) {
    let data = generate_mixed_locale_data(10_000);
    let file_path = setup_test_file(&data);

    bencher.bench(|| {
        unsafe {
            env::set_var("LC_ALL", "en_US.UTF-8");
        }
        black_box(run_util_function(
            uumain,
            &["-u", file_path.to_str().unwrap()],
        ));
    });
}

fn main() {
    divan::main();
}
