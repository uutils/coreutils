use common::util::*;

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
fn test_delim_with_field() {
    new_ucmd!()
        .args(&["--field=3", "--header", "--to=si"])
        .pipe_in("total 164
-rw-r--r--  1  2911 Feb 11 14:15 build.rs
-rw-r--r--  1 90347 Mar  3 14:30 Cargo.lock
-rw-r--r--  1  7034 Feb 11 14:15 Cargo.toml
-rw-r--r--  1  2018 Feb 11 14:15 CONTRIBUTING.md
drwxr-xr-x  2  4096 Feb 11 14:15 docs")
        .run()
        .stdout_is("total 164
-rw-r--r--  1  2.9K Feb 11 14:15 build.rs
-rw-r--r--  1 90.3K Mar  3 14:30 Cargo.lock
-rw-r--r--  1  7.0K Feb 11 14:15 Cargo.toml
-rw-r--r--  1  2.0K Feb 11 14:15 CONTRIBUTING.md
drwxr-xr-x  2  4.1K Feb 11 14:15 docs
");


    new_ucmd!()
        .args(&["--field=2-4", "--header=2", "--to=si"])
        .pipe_in("
Filesystem                                     1B-blocks         Used     Available Use% Mounted on
udev                                         33560113152            0   33560113152   0% /dev
tmpfs                                         6726156288      2158592    6723997696   1% /run")
        .run()
        .stdout_is("
Filesystem                                     1B-blocks         Used     Available Use% Mounted on
udev                                               33.6G            0         33.6G   0% /dev
tmpfs                                               6.7G         2.2M          6.7G   1% /run
");
}
