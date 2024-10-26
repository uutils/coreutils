// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore endregion
// spell-checker:ignore setpriority
// spell-checker:ignore xfail
// spell-checker:ignore xpass

use chrono::{format::StrftimeItems, Local};
use clap::{command, Args, Parser, Subcommand};
use core::str;
use regex::bytes::{Captures, Regex};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env::{self, VarError},
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, BufWriter, Cursor, ErrorKind, Read, StdoutLock, Write},
    path::{Component, Path, PathBuf},
    process::ExitCode,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use types::Inner;

#[derive(Parser)]
#[command(about, version)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct AnalyzeResultsArguments {
    #[arg(long = "hash-output-file")]
    hash_output_file: PathBuf,

    #[arg(long = "output-file")]
    output_file: PathBuf,

    #[arg(long = "root-suite-log-file")]
    root_suite_log_file: PathBuf,

    #[arg(long = "sha-hash-from-ci")]
    sha_hash_from_ci: String,

    #[arg(long = "suite-log-file")]
    suite_log_file: PathBuf,
}

#[derive(Args)]
struct CompareResultArguments {
    #[arg(long = "ignore-intermittent-tests-file")]
    ignore_intermittent_tests_file: PathBuf,

    #[arg(long = "new-summary-file")]
    new_summary_file: PathBuf,

    #[arg(long = "ref-summary-file")]
    ref_summary_file: PathBuf,
}

#[derive(Args)]
struct ExtractDataFromTestLogsArguments {
    #[arg(long = "test-directory-path")]
    test_directory_path: PathBuf,

    #[arg(long = "output-path")]
    output_path: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Parse the test log file, extracting and serializing the data needed by "compare-result"
    #[command(name = "analyze-results")]
    AnalyzeResults(AnalyzeResultsArguments),

    /// Compare the current results to the last results gathered from the main branch to highlight if a pull request
    /// is making the results better/worse
    #[command(name = "compare-result")]
    CompareResult(CompareResultArguments),

    /// Convert test log into a JSON file
    #[command(name = "extract-data-from-test-logs")]
    ExtractDataFromTestLogs(ExtractDataFromTestLogsArguments),
}

fn main() -> ExitCode {
    env::set_var("RUST_BACKTRACE", "1");

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let arguments = Arguments::parse();

    let result = start(arguments);

    match result {
        Ok(ex) => ex,
        Err(er) => {
            tracing::error!(
                backtrace = %er.backtrace(),
                error = %er,
            );

            ExitCode::FAILURE
        }
    }
}

fn start(arguments: Arguments) -> anyhow::Result<ExitCode> {
    match arguments.command {
        Command::AnalyzeResults(an) => analyze_results(an),
        Command::CompareResult(co) => compare_result(co),
        Command::ExtractDataFromTestLogs(ex) => extract_data_from_test_logs(ex),
    }
}

