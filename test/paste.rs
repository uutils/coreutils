use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

static PROGNAME: &'static str = "./paste";

#[test]
fn test_combine_pairs_of_lines() {
    let po = Command::new(PROGNAME)
                 .arg("-s")
                 .arg("-d")
                 .arg("\t\n")
                 .arg("html_colors.txt")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let mut f = File::open(Path::new("html_colors.expected"))
                    .unwrap_or_else(|err| panic!("{}", err));
    let mut expected = vec!();
    match f.read_to_end(&mut expected) {
        Ok(_) => {}
        Err(err) => panic!("{}", err),
    }
    assert_eq!(String::from_utf8(po.stdout).unwrap(),
               String::from_utf8(expected).unwrap());
}
