// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

use uucore::translate;

use windows_sys::Win32::Foundation::{ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED};
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_SPARSE_FILE, FILE_SHARE_READ};

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
};

/// Fallback cluster size used when the destination volume's cluster size cannot
/// be queried. 4 KiB is the default NTFS cluster size for volumes up to 16 TB.
const DEFAULT_CLUSTER_SIZE: usize = 4096;

/// Cluster (allocation unit) size of the volume that holds `dest`. The size is
/// configurable at format time (512 B up to 2 MiB), so it is read at runtime.
fn cluster_size(dest: &Path) -> std::io::Result<usize> {
    // Resolve the mount-point root of the volume holding `dest`; this handles
    // both plain drive letters and volumes mounted on a directory.
    let root = uucore::fs::volume_path_name(dest)?;
    let info = uucore::fs::disk_free_space(&root)?;
    Ok(info.sectors_per_cluster as usize * info.bytes_per_sector as usize)
}

/// Buffer (and hole-detection) size for the sparse copy of `dest`.
///
/// NTFS deallocates sparse space in "compression units" of 16 clusters when the
/// cluster size is 4 KiB or smaller — e.g. 64 KiB on the 4 KiB default, which is
/// why a sparse copy's allocated ranges land on 64 KiB boundaries. For larger
/// clusters NTFS has no compression unit and the granularity is the cluster
/// itself. Matching that granularity makes each read as large as possible while
/// never writing — and so never forcing allocation of — a region that could have
/// stayed a hole. The result is capped so a volume with very large clusters does
/// not allocate an oversized buffer; a smaller buffer still yields exact holes,
/// just with more reads. The block size is a granularity knob, not a correctness
/// input, so a failed cluster-size query falls back to [`DEFAULT_CLUSTER_SIZE`].
fn sparse_block_size(dest: &Path) -> usize {
    /// Clusters per NTFS compression unit (only for cluster sizes <= 4 KiB).
    const COMPRESSION_UNIT_CLUSTERS: usize = 16;
    /// Largest cluster size that uses compression units; above this the sparse
    /// granularity is the cluster size itself.
    const MAX_COMPRESSED_CLUSTER: usize = 4096;
    /// Upper bound on the buffer so large clusters don't allocate excessively.
    const MAX_BLOCK_SIZE: usize = 1024 * 1024;

    let cluster = cluster_size(dest)
        .ok()
        .filter(|&c| c != 0)
        .unwrap_or(DEFAULT_CLUSTER_SIZE);
    let granularity = if cluster <= MAX_COMPRESSED_CLUSTER {
        cluster.saturating_mul(COMPRESSION_UNIT_CLUSTERS)
    } else {
        cluster
    };
    granularity.min(MAX_BLOCK_SIZE)
}

/// Read from `reader` until `buf` is full or EOF is reached, returning the
/// number of bytes read.
///
/// Like `read_exact`, but EOF is not an error. Filling the whole buffer keeps
/// the hole-detection blocks of [`sparse_copy`] aligned to the sparse
/// allocation granularity; a short read would misalign every subsequent block.
/// The `Interrupted` arm mirrors std's own read loops, although Windows file
/// I/O does not produce it.
fn read_full(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match reader.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(total)
}

