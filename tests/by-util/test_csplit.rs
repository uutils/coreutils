// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use glob::glob;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

/// Returns a string of numbers with the given range, each on a new line.
/// The upper bound is not included.
fn generate(from: u32, to: u32) -> String {
    (from..to).fold(String::new(), |acc, v| format!("{acc}{v}\n"))
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-", "10"])
        .pipe_in(generate(1, 51))
        .succeeds()
        .stdout_only("18\n123\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 51));
}

#[test]
fn test_up_to_line() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10"])
        .succeeds()
        .stdout_only("18\n123\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 51));
}

#[test]
fn test_up_to_line_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "{2}"])
        .succeeds()
        .stdout_only("18\n30\n30\n63\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 20));
    assert_eq!(at.read("xx02"), generate(20, 30));
    assert_eq!(at.read("xx03"), generate(30, 51));
}

#[test]
fn test_up_to_line_sequence() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "25"])
        .succeeds()
        .stdout_only("18\n45\n78\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 25));
    assert_eq!(at.read("xx02"), generate(25, 51));
}

#[test]
fn test_up_to_match() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/"])
        .succeeds()
        .stdout_only("16\n125\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 9));
    assert_eq!(at.read("xx01"), generate(9, 51));
}

#[test]
fn test_up_to_match_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "{2}"])
        .succeeds()
        .stdout_only("16\n29\n30\n66\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx00"), generate(1, 9));
    assert_eq!(at.read("xx01"), generate(9, 19));
    assert_eq!(at.read("xx02"), generate(19, 29));
    assert_eq!(at.read("xx03"), generate(29, 51));
}

#[test]
fn test_up_to_match_sequence() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "/5$/"])
        .succeeds()
        .stdout_only("16\n17\n108\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 9));
    assert_eq!(at.read("xx01"), generate(9, 15));
    assert_eq!(at.read("xx02"), generate(15, 51));
}

#[test]
fn test_up_to_match_offset() {
    for offset in ["3", "+3"] {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.args(&["numbers50.txt", &format!("/9$/{offset}")])
            .succeeds()
            .stdout_only("24\n117\n");

        let count = glob(&at.plus_as_string("xx*"))
            .expect("there should be splits created")
            .count();
        assert_eq!(count, 2);
        assert_eq!(at.read("xx00"), generate(1, 12));
        assert_eq!(at.read("xx01"), generate(12, 51));
        at.remove("xx00");
        at.remove("xx01");
    }
}

#[test]
fn test_up_to_match_offset_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/+3", "{2}"])
        .succeeds()
        .stdout_only("24\n30\n30\n57\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx00"), generate(1, 12));
    assert_eq!(at.read("xx01"), generate(12, 22));
    assert_eq!(at.read("xx02"), generate(22, 32));
    assert_eq!(at.read("xx03"), generate(32, 51));
}

#[test]
fn test_up_to_match_negative_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/-3"])
        .succeeds()
        .stdout_only("10\n131\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 6));
    assert_eq!(at.read("xx01"), generate(6, 51));
}

#[test]
fn test_up_to_match_negative_offset_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/-3", "{2}"])
        .succeeds()
        .stdout_only("10\n26\n30\n75\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx00"), generate(1, 6));
    assert_eq!(at.read("xx01"), generate(6, 16));
    assert_eq!(at.read("xx02"), generate(16, 26));
    assert_eq!(at.read("xx03"), generate(26, 51));
}

#[test]
fn test_up_to_match_repeat_always() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "{*}"])
        .succeeds()
        .stdout_only("16\n29\n30\n30\n30\n6\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 6);
    assert_eq!(at.read("xx00"), generate(1, 9));
    assert_eq!(at.read("xx01"), generate(9, 19));
    assert_eq!(at.read("xx02"), generate(19, 29));
    assert_eq!(at.read("xx03"), generate(29, 39));
    assert_eq!(at.read("xx04"), generate(39, 49));
    assert_eq!(at.read("xx05"), generate(49, 51));
}

