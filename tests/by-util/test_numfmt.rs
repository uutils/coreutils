use crate::common::util::*;

#[test]
fn test_from_si() {
    new_ucmd!()
        .args(&["--from=si"])
        .pipe_in("1000\n1.1M\n0.1G")
        .run()
        .stdout_is("1000\n1100000\n100000000\n");
}

#[test]
fn test_from_iec() {
    new_ucmd!()
        .args(&["--from=iec"])
        .pipe_in("1024\n1.1M\n0.1G")
        .run()
        .stdout_is("1024\n1153434\n107374182\n");
}

#[test]
fn test_from_iec_i() {
    new_ucmd!()
        .args(&["--from=iec-i"])
        .pipe_in("1024\n1.1Mi\n0.1Gi")
        .run()
        .stdout_is("1024\n1153434\n107374182\n");
}

#[test]
fn test_from_auto() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1K\n1Ki")
        .run()
        .stdout_is("1000\n1024\n");
}

#[test]
fn test_to_si() {
    new_ucmd!()
        .args(&["--to=si"])
        .pipe_in("1000\n1100000\n100000000")
        .run()
        .stdout_is("1.0K\n1.1M\n100.0M\n");
}

#[test]
fn test_to_iec() {
    new_ucmd!()
        .args(&["--to=iec"])
        .pipe_in("1024\n1153434\n107374182")
        .run()
        .stdout_is("1.0K\n1.1M\n102.4M\n");
}

#[test]
fn test_to_iec_i() {
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("1024\n1153434\n107374182")
        .run()
        .stdout_is("1.0Ki\n1.1Mi\n102.4Mi\n");
}

#[test]
fn test_input_from_free_arguments() {
    new_ucmd!()
        .args(&["--from=si", "1K", "1.1M", "0.1G"])
        .run()
        .stdout_is("1000\n1100000\n100000000\n");
}

#[test]
fn test_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .run()
        .stdout_is("    1000\n 1100000\n100000000\n");
}

#[test]
fn test_negative_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=-8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .run()
        .stdout_is("1000    \n1100000 \n100000000\n");
}

#[test]
fn test_header() {
    new_ucmd!()
        .args(&["--from=si", "--header=2"])
        .pipe_in("header\nheader2\n1K\n1.1M\n0.1G")
        .run()
        .stdout_is("header\nheader2\n1000\n1100000\n100000000\n");
}

#[test]
fn test_header_default() {
    new_ucmd!()
        .args(&["--from=si", "--header"])
        .pipe_in("header\n1K\n1.1M\n0.1G")
        .run()
        .stdout_is("header\n1000\n1100000\n100000000\n");
}

#[test]
fn test_header_error_if_non_numeric() {
    new_ucmd!()
        .args(&["--header=two"])
        .run()
        .stderr_is("numfmt: invalid header value ‘two’");
}

#[test]
fn test_header_error_if_0() {
    new_ucmd!()
        .args(&["--header=0"])
        .run()
        .stderr_is("numfmt: invalid header value ‘0’");
}

#[test]
fn test_header_error_if_negative() {
    new_ucmd!()
        .args(&["--header=-3"])
        .run()
        .stderr_is("numfmt: invalid header value ‘-3’");
}

#[test]
fn test_negative() {
    new_ucmd!()
        .args(&["--from=si"])
        .pipe_in("-1000\n-1.1M\n-0.1G")
        .run()
        .stdout_is("-1000\n-1100000\n-100000000\n");
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("-1024\n-1153434\n-107374182")
        .run()
        .stdout_is("-1.0Ki\n-1.1Mi\n-102.4Mi\n");
}

#[test]
fn test_no_op() {
    new_ucmd!()
        .pipe_in("1024\n1234567")
        .run()
        .stdout_is("1024\n1234567\n");
}

#[test]
fn test_normalize() {
    new_ucmd!()
        .args(&["--from=si", "--to=si"])
        .pipe_in("10000000K\n0.001K")
        .run()
        .stdout_is("10.0G\n1\n");
}

#[test]
fn test_si_to_iec() {
    new_ucmd!()
        .args(&["--from=si", "--to=iec", "15334263563K"])
        .run()
        .stdout_is("13.9T\n");
}

#[test]
fn test_should_report_invalid_empty_number_on_empty_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("\n")
        .run()
        .stderr_is("numfmt: invalid number: ‘’\n");
}

#[test]
fn test_should_report_invalid_empty_number_on_blank_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("  \t  \n")
        .run()
        .stderr_is("numfmt: invalid number: ‘’\n");
}

#[test]
fn test_should_report_invalid_suffix_on_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1k")
        .run()
        .stderr_is("numfmt: invalid suffix in input: ‘1k’\n");

    // GNU numfmt reports this one as “invalid number”
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("NaN")
        .run()
        .stderr_is("numfmt: invalid suffix in input: ‘NaN’\n");
}

#[test]
fn test_should_report_invalid_number_with_interior_junk() {
    // GNU numfmt reports this as “invalid suffix”
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1x0K")
        .run()
        .stderr_is("numfmt: invalid number: ‘1x0K’\n");
}

#[test]
fn test_should_skip_leading_space_from_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in(" 2Ki")
        .run()
        .stdout_is("2048\n");

    // multiline
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("\t1Ki\n  2K")
        .run()
        .stdout_is("1024\n2000\n");
}

#[test]
fn test_should_convert_only_first_number_in_line() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1Ki 2M 3G")
        .run()
        .stdout_is("1024 2M 3G\n");
}

#[test]
fn test_leading_whitespace_should_imply_padding() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("   1K")
        .run()
        .stdout_is(" 1000\n");


    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("    202Ki")
        .run()
        .stdout_is("   206848\n");
}

#[test]
fn test_should_calculate_implicit_padding_per_line() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("   1Ki\n        2K")
        .run()
        .stdout_is("  1024\n      2000\n");
}

#[test]
fn test_leading_whitespace_in_free_argument_should_imply_padding() {
    new_ucmd!()
        .args(&["--from=auto", "   1Ki"])
        .run()
        .stdout_is("  1024\n");
}

#[test]
fn test_should_calculate_implicit_padding_per_free_argument() {
    new_ucmd!()
        .args(&["--from=auto", "   1Ki", "        2K"])
        .pipe_in("   1Ki\n        2K")
        .run()
        .stdout_is("  1024\n      2000\n");
}
