//
// This file is part of the uutils coreutils package.
//
// (c) mahkoh (ju.orth [at] gmail [dot] com)
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

use common::util::*;


#[test]
fn test_op_prec_and_or_1() {
    new_ucmd!()
        .args(&[" ", "-o", "", "-a", ""])
        .succeeds();
}

#[test]
fn test_op_prec_and_or_2() {
    new_ucmd!()
        .args(&["", "-a", "", "-o", " ", "-a", " "])
        .succeeds();
}

#[test]
fn test_or_as_filename() {
    new_ucmd!()
        .args(&["x", "-a", "-z", "-o"])
        .fails();
}