#[test]
fn test_up_to_match_repeat_over() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "{50}"])
        .fails()
        .stdout_is("16\n29\n30\n30\n30\n6\n")
        .stderr_is("csplit: '/9$/': match not found on repetition 5\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "{50}", "-k"])
        .fails()
        .stdout_is("16\n29\n30\n30\n30\n6\n")
        .stderr_is("csplit: '/9$/': match not found on repetition 5\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 6);
    assert_eq!(at.read("xx00"), generate(1, 9));
    assert_eq!(at.read("xx01"), generate(9, 19));
    assert_eq!(at.read("xx02"), generate(19, 29));
    assert_eq!(at.read("xx03"), generate(29, 39));
    assert_eq!(at.read("xx04"), generate(39, 49));
    assert_eq!(at.read("xx05"), generate(49, 51));
}

#[test]
fn test_skip_to_match() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%23%"])
        .succeeds()
        .stdout_only("84\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(23, 51));
}

#[test]
fn test_skip_to_match_sequence1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "%^4%"])
        .succeeds()
        .stdout_only("33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(40, 51));
}

#[test]
fn test_skip_to_match_sequence2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{1}", "%^4%"])
        .succeeds()
        .stdout_only("33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(40, 51));
}

#[test]
fn test_skip_to_match_sequence3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{1}", "/^4/"])
        .succeeds()
        .stdout_only("60\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(20, 40));
    assert_eq!(at.read("xx01"), generate(40, 51));
}

#[test]
fn test_skip_to_match_sequence4() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "/^4/"])
        .succeeds()
        .stdout_only("90\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(10, 40));
    assert_eq!(at.read("xx01"), generate(40, 51));
}

#[test]
fn test_skip_to_match_offset() {
    for offset in ["3", "+3"] {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.args(&["numbers50.txt", &format!("%23%{offset}")])
            .succeeds()
            .stdout_only("75\n");

        let count = glob(&at.plus_as_string("xx*"))
            .expect("there should be splits created")
            .count();
        assert_eq!(count, 1);
        assert_eq!(at.read("xx00"), generate(26, 51));
        at.remove("xx00");
    }
}

#[test]
fn test_skip_to_match_offset_suppress_empty() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-z", "-", "%a%1"])
        .pipe_in("a\n")
        .succeeds()
        .no_output();
    assert!(!at.file_exists("xx00"));
}

#[test]
fn test_skip_to_match_negative_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%23%-3"])
        .succeeds()
        .stdout_only("93\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(20, 51));
}

#[test]
fn test_skip_to_match_repeat_always() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{*}"])
        .succeeds()
        .no_stdout();

    let count = glob(&at.plus_as_string("xx*")).unwrap().count();
    assert_eq!(count, 0);
}

#[test]
fn test_mix() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "13", "%25%", "/0$/"])
        .succeeds()
        .stdout_only("27\n15\n63\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 13));
    assert_eq!(at.read("xx01"), generate(25, 30));
    assert_eq!(at.read("xx02"), generate(30, 51));
}

#[test]
fn test_option_keep() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-k", "numbers50.txt", "/20/", "/nope/"])
        .fails()
        .stderr_is("csplit: '/nope/': match not found\n")
        .stdout_is("48\n93\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 20));
    assert_eq!(at.read("xx01"), generate(20, 51));
}

#[test]
fn test_option_quiet() {
    for arg in ["-q", "--quiet", "-s", "--silent"] {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.args(&[arg, "numbers50.txt", "13", "%25%", "/0$/"])
            .succeeds()
            .no_stdout();

        let count = glob(&at.plus_as_string("xx*"))
            .expect("there should be splits created")
            .count();
        assert_eq!(count, 3);
        assert_eq!(at.read("xx00"), generate(1, 13));
        assert_eq!(at.read("xx01"), generate(25, 30));
        assert_eq!(at.read("xx02"), generate(30, 51));
        at.remove("xx00");
        at.remove("xx01");
        at.remove("xx02");
    }
}

