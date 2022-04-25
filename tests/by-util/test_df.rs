// spell-checker:ignore udev pcent iuse itotal iused ipcent
use std::collections::HashSet;

use crate::common::util::*;

#[test]
fn test_df_compatible_no_size_arg() {
    new_ucmd!().arg("-a").succeeds();
}

#[test]
fn test_df_shortened_long_argument() {
    new_ucmd!().arg("--a").succeeds();
}

#[test]
fn test_df_compatible() {
    new_ucmd!().arg("-ah").succeeds();
}

#[test]
fn test_df_compatible_type() {
    new_ucmd!().arg("-aT").succeeds();
}

#[test]
fn test_df_compatible_si() {
    new_ucmd!().arg("-aH").succeeds();
}

#[test]
fn test_df_arguments_override_themselves() {
    new_ucmd!().args(&["--help", "--help"]).succeeds();
    new_ucmd!().arg("-aa").succeeds();
    new_ucmd!()
        .args(&["--block-size=3000", "--block-size=1000"])
        .succeeds();
    new_ucmd!().args(&["--total", "--total"]).succeeds();
    new_ucmd!().arg("-hh").succeeds();
    new_ucmd!().arg("-HH").succeeds();
    new_ucmd!().arg("-ii").succeeds();
    new_ucmd!().arg("-kk").succeeds();
    new_ucmd!().arg("-ll").succeeds();
    new_ucmd!().args(&["--no-sync", "--no-sync"]).succeeds();
    new_ucmd!().arg("-PP").succeeds();
    new_ucmd!().args(&["--sync", "--sync"]).succeeds();
    new_ucmd!().arg("-TT").succeeds();
}

#[test]
fn test_df_conflicts_overriding() {
    new_ucmd!().arg("-hH").succeeds();
    new_ucmd!().arg("-Hh").succeeds();
    new_ucmd!().args(&["--no-sync", "--sync"]).succeeds();
    new_ucmd!().args(&["--sync", "--no-sync"]).succeeds();
    new_ucmd!().args(&["-k", "--block-size=3000"]).succeeds();
    new_ucmd!().args(&["--block-size=3000", "-k"]).succeeds();
}

#[test]
fn test_df_output_arg() {
    new_ucmd!().args(&["--output=source", "-iPT"]).fails();
    new_ucmd!().args(&["-iPT", "--output=source"]).fails();
    new_ucmd!()
        .args(&["--output=source", "--output=source"])
        .fails();
}