/// Write the whole `buf` to `file` at `offset`, looping over partial writes.
///
/// The Windows-only `seek_write` has no `write_all`-style counterpart (unlike
/// the Unix `write_all_at`), so dropped tails on partial writes are handled
/// here. The `Interrupted` arm mirrors std's own write loops, although Windows
/// file I/O does not produce it.
fn write_all_at(file: &File, mut buf: &[u8], mut offset: u64) -> std::io::Result<()> {
    while !buf.is_empty() {
        match file.seek_write(buf, offset) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            Ok(n) => {
                buf = &buf[n..];
                offset += n as u64;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Perform a sparse copy from the open `src_file` to `dest` for
/// `--sparse=always`.
///
/// The destination is flagged sparse, sized to match the source, and only the
/// blocks of the source that contain at least one non-zero byte are written;
/// runs of zeros are left as holes. This mirrors the Linux `sparse_copy` path.
///
/// If the destination filesystem does not support sparse files (e.g. FAT), the
/// `FSCTL_SET_SPARSE` call fails with `ERROR_INVALID_FUNCTION` or
/// `ERROR_NOT_SUPPORTED` and we fall back to a full byte-for-byte copy so that
/// the zero runs are written out as real zeros. Any other error is propagated.
fn sparse_copy(src_file: &mut File, dest: &Path) -> std::io::Result<()> {
    let dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        // Deny external writers and deleters while the copy is in progress so
        // the destination cannot be corrupted mid-copy; readers are unaffected.
        .share_mode(FILE_SHARE_READ)
        .open(dest)?;

    let size = src_file.metadata()?.len();

    match uucore::fs::set_file_sparse(&dst_file) {
        Ok(()) => {}
        // Sparse files unsupported here: do a plain copy so the result is still
        // a faithful copy of the source.
        Err(e)
            if e.raw_os_error() == Some(ERROR_INVALID_FUNCTION as i32)
                || e.raw_os_error() == Some(ERROR_NOT_SUPPORTED as i32) =>
        {
            std::io::copy(src_file, &mut &dst_file)?;
            return Ok(());
        }
        Err(e) => return Err(e),
    }

    dst_file.set_len(size)?;

    // Detect holes at the destination volume's sparse allocation granularity.
    let mut buf = vec![0u8; sparse_block_size(dest)];
    let mut offset: u64 = 0;
    loop {
        let read = read_full(src_file, &mut buf)?;
        if read == 0 {
            break;
        }
        let chunk = &buf[..read];
        // Only write blocks that contain data; unwritten ranges remain holes.
        if chunk.iter().any(|&b| b != 0) {
            write_all_at(&dst_file, chunk, offset)?;
        }
        offset += read as u64;
    }
    Ok(())
}

/// Whether the open `file` has the Windows sparse attribute set.
///
/// Checked on the handle rather than the path so the decision applies to the
/// same file that is subsequently copied.
fn is_sparse(file: &File) -> std::io::Result<bool> {
    Ok(file.metadata()?.file_attributes() & FILE_ATTRIBUTE_SPARSE_FILE != 0)
}

/// Copies `source` to `dest`, honoring `--sparse` on Windows.
///
/// Windows has no copy-on-write reflink support, so any `--reflink` other than
/// the default `never` is rejected. Sparse copies are implemented via the
/// `FSCTL_SET_SPARSE` device control.
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
) -> CopyResult<CopyDebug> {
    if reflink_mode != ReflinkMode::Never {
        return Err(translate!("cp-error-reflink-not-supported")
            .to_string()
            .into());
    }

    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unsupported,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Unsupported,
    };

    let context_err = |e| CpError::IoErrContext(e, context.to_owned());
    match sparse_mode {
        SparseMode::Always => {
            copy_debug.sparse_detection = SparseDebug::Zeros;
            let mut src_file = File::open(source).map_err(context_err)?;
            sparse_copy(&mut src_file, dest).map_err(context_err)?;
        }
        // `--sparse=auto` (the default) preserves holes only when the source is
        // already sparse, matching GNU. A sparse source is re-copied sparsely;
        // anything else is a plain copy that never introduces new holes.
        //
        // `sparse_copy` re-derives holes by scanning for zero runs rather than
        // mirroring the source's exact allocated-range layout. The content is
        // identical and the result is sparse; a byte-exact hole layout would
        // instead query `FSCTL_QUERY_ALLOCATED_RANGES`, which is out of scope.
        SparseMode::Auto => {
            let mut src_file = File::open(source).map_err(context_err)?;
            if is_sparse(&src_file).map_err(context_err)? {
                copy_debug.sparse_detection = SparseDebug::Zeros;
                sparse_copy(&mut src_file, dest).map_err(context_err)?;
            } else {
                std::fs::copy(source, dest).map_err(context_err)?;
            }
        }
        // `--sparse=never` performs a plain copy that never introduces holes.
        SparseMode::Never => {
            std::fs::copy(source, dest).map_err(context_err)?;
        }
    }

    Ok(copy_debug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// A reader that returns at most 3 bytes per `read` call, exercising the
    /// partial-read handling of `read_full`.
    struct ShortReader<'a>(&'a [u8]);

    impl Read for ShortReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.0.len().min(buf.len()).min(3);
            buf[..n].copy_from_slice(&self.0[..n]);
            self.0 = &self.0[n..];
            Ok(n)
        }
    }

    #[test]
    fn read_full_fills_buffer_across_short_reads() {
        let data: Vec<u8> = (0..=255).collect();
        let mut reader = ShortReader(&data);
        let mut buf = [0u8; 256];
        assert_eq!(read_full(&mut reader, &mut buf).unwrap(), 256);
        assert_eq!(&buf[..], &data[..]);
    }

    #[test]
    fn read_full_returns_partial_count_at_eof() {
        let data = [7u8; 10];
        let mut reader = ShortReader(&data);
        let mut buf = [0u8; 256];
        assert_eq!(read_full(&mut reader, &mut buf).unwrap(), 10);
        assert_eq!(&buf[..10], &data[..]);
        assert_eq!(read_full(&mut reader, &mut buf).unwrap(), 0);
    }

    #[test]
    fn write_all_at_writes_at_offset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("write_all_at");
        let file = File::create(&path).unwrap();
        write_all_at(&file, b"data", 8).unwrap();
        drop(file);
        let contents = std::fs::read(&path).unwrap();
        assert_eq!(contents, b"\0\0\0\0\0\0\0\0data");
    }
}
