// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uu_checksum_common::declare_standalone;
use uucore::checksum::AlgoKind;

declare_standalone!("sha512sum", AlgoKind::Sha512);
