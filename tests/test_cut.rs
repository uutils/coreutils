use common::util::*;


static INPUT: &'static str = "lists.txt";

struct TestedSequence<'b> {
   name : &'b str,
   sequence: &'b str
}

static EXAMPLE_SEQUENCES: &'static [TestedSequence<'static>] = &[
    TestedSequence{ name: "singular", sequence:"2" },
    TestedSequence{ name: "prefix", sequence: "-2" },
    TestedSequence{ name: "suffix", sequence: "2-" },
    TestedSequence{ name: "range", sequence: "2-4" },
    TestedSequence{ name: "aggregate", sequence: "9-,6-7,-2,4" },
    TestedSequence{ name: "subsumed", sequence: "2-,3" }
];

static COMPLEX_SEQUENCE: &'static TestedSequence<'static> = &TestedSequence{ name: "", sequence: "9-,6-7,-2,4" };

#[test]
fn test_byte_sequence() {
    for param in vec!["-b", "--bytes"] {
        for example_seq in EXAMPLE_SEQUENCES {
            new_ucmd!().args(&[param, example_seq.sequence, INPUT])
                .succeeds().stdout_only_fixture(format!("sequences/byte_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_char_sequence() {
    for param in vec!["-c", "--characters"] {
        for example_seq in EXAMPLE_SEQUENCES {
            //as of coreutils 8.25 a char range is effectively the same as a byte range; there is no distinct treatment of utf8 chars.
            new_ucmd!().args(&[param, example_seq.sequence, INPUT])
                .succeeds().stdout_only_fixture(format!("sequences/byte_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_field_sequence() {
    for param in vec!["-f", "--fields"] {
        for example_seq in EXAMPLE_SEQUENCES {
            new_ucmd!().args(&[param, example_seq.sequence, INPUT])
                .succeeds().stdout_only_fixture(format!("sequences/field_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_specify_delimiter() {
    for param in vec!["-d", "--delimiter"] {
        new_ucmd!().args(&[param, ":", "-f", COMPLEX_SEQUENCE.sequence, INPUT])
            .succeeds().stdout_only_fixture("delimiter_specified.expected");
    }
}

#[test]
fn test_output_delimiter() {
    // we use -d here to ensure output delimiter 
    // is applied to the current, and not just the default, input delimiter
    new_ucmd!().args(&["-d:", "--output-delimiter=@", "-f", COMPLEX_SEQUENCE.sequence, INPUT])
        .succeeds().stdout_only_fixture("output_delimiter.expected");
}

#[test]
fn test_complement() {
    new_ucmd!().args(&["-d_","--complement", "-f", "2"])
        .pipe_in("9_1\n8_2\n7_3")
        .succeeds().stdout_only("9\n8\n7\n");
}

#[test]
fn test_zero_terminated() {
    new_ucmd!().args(&["-d_","-z", "-f", "1"])
        .pipe_in("9_1\n8_2\n\07_3")
        .succeeds().stdout_only("9\07\0");
}

#[test]
fn test_only_delimited() {
    for param in vec!["-s", "--only-delimited"] {
        new_ucmd!().args(&["-d_", param, "-f", "1"])
            .pipe_in("91\n82\n7_3")
            .succeeds().stdout_only("7\n");
    }
}

#[test]
fn test_zero_terminated_only_delimited() {
    new_ucmd!().args(&["-d_","-z", "-s", "-f", "1"])
        .pipe_in("91\n\082\n7_3")
        .succeeds().stdout_only("82\n7\0");
}