fn analyze_results(analyze_results_arguments: AnalyzeResultsArguments) -> anyhow::Result<ExitCode> {
    struct ParsedLogFile {
        error: i64,
        fail: i64,
        pass: i64,
        skip: i64,
        total: i64,
        xfail: i64,
        xpass: i64,
        //
        error_hash_set: HashSet<String>,
        fail_hash_set: HashSet<String>,
        skip_hash_set: HashSet<String>,
    }

    let error_regex = Regex::new(r"^ERROR: (.+)\n?$")?;
    let fail_regex = Regex::new(r"^FAIL: (.+)\n?$")?;
    let skip_regex = Regex::new(r"^SKIP: (.+)\n?$")?;
    let summary_regex = Regex::new(r"^# ([A-Z]+): +([0-9]+)\n?$")?;

    let AnalyzeResultsArguments {
        hash_output_file,
        output_file,
        root_suite_log_file,
        sha_hash_from_ci,
        suite_log_file,
    } = analyze_results_arguments;

    let hash_output_file_path = hash_output_file.as_path();
    let output_file_path = output_file.as_path();
    let root_suite_log_file_path = root_suite_log_file.as_path();
    let suite_log_file_path = suite_log_file.as_path();

    if !root_suite_log_file_path.exists() {
        anyhow::bail!(
            "Path '{}' (argument '--root-suite-log-file') does not exist",
            root_suite_log_file_path.display()
        );
    }

    if !suite_log_file_path.exists() {
        anyhow::bail!(
            "Path '{}' (argument '--suite-log-file') does not exist",
            suite_log_file_path.display()
        );
    }

    fn parse_log_file(
        buffer: &mut Vec<u8>,
        error_regex: &Regex,
        fail_regex: &Regex,
        skip_regex: &Regex,
        summary_regex: &Regex,
        log_file_path: &Path,
    ) -> anyhow::Result<ParsedLogFile> {
        const TYPES_COUNT: usize = 7;

        fn extract_test_path(
            captures: Captures,
            hash_set: &mut HashSet<String>,
        ) -> anyhow::Result<()> {
            let test_path = captures
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("Required capture group not found"))?;

            // Must be UTF-8 at this point
            let test_path_str = str::from_utf8(test_path.as_bytes())?;

            let value_did_not_already_exist = hash_set.insert(test_path_str.to_owned());

            anyhow::ensure!(value_did_not_already_exist);

            Ok(())
        }

        let file = File::open(log_file_path)?;

        // Some of these files have non-UTF-8 bytes
        let mut buf_reader = BufReader::new(file);

        let mut summary_hash_map = HashMap::<String, i64>::with_capacity(TYPES_COUNT);

        loop {
            buffer.clear();

            match buf_reader.read_until(b'\n', buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }

                    // Looks like:

                    /*
                    # TOTAL: 35
                    # PASS:  16
                    # SKIP:  8
                    # XFAIL: 0
                    # FAIL:  11
                    # XPASS: 0
                    # ERROR: 0
                    */

                    let slice = buffer.as_slice();

                    if !slice.starts_with(b"# ") {
                        continue;
                    }

                    if let Some(ca) = summary_regex.captures(slice) {
                        let result_type = ca
                            .get(1)
                            .ok_or_else(|| anyhow::anyhow!("Required capture group not found"))?;

                        let count = ca
                            .get(2)
                            .ok_or_else(|| anyhow::anyhow!("Required capture group not found"))?;

                        // Must be UTF-8 at this point
                        let result_type_str = str::from_utf8(result_type.as_bytes())?;
                        let count_str = str::from_utf8(count.as_bytes())?;

                        let existing_value = summary_hash_map
                            .insert(result_type_str.to_owned(), count_str.parse::<i64>()?);

                        anyhow::ensure!(existing_value.is_none());

                        if summary_hash_map.len() == TYPES_COUNT {
                            break;
                        }
                    }
                }
                Err(er) => {
                    if er.kind() == ErrorKind::Interrupted {
                        continue;
                    }

                    anyhow::bail!(er);
                }
            }
        }

        anyhow::ensure!(summary_hash_map.len() == TYPES_COUNT);

        let get_value = |key: &str| match summary_hash_map.get(key) {
            Some(count) => Ok(*count),
            None => {
                anyhow::bail!("Required value '{key}' missing from summary")
            }
        };

        let mut error_hash_set = HashSet::<String>::new();
        let mut fail_hash_set = HashSet::<String>::new();
        let mut skip_hash_set = HashSet::<String>::new();

        loop {
            buffer.clear();

            match buf_reader.read_until(b'\n', buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }

                    let slice = buffer.as_slice();

                    if let Some(ca) = error_regex.captures(slice) {
                        extract_test_path(ca, &mut error_hash_set)?;
                    } else if let Some(ca) = fail_regex.captures(slice) {
                        extract_test_path(ca, &mut fail_hash_set)?;
                    } else if let Some(ca) = skip_regex.captures(slice) {
                        extract_test_path(ca, &mut skip_hash_set)?;
                    }
                }
                Err(er) => {
                    if er.kind() == ErrorKind::Interrupted {
                        continue;
                    }

                    anyhow::bail!(er);
                }
            }
        }

        let parsed_log_file = ParsedLogFile {
            error: get_value("ERROR")?,
            fail: get_value("FAIL")?,
            pass: get_value("PASS")?,
            skip: get_value("SKIP")?,
            total: get_value("TOTAL")?,
            xfail: get_value("XFAIL")?,
            xpass: get_value("XPASS")?,
            // New
            error_hash_set,
            fail_hash_set,
            skip_hash_set,
        };

        {
            // Suppress unused code warning
            let _ = parsed_log_file.xfail;
        }

        Ok(parsed_log_file)
    }

    let mut buffer = Vec::<u8>::new();

    let suite_log_file_parsed = parse_log_file(
        &mut buffer,
        &error_regex,
        &fail_regex,
        &skip_regex,
        &summary_regex,
        suite_log_file_path,
    )?;

    let root_suite_log_parsed = parse_log_file(
        &mut buffer,
        &error_regex,
        &fail_regex,
        &skip_regex,
        &summary_regex,
        root_suite_log_file_path,
    )?;

    // Total of tests executed
    // They are the normal number of tests as they are skipped in the normal run
    let total = suite_log_file_parsed.total;

    // This is the sum of the two test suites.
    // In the normal run, they are SKIP
    let pass = suite_log_file_parsed.pass + root_suite_log_parsed.pass;

    // As some of the tests executed as root as still SKIP (ex: selinux), we
    // need to some maths:
    // Number of tests skip as user - total test as root + skipped as root
    let skip =
        suite_log_file_parsed.skip - root_suite_log_parsed.total + root_suite_log_parsed.skip;

    // They used to be SKIP, now they fail (this is a good news)
    let fail = suite_log_file_parsed.fail + root_suite_log_parsed.fail;

    let xpass = suite_log_file_parsed.xpass;

    // They used to be SKIP, now they error (this is a good news)
    let error = suite_log_file_parsed.error + root_suite_log_parsed.error;

    let mut stdout_lock = io::stdout().lock();

    if matches!(total, 0 | 1) {
        ci_error(
            &mut stdout_lock,
            format_args!(
                "Failed to parse test results from '{}' (argument '--suite-log-file'); failing early",
                suite_log_file_path.display()
            ),
        )?;

        return Ok(ExitCode::FAILURE);
    }

    let message = format!("GNU tests summary = TOTAL: {total} / PASS: {pass} / FAIL: {fail} / ERROR: {error} / SKIP: {skip}");

    writeln!(&mut stdout_lock, "{message}")?;

    if error > 0 || fail > 0 {
        ci_warning(&mut stdout_lock, format_args!("{message}"))?;
    }

    let combined_sort_vec = |selector: for<'a> fn(&'a ParsedLogFile) -> &'a HashSet<String>| {
        let from_root_suite_log_file_parsed = selector(&root_suite_log_parsed);
        let from_suite_log_file_parsed = selector(&suite_log_file_parsed);

        let union = from_root_suite_log_file_parsed.union(from_suite_log_file_parsed);

        let mut vec = union.map(ToOwned::to_owned).collect::<Vec<_>>();

        let from_root_suite_log_file_parsed_len = from_root_suite_log_file_parsed.len();
        let from_suite_log_file_parsed_len = from_suite_log_file_parsed.len();
        let vec_len = vec.len();

        if vec_len != (from_root_suite_log_file_parsed_len + from_suite_log_file_parsed_len) {
            tracing::debug!(
                from_root_suite_log_file_parsed_len,
                from_suite_log_file_parsed_len,
                vec_len,
                "Some tests were run in both the root and non-root test executions"
            );
        }

        vec.sort();

        anyhow::Result::<_>::Ok(vec)
    };

    let combined_error_vec = combined_sort_vec(|pa| &pa.error_hash_set)?;
    let combined_fail_vec = combined_sort_vec(|pa| &pa.fail_hash_set)?;
    let combined_skip_vec = combined_sort_vec(|pa| &pa.skip_hash_set)?;

    // Serialize output
    let outer_serialized = {
        let date_time_formatted = {
            const RFC_EMAIL: StrftimeItems<'_> = StrftimeItems::new("%a, %d %h %Y %T %z");

            let date_time = Local::now();

            date_time.format_with_items(RFC_EMAIL)
        };

        let inner = Inner {
            error: error.to_string(),
            fail: fail.to_string(),
            pass: pass.to_string(),
            sha: sha_hash_from_ci,
            skip: skip.to_string(),
            total: total.to_string(),
            xpass: xpass.to_string(),
            // New fields:
            error_list: Some(combined_error_vec),
            fail_list: Some(combined_fail_vec),
            skip_list: Some(combined_skip_vec),
        };

        let mut outer = serde_json::Map::with_capacity(1);

        let existing_value = outer.insert(
            date_time_formatted.to_string(),
            serde_json::to_value(&inner)?,
        );

        anyhow::ensure!(existing_value.is_none());

        serde_json::to_vec(&outer)?
    };

    // Write output
    {
        let output_file_path_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output_file_path)?;

        let mut buf_writer = BufWriter::new(output_file_path_file);

        buf_writer.write_all(outer_serialized.as_slice())?;
    }

    // Write output hash
    {
        anyhow::ensure!(hash_output_file_path.exists());

        let mut hash_output_file_path_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(hash_output_file_path)?;

        anyhow::ensure!(hash_output_file_path_file.metadata()?.len() == 0);

        let sha1_hash_hex = get_sha1_hash_hex(outer_serialized.as_slice());

        hash_output_file_path_file.write_all(sha1_hash_hex.as_bytes())?;
    }

    Ok(ExitCode::SUCCESS)
}

