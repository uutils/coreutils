// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore udev pcent iuse itotal iused ipcent
#![allow(
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_cmp
)]

use std::collections::HashSet;

#[cfg(not(any(target_os = "freebsd", target_os = "windows")))]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

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
fn test_df_compatible_sync() {
    new_ucmd!().arg("--sync").succeeds();
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
            "Avail",
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
            "Avail",
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
            "Avail",
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
            "Avail",
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
fn test_default_headers() {
    let expected = if cfg!(target_os = "macos") {
        vec![
            "Filesystem",
            "1K-blocks",
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
            "1K-blocks",
            "Used",
            "Available",
            "Use%",
            "Mounted",
            "on",
        ]
    };
    let output = new_ucmd!().succeeds().stdout_move_str();
    let actual = output.lines().take(1).collect::<Vec<&str>>()[0];
    let actual = actual.split_whitespace().collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn test_precedence_of_human_readable_and_si_header_over_output_header() {
    let args = ["-h", "--human-readable", "-H", "--si"];

    for arg in args {
        let output = new_ucmd!()
            .args(&[arg, "--output=size"])
            .succeeds()
            .stdout_move_str();
        let header = output.lines().next().unwrap();
        assert_eq!(header, " Size");
    }
}

#[test]
fn test_used_header_starts_with_space() {
    let output = new_ucmd!()
        // using -h here to ensure the width of the column's content is <= 4
        .args(&["-h", "--output=used"])
        .succeeds()
        .stdout_move_str();
    let header = output.lines().next().unwrap();
    assert_eq!(header, " Used");
}

#[test]
#[cfg(not(target_os = "freebsd"))] // FIXME: fix this test for FreeBSD
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
#[cfg(all(unix, not(target_os = "freebsd")))] // FIXME: fix this test for FreeBSD
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
#[cfg(not(target_os = "freebsd"))] // FIXME: fix this test for FreeBSD
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
#[cfg(not(any(target_os = "freebsd", target_os = "windows")))] // FIXME: fix test for FreeBSD & Win
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
        .stderr_is("df: file system type 'ext4' both selected and excluded\n");
    new_ucmd!()
        .args(&["-t", "ext4", "-x", "ext4", "-t", "ext3", "-x", "ext3"])
        .fails()
        .stderr_is(
            "df: file system type 'ext4' both selected and excluded\n\
             df: file system type 'ext3' both selected and excluded\n",
        );
}

#[cfg_attr(
    all(target_arch = "aarch64", target_os = "linux"),
    ignore = "Issue #7158 - Test not supported on ARM64 Linux"
)]
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

/// Test that the "total" label appears in the correct column.
///
/// The "total" label should appear in the "source" column, or in the
/// "target" column if "source" is not visible.
#[cfg(not(target_os = "freebsd"))] // FIXME: fix this test for FreeBSD
#[test]
fn test_total_label_in_correct_column() {
    let output = new_ucmd!()
        .args(&["--output=source", "--total", "."])
        .succeeds()
        .stdout_move_str();
    let last_line = output.lines().last().unwrap();
    assert_eq!(last_line.trim(), "total");

    let output = new_ucmd!()
        .args(&["--output=target", "--total", "."])
        .succeeds()
        .stdout_move_str();
    let last_line = output.lines().last().unwrap();
    assert_eq!(last_line.trim(), "total");

    let output = new_ucmd!()
        .args(&["--output=source,target", "--total", "."])
        .succeeds()
        .stdout_move_str();
    let last_line = output.lines().last().unwrap();
    assert_eq!(
        last_line.split_whitespace().collect::<Vec<&str>>(),
        vec!["total", "-"]
    );

    let output = new_ucmd!()
        .args(&["--output=target,source", "--total", "."])
        .succeeds()
        .stdout_move_str();
    let last_line = output.lines().last().unwrap();
    assert_eq!(
        last_line.split_whitespace().collect::<Vec<&str>>(),
        vec!["-", "total"]
    );
}