#[test]
fn test_option_prefix() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["--prefix", "dog", "numbers50.txt", "13", "%25%", "/0$/"])
        .succeeds()
        .stdout_only("27\n15\n63\n");

    let count = glob(&at.plus_as_string("dog*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("dog00"), generate(1, 13));
    assert_eq!(at.read("dog01"), generate(25, 30));
    assert_eq!(at.read("dog02"), generate(30, 51));
}

#[test]
fn test_negative_offset_at_start() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-", "/a/-1", "{*}"])
        .pipe_in("\na\n")
        .succeeds()
        .stdout_only("0\n3\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), "");
    assert_eq!(at.read("xx01"), "\na\n");
}

#[test]
fn test_up_to_match_option_suppress_matched() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "--suppress-matched", "/0$/", "{*}"])
        .succeeds()
        .stdout_only("18\n27\n27\n27\n27\n0\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 6);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(11, 20));
    assert_eq!(at.read("xx02"), generate(21, 30));
    assert_eq!(at.read("xx03"), generate(31, 40));
    assert_eq!(at.read("xx04"), generate(41, 50));
    assert_eq!(at.read("xx05"), "");
}

#[test]
fn test_up_to_match_offset_option_suppress_matched() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "--suppress-matched", "/10/+4"])
        .succeeds()
        .stdout_only("30\n108\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 14));
    assert_eq!(at.read("xx01"), generate(15, 51));
}

#[test]
fn test_up_to_match_negative_offset_option_suppress_matched() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "--suppress-matched", "/10/-4"])
        .succeeds()
        .stdout_only("10\n129\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 6));
    assert_eq!(at.read("xx01"), generate(7, 51));
}

#[test]
fn test_up_to_line_option_suppress_matched() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "--suppress-matched", "10"])
        .succeeds()
        .stdout_only("18\n120\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(11, 51));
}

#[test]
fn test_skip_to_match_option_suppress_matched() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "--suppress-matched", "%0$%"])
        .succeeds()
        .stdout_only("120\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(11, 51));
}

#[test]
fn test_option_elide_empty_file1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "--suppress-matched", "-z", "/0$/", "{*}"])
        .succeeds()
        .stdout_only("18\n27\n27\n27\n27\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 5);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(11, 20));
    assert_eq!(at.read("xx02"), generate(21, 30));
    assert_eq!(at.read("xx03"), generate(31, 40));
    assert_eq!(at.read("xx04"), generate(41, 50));
}

#[test]
fn test_option_elide_empty_file2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-", "-z", "/a/-1", "{*}"])
        .pipe_in("\na\n")
        .succeeds()
        .stdout_only("3\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), "\na\n");
}

#[test]
fn test_up_to_match_context_overflow() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/45/+10"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/45/+10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.args(&["numbers50.txt", "/45/+10", "-k"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/45/+10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(1, 51));
}

#[test]
fn test_skip_to_match_context_underflow() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%5%-10"])
        .fails()
        .stdout_is("")
        .stderr_is("csplit: '%5%-10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%5%-10", "-k"])
        .fails()
        .stdout_is("")
        .stderr_is("csplit: '%5%-10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_match_context_overflow() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%45%+10"])
        .fails()
        .stderr_is("csplit: '%45%+10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%45%+10", "-k"])
        .fails()
        .stderr_only("csplit: '%45%+10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_up_to_no_match1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/4/", "/nope/"])
        .fails()
        .stdout_is("6\n135\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/4/", "/nope/", "-k"])
        .fails()
        .stdout_is("6\n135\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 4));
    assert_eq!(at.read("xx01"), generate(4, 51));
}

#[test]
fn test_up_to_no_match2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/4/", "/nope/", "{50}"])
        .fails()
        .stdout_is("6\n135\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/4/", "/nope/", "{50}", "-k"])
        .fails()
        .stdout_is("6\n135\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 4));
    assert_eq!(at.read("xx01"), generate(4, 51));
}

