// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;
use uu_ls::uumain;
use uucore::benchmark::run_util_function;

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

/// Create a deterministic directory tree for benchmarking ls -R performance
fn create_directory_tree(
    base_dir: &Path,
    depth: usize,
    dirs_per_level: usize,
    files_per_dir: usize,
) -> std::io::Result<()> {
    if depth == 0 {
        return Ok(());
    }

    // Create files in current directory
    for file_idx in 0..files_per_dir {
        let file_path = base_dir.join(format!("file_{file_idx:04}.txt"));
        let mut file = File::create(&file_path)?;
        writeln!(file, "This is file {file_idx} at depth {depth}")?;
    }

    // Create subdirectories and recurse
    for dir_idx in 0..dirs_per_level {
        let dir_path = base_dir.join(format!("subdir_{dir_idx:04}"));
        fs::create_dir(&dir_path)?;
        create_directory_tree(&dir_path, depth - 1, dirs_per_level, files_per_dir)?;
    }

    Ok(())
}

/// Create a wide directory tree (many files/dirs at shallow depth)
fn create_wide_tree(base_dir: &Path, total_files: usize, total_dirs: usize) -> std::io::Result<()> {
    // Create many files in root
    for file_idx in 0..total_files {
        let file_path = base_dir.join(format!("wide_file_{file_idx:06}.txt"));
        let mut file = File::create(&file_path)?;
        writeln!(file, "Wide tree file {file_idx}")?;
    }

    // Create many directories with few files each
    let files_per_subdir = 5;
    for dir_idx in 0..total_dirs {
        let dir_path = base_dir.join(format!("wide_dir_{dir_idx:06}"));
        fs::create_dir(&dir_path)?;

        for file_idx in 0..files_per_subdir {
            let file_path = dir_path.join(format!("file_{file_idx}.txt"));
            let mut file = File::create(&file_path)?;
            writeln!(file, "File {file_idx} in wide dir {dir_idx}")?;
        }
    }

    Ok(())
}

/// Create a deep directory tree (few files/dirs but deep nesting)
fn create_deep_tree(base_dir: &Path, depth: usize, files_per_level: usize) -> std::io::Result<()> {
    let mut current_dir = base_dir.to_path_buf();

    for level in 0..depth {
        // Create files at this level
        for file_idx in 0..files_per_level {
            let file_path = current_dir.join(format!("deep_file_{level}_{file_idx}.txt"));
            let mut file = File::create(&file_path)?;
            writeln!(file, "File {file_idx} at depth level {level}")?;
        }

        // Create next level directory
        if level < depth - 1 {
            let next_dir = current_dir.join(format!("level_{:04}", level + 1));
            fs::create_dir(&next_dir)?;
            current_dir = next_dir;
        }
    }

    Ok(())
}

/// Create a tree with mixed file types and permissions for comprehensive testing
fn create_mixed_tree(base_dir: &Path) -> std::io::Result<()> {
    let extensions = ["txt", "log", "dat", "tmp", "bak", "cfg"];
    let sizes = [0, 100, 1024, 10240];

    for (i, ext) in extensions.iter().enumerate() {
        for (j, &size) in sizes.iter().enumerate() {
            let file_path = base_dir.join(format!("mixed_file_{i}_{j}.{ext}"));
            let mut file = File::create(&file_path)?;

            if size > 0 {
                let content = "x".repeat(size);
                file.write_all(content.as_bytes())?;
            }

            // Set permissions only on Unix platforms
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(match (i + j) % 4 {
                    0 => 0o644,
                    1 => 0o755,
                    2 => 0o600,
                    _ => 0o444,
                });
                fs::set_permissions(&file_path, perms)?;
            }
        }
    }

    // Create some subdirectories
    for i in 0..5 {
        let dir_path = base_dir.join(format!("mixed_subdir_{i}"));
        fs::create_dir(&dir_path)?;

        for j in 0..3 {
            let file_path = dir_path.join(format!("sub_file_{j}.txt"));
            let mut file = File::create(&file_path)?;
            writeln!(file, "File {j} in subdir {i}")?;
        }
    }

    Ok(())
}

/// Benchmark ls -R on balanced directory tree
#[divan::bench(args = [(3, 4, 8), (4, 3, 6), (5, 2, 10)])]
fn ls_recursive_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir).unwrap();
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on balanced directory tree (tests PR #8728 optimization)
#[divan::bench(args = [(3, 4, 8), (4, 3, 6), (5, 2, 10)])]
fn ls_recursive_long_all_balanced_tree(
    bencher: Bencher,
    (depth, dirs_per_level, files_per_dir): (usize, usize, usize),
) {
    let temp_dir = TempDir::new().unwrap();
    create_directory_tree(temp_dir.path(), depth, dirs_per_level, files_per_dir).unwrap();
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on wide directory structures
#[divan::bench(args = [(1000, 200), (5000, 500), (10000, 1000)])]
fn ls_recursive_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_wide_tree(temp_dir.path(), total_files, total_dirs).unwrap();
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on wide directory structures
#[divan::bench(args = [(1000, 200), (5000, 500)])]
fn ls_recursive_long_all_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_wide_tree(temp_dir.path(), total_files, total_dirs).unwrap();
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on deep directory structures
#[divan::bench(args = [(20, 3), (50, 2), (100, 1)])]
fn ls_recursive_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_deep_tree(temp_dir.path(), depth, files_per_level).unwrap();
    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on deep directory structures
#[divan::bench(args = [(20, 3), (50, 2)])]
fn ls_recursive_long_all_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    create_deep_tree(temp_dir.path(), depth, files_per_level).unwrap();
    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

/// Benchmark ls -R on mixed file types (comprehensive real-world test)
#[divan::bench]
fn ls_recursive_mixed_tree(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    create_mixed_tree(temp_dir.path()).unwrap();

    for i in 0..10 {
        let subdir = temp_dir.path().join(format!("mixed_branch_{i}"));
        fs::create_dir(&subdir).unwrap();
        create_mixed_tree(&subdir).unwrap();
    }

    bench_ls_with_args(bencher, &temp_dir, &[]);
}

/// Benchmark ls -R -a -l on mixed file types (most comprehensive test)
#[divan::bench]
fn ls_recursive_long_all_mixed_tree(bencher: Bencher) {
    let temp_dir = TempDir::new().unwrap();
    create_mixed_tree(temp_dir.path()).unwrap();

    for i in 0..10 {
        let subdir = temp_dir.path().join(format!("mixed_branch_{i}"));
        fs::create_dir(&subdir).unwrap();
        create_mixed_tree(&subdir).unwrap();
    }

    bench_ls_with_args(bencher, &temp_dir, &["-a", "-l"]);
}

fn main() {
    divan::main();
}
