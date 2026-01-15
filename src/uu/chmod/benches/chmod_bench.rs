// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{black_box, Bencher};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;
use uu_chmod::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

struct CountingAlloc;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            let size = layout.size();
            let current = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
            update_peak(current);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        if !ptr.is_null() {
            DEALLOCS.fetch_add(1, Ordering::Relaxed);
            CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            let old_size = layout.size();
            if new_size > old_size {
                let delta = new_size - old_size;
                let current = CURRENT.fetch_add(delta, Ordering::Relaxed) + delta;
                update_peak(current);
            } else {
                CURRENT.fetch_sub(old_size - new_size, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

#[inline]
fn update_peak(current: usize) {
    let mut peak = PEAK.load(Ordering::Relaxed);
    while current > peak {
        match PEAK.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

#[derive(Clone, Copy)]
struct AllocStats {
    current: usize,
    peak: usize,
    allocs: usize,
    deallocs: usize,
}

fn reset_stats_for_interval() -> usize {
    let baseline = CURRENT.load(Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    ALLOCS.store(0, Ordering::Relaxed);
    DEALLOCS.store(0, Ordering::Relaxed);
    baseline
}

fn alloc_stats() -> AllocStats {
    AllocStats {
        current: CURRENT.load(Ordering::Relaxed),
        peak: PEAK.load(Ordering::Relaxed),
        allocs: ALLOCS.load(Ordering::Relaxed),
        deallocs: DEALLOCS.load(Ordering::Relaxed),
    }
}

fn mem_enabled() -> bool {
    std::env::var_os("UU_CHMOD_MEM").is_some()
}

fn run_chmod(args: &[&str]) {
    black_box(run_util_function(uumain, args));
}

fn maybe_report_allocs(label: &str, args: &[&str]) {
    if !mem_enabled() {
        return;
    }

    let baseline = reset_stats_for_interval();
    run_chmod(args);
    let stats = alloc_stats();
    let peak_delta = stats.peak.saturating_sub(baseline);
    let current_delta = stats.current.saturating_sub(baseline);

    eprintln!(
        "chmod mem {label}: peak={}B current={}B allocs={} deallocs={}",
        peak_delta, current_delta, stats.allocs, stats.deallocs
    );
}

#[cfg(unix)]
fn cap_dirs_by_rlimit(total_dirs: usize) -> usize {
    use uucore::libc::{getrlimit, rlimit, RLIMIT_NOFILE, RLIM_INFINITY};

    let mut lim = rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    let rc = unsafe { getrlimit(RLIMIT_NOFILE, &mut lim) };
    if rc != 0 || lim.rlim_cur == RLIM_INFINITY {
        return total_dirs;
    }

    let headroom = 32;
    let cap = lim.rlim_cur.saturating_sub(headroom).max(1) as usize;
    total_dirs.min(cap)
}

#[cfg(not(unix))]
fn cap_dirs_by_rlimit(total_dirs: usize) -> usize {
    total_dirs
}

fn bench_chmod_recursive(bencher: Bencher, temp_dir: &TempDir, label: &str) {
    let temp_path = temp_dir.path().to_str().unwrap();
    let args = ["-R", "755", temp_path];

    maybe_report_allocs(label, &args);

    bencher.bench(|| {
        run_chmod(&args);
    });
}

#[divan::bench(args = [(2000, 200)])]
fn chmod_recursive_wide_tree(bencher: Bencher, (total_files, total_dirs): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    let capped_dirs = cap_dirs_by_rlimit(total_dirs);
    fs_tree::create_wide_tree(temp_dir.path(), total_files, capped_dirs);
    let label = format!("wide files={total_files} dirs={capped_dirs}");
    bench_chmod_recursive(bencher, &temp_dir, &label);
}

#[divan::bench(args = [(200, 2)])]
fn chmod_recursive_deep_tree(bencher: Bencher, (depth, files_per_level): (usize, usize)) {
    let temp_dir = TempDir::new().unwrap();
    fs_tree::create_deep_tree(temp_dir.path(), depth, files_per_level);
    let label = format!("deep depth={depth} files_per_level={files_per_level}");
    bench_chmod_recursive(bencher, &temp_dir, &label);
}

fn main() {
    divan::main();
}
