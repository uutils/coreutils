#[macro_use]
mod common;

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


#[test]
fn test_op_prec_and_or_1() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let exit_success = ucmd.arg(" ")
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
    let (_, mut ucmd) = testing(UTIL_NAME);
    let exit_success = ucmd.arg("")
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
    let (_, mut ucmd) = testing(UTIL_NAME);
    let exit_success = ucmd.arg("x")
                           .arg("-a")
                           .arg("-z")
                           .arg("-o")
                           .run()
                           .success;
    assert!(!exit_success);
}
