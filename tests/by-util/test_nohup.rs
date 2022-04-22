use crate::common::util::*;
use std::thread::sleep;

// General observation: nohup.out will not be created in tests run by cargo test
// because stdin/stdout is not attached to a TTY.
// All that can be tested is the side-effects.

#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_vendor = "apple"
))]
fn test_nohup_multiple_args_and_flags() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.args(&["touch", "-t", "1006161200", "file1", "file2"])
        .succeeds();
    sleep(std::time::Duration::from_millis(10));

    assert!(at.file_exists("file1"));
    assert!(at.file_exists("file2"));
}
