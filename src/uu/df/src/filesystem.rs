//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
#[cfg(windows)]
use std::path::Path;

#[cfg(unix)]
use uucore::fsext::statfs;
use uucore::fsext::{FsUsage, MountInfo};

#[derive(Debug, Clone)]
pub(crate) struct Filesystem {
    pub mount_info: MountInfo,
    pub usage: FsUsage,
}

impl Filesystem {
    // TODO: resolve uuid in `mount_info.dev_name` if exists
    pub(crate) fn new(mount_info: MountInfo) -> Option<Self> {
        let _stat_path = if !mount_info.mount_dir.is_empty() {
            mount_info.mount_dir.clone()
        } else {
            #[cfg(unix)]
            {
                mount_info.dev_name.clone()
            }
            #[cfg(windows)]
            {
                // On windows, we expect the volume id
                mount_info.dev_id.clone()
            }
        };
        #[cfg(unix)]
        let usage = FsUsage::new(statfs(_stat_path).ok()?);
        #[cfg(windows)]
        let usage = FsUsage::new(Path::new(&_stat_path));
        Some(Self { mount_info, usage })
    }
}