fn compare_result(compare_result_arguments: CompareResultArguments) -> anyhow::Result<ExitCode> {
    let CompareResultArguments {
        ignore_intermittent_tests_file,
        new_summary_file,
        ref_summary_file,
    } = compare_result_arguments;

    let ignore_intermittent_tests_file_path = ignore_intermittent_tests_file.as_path();
    let new_summary_file_path = new_summary_file.as_path();
    let ref_summary_file_path = ref_summary_file.as_path();

    let mut stdout_lock = io::stdout().lock();

    if !ref_summary_file.exists() {
        ci_warning(
            &mut stdout_lock,
            format_args!(
                "Skipping test summary comparison; no prior reference summary is available."
            ),
        )?;

        return Ok(ExitCode::SUCCESS);
    }

    let ref_summary_file_string = fs::read_to_string(ref_summary_file_path)?;

    // Print hash
    {
        let sha1_hash_hex = get_sha1_hash_hex(ref_summary_file_string.as_bytes());

        writeln!(
            &mut stdout_lock,
            "Reference SHA1/ID: {sha1_hash_hex}  {}",
            ref_summary_file_path.display()
        )?;
    }

    let var_string: String;

    let repo_default_branch_or_main = match env::var("REPO_DEFAULT_BRANCH") {
        Ok(string) => {
            var_string = string;

            var_string.as_str()
        }
        Err(var_error) => match var_error {
            VarError::NotPresent => "main",
            VarError::NotUnicode(_) => {
                anyhow::bail!("Unreachable");
            }
        },
    };

    fn parse_result_file(path: impl Read) -> anyhow::Result<Inner> {
        let mut value = serde_json::from_reader::<_, Value>(path)?;

        let value_object = value
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("`Value` is not an object"))?;

        let (_, inner_root_value_reference) = value_object
            .iter_mut()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not get inner root of result file"))?;

        let inner_root_value = inner_root_value_reference.take();

        let inner = serde_json::from_value::<Inner>(inner_root_value)?;

        Ok(inner)
    }

    let new_summary_file_file = File::open(new_summary_file_path)?;

    let new_summary = parse_result_file(BufReader::new(new_summary_file_file))?;
    let ref_summary = parse_result_file(Cursor::new(ref_summary_file_string))?;

    let error_delta = new_summary.error.parse::<i64>()? - ref_summary.error.parse::<i64>()?;
    let fail_delta = new_summary.fail.parse::<i64>()? - ref_summary.fail.parse::<i64>()?;
    let pass_delta = new_summary.pass.parse::<i64>()? - ref_summary.pass.parse::<i64>()?;
    let skip_delta = new_summary.skip.parse::<i64>()? - ref_summary.skip.parse::<i64>()?;

    let ignore_intermittent_tests_string = fs::read_to_string(ignore_intermittent_tests_file_path)?;

    let mut ignore_intermittent_tests_hash_set = HashSet::<String>::new();

    for st in ignore_intermittent_tests_string.lines() {
        let value_did_not_already_exist = ignore_intermittent_tests_hash_set.insert(st.to_owned());

        anyhow::ensure!(value_did_not_already_exist);
    }

    ci_warning(&mut stdout_lock, format_args!("Changes from '{repo_default_branch_or_main}': PASS {pass_delta:+} / FAIL {fail_delta:+} / ERROR {error_delta:+} / SKIP {skip_delta:+}"))?;

    let exit_with_failure = match (
        new_summary.error_list,
        new_summary.fail_list,
        new_summary.skip_list,
        ref_summary.error_list,
        ref_summary.fail_list,
        ref_summary.skip_list,
    ) {
        (
            Some(new_error_vec),
            Some(new_fail_vec),
            Some(new_skip_vec),
            Some(ref_error_vec),
            Some(ref_fail_vec),
            Some(ref_skip_vec),
        ) => {
            let new_error_hash_set = HashSet::<_>::from_iter(new_error_vec);
            let new_fail_hash_set = HashSet::<_>::from_iter(new_fail_vec);
            let new_skip_hash_set = HashSet::<_>::from_iter(new_skip_vec);

            let ref_error_hash_set = HashSet::<_>::from_iter(ref_error_vec);
            let ref_fail_hash_set = HashSet::<_>::from_iter(ref_fail_vec);
            let ref_skip_hash_set = HashSet::<_>::from_iter(ref_skip_vec);

            // For now, just naively combine all non-PASS types
            let mut new_all_hash_set = new_error_hash_set.clone();
            new_all_hash_set.extend(new_fail_hash_set);
            new_all_hash_set.extend(new_skip_hash_set);

            let mut ref_all_hash_set = ref_error_hash_set.clone();
            ref_all_hash_set.extend(ref_fail_hash_set);
            ref_all_hash_set.extend(ref_skip_hash_set);

            let mut added_non_pass_count = 0_usize;

            for st in new_all_hash_set.difference(&ref_all_hash_set) {
                if ignore_intermittent_tests_hash_set.contains(st) {
                    ci_warning(
                            &mut stdout_lock,
                            format_args!(
                                "Ignored intermittent test '{st}' completed with non-PASS status, but is not being considered in the overall comparison result"
                            ),
                        )?;
                } else {
                    ci_error(
                        &mut stdout_lock,
                        format_args!(
                            "Test '{st}' completed now with non-PASS status, but passed in the previous recorded test run"
                        ),
                    )?;

                    added_non_pass_count += 1;
                }
            }

            if added_non_pass_count > 0 {
                ci_error(
                    &mut stdout_lock,
                    format_args!("Non-PASS count is increased from '{repo_default_branch_or_main}' (not including ignored intermittent tests): +{added_non_pass_count}"),
                )?;

                true
            } else {
                false
            }
        }
        _ => {
            // Fall back to the old method
            ci_warning(
                &mut stdout_lock,
                format_args!("Unable to use new comparison method, falling back to the old method"),
            )?;

            if pass_delta.is_negative() {
                ci_error(
                    &mut stdout_lock,
                    format_args!(
                        "PASS count is reduced from '{repo_default_branch_or_main}': PASS -{}",
                        pass_delta.abs()
                    ),
                )?;

                true
            } else {
                false
            }
        }
    };

    let exit_code = if exit_with_failure {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    };

    Ok(exit_code)
}

