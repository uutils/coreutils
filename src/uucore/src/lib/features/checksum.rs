// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::show_warning_caps;

#[allow(clippy::comparison_chain)]
pub fn cksum_output(bad_format: i32, failed_cksum: i32, failed_open_file: i32) {
    if bad_format == 1 {
        show_warning_caps!("{} line is improperly formatted", bad_format);
    } else if bad_format > 1 {
        show_warning_caps!("{} lines are improperly formatted", bad_format);
    }

    if failed_cksum == 1 {
        show_warning_caps!("{} computed checksum did NOT match", failed_cksum);
    } else if failed_cksum > 1 {
        show_warning_caps!("{} computed checksums did NOT match", failed_cksum);
    }

    if failed_open_file == 1 {
        show_warning_caps!("{} listed file could not be read", failed_open_file);
    } else if failed_open_file > 1 {
        show_warning_caps!("{} listed files could not be read", failed_open_file);
    }
}
