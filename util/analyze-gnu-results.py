#!/usr/bin/env python3
import json
import sys


def analyze_test_results(json_data):
    # Counters for test results
    total_tests = 0
    pass_count = 0
    fail_count = 0
    skip_count = 0
    error_count = 0  # Although not in the JSON, included for compatibility

    # Analyze each utility's tests
    for utility, tests in json_data.items():
        for test_name, result in tests.items():
            total_tests += 1

            if result == "PASS":
                pass_count += 1
            elif result == "FAIL":
                fail_count += 1
            elif result == "SKIP":
                skip_count += 1

    # Return the statistics
    return {
        "TOTAL": total_tests,
        "PASS": pass_count,
        "FAIL": fail_count,
        "SKIP": skip_count,
        "ERROR": error_count,
    }


def main():
    # Check if a file argument was provided
    if len(sys.argv) != 2:
        print("Usage: python script.py <json_file>")
        sys.exit(1)

    json_file = sys.argv[1]

    try:
        # Parse the JSON data from the specified file
        with open(json_file, "r") as file:
            json_data = json.load(file)

        # Analyze the results
        results = analyze_test_results(json_data)

        # Export the results as environment variables
        # For use in shell, print export statements
        print(f"export TOTAL={results['TOTAL']}")
        print(f"export PASS={results['PASS']}")
        print(f"export SKIP={results['SKIP']}")
        print(f"export FAIL={results['FAIL']}")
        print(f"export ERROR={results['ERROR']}")

    except FileNotFoundError:
        print(f"Error: File '{json_file}' not found.", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Error: '{json_file}' is not a valid JSON", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
