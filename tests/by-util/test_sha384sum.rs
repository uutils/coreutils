// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore testf, ntestf

use uutests::at_and_ucmd;

use crate::common_checksum_tests::test_digest;

test_digest! {sha384}

// A SHA-256 digest has 64 hex digits. They are also valid base64 for 48 bytes, the size of a
// SHA-384 digest.
#[test]
fn test_check_sha256_digest_is_improperly_formatted() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("a", "a\n");

    ucmd.arg("--check")
        .pipe_in("87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7  a\n")
        .fails()
        .no_stdout()
        .stderr_is("sha384sum: 'standard input': no properly formatted checksum lines found\n");
}
