#!/usr/bin/env python3
"""
Compare GNU test results between current run and reference to identify
regressions and fixes.


Arguments:
    CURRENT_JSON       Path to the current run's aggregated results JSON file
    REFERENCE_JSON     Path to the reference (main branch) aggregated
                        results JSON file
    --ignore-file      Path to file containing list of tests to ignore
                        (for intermittent issues)
    --output           Path to output file for GitHub comment content
"""

import argparse
import json
import os
import sys


def flatten_test_results(results):
    """Convert nested JSON test results to a flat dictionary of test paths to statuses."""
    flattened = {}
    for util, tests in results.items():
        for test_name, status in tests.items():
            # Build the full test path
            test_path = f"tests/{util}/{test_name}"
            # Remove the .log extension
            test_path = test_path.replace(".log", "")
            flattened[test_path] = status
    return flattened


def load_ignore_list(ignore_file):
    """Load list of tests to ignore from file."""
    if not os.path.exists(ignore_file):
        return set()

    with open(ignore_file, "r") as f:
        return {line.strip() for line in f if line.strip() and not line.startswith("#")}


def identify_test_changes(current_flat, reference_flat):
    """
    Identify different categories of test changes between current and reference results.

    Args:
        current_flat (dict): Flattened dictionary of current test results
        reference_flat (dict): Flattened dictionary of reference test results

    Returns:
        tuple: Four lists containing regressions, fixes, newly_skipped, and newly_passing tests
    """
    # Find regressions (tests that were passing but now failing)
    regressions = []
    for test_path, status in current_flat.items():
        if status in ("FAIL", "ERROR"):
            if test_path in reference_flat:
                if reference_flat[test_path] in ("PASS", "SKIP"):
                    regressions.append(test_path)

    # Find fixes (tests that were failing but now passing)
    fixes = []
    for test_path, status in reference_flat.items():
        if status in ("FAIL", "ERROR"):
            if test_path in current_flat:
                if current_flat[test_path] == "PASS":
                    fixes.append(test_path)

    # Find newly skipped tests (were passing, now skipped)
    newly_skipped = []
    for test_path, status in current_flat.items():
        if (
            status == "SKIP"
            and test_path in reference_flat
            and reference_flat[test_path] == "PASS"
        ):
            newly_skipped.append(test_path)

    # Find newly passing tests (were skipped, now passing)
    newly_passing = []
    for test_path, status in current_flat.items():
        if (
            status == "PASS"
            and test_path in reference_flat
            and reference_flat[test_path] == "SKIP"
        ):
            newly_passing.append(test_path)

    return regressions, fixes, newly_skipped, newly_passing


def main():
    parser = argparse.ArgumentParser(
        description="Compare GNU test results and identify regressions and fixes"
    )
    parser.add_argument("current_json", help="Path to current run JSON results")
    parser.add_argument("reference_json", help="Path to reference JSON results")
    parser.add_argument(
        "--ignore-file",
        required=True,
        help="Path to file with tests to ignore (for intermittent issues)",
    )
    parser.add_argument("--output", help="Path to output file for GitHub comment")

    args = parser.parse_args()

    # Load test results
    try:
        with open(args.current_json, "r") as f:
            current_results = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        sys.stderr.write(f"Error loading current results: {e}\n")
        return 1

    try:
        with open(args.reference_json, "r") as f:
            reference_results = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        sys.stderr.write(f"Error loading reference results: {e}\n")
        sys.stderr.write("Skipping comparison as reference is not available.\n")
        return 0

    # Load ignore list (required)
    if not os.path.exists(args.ignore_file):
        sys.stderr.write(f"Error: Ignore file {args.ignore_file} does not exist\n")
        return 1

    ignore_list = load_ignore_list(args.ignore_file)
    print(f"Loaded {len(ignore_list)} tests to ignore from {args.ignore_file}")

    # Flatten result structures for easier comparison
    current_flat = flatten_test_results(current_results)
    reference_flat = flatten_test_results(reference_results)

    # Identify different categories of test changes
    regressions, fixes, newly_skipped, newly_passing = identify_test_changes(
        current_flat, reference_flat
    )

    # Filter out intermittent issues from regressions
    real_regressions = [r for r in regressions if r not in ignore_list]
    intermittent_regressions = [r for r in regressions if r in ignore_list]

    # Filter out intermittent issues from fixes
    real_fixes = [f for f in fixes if f not in ignore_list]
    intermittent_fixes = [f for f in fixes if f in ignore_list]

    # Print summary stats
    print(f"Total tests in current run: {len(current_flat)}")
    print(f"Total tests in reference: {len(reference_flat)}")
    print(f"New regressions: {len(real_regressions)}")
    print(f"Intermittent regressions: {len(intermittent_regressions)}")
    print(f"Fixed tests: {len(real_fixes)}")
    print(f"Intermittent fixes: {len(intermittent_fixes)}")
    print(f"Newly skipped tests: {len(newly_skipped)}")
    print(f"Newly passing tests (previously skipped): {len(newly_passing)}")

    output_lines = []

    # Report regressions
    if real_regressions:
        print("\nREGRESSIONS (non-intermittent failures):", file=sys.stderr)
        for test in sorted(real_regressions):
            msg = f"GNU test failed: {test}. {test} is passing on 'main'. Maybe you have to rebase?"
            print(f"::error ::{msg}", file=sys.stderr)
            output_lines.append(msg)

    # Report intermittent issues (regressions)
    if intermittent_regressions:
        print("\nINTERMITTENT ISSUES (ignored regressions):", file=sys.stderr)
        for test in sorted(intermittent_regressions):
            msg = f"Skip an intermittent issue {test} (fails in this run but passes in the 'main' branch)"
            print(f"::notice ::{msg}", file=sys.stderr)
            output_lines.append(msg)

    # Report intermittent issues (fixes)
    if intermittent_fixes:
        print("\nINTERMITTENT ISSUES (ignored fixes):", file=sys.stderr)
        for test in sorted(intermittent_fixes):
            msg = f"Skipping an intermittent issue {test} (passes in this run but fails in the 'main' branch)"
            print(f"::notice ::{msg}", file=sys.stderr)
            output_lines.append(msg)

    # Report fixes
    if real_fixes:
        print("\nFIXED TESTS:", file=sys.stderr)
        for test in sorted(real_fixes):
            msg = f"Congrats! The gnu test {test} is no longer failing!"
            print(f"::notice ::{msg}", file=sys.stderr)
            output_lines.append(msg)

    # Report newly skipped and passing tests
    if newly_skipped:
        print("\nNEWLY SKIPPED TESTS:", file=sys.stderr)
        for test in sorted(newly_skipped):
            msg = f"Note: The gnu test {test} is now being skipped but was previously passing."
            print(f"::warning ::{msg}", file=sys.stderr)
            output_lines.append(msg)

    if newly_passing:
        print("\nNEWLY PASSING TESTS (previously skipped):", file=sys.stderr)
        for test in sorted(newly_passing):
            msg = f"Congrats! The gnu test {test} is now passing!"
            print(f"::notice ::{msg}", file=sys.stderr)
            output_lines.append(msg)

    if args.output and output_lines:
        with open(args.output, "w") as f:
            for line in output_lines:
                f.write(f"{line}\n")

    # Return exit code based on whether we found regressions
    return 1 if real_regressions else 0


if __name__ == "__main__":
    sys.exit(main())
