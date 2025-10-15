<!-- spell-checker:ignore hyperfine tmpfs reflink fsxattr xattrs clonefile vmtouch APFS pathlib Btrfs fallocate journaling -->

# Benchmarking cp

`cp` copies file contents together with metadata such as permissions, ownership,
timestamps, extended attributes, and directory structures. Although copying
looks simple, `cp` exercises many filesystem features. Its performance depends
heavily on the workload shape (large sequential files, many tiny files, special
files, sparse images) and the storage stack underneath.

## Understanding cp

Most of the time spent inside `cp` falls into two broad categories:

- **Data transfer path**: When copying large contiguous files, throughput is
dominated by read/write bandwidth. The overhead from `cp` itself comes from
performing buffered reads and writes, copying memory between buffers, and the
number of system calls issued per block.
- **Metadata handling**: When recursively copying trees with thousands of small
files, performance is limited by metadata work such as `open`, `stat`,
`lstat`, attribute preservation, directory creation, and link handling.

`cp` supports many switches that alter these paths, including attribute
preservation, hard-link and reflink creation, sparse detection, and
`--remove-destination` semantics. Benchmarks should call out which pathways are
being exercised so results can be interpreted correctly.

## Benchmarking guidelines

- Build a release binary first: `cargo build --release -p uu_cp`.
- Use `hyperfine` for timing and rely on the `--prepare` hook to reset state
between runs.
- Prefer running on a fast device (RAM disk, tmpfs, NVMe) to minimize raw
storage latency when isolating the cost of the tool.
- On Linux, control the page cache where appropriate using tools like
`vmtouch` or `echo 3 > /proc/sys/vm/drop_caches` (root required). Prioritize
repeatability and stay within the policies of the host system.
- Keep the workload definition explicit. When comparing against GNU `cp` or
other implementations, ensure identical datasets and mount options.

## Large-file throughput

1. Create a clean working directory and reduce cache interference.
2. Generate an input file of known size, for example with `truncate` or `dd`.
3. Run repeated copies with `hyperfine`, deleting the destination beforehand.

```shell
mkdir -p benchmark/cp && cd benchmark/cp
truncate -s 2G input.bin
hyperfine \
  --warmup 2 \
  --prepare 'rm -f output.bin' \
  '../target/release/cp input.bin output.bin'
```

What to record:

- Achieved throughput (MB/s) for large sequential copies.
- Behavior with `--reflink=auto` or `--sparse=auto` on filesystems that
support copy-on-write or sparse regions.
- CPU overhead when enabling attribute preservation such as
`--preserve=mode,timestamps,xattr`.

If the underlying filesystem performs transparent copy-on-write (for example,
APFS via `clonefile`), consider running the same benchmark with `--reflink=never`
or on a filesystem without reflink support to measure raw data transfer.

## Many small files

Large directory trees stress metadata throughput. Pre-create a synthetic tree
and copy it recursively.

```shell
mkdir -p dataset/src
python3 - <<'PY'
from pathlib import Path
root = Path('dataset/src')
for i in range(2000):
    sub = root / f'dir_{i//200}'
    sub.mkdir(parents=True, exist_ok=True)
    for j in range(5):
        path = sub / f'file_{i}_{j}.txt'
        path.write_text('payload' * 16)
PY
hyperfine \
  --warmup 1 \
  --prepare 'rm -rf dataset/dst && mkdir -p dataset/dst' \
  '../target/release/cp -r dataset/src dataset/dst'
```

What to record:

- Time spent in directory traversal and metadata replication.
- Impact of toggling options such as `--preserve`, `--no-preserve`, `--link`,
`--hard-link`, and `--archive`.
- Behavior when symbolic links or hard links are present, especially with
`--dereference` versus `--no-dereference`.

## Copy-on-write and sparse files

`--reflink=always` can dramatically reduce work on Btrfs, XFS, APFS, and other
reflink-aware filesystems. Compare results with `--reflink=never` to understand
how much time is spent in copy-on-write system calls versus fallback copying.
Sparse workloads benefit from dedicated benchmarks as well.

```shell
truncate -s 4G sparse.img
fallocate -d sparse.img  # On filesystems that support punching holes
hyperfine \
  --prepare 'rm -f sparse-copy.img' \
  '../target/release/cp --sparse=always sparse.img sparse-copy.img'
```

Check both the elapsed time and the on-disk size of the destination (for
example using `du -h sparse-copy.img`) to confirm sparse regions are preserved.

## Evaluating attribute preservation and extras

Measure the incremental cost of individual options by enabling them one at a
time:

- Test `--preserve=context` or `--preserve=xattr` on files that actually carry
extended attributes.
- Evaluate ACL and SELinux handling with `--archive` on systems where those
features are active.
- Compare modes that remove or back up the destination (`--remove-destination`,
`--backup=numbered`) to see the impact of extra file operations.

Supplementary analysis with `strace -c` or `perf record` can show which system
calls dominate and guide optimization work.

## Interpreting results

- If a benchmark completes in well under a second, increase the dataset size to
reduce process start-up noise.
- Document filesystem features such as journaling, compression, or encryption
that may skew results.
- When changes are made to `cp`, track how system call counts, I/O patterns,
and CPU time shift between runs to catch regressions early.

Use these guidelines to isolate the workloads you care about (large sequential
transfers, directory-heavy copies, attribute preservation, reflink paths) and
collect reproducible measurements.
