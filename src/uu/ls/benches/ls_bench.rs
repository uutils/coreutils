// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::{env, fs};
use tempfile::TempDir;
use uu_ls::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

/// Helper to run ls with given arguments on a directory
fn bench_ls_with_args(bencher: Bencher, temp_dir: &TempDir, args: &[&str]) {
    let temp_path_str = temp_dir.path().to_str().unwrap();
    let mut full_args = vec!["-R"];
    full_args.extend_from_slice(args);
    full_args.push(temp_path_str);

    bencher.bench(|| {
        black_box(run_util_function(uumain, &full_args));
    });
}

/// Benchmark ls -R on balanced directory tree
#[divan::bench(args = [(6, 4, 15)])]
fn ls_recursive_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on balanced directory tree (tests PR #8728 optimization)
#[divan::bench(args = [(6, 4, 15)])]
fn ls_recursive_long_all_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_balanced_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir);
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on wide directory structures
#[divan::bench(args = [(10000, 1000)])]
fn ls_recursive_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on wide directory structures
#[divan::bench(args = [(15000, 1500)])]
fn ls_recursive_long_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_wide_tree(temp_dir.path(), total_files, total_dirs);
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on deep directory structures
#[divan::bench(args = [(200, 2)])]
fn ls_recursive_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on deep directory structures
#[divan::bench(args = [(100, 4)])]
fn ls_recursive_long_all_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on mixed file types (comprehensive real-world test)
#[divan::bench]
fn ls_recursive_mixed_tree(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_mixed_tree(temp_dir.path());

    for i in 0..10 {
        let subdir = temp_dir.path().join(format!("mixed_branch_{i}"));
        fs::create_dir(&subdir).unwrap();
        fs_tree::create_mixed_tree(&subdir);
    }

    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on mixed file types (most comprehensive test)
#[divan::bench]
fn ls_recursive_long_all_mixed_tree(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_mixed_tree(temp_dir.path());

    for i in 0..10 {
        let subdir = temp_dir.path().join(format!("mixed_branch_{i}"));
        fs::create_dir(&subdir).unwrap();
        fs_tree::create_mixed_tree(&subdir);
    }

    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

// ================ LOCALE-AWARE SORTING BENCHMARKS ================

/// Benchmark ls sorting with C locale (byte comparison) vs UTF-8 locale
#[divan::bench(args = [("ascii", 1000), ("mixed", 1000), ("ascii", 5000), ("mixed", 5000)])]
fn ls_locale_sorting(bencher: Bencher, (dataset_type, file_count): (&str, usize)) {
    let temp_dir = TempDir::new().unwrap();

    // Generate appropriate dataset
    let names: Vec<String> = match dataset_type {
        "ascii" => {
            // Pure ASCII names
            (0..file_count).map(|i| format!("file_{i:04}")).collect()
        }
        "mixed" => {
            // Mix of ASCII and Unicode names with diacritics
            let unicode_names = [
                "äpfel",
                "Äpfel",
                "über",
                "Über",
                "öffnung",
                "Öffnung",
                "café",
                "résumé",
                "naïve",
                "piñata",
                "señor",
                "niño",
                "élève",
                "château",
                "crème",
                "français",
            ];
            (0..file_count)
                .map(|i| {
                    if i % 3 == 0 {
                        unicode_names[i % unicode_names.len()].to_string() + &i.to_string()
                    } else {
                        format!("file_{i:04}")
                    }
                })
                .collect()
        }
        _ => panic!("Unknown dataset type"),
    };

    // Create files
    for name in &names {
        fs::File::create(temp_dir.path().join(name)).unwrap();
    }

    let temp_path_str = temp_dir.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-1", "--color=never", temp_path_str],
        ));
    });
}

/// Benchmark ls with C locale explicitly set (tests byte comparison fallback)
#[divan::bench(args = [500, 2000])]
fn ls_c_locale_explicit(bencher: Bencher, file_count: usize) {
    let temp_dir = TempDir::new().unwrap();

    // Create files with mixed ASCII and Unicode names
    let names: Vec<String> = (0..file_count)
        .map(|i| match i % 4 {
            0 => format!("file_{i:04}"),
            1 => format!("äpfel_{i:04}"),
            2 => format!("über_{i:04}"),
            _ => format!("café_{i:04}"),
        })
        .collect();

    for name in &names {
        fs::File::create(temp_dir.path().join(name)).unwrap();
    }

    let temp_path_str = temp_dir.path().to_str().unwrap();

    bencher.bench(|| {
        // Set C locale to force byte comparison
        unsafe {
            env::set_var("LC_ALL", "C");
        }
        black_box(run_util_function(
            uumain,
            &["-1", "--color=never", temp_path_str],
        ));
        unsafe {
            env::remove_var("LC_ALL");
        }
    });
}

/// Benchmark ls with German locale for umlauts sorting
#[divan::bench(args = [500, 2000])]
fn ls_german_locale(bencher: Bencher, file_count: usize) {
    let temp_dir = TempDir::new().unwrap();

    // Create files with German umlauts
    let german_words = [
        "Apfel", "Äpfel", "Bär", "Föhn", "Größe", "Höhe", "Käse", "Löwe", "Mädchen", "Nüsse",
        "Öffnung", "Röntgen", "Schäfer", "Tür", "Über", "Würfel",
    ];

    let names: Vec<String> = (0..file_count)
        .map(|i| {
            let base = german_words[i % german_words.len()];
            format!("{base}_{i:04}")
        })
        .collect();

    for name in &names {
        fs::File::create(temp_dir.path().join(name)).unwrap();
    }

    let temp_path_str = temp_dir.path().to_str().unwrap();

    bencher.bench(|| {
        // Set German locale for proper umlaut sorting
        unsafe {
            env::set_var("LC_ALL", "de_DE.UTF-8");
        }
        black_box(run_util_function(
            uumain,
            &["-1", "--color=never", temp_path_str],
        ));
        unsafe {
            env::remove_var("LC_ALL");
        }
    });
}

/// Benchmark impact of locale on ls -l (long listing)
#[divan::bench(args = [100, 500])]
fn ls_long_locale_comparison(bencher: Bencher, file_count: usize) {
    let temp_dir = TempDir::new().unwrap();

    // Mix of ASCII and accented characters
    let names: Vec<String> = (0..file_count)
        .map(|i| match i % 5 {
            0 => format!("normal_{i:03}"),
            1 => format!("café_{i:03}"),
            2 => format!("über_{i:03}"),
            3 => format!("piñata_{i:03}"),
            _ => format!("résumé_{i:03}"),
        })
        .collect();

    for name in &names {
        fs::File::create(temp_dir.path().join(name)).unwrap();
    }

    let temp_path_str = temp_dir.path().to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(
            uumain,
            &["-l", "--color=never", temp_path_str],
        ));
    });
}

fn main() {
    divan::main();
}
