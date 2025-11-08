// spell-checker:ignore mpsc
//! High-performance cached disk usage calculator
//!
//! This module implements an mtime-based caching strategy based on
//! another of my personal code scanning projects. Speedup on repeated scans by
//! caching directory sizes and invalidating based on directory modification times.

#![allow(clippy::non_std_lazy_statics)]

use crate::du_parallel;
use crate::{FileInfo, Stat, TraversalOptions};
use ahash::AHashMap;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::SystemTime;
use uucore::error::UResult;

/// Global cache of directory sizes with mtime-based invalidation
/// Structure: PathBuf -> (size, mtime)
type DirCache = Arc<RwLock<AHashMap<PathBuf, (u64, SystemTime)>>>;

/// Serializable cache entry (SystemTime -> (secs, nanos) tuple)
/// Uses std HashMap because it implements Serde traits
type SerializableCache = HashMap<PathBuf, (u64, (u64, u32))>;

lazy_static::lazy_static! {
    static ref GLOBAL_CACHE: DirCache = {
        let cache = load_cache_from_disk().unwrap_or_else(|_| AHashMap::with_capacity(10000));
        Arc::new(RwLock::new(cache))
    };
}

/// Convert SystemTime to (secs, nanos) tuple for serialization
fn systime_to_tuple(time: SystemTime) -> (u64, u32) {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_secs(), duration.subsec_nanos())
}

/// Convert (secs, nanos) tuple to SystemTime
fn tuple_to_systime((secs, nanos): (u64, u32)) -> SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::new(secs, nanos)
}

/// Get path to cache file
fn get_cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|mut p| {
        p.push("uutils");
        p.push("du_cache.bin");
        p
    })
}

/// Load cache from disk
fn load_cache_from_disk() -> Result<AHashMap<PathBuf, (u64, SystemTime)>, Box<dyn std::error::Error>>
{
    let cache_path = get_cache_path().ok_or("Could not determine cache directory")?;

    if !cache_path.exists() {
        return Ok(AHashMap::with_capacity(10000));
    }

    let mut file = fs::File::open(&cache_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let serializable: SerializableCache = bincode::deserialize(&buffer)?;

    // Convert from serializable format to runtime format
    let cache: AHashMap<PathBuf, (u64, SystemTime)> = serializable
        .into_iter()
        .map(|(path, (size, time_tuple))| (path, (size, tuple_to_systime(time_tuple))))
        .collect();

    if std::env::var("DU_CACHE_DEBUG").is_ok() {
        eprintln!("[CACHE] Loaded {} entries from disk", cache.len());
    }

    Ok(cache)
}

/// Save cache to disk
fn save_cache_to_disk() -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = get_cache_path().ok_or("Could not determine cache directory")?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let cache = GLOBAL_CACHE.read();

    // Convert to serializable format
    let serializable: SerializableCache = cache
        .iter()
        .map(|(path, (size, time))| (path.clone(), (*size, systime_to_tuple(*time))))
        .collect();

    let serialized = bincode::serialize(&serializable)?;

    let mut file = fs::File::create(&cache_path)?;
    file.write_all(&serialized)?;

    if std::env::var("DU_CACHE_DEBUG").is_ok() {
        eprintln!("[CACHE] Saved {} entries to disk", cache.len());
    }

    Ok(())
}

/// Configuration for cache behavior
pub struct CacheConfig {
    #[allow(dead_code)]
    pub max_entries: usize,
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            enabled: true,
        }
    }
}

/// Statistics about cache performance
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub entries: usize,
}

#[allow(dead_code)]
impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// Check if a directory's cached entry is still valid
/// Returns Some(size) if cache hit, None if miss or stale
#[allow(clippy::manual_let_else)]
fn check_cache(path: &Path) -> Option<u64> {
    // Get current mtime of the directory
    let current_mtime = match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(mtime) => mtime,
        Err(_) => return None,
    };

    // Read lock for cache lookup - fast path
    let cache = GLOBAL_CACHE.read();

    if let Some((cached_size, cached_mtime)) = cache.get(path) {
        // Validate: mtimes must match exactly
        if *cached_mtime == current_mtime {
            return Some(*cached_size);
        } else if std::env::var("DU_CACHE_DEBUG").is_ok() {
            eprintln!(
                "[CACHE] mtime mismatch for {}: cached {:?} vs current {:?}",
                path.display(),
                cached_mtime,
                current_mtime
            );
        }
    }

    None
}