#[test]
fn test_up_to_no_match3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/0$/", "{50}"])
        .fails()
        .stdout_is("18\n30\n30\n30\n30\n3\n")
        .stderr_is("csplit: '/0$/': match not found on repetition 5\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/0$/", "{50}", "-k"])
        .fails()
        .stdout_is("18\n30\n30\n30\n30\n3\n")
        .stderr_is("csplit: '/0$/': match not found on repetition 5\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 6);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 20));
    assert_eq!(at.read("xx02"), generate(20, 30));
    assert_eq!(at.read("xx03"), generate(30, 40));
    assert_eq!(at.read("xx04"), generate(40, 50));
    assert_eq!(at.read("xx05"), "50\n");
}

#[test]
fn test_up_to_no_match4() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/", "/4/"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/", "/4/", "-k"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(1, 51));
}

#[test]
fn test_up_to_no_match5() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/", "{*}"])
        .succeeds()
        .stdout_only("141\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(1, 51));
}

#[test]
fn test_up_to_no_match6() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/-5"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/-5': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/-5", "-k"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/-5': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(1, 51));
}

#[test]
fn test_up_to_no_match7() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/+5"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/+5': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/+5", "-k"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/+5': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(1, 51));
}

#[test]
fn test_skip_to_no_match1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%"])
        .fails()
        .stderr_only("csplit: '%nope%': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_no_match2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%", "{50}"])
        .fails()
        .stderr_only("csplit: '%nope%': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_no_match3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{50}"])
        .fails()
        .stderr_only("csplit: '%0$%': match not found on repetition 5\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_no_match4() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%", "/4/"])
        .fails()
        .stderr_only("csplit: '%nope%': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_no_match5() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%", "{*}"])
        .succeeds()
        .no_stderr()
        .no_stdout();

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_no_match6() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%-5"])
        .fails()
        .stderr_only("csplit: '%nope%-5': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_no_match7() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%+5"])
        .fails()
        .stderr_only("csplit: '%nope%+5': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_no_match() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%nope%"])
        .fails()
        .stderr_only("csplit: '%nope%': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/nope/"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '/nope/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_too_small_line_num() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/", "10", "/40/"])
        .succeeds()
        .stdout_only("48\n0\n60\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx00"), generate(1, 20));
    assert_eq!(at.read("xx01"), "");
    assert_eq!(at.read("xx02"), generate(20, 40));
    assert_eq!(at.read("xx03"), generate(40, 51));
}

#[test]
fn test_too_small_line_num_equal() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/", "20"])
        .succeeds()
        .stdout_only("48\n0\n93\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 20));
    assert_eq!(at.read("xx01"), "");
    assert_eq!(at.read("xx02"), generate(20, 51));
}

#[test]
fn test_too_small_line_num_elided() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "-z", "/20/", "10", "/40/"])
        .succeeds()
        .stdout_only("48\n60\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 20));
    assert_eq!(at.read("xx01"), generate(20, 40));
    assert_eq!(at.read("xx02"), generate(40, 51));
}

#[test]
fn test_too_small_line_num_negative_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/-5", "10", "/40/"])
        .succeeds()
        .stdout_only("33\n0\n75\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx00"), generate(1, 15));
    assert_eq!(at.read("xx01"), "");
    assert_eq!(at.read("xx02"), generate(15, 40));
    assert_eq!(at.read("xx03"), generate(40, 51));
}

#[test]
fn test_too_small_line_num_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/", "10", "15", "/40/"])
        .succeeds()
        .stdout_only("48\n0\n0\n60\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 5);
    assert_eq!(at.read("xx00"), generate(1, 20));
    assert_eq!(at.read("xx01"), "");
    assert_eq!(at.read("xx02"), "");
    assert_eq!(at.read("xx03"), generate(20, 40));
    assert_eq!(at.read("xx04"), generate(40, 51));
}

