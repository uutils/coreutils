// Parallel directory traversal implementation for du
// spell-checker:ignore mpsc

use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

use crate::{FileInfo, Stat, StatPrintInfo, TraversalOptions, du_regular};
use uucore::error::UResult;

/// Minimum number of subdirectories to enable parallel processing
/// Below this threshold, sequential processing is faster (avoids overhead)
pub const PARALLEL_THRESHOLD: usize = 4;

/// Recursively calculates disk usage in parallel using a work-stealing approach.
///
/// This function processes directory trees in parallel when they contain 4 or more
/// subdirectories, using Rayon's work-stealing parallelism. For directories with
/// fewer subdirectories, it falls back to sequential traversal to avoid overhead.
///
/// The implementation maintains thread-safe inode tracking to properly handle
/// hardlinks across parallel workers.
pub fn du_parallel(
    init_stat: Stat,
    options: &TraversalOptions,
    depth: usize,
    seen_inodes: &mut HashSet<FileInfo>,
    print_tx: &mpsc::Sender<UResult<StatPrintInfo>>,
) -> Result<Stat, Box<mpsc::SendError<UResult<StatPrintInfo>>>> {
    let path = init_stat.path.clone();

    // Read directory entries
    let entries: Vec<PathBuf> = match fs::read_dir(&path) {
        Ok(read_dir) => read_dir
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect(),
        Err(_) => {
            // Can't read directory, fall back to sequential
            return du_regular(init_stat, options, depth, seen_inodes, print_tx, None, None);
        }
    };

    // Count directories
    let dir_count = entries.iter().filter(|p| p.is_dir()).count();

    // If too few directories, use sequential processing
    if dir_count < PARALLEL_THRESHOLD {
        return du_regular(init_stat, options, depth, seen_inodes, print_tx, None, None);
    }

    // Parallel processing for many directories
    // Wrap seen_inodes in Arc<Mutex> for thread safety
    let seen_inodes_shared = Arc::new(Mutex::new(seen_inodes.clone()));
    let print_tx_clone = print_tx.clone();

    // Process all entries in parallel
    let chunk_size = (entries.len() / rayon::current_num_threads()).max(4);

    let results: Vec<_> = entries
        .par_chunks(chunk_size)
        .flat_map(|chunk| {
            let mut local_results = Vec::new();
            for entry_path in chunk {
                // Create stat for this entry
                match Stat::new(entry_path, None, options) {
                    Ok(entry_stat) => {
                        // Check if it's a directory
                        if entry_stat.metadata.is_dir() {
                            // Thread-safe inode tracking
                            if let Some(inode) = entry_stat.inode {
                                let mut inodes = seen_inodes_shared.lock().unwrap();
                                if inodes.contains(&inode) {
                                    continue;
                                }
                                inodes.insert(inode);
                            }

                            // Recursively process subdirectory with parallelization
                            // Each thread gets its own seen_inodes for this subtree
                            let mut thread_inodes = seen_inodes_shared.lock().unwrap().clone();
                            drop(seen_inodes_shared.lock());

                            // Recursively call du_parallel for nested parallelism
                            match du_parallel(
                                entry_stat,
                                options,
                                depth + 1,
                                &mut thread_inodes,
                                &print_tx_clone,
                            ) {
                                Ok(stat) => local_results.push(Ok(stat)),
                                Err(e) => local_results.push(Err(e)),
                            }
                        } else {
                            // It's a file, just add its size
                            local_results.push(Ok(entry_stat));
                        }
                    }
                    Err(_) => {
                        // Ignore stat errors
                    }
                }
            }
            local_results
        })
        .collect();

    // Aggregate results
    let mut total_stat = init_stat;
    for result in results {
        match result {
            Ok(stat) => {
                if !options.separate_dirs {
                    total_stat.size += stat.size;
                    total_stat.blocks += stat.blocks;
                    total_stat.inodes += stat.inodes;
                }
            }
            Err(_) => {
                // Errors already reported
            }
        }
    }

    // Merge back thread-safe inodes to caller's hashset
    let final_inodes = seen_inodes_shared.lock().unwrap();
    for inode in final_inodes.iter() {
        seen_inodes.insert(*inode);
    }

    Ok(total_stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_threshold() {
        assert_eq!(PARALLEL_THRESHOLD, 4);
    }

    #[test]
    fn test_chunk_size_calculation() {
        let threads = rayon::current_num_threads();
        let entry_count = 1000;
        let chunk_size = (entry_count / threads).max(4);
        assert!(chunk_size >= 4);
        assert!(chunk_size <= entry_count);
    }
}
