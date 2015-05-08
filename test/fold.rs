use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

static PROGNAME: &'static str = "./fold";

#[test]
fn test_default_80_column_wrap() {
    let po = Command::new(PROGNAME)
        .arg("lorem_ipsum.txt")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    fold_helper(po.stdout, "lorem_ipsum_80_column.expected");
}

#[test]
fn test_40_column_hard_cutoff() {
    let po = Command::new(PROGNAME)
        .arg("-w")
        .arg("40")
        .arg("lorem_ipsum.txt")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    fold_helper(po.stdout, "lorem_ipsum_40_column_hard.expected");
}

#[test]
fn test_40_column_word_boundary() {
    let po = Command::new(PROGNAME)
        .arg("-s")
        .arg("-w")
        .arg("40")
        .arg("lorem_ipsum.txt")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    fold_helper(po.stdout, "lorem_ipsum_40_column_word.expected");
}

fn fold_helper(output: Vec<u8>, filename: &str) {
    let mut f = File::open(Path::new(filename)).unwrap_or_else(|err| {
        panic!("{}", err)
    });
    let mut expected = vec!();
    match f.read_to_end(&mut expected) {
        Ok(_) => {},
        Err(err) => panic!("{}", err)
    }
    assert_eq!(String::from_utf8(output).unwrap(), String::from_utf8(expected).unwrap());
}