#[test]
fn test_too_small_line_num_repeat() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/", "10", "{*}"])
        .fails()
        .stderr_is("csplit: '10': line number out of range on repetition 5\n")
        .stdout_is("48\n0\n0\n30\n30\n30\n3\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/", "10", "{*}", "-k"])
        .fails()
        .stderr_is("csplit: '10': line number out of range on repetition 5\n")
        .stdout_is("48\n0\n0\n30\n30\n30\n3\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 7);
    assert_eq!(at.read("xx00"), generate(1, 20));
    assert_eq!(at.read("xx01"), "");
    assert_eq!(at.read("xx02"), "");
    assert_eq!(at.read("xx03"), generate(20, 30));
    assert_eq!(at.read("xx04"), generate(30, 40));
    assert_eq!(at.read("xx05"), generate(40, 50));
    assert_eq!(at.read("xx06"), "50\n");
}

#[test]
fn test_line_num_out_of_range1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "100"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '100': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "100", "-k"])
        .fails()
        .stdout_is("141\n")
        .stderr_is("csplit: '100': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), generate(1, 51));
}

#[test]
fn test_line_num_out_of_range2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "100"])
        .fails()
        .stdout_is("18\n123\n")
        .stderr_is("csplit: '100': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "100", "-k"])
        .fails()
        .stdout_is("18\n123\n")
        .stderr_is("csplit: '100': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 51));
}

#[test]
fn test_line_num_out_of_range3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "40", "{2}"])
        .fails()
        .stdout_is("108\n33\n")
        .stderr_is("csplit: '40': line number out of range on repetition 1\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "40", "{2}", "-k"])
        .fails()
        .stdout_is("108\n33\n")
        .stderr_is("csplit: '40': line number out of range on repetition 1\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 40));
    assert_eq!(at.read("xx01"), generate(40, 51));
}

#[test]
fn test_line_num_out_of_range4() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "40", "{*}"])
        .fails()
        .stdout_is("108\n33\n")
        .stderr_is("csplit: '40': line number out of range on repetition 1\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "40", "{*}", "-k"])
        .fails()
        .stdout_is("108\n33\n")
        .stderr_is("csplit: '40': line number out of range on repetition 1\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 40));
    assert_eq!(at.read("xx01"), generate(40, 51));
}

#[test]
fn test_skip_to_match_negative_offset_before_a_match() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/-10", "/15/"])
        .fails()
        .stdout_is("18\n123\n")
        .stderr_is("csplit: '/15/': match not found\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_skip_to_match_negative_offset_before_a_line_num() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/-10", "15"])
        .succeeds()
        .stdout_only("18\n15\n108\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 15));
    assert_eq!(at.read("xx02"), generate(15, 51));
}

#[test]
fn test_corner_case1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/10/", "11"])
        .succeeds()
        .stdout_only("18\n3\n120\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), "10\n");
    assert_eq!(at.read("xx02"), generate(11, 51));
}

#[test]
fn test_corner_case2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/10/-5", "/10/"])
        .fails()
        .stderr_is("csplit: '/10/': match not found\n")
        .stdout_is("8\n133\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_corner_case3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/15/-3", "14", "/15/"])
        .fails()
        .stderr_is("csplit: '/15/': match not found\n")
        .stdout_is("24\n6\n111\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_corner_case4() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/20/-10", "/30/-4"])
        .succeeds()
        .stdout_only("18\n48\n75\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 26));
    assert_eq!(at.read("xx02"), generate(26, 51));
}

#[test]
fn test_up_to_match_context_underflow() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/5/-10"])
        .fails()
        .stdout_is("0\n")
        .stderr_is("csplit: '/5/-10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/5/-10", "-k"])
        .fails()
        .stdout_is("0\n")
        .stderr_is("csplit: '/5/-10': line number out of range\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("counting splits")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx00"), "");
}

