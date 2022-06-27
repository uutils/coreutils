// spell-checker:ignore parenb parmrk ixany iuclc onlcr ofdel icanon noflsh

use crate::common::util::*;

#[test]
fn runs() {
    new_ucmd!().succeeds();
}

#[test]
fn print_all() {
    let res = new_ucmd!().succeeds();

    // Random selection of flags to check for
    for flag in [
        "parenb", "parmrk", "ixany", "iuclc", "onlcr", "ofdel", "icanon", "noflsh",
    ] {
        res.stdout_contains(flag);
    }
}
