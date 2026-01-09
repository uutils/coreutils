#!/usr/bin/env python3
"""
Unit tests for the GNU test results comparison script.
"""

import unittest
import json
import tempfile
import os
from unittest.mock import patch
from io import StringIO
from util.compare_test_results import (
    flatten_test_results,
    load_ignore_list,
    identify_test_changes,
    main,
)


class TestFlattenTestResults(unittest.TestCase):
    """Tests for the flatten_test_results function."""

    def test_basic_flattening(self):
        """Test basic flattening of nested test results."""
        test_data = {
            "ls": {"test1": "PASS", "test2": "FAIL"},
            "cp": {"test3": "SKIP", "test4": "ERROR"},
        }
        expected = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "FAIL",
            "tests/cp/test3": "SKIP",
            "tests/cp/test4": "ERROR",
        }
        self.assertEqual(flatten_test_results(test_data), expected)

    def test_empty_dict(self):
        """Test flattening an empty dictionary."""
        self.assertEqual(flatten_test_results({}), {})

    def test_single_util(self):
        """Test flattening results with a single utility."""
        test_data = {"ls": {"test1": "PASS", "test2": "FAIL"}}
        expected = {"tests/ls/test1": "PASS", "tests/ls/test2": "FAIL"}
        self.assertEqual(flatten_test_results(test_data), expected)

    def test_empty_tests(self):
        """Test flattening with a utility that has no tests."""
        test_data = {"ls": {}, "cp": {"test1": "PASS"}}
        expected = {"tests/cp/test1": "PASS"}
        self.assertEqual(flatten_test_results(test_data), expected)

    def test_log_extension_removal(self):
        """Test that .log extensions are removed."""
        test_data = {"ls": {"test1.log": "PASS", "test2": "FAIL"}}
        expected = {"tests/ls/test1": "PASS", "tests/ls/test2": "FAIL"}
        self.assertEqual(flatten_test_results(test_data), expected)


class TestLoadIgnoreList(unittest.TestCase):
    """Tests for the load_ignore_list function."""

    def test_load_ignores(self):
        """Test loading ignore list from a file."""
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp.write(
                "tests/tail/inotify-dir-recreate\ntests/timeout/timeout\ntests/rm/rm1\n"
            )
            tmp_path = tmp.name
        try:
            ignore_list = load_ignore_list(tmp_path)
            self.assertEqual(
                ignore_list,
                {
                    "tests/tail/inotify-dir-recreate",
                    "tests/timeout/timeout",
                    "tests/rm/rm1",
                },
            )
        finally:
            os.unlink(tmp_path)

    def test_empty_file(self):
        """Test loading an empty ignore file."""
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp_path = tmp.name
        try:
            ignore_list = load_ignore_list(tmp_path)
            self.assertEqual(ignore_list, set())
        finally:
            os.unlink(tmp_path)

    def test_with_comments_and_blanks(self):
        """Test loading ignore file with comments and blank lines."""
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp.write(
                "tests/tail/inotify-dir-recreate\n# A comment\n\ntests/timeout/timeout\n#Indented comment\n  tests/rm/rm1  \n"
            )
            tmp_path = tmp.name
        try:
            ignore_list = load_ignore_list(tmp_path)
            self.assertEqual(
                ignore_list,
                {
                    "tests/tail/inotify-dir-recreate",
                    "tests/timeout/timeout",
                    "tests/rm/rm1",
                },
            )
        finally:
            os.unlink(tmp_path)

    def test_nonexistent_file(self):
        """Test behavior with a nonexistent file."""
        result = load_ignore_list("/nonexistent/file/path")
        self.assertEqual(result, set())


