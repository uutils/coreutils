use common::util::*;


#[test]
fn test_current_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let actual = ucmd.arg(".").run().stdout;
    let expect = at.root_dir_resolved() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    let actual = ucmd.arg(dir).run().stdout;
    let expect = at.root_dir_resolved() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    let actual = new_ucmd!().arg(dir).run().stdout;
    let expect = get_root_path().to_owned() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}
