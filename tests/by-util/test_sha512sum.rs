// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::common_checksum_tests::test_digest;
use uutests::new_ucmd;
// spell-checker:ignore testf, ntestf

test_digest! {sha512}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_conflicting_arg() {
    new_ucmd!().arg("--tag").arg("--check").fails_with_code(1);
    new_ucmd!().arg("--tag").arg("--text").fails_with_code(1);
}