class TestIdentifyTestChanges(unittest.TestCase):
    """Tests for the identify_test_changes function."""

    def test_regressions(self):
        """Test identifying regressions."""
        current = {
            "tests/ls/test1": "FAIL",
            "tests/ls/test2": "ERROR",
            "tests/cp/test3": "PASS",
            "tests/cp/test4": "SKIP",
        }
        reference = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "PASS",
            "tests/cp/test3": "PASS",
            "tests/cp/test4": "FAIL",
        }
        regressions, _, _, _, _ = identify_test_changes(current, reference)
        self.assertEqual(sorted(regressions), ["tests/ls/test1", "tests/ls/test2"])

    def test_fixes(self):
        """Test identifying fixes."""
        current = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "PASS",
            "tests/cp/test3": "FAIL",
            "tests/cp/test4": "SKIP",
        }
        reference = {
            "tests/ls/test1": "FAIL",
            "tests/ls/test2": "ERROR",
            "tests/cp/test3": "PASS",
            "tests/cp/test4": "FAIL",
        }
        _, fixes, _, _, _ = identify_test_changes(current, reference)
        self.assertEqual(sorted(fixes), ["tests/ls/test1", "tests/ls/test2"])

    def test_newly_skipped(self):
        """Test identifying newly skipped tests."""
        current = {
            "tests/ls/test1": "SKIP",
            "tests/ls/test2": "SKIP",
            "tests/cp/test3": "PASS",
        }
        reference = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "FAIL",
            "tests/cp/test3": "PASS",
        }
        _, _, newly_skipped, _, _ = identify_test_changes(current, reference)
        self.assertEqual(newly_skipped, ["tests/ls/test1"])

    def test_newly_passing(self):
        """Test identifying newly passing tests."""
        current = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "PASS",
            "tests/cp/test3": "SKIP",
        }
        reference = {
            "tests/ls/test1": "SKIP",
            "tests/ls/test2": "FAIL",
            "tests/cp/test3": "SKIP",
        }
        _, _, _, newly_passing, _ = identify_test_changes(current, reference)
        self.assertEqual(newly_passing, ["tests/ls/test1"])

    def test_all_categories(self):
        """Test identifying all categories of changes simultaneously."""
        current = {
            "tests/ls/test1": "FAIL",  # Regression
            "tests/ls/test2": "PASS",  # Fix
            "tests/cp/test3": "SKIP",  # Newly skipped
            "tests/cp/test4": "PASS",  # Newly passing
            "tests/rm/test5": "PASS",  # No change
            "tests/rm/test6": "FAIL",  # Newly failing
        }
        reference = {
            "tests/ls/test1": "PASS",  # Regression
            "tests/ls/test2": "FAIL",  # Fix
            "tests/cp/test3": "PASS",  # Newly skipped
            "tests/cp/test4": "SKIP",  # Newly passing
            "tests/rm/test5": "PASS",  # No change
            "tests/rm/test6": "SKIP",  # Newly failing
        }
        regressions, fixes, newly_skipped, newly_passing, newly_failing = (
            identify_test_changes(current, reference)
        )
        self.assertEqual(regressions, ["tests/ls/test1"])
        self.assertEqual(fixes, ["tests/ls/test2"])
        self.assertEqual(newly_skipped, ["tests/cp/test3"])
        self.assertEqual(newly_passing, ["tests/cp/test4"])
        self.assertEqual(newly_failing, ["tests/rm/test6"])

    def test_new_and_removed_tests(self):
        """Test handling of tests that are only in one of the datasets."""
        current = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "FAIL",
            "tests/cp/new_test": "PASS",
        }
        reference = {
            "tests/ls/test1": "PASS",
            "tests/ls/test2": "PASS",
            "tests/rm/old_test": "FAIL",
        }
        regressions, fixes, newly_skipped, newly_passing, newly_failing = (
            identify_test_changes(current, reference)
        )
        self.assertEqual(regressions, ["tests/ls/test2"])
        self.assertEqual(fixes, [])
        self.assertEqual(newly_skipped, [])
        self.assertEqual(newly_passing, [])
        self.assertEqual(newly_failing, [])

    def test_newly_failing(self):
        """Test identifying newly failing tests (SKIP -> FAIL)."""
        current = {
            "tests/ls/test1": "FAIL",
            "tests/ls/test2": "ERROR",
            "tests/cp/test3": "PASS",
        }
        reference = {
            "tests/ls/test1": "SKIP",
            "tests/ls/test2": "SKIP",
            "tests/cp/test3": "SKIP",
        }
        _, _, _, _, newly_failing = identify_test_changes(current, reference)
        self.assertEqual(sorted(newly_failing), ["tests/ls/test1", "tests/ls/test2"])

    def test_skip_to_fail_not_regression(self):
        """Test that SKIP -> FAIL is not counted as a regression."""
        current = {
            "tests/ls/test1": "FAIL",
            "tests/ls/test2": "FAIL",
        }
        reference = {
            "tests/ls/test1": "SKIP",
            "tests/ls/test2": "PASS",
        }
        regressions, _, _, _, newly_failing = identify_test_changes(current, reference)
        self.assertEqual(regressions, ["tests/ls/test2"])
        self.assertEqual(newly_failing, ["tests/ls/test1"])


