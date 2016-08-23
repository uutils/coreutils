use common::util::*;


#[test]
fn test_current_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(".").run().stdout_is(at.root_dir_resolved());
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    ucmd.arg(dir).run().stdout_is(at.root_dir_resolved());
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    new_ucmd!().arg(dir).run().stdout_is(get_root_path());
}