// the offset is out of range because of the first pattern
#[test]
fn test_line_num_range_with_up_to_match1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "/12/-5"])
        .fails()
        .stderr_is("csplit: '/12/-5': line number out of range\n")
        .stdout_is("18\n0\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "/12/-5", "-k"])
        .fails()
        .stderr_is("csplit: '/12/-5': line number out of range\n")
        .stdout_is("18\n0\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), "");
}

// the offset is out of range because more lines are needed than physically available
#[test]
fn test_line_num_range_with_up_to_match2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "/12/-15"])
        .fails()
        .stderr_is("csplit: '/12/-15': line number out of range\n")
        .stdout_is("18\n0\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "/12/-15", "-k"])
        .fails()
        .stderr_is("csplit: '/12/-15': line number out of range\n")
        .stdout_is("18\n0\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), "");
}

// NOTE: output different than gnu's: the pattern /10/ is matched but should not
#[test]
fn test_line_num_range_with_up_to_match3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "/10/", "-k"])
        .fails()
        .stderr_is("csplit: '/10/': match not found\n")
        .stdout_is("18\n123\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 51));

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/10/", "10"])
        .succeeds()
        .stdout_only("18\n0\n123\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), "");
    assert_eq!(at.read("xx02"), generate(10, 51));
}

#[test]
fn precision_format() {
    for f in ["%#6.3x", "%0#6.3x"] {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.args(&["numbers50.txt", "10", "--suffix-format", f])
            .succeeds()
            .stdout_only("18\n123\n");

        let count = glob(&at.plus_as_string("xx*"))
            .expect("there should be splits created")
            .count();
        assert_eq!(count, 2);
        assert_eq!(at.read("xx   000"), generate(1, 10));
        assert_eq!(at.read("xx 0x001"), generate(10, 51));
    }
}

#[test]
fn zero_error() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("in");
    ucmd.args(&["in", "0"])
        .fails()
        .stderr_contains("0: line number must be greater");
}

#[test]
fn no_such_file() {
    new_ucmd!()
        .args(&["in", "0"])
        .fails()
        .stderr_contains("cannot open 'in' for reading: No such file or directory");
}

#[test]
fn repeat_everything() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[
        "numbers50.txt",
        "--suppress-matched",
        "--suppress-matched",
        "-kzsn", // spell-checker:disable-line
        "2",
        "-szkn3", // spell-checker:disable-line
        "-b",
        "%03d",
        "-b%03x",
        "-f",
        "xxy_",
        "-fxxz_", // spell-checker:disable-line
        "/13/",
        "9",
        "{5}",
    ])
    .fails()
    .no_stdout()
    .code_is(1)
    .stderr_only("csplit: '9': line number out of range on repetition 5\n");
    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be some splits created")
        .count();
    assert_eq!(count, 6);
    assert_eq!(at.read("xxz_000"), generate(1, 12 + 1));
    assert_eq!(at.read("xxz_001"), generate(14, 17 + 1)); // FIXME: GNU starts at 15
    assert_eq!(at.read("xxz_002"), generate(19, 26 + 1));
    assert_eq!(at.read("xxz_003"), generate(28, 35 + 1));
    assert_eq!(at.read("xxz_004"), generate(37, 44 + 1));
    assert_eq!(at.read("xxz_005"), generate(46, 50 + 1));
}

#[cfg(unix)]
#[test]
fn test_named_pipe_input_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    let mut fifo_writer =
        create_named_pipe_with_writer(&at.plus_as_string("fifo"), &generate(1, 51));

    let result = ucmd.args(&["fifo", "10"]).succeeds();
    fifo_writer.kill().unwrap();
    fifo_writer.wait().unwrap();
    result.stdout_only("18\n123\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx00"), generate(1, 10));
    assert_eq!(at.read("xx01"), generate(10, 51));
}

#[cfg(unix)]
fn create_named_pipe_with_writer(path: &str, data: &str) -> std::process::Child {
    // cSpell:ignore IRWXU
    nix::unistd::mkfifo(path, nix::sys::stat::Mode::S_IRWXU).unwrap();
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("printf '{}' > {path}", data))
        .spawn()
        .unwrap()
}

#[test]
fn test_directory_input_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("test_directory");

    #[cfg(unix)]
    ucmd.args(&["test_directory", "1"])
        .fails()
        .code_is(1)
        .stderr_only("csplit: read error: Is a directory\n");
    #[cfg(windows)]
    ucmd.args(&["test_directory", "1"])
        .fails()
        .code_is(1)
        .stderr_only("csplit: cannot open 'test_directory' for reading: Permission denied\n");
}
