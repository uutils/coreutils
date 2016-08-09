//
// This file is part of the uutils coreutils package.
//
// (c) mahkoh (ju.orth [at] gmail [dot] com)
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

use common::util::*;

static UTIL_NAME: &'static str = "test";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_op_prec_and_or_1() {
    let exit_success = new_ucmd()
                           .arg(" ")
                           .arg("-o")
                           .arg("")
                           .arg("-a")
                           .arg("")
                           .run()
                           .success;
    assert!(exit_success);
}

#[test]
fn test_op_prec_and_or_2() {
    let exit_success = new_ucmd()
                           .arg("")
                           .arg("-a")
                           .arg("")
                           .arg("-o")
                           .arg(" ")
                           .arg("-a")
                           .arg(" ")
                           .run()
                           .success;
    assert!(exit_success);
}

#[test]
fn test_or_as_filename() {
    let exit_success = new_ucmd()
        .arg("x")
                           .arg("-a")
                           .arg("-z")
                           .arg("-o")
                           .run()
                           .success;
    assert!(!exit_success);
}