class TestMainFunction(unittest.TestCase):
    """Integration tests for the main function."""

    def setUp(self):
        """Set up test files needed for main function tests."""
        self.current_data = {
            "ls": {
                "test1": "PASS",
                "test2": "FAIL",
                "test3": "PASS",
                "test4": "SKIP",
                "test5": "PASS",
            },
            "cp": {"test1": "PASS", "test2": "PASS"},
        }
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            json.dump(self.current_data, tmp)
            self.current_json = tmp.name

        self.reference_data = {
            "ls": {
                "test1": "PASS",
                "test2": "PASS",
                "test3": "FAIL",
                "test4": "PASS",
                "test5": "SKIP",
            },
            "cp": {"test1": "FAIL", "test2": "ERROR"},
        }
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            json.dump(self.reference_data, tmp)
            self.reference_json = tmp.name

        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp.write("tests/ls/test2\ntests/cp/test1\n")
            self.ignore_file = tmp.name

        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            self.output_file = tmp.name

    def tearDown(self):
        """Clean up test files after tests."""
        for file_path in [
            self.current_json,
            self.reference_json,
            self.ignore_file,
            self.output_file,
        ]:
            if os.path.exists(file_path):
                os.unlink(file_path)

    def test_main_exit_code_with_real_regressions(self):
        """Test main function exit code with real regressions."""

        current_flat = flatten_test_results(self.current_data)
        reference_flat = flatten_test_results(self.reference_data)

        regressions, _, _, _, _ = identify_test_changes(current_flat, reference_flat)

        self.assertIn("tests/ls/test2", regressions)

        ignore_list = load_ignore_list(self.ignore_file)

        real_regressions = [r for r in regressions if r not in ignore_list]

        self.assertNotIn("tests/ls/test2", real_regressions)

        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp.write(
                "tests/cp/test1\n"
            )  # only ignore tests/cp/test1, not tests/ls/test2
            new_ignore_file = tmp.name

        try:
            new_ignore_list = load_ignore_list(new_ignore_file)

            new_real_regressions = [r for r in regressions if r not in new_ignore_list]

            # tests/ls/test2 should now be in real_regressions
            self.assertIn("tests/ls/test2", new_real_regressions)

            # In main(), this would cause a non-zero exit code
            would_exit_with_error = len(new_real_regressions) > 0
            self.assertTrue(would_exit_with_error)
        finally:
            os.unlink(new_ignore_file)

    def test_filter_intermittent_fixes(self):
        """Test that fixes in the ignore list are filtered properly."""
        current_flat = flatten_test_results(self.current_data)
        reference_flat = flatten_test_results(self.reference_data)

        _, fixes, _, _, _ = identify_test_changes(current_flat, reference_flat)

        # tests/cp/test1 and tests/cp/test2 should be fixed but tests/cp/test1 is in ignore list
        self.assertIn("tests/cp/test1", fixes)
        self.assertIn("tests/cp/test2", fixes)

        ignore_list = load_ignore_list(self.ignore_file)
        real_fixes = [f for f in fixes if f not in ignore_list]
        intermittent_fixes = [f for f in fixes if f in ignore_list]

        # tests/cp/test1 should be identified as intermittent
        self.assertIn("tests/cp/test1", intermittent_fixes)
        # tests/cp/test2 should be identified as a real fix
        self.assertIn("tests/cp/test2", real_fixes)


