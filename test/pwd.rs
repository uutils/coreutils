use std::env;
use std::process::Command;
use std::str;

static PROGNAME: &'static str = "./pwd";

#[test]
fn test_default() {
    let po = Command::new(PROGNAME)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    let expected = env::current_dir().unwrap().into_os_string().into_string().unwrap();
    assert_eq!(out, expected);
}
