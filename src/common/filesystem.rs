/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joseph Crail <jbcrail@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// Based on the pattern using by Cargo, I created a shim over the
// standard PathExt trait, so that the unstable path methods could
// be backported to stable (<= 1.1). This will likely be dropped
// when the path trait stabilizes.

use std::fs;
use std::io;
use std::path::Path;

pub trait UUPathExt {
    fn uu_exists(&self) -> bool;
    fn uu_is_file(&self) -> bool;
    fn uu_is_dir(&self) -> bool;
    fn uu_metadata(&self) -> io::Result<fs::Metadata>;
}

impl UUPathExt for Path {
    fn uu_exists(&self) -> bool {
        fs::metadata(self).is_ok()
    }

    fn uu_is_file(&self) -> bool {
        fs::metadata(self).map(|m| m.is_file()).unwrap_or(false)
    }

    fn uu_is_dir(&self) -> bool {
        fs::metadata(self).map(|m| m.is_dir()).unwrap_or(false)
    }

    fn uu_metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self)
    }
}
