// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_uniq::uumain;
use uucore::benchmark::{run_util_function, setup_test_file};

/// Generate data with many consecutive duplicate lines
/// This directly tests the core optimization of PR #8703 - avoiding allocations when comparing lines
fn generate_duplicate_heavy_data(num_groups: usize, duplicates_per_group: usize) -> Vec<u8> {
    let mut data = Vec::new();

    for group in 0..num_groups {
        // Generate a line with realistic content
        let line = format!(
            "Line content for group {group:06} with additional text to make it more realistic for testing performance\n"
        );

        // Repeat the line multiple times (this is what PR #8703 optimizes)
        for _ in 0..duplicates_per_group {
            data.extend_from_slice(line.as_bytes());
        }
    }

    data
}

/// Benchmark 1: Heavy duplicates - the main optimization target
/// Many consecutive duplicate lines that stress the line comparison optimization
#[divan::bench(args = [10_000])]
fn uniq_heavy_duplicates(bencher: Bencher, num_lines: usize) {
    // Create 1000 groups with ~10,000 duplicates each
    // This maximizes the benefit of PR #8703's optimization
    let num_groups = 1000;
    let duplicates_per_group = num_lines / num_groups;
    let data = generate_duplicate_heavy_data(num_groups, duplicates_per_group);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark 2: Mixed duplicates with counting
/// Tests the -c flag with a mix of duplicate groups
#[divan::bench(args = [10_000])]
fn uniq_with_count(bencher: Bencher, num_lines: usize) {
    // Create more groups with fewer duplicates for varied counting
    let num_groups = num_lines / 100;
    let data = generate_duplicate_heavy_data(num_groups, 100);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-c", file_path_str]));
    });
}

/// Benchmark 3: Case-insensitive comparison with duplicates
/// Tests the -i flag which requires case folding during comparison
#[divan::bench(args = [10_000])]
fn uniq_case_insensitive(bencher: Bencher, num_lines: usize) {
    let mut data = Vec::new();
    let words = [
        "Hello",
        "WORLD",
        "Testing",
        "UNIQ",
        "Benchmark",
        "Performance",
    ];

    // Generate groups of case variations
    for i in 0..num_lines {
        let word = words[(i / 50) % words.len()];

        // Create case variations that should be treated as duplicates with -i
        let variation = match i % 4 {
            0 => word.to_lowercase(),
            1 => word.to_uppercase(),
            2 => word.to_string(),
            _ => {
                // Mixed case
                word.chars()
                    .enumerate()
                    .map(|(idx, c)| {
                        if idx % 2 == 0 {
                            c.to_lowercase().to_string()
                        } else {
                            c.to_uppercase().to_string()
                        }
                    })
                    .collect()
            }
        };

        data.extend_from_slice(format!("{variation}\n").as_bytes());
    }

    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &["-i", file_path_str]));
    });
}

fn main() {
    divan::main();
}
