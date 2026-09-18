// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// Allocation-counting benchmark for `rm -r`.
//
// Run with:
//   cargo bench -p uu_rm --bench rm_alloc_count
//
// To compare against the pre-iterative baseline, check out the commit that
// predates "rm: replace recursive traversal with iterative stack + shared
// PathBuf", then run this bench on both sides:
//
//   BASE=$(git log --format="%H %s" | \
//          grep -m1 "replace recursive traversal" | awk '{print $1}')
//   git stash
//   git checkout "$BASE^" -- src/uu/rm/src/platform/unix.rs \
//                             src/uucore/src/lib/features/safe_traversal.rs
//   cargo bench -p uu_rm --bench rm_alloc_count
//   git checkout HEAD -- src/uu/rm/src/platform/unix.rs \
//                        src/uucore/src/lib/features/safe_traversal.rs
//   git stash pop
//   cargo bench -p uu_rm --bench rm_alloc_count

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::TempDir;
use uu_rm::uumain;
use uucore::benchmark::{fs_tree, run_util_function};

// ── Counting allocator ────────────────────────────────────────────────────────

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn measure(label: &str, f: impl FnOnce()) {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    f();
    let count = ALLOC_COUNT.load(Ordering::Relaxed);
    println!("{label:<50} {count:>8} heap allocations");
}

// ── Scenarios ────────────────────────────────────────────────────────────────

/// Balanced tree: depth=5, branches=5, 10 files/dir → 3 906 dirs, 39 060 files.
/// This is the same structure used by `rm_recursive_tree` in rm_bench.rs.
fn bench_balanced_tree() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().join("tree");
    std::fs::create_dir(&root).unwrap();
    fs_tree::create_balanced_tree(&root, 5, 5, 10);
    let path = root.to_str().unwrap().to_string();

    measure("rm -r  balanced(depth=5, branches=5, files=10)", || {
        run_util_function(uumain, &["-r", &path]);
    });
}

/// Deep linear chain: 800 levels, 1 file per level.
/// Depth is now capped at ~800 — each StackFrame holds exactly one fd
/// (DirFd::into_iter_dir transfers ownership without dup), so the effective
/// limit is RLIMIT_NOFILE (≈ 1 024) rather than RLIMIT_NOFILE / 2.
fn bench_deep_chain() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().join("deep");
    std::fs::create_dir(&root).unwrap();
    fs_tree::create_deep_tree(&root, 800, 1);
    let path = root.to_str().unwrap().to_string();

    measure("rm -r  deep chain (depth=800, 1 file/level)   ", || {
        run_util_function(uumain, &["-r", &path]);
    });
}

fn main() {
    println!();
    println!("=== rm heap-allocation counts ===");
    println!();
    bench_balanced_tree();
    bench_deep_chain();
    println!();
}