class TestOutputFunctionality(unittest.TestCase):
    """Tests focused on the output generation of the script."""

    def setUp(self):
        """Set up test files needed for output tests."""
        self.current_data = {
            "ls": {
                "test1": "PASS",
                "test2": "FAIL",  # Regression but in ignore list
                "test3": "PASS",  # Fix
            },
            "cp": {
                "test1": "PASS",  # Fix but in ignore list
                "test2": "SKIP",  # Newly skipped
                "test4": "PASS",  # Newly passing
            },
        }
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            json.dump(self.current_data, tmp)
            self.current_json = tmp.name

        self.reference_data = {
            "ls": {
                "test1": "PASS",  # No change
                "test2": "PASS",  # Regression but in ignore list
                "test3": "FAIL",  # Fix
            },
            "cp": {
                "test1": "FAIL",  # Fix but in ignore list
                "test2": "PASS",  # Newly skipped
                "test4": "SKIP",  # Newly passing
            },
        }
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            json.dump(self.reference_data, tmp)
            self.reference_json = tmp.name

        # Create an ignore file
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp.write("tests/ls/test2\ntests/cp/test1\n")
            self.ignore_file = tmp.name

    def tearDown(self):
        """Clean up test files after tests."""
        for file_path in [self.current_json, self.reference_json, self.ignore_file]:
            if os.path.exists(file_path):
                os.unlink(file_path)

        if hasattr(self, "output_file") and os.path.exists(self.output_file):
            os.unlink(self.output_file)

    @patch("sys.stdout", new_callable=StringIO)
    @patch("sys.stderr", new_callable=StringIO)
    def test_console_output_formatting(self, mock_stderr, mock_stdout):
        """Test the formatting of console output."""
        with patch(
            "sys.argv",
            [
                "compare_test_results.py",
                self.current_json,
                self.reference_json,
                "--ignore-file",
                self.ignore_file,
            ],
        ):
            try:
                main()
            except SystemExit:
                pass  # Expected to exit with a status code

        stdout_content = mock_stdout.getvalue()
        self.assertIn("Total tests in current run:", stdout_content)
        self.assertIn("New regressions: 0", stdout_content)
        self.assertIn("Intermittent regressions: 1", stdout_content)
        self.assertIn("Fixed tests: 1", stdout_content)
        self.assertIn("Intermittent fixes: 1", stdout_content)
        self.assertIn("Newly skipped tests: 1", stdout_content)
        self.assertIn("Newly passing tests (previously skipped): 1", stdout_content)

        stderr_content = mock_stderr.getvalue()
        self.assertIn("INTERMITTENT ISSUES (ignored regressions):", stderr_content)
        self.assertIn("Skip an intermittent issue tests/ls/test2", stderr_content)
        self.assertIn("INTERMITTENT ISSUES (ignored fixes):", stderr_content)
        self.assertIn("Skipping an intermittent issue tests/cp/test1", stderr_content)
        self.assertIn("FIXED TESTS:", stderr_content)
        self.assertIn(
            "Congrats! The gnu test tests/ls/test3 is no longer failing!",
            stderr_content,
        )
        self.assertIn("NEWLY SKIPPED TESTS:", stderr_content)
        self.assertIn("Note: The gnu test tests/cp/test2", stderr_content)
        self.assertIn("NEWLY PASSING TESTS (previously skipped):", stderr_content)
        self.assertIn(
            "Congrats! The gnu test tests/cp/test4 is now passing!", stderr_content
        )

    def test_file_output_generation(self):
        """Test that the output file is generated correctly."""
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            self.output_file = tmp.name

        with patch(
            "sys.argv",
            [
                "compare_test_results.py",
                self.current_json,
                self.reference_json,
                "--ignore-file",
                self.ignore_file,
                "--output",
                self.output_file,
            ],
        ):
            try:
                main()
            except SystemExit:
                pass  # Expected to exit with a status code

        self.assertTrue(os.path.exists(self.output_file))

        with open(self.output_file, "r") as f:
            output_content = f.read()

        self.assertIn("Skip an intermittent issue tests/ls/test2", output_content)
        self.assertIn("Skipping an intermittent issue tests/cp/test1", output_content)
        self.assertIn(
            "Congrats! The gnu test tests/ls/test3 is no longer failing!",
            output_content,
        )
        self.assertIn("Note: The gnu test tests/cp/test2", output_content)
        self.assertIn(
            "Congrats! The gnu test tests/cp/test4 is now passing!", output_content
        )

    def test_exit_code_with_no_regressions(self):
        """Test that the script exits with code 0 when there are no regressions."""
        with patch(
            "sys.argv",
            [
                "compare_test_results.py",
                self.current_json,
                self.reference_json,
                "--ignore-file",
                self.ignore_file,
            ],
        ):
            # Instead of assertRaises, just call main() and check its return value
            exit_code = main()
            # Since all regressions are in the ignore list, should exit with 0
            self.assertEqual(exit_code, 0)

    def test_exit_code_with_regressions(self):
        """Test that the script exits with code 1 when there are real regressions."""
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as tmp:
            tmp.write("tests/cp/test1\n")  # Only ignore cp/test1
            new_ignore_file = tmp.name

        try:
            with patch(
                "sys.argv",
                [
                    "compare_test_results.py",
                    self.current_json,
                    self.reference_json,
                    "--ignore-file",
                    new_ignore_file,
                ],
            ):
                # Just call main() and check its return value
                exit_code = main()
                # Since ls/test2 is now a real regression, should exit with 1
                self.assertEqual(exit_code, 1)
        finally:
            os.unlink(new_ignore_file)

    def test_github_actions_formatting(self):
        """Test that the output is formatted for GitHub Actions."""
        with patch("sys.stderr", new_callable=StringIO) as mock_stderr:
            with patch(
                "sys.argv",
                [
                    "compare_test_results.py",
                    self.current_json,
                    self.reference_json,
                    "--ignore-file",
                    self.ignore_file,
                ],
            ):
                try:
                    main()
                except SystemExit:
                    pass  # Expected to exit with a status code

            stderr_content = mock_stderr.getvalue()

            self.assertIn(
                "::notice ::", stderr_content
            )  # For fixes and informational messages
            self.assertIn("::warning ::", stderr_content)  # For newly skipped tests


if __name__ == "__main__":
    unittest.main()