#[test]
fn test_df_output() {
    let expected = if cfg!(target_os = "macos") {
        vec![
            "Filesystem",
            "Size",
            "Used",
            "Available",
            "Capacity",
            "Use%",
            "Mounted",
            "on",
        ]
    } else {
        vec![
            "Filesystem",
            "Size",
            "Used",
            "Available",
            "Use%",
            "Mounted",
            "on",
        ]
    };
    let output = new_ucmd!()
        .arg("-H")
        .arg("--total")
        .succeeds()
        .stdout_move_str();
    let actual = output.lines().take(1).collect::<Vec<&str>>()[0];
    let actual = actual.split_whitespace().collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn test_df_output_overridden() {
    let expected = if cfg!(target_os = "macos") {
        vec![
            "Filesystem",
            "Size",
            "Used",
            "Available",
            "Capacity",
            "Use%",
            "Mounted",
            "on",
        ]
    } else {
        vec![
            "Filesystem",
            "Size",
            "Used",
            "Available",
            "Use%",
            "Mounted",
            "on",
        ]
    };
    let output = new_ucmd!()
        .arg("-hH")
        .arg("--total")
        .succeeds()
        .stdout_move_str();
    let actual = output.lines().take(1).collect::<Vec<&str>>()[0];
    let actual = actual.split_whitespace().collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn test_total_option_with_single_dash() {
    // These should fail because `-total` should have two dashes,
    // not just one.
    new_ucmd!().arg("-total").fails();
}

/// Test that the order of rows in the table does not change across executions.
#[test]
fn test_order_same() {
    let output1 = new_ucmd!()
        .arg("--output=source")
        .succeeds()
        .stdout_move_str();
    let output2 = new_ucmd!()
        .arg("--output=source")
        .succeeds()
        .stdout_move_str();
    assert_eq!(output1, output2);
}

/// Test of mount point begin repeated
#[cfg(unix)]
#[test]
fn test_output_mp_repeat() {
    let output1 = new_ucmd!().arg("/").arg("/").succeeds().stdout_move_str();
    let output1: Vec<String> = output1
        .lines()
        .map(|l| String::from(l.split_once(' ').unwrap().0))
        .collect();
    assert_eq!(3, output1.len());
    assert_eq!(output1[1], output1[2]);
}

#[test]
fn test_output_conflict_options() {
    for option in ["-i", "-T", "-P"] {
        new_ucmd!().arg("--output=source").arg(option).fails();
    }
}

#[test]
fn test_output_option() {
    new_ucmd!().arg("--output").succeeds();
    new_ucmd!().arg("--output=source,target").succeeds();
    new_ucmd!().arg("--output=invalid_option").fails();
}

#[test]
fn test_output_option_without_equals_sign() {
    new_ucmd!().arg("--output").arg(".").succeeds();
}

#[test]
fn test_type_option() {
    let fs_types = new_ucmd!()
        .arg("--output=fstype")
        .succeeds()
        .stdout_move_str();
    let fs_type = fs_types.lines().nth(1).unwrap().trim();

    new_ucmd!().args(&["-t", fs_type]).succeeds();
    new_ucmd!()
        .args(&["-t", fs_type, "-t", "nonexisting"])
        .succeeds();
    new_ucmd!()
        .args(&["-t", "nonexisting"])
        .fails()
        .stderr_contains("no file systems processed");
}

#[test]
fn test_type_option_with_file() {
    let fs_type = new_ucmd!()
        .args(&["--output=fstype", "."])
        .succeeds()
        .stdout_move_str();
    let fs_type = fs_type.lines().nth(1).unwrap().trim();

    new_ucmd!().args(&["-t", fs_type, "."]).succeeds();
    new_ucmd!()
        .args(&["-t", "nonexisting", "."])
        .fails()
        .stderr_contains("no file systems processed");

    let fs_types = new_ucmd!()
        .arg("--output=fstype")
        .succeeds()
        .stdout_move_str();
    let fs_types: Vec<_> = fs_types
        .lines()
        .skip(1)
        .filter(|t| t.trim() != fs_type && t.trim() != "")
        .collect();

    if !fs_types.is_empty() {
        new_ucmd!()
            .args(&["-t", fs_types[0], "."])
            .fails()
            .stderr_contains("no file systems processed");
    }
}

#[test]
fn test_exclude_type_option() {
    new_ucmd!().args(&["-x", "ext4", "-x", "ext3"]).succeeds();
}

#[test]
fn test_exclude_all_types() {
    let fs_types = new_ucmd!()
        .arg("--output=fstype")
        .succeeds()
        .stdout_move_str();
    let fs_types: HashSet<_> = fs_types.lines().skip(1).collect();

    let mut args = Vec::new();

    for fs_type in fs_types {
        args.push("-x");
        args.push(fs_type.trim_end());
    }

    new_ucmd!()
        .args(&args)
        .fails()
        .stderr_contains("no file systems processed");
}

#[test]
fn test_include_exclude_same_type() {
    new_ucmd!()
        .args(&["-t", "ext4", "-x", "ext4"])
        .fails()
        .stderr_is("df: file system type 'ext4' both selected and excluded");
    new_ucmd!()
        .args(&["-t", "ext4", "-x", "ext4", "-t", "ext3", "-x", "ext3"])
        .fails()
        .stderr_is(
            "df: file system type 'ext4' both selected and excluded\n\
             df: file system type 'ext3' both selected and excluded",
        );
}

#[test]
fn test_total() {
    // Example output:
    //
    //     Filesystem            1K-blocks     Used Available Use% Mounted on
    //     udev                    3858016        0   3858016   0% /dev
    //     ...
    //     /dev/loop14               63488    63488         0 100% /snap/core20/1361
    //     total                 258775268 98099712 148220200  40% -
    let output = new_ucmd!().arg("--total").succeeds().stdout_move_str();

    // Skip the header line.
    let lines: Vec<&str> = output.lines().skip(1).collect();

    // Parse the values from the last row.
    let last_line = lines.last().unwrap();
    let mut iter = last_line.split_whitespace();
    assert_eq!(iter.next().unwrap(), "total");
    let reported_total_size = iter.next().unwrap().parse().unwrap();
    let reported_total_used = iter.next().unwrap().parse().unwrap();
    let reported_total_avail = iter.next().unwrap().parse().unwrap();

    // Loop over each row except the last, computing the sum of each column.
    let mut computed_total_size = 0;
    let mut computed_total_used = 0;
    let mut computed_total_avail = 0;
    let n = lines.len();
    for line in &lines[..n - 1] {
        let mut iter = line.split_whitespace();
        iter.next().unwrap();
        computed_total_size += iter.next().unwrap().parse::<u64>().unwrap();
        computed_total_used += iter.next().unwrap().parse::<u64>().unwrap();
        computed_total_avail += iter.next().unwrap().parse::<u64>().unwrap();
    }

    // Check that the sum of each column matches the reported value in
    // the last row.
    assert_eq!(computed_total_size, reported_total_size);
    assert_eq!(computed_total_used, reported_total_used);
    assert_eq!(computed_total_avail, reported_total_avail);
}

#[test]
fn test_use_percentage() {
    let output = new_ucmd!()
        .args(&["--total", "--output=used,avail,pcent"])
        .succeeds()
        .stdout_move_str();

    // Skip the header line.
    let lines: Vec<&str> = output.lines().skip(1).collect();

    for line in lines {
        let mut iter = line.split_whitespace();
        let reported_used = iter.next().unwrap().parse::<f64>().unwrap();
        let reported_avail = iter.next().unwrap().parse::<f64>().unwrap();
        let reported_percentage = iter.next().unwrap();
        let reported_percentage = reported_percentage[..reported_percentage.len() - 1]
            .parse::<u8>()
            .unwrap();
        let computed_percentage =
            (100.0 * (reported_used / (reported_used + reported_avail))).ceil() as u8;

        assert_eq!(computed_percentage, reported_percentage);
    }
}

#[test]
fn test_iuse_percentage() {
    let output = new_ucmd!()
        .args(&["--total", "--output=itotal,iused,ipcent"])
        .succeeds()
        .stdout_move_str();

    // Skip the header line.
    let lines: Vec<&str> = output.lines().skip(1).collect();

    for line in lines {
        let mut iter = line.split_whitespace();
        let reported_inodes = iter.next().unwrap().parse::<f64>().unwrap();
        let reported_iused = iter.next().unwrap().parse::<f64>().unwrap();
        let reported_percentage = iter.next().unwrap();

        if reported_percentage == "-" {
            assert_eq!(0.0, reported_inodes);
            assert_eq!(0.0, reported_iused);
        } else {
            let reported_percentage = reported_percentage[..reported_percentage.len() - 1]
                .parse::<u8>()
                .unwrap();
            let computed_percentage = (100.0 * (reported_iused / reported_inodes)).ceil() as u8;

            assert_eq!(computed_percentage, reported_percentage);
        }
    }
}

#[test]
fn test_block_size_1024() {
    fn get_header(block_size: u64) -> String {
        let output = new_ucmd!()
            .args(&["-B", &format!("{}", block_size), "--output=size"])
            .succeeds()
            .stdout_move_str();
        output.lines().next().unwrap().to_string()
    }

    assert_eq!(get_header(1024), "1K-blocks");
    assert_eq!(get_header(2048), "2K-blocks");
    assert_eq!(get_header(4096), "4K-blocks");
    assert_eq!(get_header(1024 * 1024), "1M-blocks");
    assert_eq!(get_header(2 * 1024 * 1024), "2M-blocks");
    assert_eq!(get_header(1024 * 1024 * 1024), "1G-blocks");
    assert_eq!(get_header(34 * 1024 * 1024 * 1024), "34G-blocks");
}

#[test]
fn test_block_size_with_suffix() {
    fn get_header(block_size: &str) -> String {
        let output = new_ucmd!()
            .args(&["-B", block_size, "--output=size"])
            .succeeds()
            .stdout_move_str();
        output.lines().next().unwrap().to_string()
    }

    assert_eq!(get_header("K"), "1K-blocks");
    assert_eq!(get_header("M"), "1M-blocks");
    assert_eq!(get_header("G"), "1G-blocks");
    assert_eq!(get_header("1K"), "1K-blocks");
    assert_eq!(get_header("1M"), "1M-blocks");
    assert_eq!(get_header("1G"), "1G-blocks");
    assert_eq!(get_header("1KiB"), "1K-blocks");
    assert_eq!(get_header("1MiB"), "1M-blocks");
    assert_eq!(get_header("1GiB"), "1G-blocks");
    // TODO enable the following asserts when #3193 is resolved
    //assert_eq!(get_header("1KB"), "1kB-blocks");
    //assert_eq!(get_header("1MB"), "1MB-blocks");
    //assert_eq!(get_header("1GB"), "1GB-blocks");
}

#[test]
fn test_output_selects_columns() {
    let output = new_ucmd!()
        .args(&["--output=source"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(output.lines().next().unwrap().trim_end(), "Filesystem");

    let output = new_ucmd!()
        .args(&["--output=source,target"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        output
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["Filesystem", "Mounted", "on"]
    );

    let output = new_ucmd!()
        .args(&["--output=source,target,used"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        output
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["Filesystem", "Mounted", "on", "Used"]
    );
}

#[test]
fn test_output_multiple_occurrences() {
    let output = new_ucmd!()
        .args(&["--output=source", "--output=target"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        output
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["Filesystem", "Mounted", "on"]
    );
}

#[test]
fn test_output_file_all_filesystems() {
    // When run with no positional arguments, `df` lets "-" represent
    // the "File" entry for each row.
    let output = new_ucmd!()
        .arg("--output=file")
        .succeeds()
        .stdout_move_str();
    let mut lines = output.lines();
    assert_eq!(lines.next().unwrap(), "File");
    for line in lines {
        assert_eq!(line, "-   ");
    }
}

#[test]
fn test_output_file_specific_files() {
    // Create three files.
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    at.touch("c");

    // When run with positional arguments, the filesystems should
    // appear in the "File" column.
    let output = ucmd
        .args(&["--output=file", "a", "b", "c"])
        .succeeds()
        .stdout_move_str();
    let actual: Vec<&str> = output.lines().collect();
    assert_eq!(actual, vec!["File", "a   ", "b   ", "c   "]);
}

#[test]
fn test_file_column_width_if_filename_contains_unicode_chars() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("äöü.txt");

    let output = ucmd
        .args(&["--output=file,target", "äöü.txt"])
        .succeeds()
        .stdout_move_str();
    let actual = output.lines().next().unwrap();
    // expected width: 7 chars (length of äöü.txt) + 1 char (column separator)
    assert_eq!(actual, "File    Mounted on");
}

#[test]
fn test_output_field_no_more_than_once() {
    new_ucmd!()
        .arg("--output=target,source,target")
        .fails()
        .usage_error("option --output: field 'target' used more than once");
}

#[test]
fn test_nonexistent_file() {
    new_ucmd!()
        .arg("does-not-exist")
        .fails()
        .stderr_only("df: does-not-exist: No such file or directory");
    new_ucmd!()
        .args(&["--output=file", "does-not-exist", "."])
        .fails()
        .stderr_is("df: does-not-exist: No such file or directory\n")
        .stdout_is("File\n.   \n");
}