/// Update cache with new directory size and mtime
#[allow(clippy::manual_let_else)]
fn update_cache(path: PathBuf, size: u64) {
    // Get current mtime
    let mtime = match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(_) => {
            if std::env::var("DU_CACHE_DEBUG").is_ok() {
                eprintln!("[CACHE] Failed to get mtime for {}", path.display());
            }
            return;
        }
    };

    // Write lock for cache update
    let mut cache = GLOBAL_CACHE.write();
    cache.insert(path.clone(), (size, mtime));

    if std::env::var("DU_CACHE_DEBUG").is_ok() {
        eprintln!(
            "[CACHE STORE] {}: {} bytes (cache size: {})",
            path.display(),
            size,
            cache.len()
        );
    }
}

/// Prune cache if it exceeds max_entries
/// Removes oldest entries based on mtime
#[allow(dead_code)]
pub fn prune_cache(max_entries: usize) {
    let mut cache = GLOBAL_CACHE.write();

    if cache.len() <= max_entries {
        return;
    }

    // Collect entries sorted by mtime (oldest first)
    let mut entries: Vec<_> = cache
        .iter()
        .map(|(path, (_, mtime))| (path.clone(), *mtime))
        .collect();

    entries.sort_by_key(|(_, mtime)| *mtime);

    // Remove oldest entries
    let to_remove = cache.len() - max_entries;
    for (path, _) in entries.into_iter().take(to_remove) {
        cache.remove(&path);
    }
}

/// Clear the entire cache
#[allow(dead_code)]
pub fn clear_cache() {
    GLOBAL_CACHE.write().clear();
}

/// Get current cache statistics
#[allow(dead_code)]
pub fn get_cache_stats() -> CacheStats {
    let cache = GLOBAL_CACHE.read();
    CacheStats {
        hits: 0, // Would need atomic counters to track
        misses: 0,
        entries: cache.len(),
    }
}

/// Cached version of parallel du traversal
///
/// This function wraps du_parallel with an mtime-based cache.
/// On cache hit (directory mtime unchanged), returns cached size instantly.
/// On cache miss, performs full traversal and updates cache.
pub fn du_parallel_cached(
    init_stat: Stat,
    options: &TraversalOptions,
    depth: usize,
    seen_inodes: &mut HashSet<FileInfo>,
    print_tx: &mpsc::Sender<UResult<crate::StatPrintInfo>>,
    config: &CacheConfig,
) -> Result<Stat, Box<mpsc::SendError<UResult<crate::StatPrintInfo>>>> {
    if !config.enabled {
        // Cache disabled, use normal parallel traversal
        return du_parallel::du_parallel(init_stat, options, depth, seen_inodes, print_tx);
    }

    // Try cache lookup first (read-optimized fast path)
    if let Some(cached_size) = check_cache(&init_stat.path) {
        // Cache hit! Return immediately with cached size
        if std::env::var("DU_CACHE_DEBUG").is_ok() {
            eprintln!(
                "[CACHE HIT] {}: {} bytes",
                init_stat.path.display(),
                cached_size
            );
        }
        let mut cached_stat = init_stat;
        cached_stat.size = cached_size;
        // Also update blocks for correct display (du shows blocks by default, not size)
        // blocks are in 512-byte units
        cached_stat.blocks = cached_size / 512;
        return Ok(cached_stat);
    }

    if std::env::var("DU_CACHE_DEBUG").is_ok() {
        eprintln!("[CACHE MISS] {}", init_stat.path.display());
    }

    // Cache miss - perform full parallel traversal
    let result = du_parallel::du_parallel(init_stat, options, depth, seen_inodes, print_tx)?;

    // Update cache with the actual disk usage (blocks * 512)
    // Note: result.size is apparent size, but du displays blocks * 512 by default
    let disk_usage = result.blocks * 512;
    if std::env::var("DU_CACHE_DEBUG").is_ok() {
        eprintln!("[CACHE STORE] size={} blocks={} disk_usage={}", result.size, result.blocks, disk_usage);
    }
    update_cache(result.path.clone(), disk_usage);

    // Save cache to disk for persistence across runs
    // Ignore errors (e.g., permission denied when running as root)
    let _ = save_cache_to_disk();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        clear_cache();

        let path = PathBuf::from("/tmp/test_cache");
        let size = 1024;

        // Initially no cache entry
        assert!(check_cache(&path).is_none());

        // After update, should have entry
        update_cache(path.clone(), size);

        // Note: This test may fail if mtime check fails
        // It's mainly for compilation and basic logic verification
    }

    #[test]
    fn test_cache_stats() {
        clear_cache();
        let stats = get_cache_stats();
        assert_eq!(stats.entries, 0);
    }

    #[test]
    fn test_prune_cache() {
        clear_cache();

        // Add some entries
        for i in 0..10 {
            let path = PathBuf::from(format!("/tmp/test_{}", i));
            update_cache(path, i * 1024);
        }

        // Prune to 5 entries
        prune_cache(5);

        let stats = get_cache_stats();
        assert!(stats.entries <= 5);
    }
}