#[test]
fn test_use_percentage() {
    let output = new_ucmd!()
        // set block size = 1, otherwise the returned values for
        // "used" and "avail" will be rounded. And using them to calculate
        // the "percentage" values might lead to a mismatch with the returned
        // "percentage" values.
        .args(&["--total", "--output=used,avail,pcent", "--block-size=1"])
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
fn test_default_block_size() {
    let output = new_ucmd!()
        .arg("--output=size")
        .succeeds()
        .stdout_move_str();
    let header = output.lines().next().unwrap().trim().to_string();

    assert_eq!(header, "1K-blocks");

    let output = new_ucmd!()
        .arg("--output=size")
        .env("POSIXLY_CORRECT", "1")
        .succeeds()
        .stdout_move_str();
    let header = output.lines().next().unwrap().trim().to_string();

    assert_eq!(header, "512B-blocks");
}

#[test]
fn test_default_block_size_in_posix_portability_mode() {
    fn get_header(s: &str) -> String {
        s.lines()
            .next()
            .unwrap()
            .to_string()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .trim()
            .to_string()
    }

    let output = new_ucmd!().arg("-P").succeeds().stdout_move_str();
    assert_eq!(get_header(&output), "1024-blocks");

    let output = new_ucmd!()
        .arg("-P")
        .env("POSIXLY_CORRECT", "1")
        .succeeds()
        .stdout_move_str();
    assert_eq!(get_header(&output), "512-blocks");
}

#[test]
fn test_block_size_1024() {
    fn get_header(block_size: u64) -> String {
        let output = new_ucmd!()
            .args(&["-B", &format!("{block_size}"), "--output=size"])
            .succeeds()
            .stdout_move_str();
        output.lines().next().unwrap().trim().to_string()
    }

    assert_eq!(get_header(1024), "1K-blocks");
    assert_eq!(get_header(2048), "2K-blocks");
    assert_eq!(get_header(4096), "4K-blocks");
    assert_eq!(get_header(1024 * 1024), "1M-blocks");
    assert_eq!(get_header(2 * 1024 * 1024), "2M-blocks");
    assert_eq!(get_header(1024 * 1024 * 1024), "1G-blocks");
    assert_eq!(get_header(34 * 1024 * 1024 * 1024), "34G-blocks");

    // multiples of both 1024 and 1000
    assert_eq!(get_header(128_000), "128kB-blocks");
    assert_eq!(get_header(1000 * 1024), "1.1MB-blocks");
    assert_eq!(get_header(1_000_000_000_000), "1TB-blocks");
}

#[test]
fn test_block_size_with_suffix() {
    fn get_header(block_size: &str) -> String {
        let output = new_ucmd!()
            .args(&["-B", block_size, "--output=size"])
            .succeeds()
            .stdout_move_str();
        output.lines().next().unwrap().trim().to_string()
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
    assert_eq!(get_header("1KB"), "1kB-blocks");
    assert_eq!(get_header("1MB"), "1MB-blocks");
    assert_eq!(get_header("1GB"), "1GB-blocks");
}

#[test]
fn test_block_size_in_posix_portability_mode() {
    fn get_header(block_size: &str) -> String {
        let output = new_ucmd!()
            .args(&["-P", "-B", block_size])
            .succeeds()
            .stdout_move_str();
        output
            .lines()
            .next()
            .unwrap()
            .to_string()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .trim()
            .to_string()
    }

    assert_eq!(get_header("1024"), "1024-blocks");
    assert_eq!(get_header("1K"), "1024-blocks");
    assert_eq!(get_header("1KB"), "1000-blocks");
    assert_eq!(get_header("1M"), "1048576-blocks");
    assert_eq!(get_header("1MB"), "1000000-blocks");
}

#[test]
fn test_block_size_from_env() {
    fn get_header(env_var: &str, env_value: &str) -> String {
        let output = new_ucmd!()
            .arg("--output=size")
            .env(env_var, env_value)
            .succeeds()
            .stdout_move_str();
        output.lines().next().unwrap().trim().to_string()
    }

    assert_eq!(get_header("DF_BLOCK_SIZE", "111"), "111B-blocks");
    assert_eq!(get_header("BLOCK_SIZE", "222"), "222B-blocks");
    assert_eq!(get_header("BLOCKSIZE", "333"), "333B-blocks");
}

#[test]
fn test_block_size_from_env_precedences() {
    fn get_header(one: (&str, &str), two: (&str, &str)) -> String {
        let (k1, v1) = one;
        let (k2, v2) = two;
        let output = new_ucmd!()
            .arg("--output=size")
            .env(k1, v1)
            .env(k2, v2)
            .succeeds()
            .stdout_move_str();
        output.lines().next().unwrap().trim().to_string()
    }

    let df_block_size = ("DF_BLOCK_SIZE", "111");
    let block_size = ("BLOCK_SIZE", "222");
    let blocksize = ("BLOCKSIZE", "333");

    assert_eq!(get_header(df_block_size, block_size), "111B-blocks");
    assert_eq!(get_header(df_block_size, blocksize), "111B-blocks");
    assert_eq!(get_header(block_size, blocksize), "222B-blocks");
}

#[test]
fn test_precedence_of_block_size_arg_over_env() {
    let output = new_ucmd!()
        .args(&["-B", "999", "--output=size"])
        .env("DF_BLOCK_SIZE", "111")
        .succeeds()
        .stdout_move_str();
    let header = output.lines().next().unwrap().trim().to_string();

    assert_eq!(header, "999B-blocks");
}

#[test]
fn test_invalid_block_size_from_env() {
    let default_block_size_header = "1K-blocks";

    let output = new_ucmd!()
        .arg("--output=size")
        .env("DF_BLOCK_SIZE", "invalid")
        .succeeds()
        .stdout_move_str();
    let header = output.lines().next().unwrap().trim().to_string();

    assert_eq!(header, default_block_size_header);

    let output = new_ucmd!()
        .arg("--output=size")
        .env("DF_BLOCK_SIZE", "invalid")
        .env("BLOCK_SIZE", "222")
        .succeeds()
        .stdout_move_str();
    let header = output.lines().next().unwrap().trim().to_string();

    assert_eq!(header, default_block_size_header);
}

#[test]
fn test_ignore_block_size_from_env_in_posix_portability_mode() {
    let default_block_size_header = "1024-blocks";

    let output = new_ucmd!()
        .arg("-P")
        .env("DF_BLOCK_SIZE", "111")
        .env("BLOCK_SIZE", "222")
        .env("BLOCKSIZE", "333")
        .succeeds()
        .stdout_move_str();
    let header = output
        .lines()
        .next()
        .unwrap()
        .to_string()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .trim()
        .to_string();

    assert_eq!(header, default_block_size_header);
}

#[test]
fn test_too_large_block_size() {
    fn run_command(size: &str) {
        new_ucmd!()
            .arg(format!("--block-size={size}"))
            .fails()
            .stderr_contains(format!("--block-size argument '{size}' too large"));
    }

    let too_large_sizes = vec!["1Y", "1Z"];

    for size in too_large_sizes {
        run_command(size);
    }
}

#[test]
fn test_invalid_block_size() {
    new_ucmd!()
        .arg("--block-size=x")
        .fails()
        .stderr_contains("invalid --block-size argument 'x'");

    new_ucmd!()
        .arg("--block-size=0")
        .fails()
        .stderr_contains("invalid --block-size argument '0'");

    new_ucmd!()
        .arg("--block-size=0K")
        .fails()
        .stderr_contains("invalid --block-size argument '0K'");
}

#[test]
fn test_invalid_block_size_suffix() {
    new_ucmd!()
        .arg("--block-size=1H")
        .fails()
        .stderr_contains("invalid suffix in --block-size argument '1H'");

    new_ucmd!()
        .arg("--block-size=1.2")
        .fails()
        .stderr_contains("invalid suffix in --block-size argument '1.2'");
}

#[test]
fn test_output_selects_columns() {
    let output = new_ucmd!()
        .args(&["--output=source"])
        .succeeds()
        .stdout_move_str();
    assert_eq!(output.lines().next().unwrap(), "Filesystem");

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
        assert_eq!(line, "-");
    }
}

#[test]
#[cfg(not(any(target_os = "freebsd", target_os = "windows")))] // FIXME: fix test for FreeBSD & Win
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
    assert_eq!(actual, vec!["File", "a", "b", "c"]);
}

#[test]
#[cfg(not(any(target_os = "freebsd", target_os = "windows")))] // FIXME: fix test for FreeBSD & Win
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
#[cfg(not(any(target_os = "freebsd", target_os = "windows")))] // FIXME: fix test for FreeBSD & Win
fn test_nonexistent_file() {
    new_ucmd!()
        .arg("does-not-exist")
        .fails()
        .stderr_only("df: does-not-exist: No such file or directory\n");
    new_ucmd!()
        .args(&["--output=file", "does-not-exist", "."])
        .fails()
        .stderr_is("df: does-not-exist: No such file or directory\n")
        .stdout_is("File\n.\n");
}
