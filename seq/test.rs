use std::{run,str};

#[test]
fn test_count_up() {
    let p = run::process_output("build/seq", [~"10"]).unwrap();
    let out = str::from_utf8(p.output).unwrap().into_owned();
    assert_eq!(out, ~"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down() {
    let p = run::process_output("build/seq", [~"--", ~"5", ~"-1", ~"1"]).unwrap();
    let out = str::from_utf8(p.output).unwrap().into_owned();
    assert_eq!(out, ~"5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator() {
    let p = run::process_output("build/seq", [~"-s", ~",", ~"-t", ~"!", ~"2", ~"6"]).unwrap();
    let out = str::from_utf8(p.output).unwrap().into_owned();
    assert_eq!(out, ~"2,3,4,5,6!");
}

#[test]
fn test_equalize_widths() {
    let p = run::process_output("build/seq", [~"-w",  ~"5", ~"10"]).unwrap();
    let out = str::from_utf8(p.output).unwrap().into_owned();
    assert_eq!(out, ~"05\n06\n07\n08\n09\n10\n");
}
