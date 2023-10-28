// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

#[test]
fn test_z85_not_padded() {
    // The z85 crate deviates from the standard in some cases; we have to catch those
    new_ucmd!()
        .args(&["--z85", "-d"])
        .pipe_in("##########")
        .fails()
        .stderr_only("basenc: error: invalid input\n");
    new_ucmd!()
        .args(&["--z85"])
        .pipe_in("123")
        .fails()
        .stderr_only("basenc: error: invalid input (length must be multiple of 4 characters)\n");
}
