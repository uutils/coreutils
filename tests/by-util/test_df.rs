// spell-checker:ignore udev
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
fn test_df_output() {
    // TODO These should fail because `-total` should have two dashes,
    // not just one. But they don't fail.
    if cfg!(target_os = "macos") {
        new_ucmd!().arg("-H").arg("-total").succeeds().
        stdout_only("Filesystem               Size         Used    Available     Capacity  Use% Mounted on       \n");
    } else {
        new_ucmd!().arg("-H").arg("-total").succeeds().stdout_only(
            "Filesystem               Size         Used    Available  Use% Mounted on       \n",
        );
    }
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
fn test_type_option() {
    new_ucmd!().args(&["-t", "ext4", "-t", "ext3"]).succeeds();
}

#[test]
fn test_exclude_type_option() {
    new_ucmd!().args(&["-x", "ext4", "-x", "ext3"]).succeeds();
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
    // Example output:
    //
    //     Filesystem            1K-blocks     Used Available Use% Mounted on
    //     udev                    3858016        0   3858016   0% /dev
    //     ...
    //     /dev/loop14               63488    63488         0 100% /snap/core20/1361
    let output = new_ucmd!().succeeds().stdout_move_str();

    // Skip the header line.
    let lines: Vec<&str> = output.lines().skip(1).collect();

    for line in lines {
        let mut iter = line.split_whitespace();
        iter.next();
        let reported_size = iter.next().unwrap().parse::<f64>().unwrap();
        let reported_used = iter.next().unwrap().parse::<f64>().unwrap();
        // Skip "Available" column
        iter.next();
        if cfg!(target_os = "macos") {
            // Skip "Capacity" column
            iter.next();
        }
        let reported_percentage = iter.next().unwrap();
        let reported_percentage = reported_percentage[..reported_percentage.len() - 1]
            .parse::<u8>()
            .unwrap();
        let computed_percentage = (100.0 * (reported_used / reported_size)).ceil() as u8;

        assert_eq!(computed_percentage, reported_percentage);
    }
}

#[test]
fn test_block_size_1024() {
    fn get_header(block_size: u64) -> String {
        // TODO When #3057 is resolved, we should just use
        //
        //     new_ucmd!().arg("--output=size").succeeds().stdout_move_str();
        //
        // instead of parsing the entire `df` table as a string.
        let output = new_ucmd!()
            .args(&["-B", &format!("{}", block_size)])
            .succeeds()
            .stdout_move_str();
        let first_line = output.lines().next().unwrap();
        let mut column_labels = first_line.split_whitespace();
        let size_column_label = column_labels.nth(1).unwrap();
        size_column_label.into()
    }

    assert_eq!(get_header(1024), "1K-blocks");
    assert_eq!(get_header(2048), "2K-blocks");
    assert_eq!(get_header(4096), "4K-blocks");
    assert_eq!(get_header(1024 * 1024), "1M-blocks");
    assert_eq!(get_header(2 * 1024 * 1024), "2M-blocks");
    assert_eq!(get_header(1024 * 1024 * 1024), "1G-blocks");
    assert_eq!(get_header(34 * 1024 * 1024 * 1024), "34G-blocks");
}

// TODO The spacing does not match GNU df. Also we need to remove
// trailing spaces from the heading row.
#[test]
fn test_output_selects_columns() {
    let output = new_ucmd!()
        .args(&["--output=source"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(output.lines().next().unwrap(), "Filesystem       ");

    let output = new_ucmd!()
        .args(&["--output=source,target"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        output.lines().next().unwrap(),
        "Filesystem       Mounted on       "
    );

    let output = new_ucmd!()
        .args(&["--output=source,target,used"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        output.lines().next().unwrap(),
        "Filesystem       Mounted on               Used "
    );
}

#[test]
fn test_output_multiple_occurrences() {
    let output = new_ucmd!()
        .args(&["--output=source", "--output=target"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        output.lines().next().unwrap(),
        "Filesystem       Mounted on       "
    );
}

// TODO Fix the spacing.
#[test]
fn test_output_file_all_filesystems() {
    // When run with no positional arguments, `df` lets "-" represent
    // the "File" entry for each row.
    let output = new_ucmd!()
        .arg("--output=file")
        .succeeds()
        .stdout_move_str();
    let mut lines = output.lines();
    assert_eq!(lines.next().unwrap(), "File            ");
    for line in lines {
        assert_eq!(line, "-               ");
    }
}

// TODO Fix the spacing.
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
    assert_eq!(
        actual,
        vec![
            "File            ",
            "a               ",
            "b               ",
            "c               "
        ]
    );
}
