#!/usr/bin/env python3

"""
GNU Test Results Analyzer and Aggregator

This script analyzes and aggregates test results from the GNU test suite.
It parses JSON files containing test results (PASS/FAIL/SKIP/ERROR) and:
1. Counts the number of tests in each result category
2. Can aggregate results from multiple JSON files with priority ordering
3. Outputs shell export statements for use in GitHub Actions workflows

Priority order for aggregation (highest to lowest):
- PASS: Takes precedence over all other results (best outcome)
- FAIL: Takes precedence over ERROR and SKIP
- ERROR: Takes precedence over SKIP
- SKIP: Lowest priority

Usage:
  - Single file:
    python analyze-gnu-results.py test-results.json

  - Multiple files (with aggregation):
    python analyze-gnu-results.py file1.json file2.json

  - With output file for aggregated results:
    python analyze-gnu-results.py -o=output.json file1.json file2.json

Output:
  Prints shell export statements for TOTAL, PASS, FAIL, SKIP, XPASS, and ERROR
  that can be evaluated in a shell environment.
"""

import json
import sys
from collections import defaultdict


def get_priority(result):
    """Return a priority value for result status (lower is higher priority)"""
    priorities = {
        "PASS": 0,  # PASS is highest priority (best result)
        "FAIL": 1,  # FAIL is second priority
        "ERROR": 2,  # ERROR is third priority
        "SKIP": 3,  # SKIP is lowest priority
    }
    return priorities.get(result, 4)  # Unknown states have lowest priority


def aggregate_results(json_files):
    """
    Aggregate test results from multiple JSON files.
    Prioritizes results in the order: SKIP > ERROR > FAIL > PASS
    """
    # Combined results dictionary
    combined_results = defaultdict(dict)

    # Process each JSON file
    for json_file in json_files:
        try:
            with open(json_file, "r") as f:
                data = json.load(f)

            # For each utility and its tests
            for utility, tests in data.items():
                for test_name, result in tests.items():
                    # If this test hasn't been seen yet, add it
                    if test_name not in combined_results[utility]:
                        combined_results[utility][test_name] = result
                    else:
                        # If it has been seen, apply priority rules
                        current_priority = get_priority(
                            combined_results[utility][test_name]
                        )
                        new_priority = get_priority(result)

                        # Lower priority value means higher precedence
                        if new_priority < current_priority:
                            combined_results[utility][test_name] = result
        except FileNotFoundError:
            print(f"Warning: File '{json_file}' not found.", file=sys.stderr)
            continue
        except json.JSONDecodeError:
            print(f"Warning: '{json_file}' is not a valid JSON file.", file=sys.stderr)
            continue

    return combined_results


def analyze_test_results(json_data):
    """
    Analyze test results from GNU test suite JSON data.
    Counts PASS, FAIL, SKIP results for all tests.
    """
    # Counters for test results
    total_tests = 0
    pass_count = 0
    fail_count = 0
    skip_count = 0
    xpass_count = 0  # Not in JSON data but included for compatibility
    error_count = 0  # Not in JSON data but included for compatibility

    # Analyze each utility's tests
    for utility, tests in json_data.items():
        for test_name, result in tests.items():
            total_tests += 1

            match result:
                case "PASS":
                    pass_count += 1
                case "FAIL":
                    fail_count += 1
                case "SKIP":
                    skip_count += 1
                case "ERROR":
                    error_count += 1
                case "XPASS":
                    xpass_count += 1

    # Return the statistics
    return {
        "TOTAL": total_tests,
        "PASS": pass_count,
        "FAIL": fail_count,
        "SKIP": skip_count,
        "XPASS": xpass_count,
        "ERROR": error_count,
    }


def main():
    """
    Main function to process JSON files and export variables.
    Supports both single file analysis and multi-file aggregation.
    """
    # Check if file arguments were provided
    if len(sys.argv) < 2:
        print("Usage: python analyze-gnu-results.py <json> [json ...]")
        print("       For multiple files, results will be aggregated")
        print("       Priority SKIP > ERROR > FAIL > PASS")
        sys.exit(1)

    json_files = sys.argv[1:]
    output_file = None

    # Check if the first argument is an output file (starts with -)
    if json_files[0].startswith("-o="):
        output_file = json_files[0][3:]
        json_files = json_files[1:]

    # Process the files
    if len(json_files) == 1:
        # Single file analysis
        try:
            with open(json_files[0], "r") as file:
                json_data = json.load(file)
            results = analyze_test_results(json_data)
        except FileNotFoundError:
            print(f"Error: File '{json_files[0]}' not found.", file=sys.stderr)
            sys.exit(1)
        except json.JSONDecodeError:
            print(
                f"Error: '{json_files[0]}' is not a valid JSON file.", file=sys.stderr
            )
            sys.exit(1)
    else:
        # Multiple files - aggregate them
        json_data = aggregate_results(json_files)
        results = analyze_test_results(json_data)

        # Save aggregated data if output file is specified
        if output_file:
            with open(output_file, "w") as f:
                json.dump(json_data, f, indent=2)

    # Print export statements for shell evaluation
    print(f"export TOTAL={results['TOTAL']}")
    print(f"export PASS={results['PASS']}")
    print(f"export SKIP={results['SKIP']}")
    print(f"export FAIL={results['FAIL']}")
    print(f"export XPASS={results['XPASS']}")
    print(f"export ERROR={results['ERROR']}")


if __name__ == "__main__":
    main()
