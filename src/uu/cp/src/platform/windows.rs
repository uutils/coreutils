// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink ioctl Ioctl
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{FileExt, MetadataExt};
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use uucore::translate;

use windows_sys::Win32::Foundation::MAX_PATH;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_SPARSE_FILE, GetDiskFreeSpaceW, GetVolumePathNameW,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
};

/// Fallback cluster size used when the destination volume's cluster size cannot
/// be queried. 4 KiB is the default NTFS cluster size for volumes up to 16 TB.
const DEFAULT_CLUSTER_SIZE: usize = 4096;

/// Best-effort lookup of the cluster (allocation unit) size of the volume that
/// holds `dest`. The size is configurable at format time (512 B up to 2 MiB), so
/// it is read at runtime. Falls back to [`DEFAULT_CLUSTER_SIZE`] on failure.
fn cluster_size(dest: &Path) -> usize {
    let path: Vec<u16> = dest
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Resolve the mount-point root of the volume holding `dest`; this handles
    // both plain drive letters and volumes mounted on a directory.
    let mut root = [0u16; MAX_PATH as usize];
    // SAFETY: `path` is a valid null-terminated wide string, and `root` with
    // length `MAX_PATH` is a valid, correctly-sized output buffer.
    let ok = unsafe { GetVolumePathNameW(path.as_ptr(), root.as_mut_ptr(), MAX_PATH) };
    if ok == 0 {
        return DEFAULT_CLUSTER_SIZE;
    }

    let mut sectors_per_cluster: u32 = 0;
    let mut bytes_per_sector: u32 = 0;
    let mut free_clusters: u32 = 0;
    let mut total_clusters: u32 = 0;
    // SAFETY: `root` is a valid null-terminated wide string produced by
    // `GetVolumePathNameW`; the four out-params are valid `u32` pointers.
    let ok = unsafe {
        GetDiskFreeSpaceW(
            root.as_ptr(),
            &raw mut sectors_per_cluster,
            &raw mut bytes_per_sector,
            &raw mut free_clusters,
            &raw mut total_clusters,
        )
    };
    if ok == 0 {
        return DEFAULT_CLUSTER_SIZE;
    }

    match sectors_per_cluster as usize * bytes_per_sector as usize {
        0 => DEFAULT_CLUSTER_SIZE,
        cluster => cluster,
    }
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
/// just with more reads.
fn sparse_block_size(dest: &Path) -> usize {
    /// Clusters per NTFS compression unit (only for cluster sizes <= 4 KiB).
    const COMPRESSION_UNIT_CLUSTERS: usize = 16;
    /// Largest cluster size that uses compression units; above this the sparse
    /// granularity is the cluster size itself.
    const MAX_COMPRESSED_CLUSTER: usize = 4096;
    /// Upper bound on the buffer so large clusters don't allocate excessively.
    const MAX_BLOCK_SIZE: usize = 1024 * 1024;

    let cluster = cluster_size(dest);
    let granularity = if cluster <= MAX_COMPRESSED_CLUSTER {
        cluster.saturating_mul(COMPRESSION_UNIT_CLUSTERS)
    } else {
        cluster
    };
    granularity.min(MAX_BLOCK_SIZE)
}

/// Flag `file` as sparse using the Windows `FSCTL_SET_SPARSE` control code.
///
/// Once a file is marked sparse, regions within its length that are never
/// written are not allocated on disk and read back as zeros.
fn set_sparse(file: &File) -> std::io::Result<()> {
    let mut bytes_returned: u32 = 0;
    // SAFETY: `file.as_raw_handle()` is a valid, open file handle owned by `file`.
    // `FSCTL_SET_SPARSE` takes no input or output buffer, so the buffer pointers
    // are null with zero lengths; `bytes_returned` is a valid out-parameter.
    let ok = unsafe {
        DeviceIoControl(
            // `as_raw_handle()` yields the std `*mut c_void`; `.cast()` reinterprets
            // it as the windows-sys `HANDLE` without an identity `as` pointer cast
            // (which clippy::ptr_as_ptr flags).
            file.as_raw_handle().cast(),
            FSCTL_SET_SPARSE,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &raw mut bytes_returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Perform a sparse copy from `source` to `dest` for `--sparse=always`.
///
/// The destination is flagged sparse, sized to match the source, and only the
/// blocks of `source` that contain at least one non-zero byte are written; runs
/// of zeros are left as holes. This mirrors the Linux `sparse_copy` path.
///
/// If the destination filesystem does not support sparse files (e.g. FAT), the
/// `FSCTL_SET_SPARSE` call fails and we fall back to a full byte-for-byte copy
/// so that the zero runs are written out as real zeros rather than left as
/// undefined extended ranges.
fn sparse_copy(source: &Path, dest: &Path) -> std::io::Result<()> {
    let mut src_file = File::open(source)?;
    let dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest)?;

    let size = src_file.metadata()?.len();

    if set_sparse(&dst_file).is_err() {
        // Sparse files unsupported here: do a plain copy so the result is still
        // a faithful copy of the source.
        std::io::copy(&mut src_file, &mut &dst_file)?;
        return Ok(());
    }

    dst_file.set_len(size)?;

    // Detect holes at the destination volume's sparse allocation granularity.
    let mut buf = vec![0u8; sparse_block_size(dest)];
    let mut offset: u64 = 0;
    loop {
        let read = src_file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        let chunk = &buf[..read];
        // Only write blocks that contain data; unwritten ranges remain holes.
        if chunk.iter().any(|&b| b != 0) {
            dst_file.seek_write(chunk, offset)?;
        }
        offset += read as u64;
    }
    Ok(())
}

/// Whether `path` is already a sparse file (has the sparse file attribute set).
fn is_sparse(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.file_attributes() & FILE_ATTRIBUTE_SPARSE_FILE != 0)
        .unwrap_or(false)
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

    match sparse_mode {
        SparseMode::Always => {
            copy_debug.sparse_detection = SparseDebug::Zeros;
            sparse_copy(source, dest).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
        }
        // `--sparse=auto` (the default) preserves holes only when the source is
        // already sparse, matching GNU. A sparse source is re-copied sparsely;
        // anything else is a plain copy that never introduces new holes.
        //
        // `sparse_copy` re-derives holes by scanning for zero runs rather than
        // mirroring the source's exact allocated-range layout. The content is
        // identical and the result is sparse; a byte-exact hole layout would
        // instead query `FSCTL_QUERY_ALLOCATED_RANGES`, which is out of scope.
        SparseMode::Auto if is_sparse(source) => {
            copy_debug.sparse_detection = SparseDebug::Zeros;
            sparse_copy(source, dest).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
        }
        // `--sparse=auto` on a non-sparse source, and `--sparse=never`, perform a
        // plain copy that never introduces holes.
        SparseMode::Auto | SparseMode::Never => {
            std::fs::copy(source, dest)
                .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
        }
    }

    Ok(copy_debug)
}