fn extract_data_from_test_logs(
    extract_data_from_test_logs_arguments: ExtractDataFromTestLogsArguments,
) -> anyhow::Result<ExitCode> {
    // Some .log files are malformed, so relax the regular expression:

    /*
    diff -u /dev/null err
    --- /dev/null	1970-01-01
    +++ err	1970-01-01
    +nice: warning: setpriority: Permission denied (os error 13)PASS tests/nice/nice.sh (exit status: 0)
    */

    let regex = Regex::new(r"(ERROR|FAIL|PASS|SKIP) [^ ]+ \(exit status: [0-9]+\)\n?$")?;

    let ExtractDataFromTestLogsArguments {
        output_path,
        mut test_directory_path,
    } = extract_data_from_test_logs_arguments;

    if !test_directory_path.exists() {
        anyhow::bail!(
            "Path '{}' (argument '--test-directory-path') does not exist",
            test_directory_path.display()
        );
    }

    // Ensure it ends in a /
    test_directory_path.push("");

    let test_directory_path_with_trailing_slash = test_directory_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

    let pattern = format!("{test_directory_path_with_trailing_slash}*/**/*.log");

    let mut b_tree_map = BTreeMap::<String, BTreeMap<String, String>>::new();

    let mut buffer = Vec::<u8>::with_capacity(4_096);

    for result in glob::glob(pattern.as_str())? {
        let path_buf = result?;

        let path_buf_path = path_buf.as_path();

        let path = path_buf.strip_prefix(test_directory_path_with_trailing_slash)?;

        let mut path_components = path.components();

        let first = path_components
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not get first `Component` of `&Path`"))?;

        let first_str = component_to_str(&first)?;

        let last = path_components
            .last()
            .ok_or_else(|| anyhow::anyhow!("Could not get last `Component` of `&Path`"))?;

        let last_str = component_to_str(&last)?;

        let second_level = b_tree_map.entry(first_str.to_owned()).or_default();

        let path_buf_path_file = File::open(path_buf_path)?;

        // Some of these files have non-UTF-8 bytes
        let mut buf_reader = BufReader::new(path_buf_path_file);

        let mut found_matching_line = false;

        loop {
            buffer.clear();

            match buf_reader.read_until(b'\n', &mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }

                    if let Some(ca) = regex.captures(buffer.as_slice()) {
                        anyhow::ensure!(!found_matching_line);

                        found_matching_line = true;

                        let capture_group_one = ca.get(1).ok_or_else(|| {
                            anyhow::anyhow!("Could not get capture group with index 1")
                        })?;

                        // This part needs to be UTF-8
                        let capture_group_one_str = str::from_utf8(capture_group_one.as_bytes())?;

                        let existing_value = second_level
                            .insert(last_str.to_owned(), capture_group_one_str.to_owned());

                        anyhow::ensure!(existing_value.is_none());
                    }
                }
                Err(er) => {
                    if er.kind() == ErrorKind::Interrupted {
                        continue;
                    }

                    anyhow::bail!(er);
                }
            }
        }
    }

    let mut b_tree_map_is_empty = true;

    for bt in b_tree_map.values() {
        if bt.values().next().is_some() {
            b_tree_map_is_empty = false;

            break;
        }
    }

    anyhow::ensure!(!b_tree_map_is_empty);

    let output_path_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output_path.as_path())?;

    let mut output_path_file_buf_writer = BufWriter::new(output_path_file);

    serde_json::to_writer(&mut output_path_file_buf_writer, &b_tree_map)?;

    Ok(ExitCode::SUCCESS)
}

