use common::util::*;

#[test]
fn test_from_si() {
    new_ucmd!()
        .args(&["--from=si"])
        .pipe_in("1000\n1.1M\n0.1G")
        .run()
        .stdout_is("1000\n1100000\n100000000");
}

#[test]
fn test_from_iec() {
    new_ucmd!()
        .args(&["--from=iec"])
        .pipe_in("1024\n1.1M\n0.1G")
        .run()
        .stdout_is("1024\n1153434\n107374182");
}

#[test]
fn test_from_iec_i() {
    new_ucmd!()
        .args(&["--from=iec-i"])
        .pipe_in("1024\n1.1Mi\n0.1Gi")
        .run()
        .stdout_is("1024\n1153434\n107374182");
}

#[test]
fn test_from_auto() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1K\n1Ki")
        .run()
        .stdout_is("1000\n1024");
}

#[test]
fn test_to_si() {
    new_ucmd!()
        .args(&["--to=si"])
        .pipe_in("1000\n1100000\n100000000")
        .run()
        .stdout_is("1.0K\n1.1M\n100.0M");
}

#[test]
fn test_to_iec() {
    new_ucmd!()
        .args(&["--to=iec"])
        .pipe_in("1024\n1153434\n107374182")
        .run()
        .stdout_is("1.0K\n1.1M\n102.4M");
}

#[test]
fn test_to_iec_i() {
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("1024\n1153434\n107374182")
        .run()
        .stdout_is("1.0Ki\n1.1Mi\n102.4Mi");
}

#[test]
fn test_input_from_free_arguments() {
    new_ucmd!()
        .args(&["--from=si", "1K", "1.1M", "0.1G"])
        .run()
        .stdout_is("1000\n1100000\n100000000");
}

#[test]
fn test_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .run()
        .stdout_is("    1000\n 1100000\n100000000");
}

#[test]
fn test_negative_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=-8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .run()
        .stdout_is("1000    \n1100000 \n100000000");
}

#[test]
fn test_header() {
    new_ucmd!()
        .args(&["--from=si", "--header=2"])
        .pipe_in("header\nheader2\n1K\n1.1M\n0.1G")
        .run()
        .stdout_is("header\nheader2\n1000\n1100000\n100000000");
}

#[test]
fn test_header_default() {
    new_ucmd!()
        .args(&["--from=si", "--header"])
        .pipe_in("header\n1K\n1.1M\n0.1G")
        .run()
        .stdout_is("header\n1000\n1100000\n100000000");
}