/* #region Helper functions */
fn component_to_str<'a>(component: &'a Component) -> anyhow::Result<&'a str> {
    match component {
        Component::Normal(os) => os
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Could not convert `OsStr` to `&str`")),
        _ => {
            anyhow::bail!("Unexpected `Component` type found")
        }
    }
}

fn get_sha1_hash_hex(bytes: &[u8]) -> String {
    let mut core_wrapper = Sha1::new();

    core_wrapper.update(bytes);

    let finalize_result = core_wrapper.finalize();

    const_hex::encode(finalize_result.as_slice())
}

fn ci_error(stdout_lock: &mut StdoutLock, arguments: std::fmt::Arguments) -> anyhow::Result<()> {
    // Special syntax: https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions#setting-a-warning-message
    writeln!(stdout_lock, "::error ::{arguments}")?;

    Ok(())
}

fn ci_warning(stdout_lock: &mut StdoutLock, arguments: std::fmt::Arguments) -> anyhow::Result<()> {
    // Special syntax: https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions#setting-a-warning-message
    writeln!(stdout_lock, "::warning ::{arguments}")?;

    Ok(())
}
/* #endregion */

mod types {
    use serde_derive::{Deserialize, Serialize};

    // The result files look like:

    /*
    {
      "Thu, 24 Oct 2024 18:38:26 +0000": {
        "sha": "91435d6785a680961bd3d1da58c6323bd187324c",
        "total": "613",
        "pass": "476",
        "skip": "43",
        "fail": "94",
        "xpass": "0",
        "error": "0"
      }
    }
    */

    // Since the outer key changes, the outer level has to be parsed manually

    #[derive(Serialize, Deserialize)]
    pub struct Inner {
        pub sha: String,
        pub total: String,
        pub pass: String,
        pub skip: String,
        pub fail: String,
        pub xpass: String,
        pub error: String,
        // New fields:
        pub error_list: Option<Vec<String>>,
        pub fail_list: Option<Vec<String>>,
        pub skip_list: Option<Vec<String>>,
    }
}
