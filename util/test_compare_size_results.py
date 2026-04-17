#!/usr/bin/env python3
# spell-checker:ignore newbin oldbin
"""Unit tests for the per-binary size comparison script."""

import json
import os
import sys
import tempfile
import unittest

from util.compare_size_results import compare, format_report, human_kb, load_sizes, main


def _write_json(data):
    fd, path = tempfile.mkstemp(suffix=".json")
    with os.fdopen(fd, "w") as f:
        json.dump(data, f)
    return path


class TestCompareSizeResults(unittest.TestCase):
    def test_human_kb(self):
        self.assertEqual(human_kb(512), "512 KB")
        self.assertEqual(human_kb(1536), "1.50 MB")
        self.assertEqual(human_kb(1024 * 1024), "1.00 GB")
        self.assertEqual(human_kb(-2048), "-2.00 MB")

    def test_load_sizes_date_keyed(self):
        path = _write_json(
            {
                "Mon, 01 Jan 2024 00:00:00 +0000": {
                    "sha": "old",
                    "sizes": {"ls": 1000},
                },
                "Tue, 02 Jan 2024 00:00:00 +0000": {
                    "sha": "new",
                    "sizes": {"ls": 1100},
                },
            }
        )
        try:
            sha, sizes = load_sizes(path)
            self.assertEqual(sha, "new")
            self.assertEqual(sizes, {"ls": 1100})
        finally:
            os.unlink(path)

    def test_load_sizes_flat_fallback(self):
        path = _write_json({"ls": "1064"})
        try:
            sha, sizes = load_sizes(path)
            self.assertIsNone(sha)
            self.assertEqual(sizes, {"ls": 1064})
        finally:
            os.unlink(path)

    def test_compare_thresholds(self):
        # Both thresholds met -> significant (growth and shrinkage).
        sig, *_ = compare({"ls": 1100}, {"ls": 1000}, 0.05, 4)
        self.assertEqual(len(sig), 1)
        self.assertEqual(sig[0]["delta"], 100)

        sig, *_ = compare({"ls": 900}, {"ls": 1000}, 0.05, 4)
        self.assertEqual(sig[0]["delta"], -100)

        # Only relative met (10% but 2 KB) -> not significant.
        sig, *_ = compare({"t": 22}, {"t": 20}, 0.05, 4)
        self.assertEqual(sig, [])

        # Only absolute met (10 KB but 0.01%) -> not significant.
        sig, *_ = compare({"b": 100010}, {"b": 100000}, 0.05, 4)
        self.assertEqual(sig, [])

    def test_compare_threshold_boundaries(self):
        # Exactly at the threshold (4 KB AND 5%) -> significant: the script
        # uses >= on both sides.
        sig, *_ = compare({"ls": 84}, {"ls": 80}, 0.05, 4)
        self.assertEqual(len(sig), 1)
        self.assertEqual(sig[0]["delta"], 4)
        self.assertAlmostEqual(sig[0]["rel"], 0.05)

        # Just below absolute floor: 3 KB / 3.75% -> not significant.
        sig, *_ = compare({"ls": 83}, {"ls": 80}, 0.05, 4)
        self.assertEqual(sig, [])

        # Absolute floor met exactly (4 KB) but relative just below (4%).
        sig, *_ = compare({"ls": 104}, {"ls": 100}, 0.05, 4)
        self.assertEqual(sig, [])

        # Relative just below (4.99%) with comfortable absolute -> rejected.
        sig, *_ = compare({"ls": 10499}, {"ls": 10000}, 0.05, 4)
        self.assertEqual(sig, [])

        # Symmetric shrinkage at the boundary -> still significant.
        sig, *_ = compare({"ls": 76}, {"ls": 80}, 0.05, 4)
        self.assertEqual(len(sig), 1)
        self.assertEqual(sig[0]["delta"], -4)

    def test_compare_added_removed_and_totals(self):
        sig, added, removed, totals = compare(
            {"ls": 1000, "newbin": 500}, {"ls": 1000, "oldbin": 800}, 0.05, 4
        )
        self.assertEqual(sig, [])
        self.assertEqual(added, [("newbin", 500)])
        self.assertEqual(removed, [("oldbin", 800)])
        # Totals must only consider binaries present in both runs.
        self.assertEqual(totals, {"current": 1000, "reference": 1000})

    def test_compare_sort_and_zero_reference(self):
        sig, *_ = compare({"a": 1100, "b": 2000}, {"a": 1000, "b": 1000}, 0.05, 4)
        self.assertEqual([c["name"] for c in sig], ["b", "a"])
        # Zero reference must not crash.
        sig, *_ = compare({"ls": 1000}, {"ls": 0}, 0.05, 4)
        self.assertEqual(len(sig), 1)

    def test_format_report_renders_changes(self):
        sig = [{"name": "ls", "old": 1000, "new": 1100, "delta": 100, "rel": 0.10}]
        report = format_report(
            sig,
            [("new", 5)],
            [("old", 8)],
            {"current": 1100, "reference": 1000},
            0.05,
            4,
        )
        for s in ("ls", "+10.00%", "New binaries", "new", "Removed binaries", "old"):
            self.assertIn(s, report)

    def _run_main(self, argv):
        old = sys.argv
        sys.argv = argv
        try:
            return main()
        finally:
            sys.argv = old

    def test_main_writes_only_when_significant(self):
        cur = _write_json({"d": {"sha": "n", "sizes": {"ls": 1100}}})
        ref = _write_json({"d": {"sha": "o", "sizes": {"ls": 1000}}})
        same = _write_json({"d": {"sha": "n", "sizes": {"ls": 1000}}})
        out_sig = tempfile.mktemp(suffix=".txt")
        out_none = tempfile.mktemp(suffix=".txt")
        try:
            self.assertEqual(self._run_main(["x", cur, ref, "--output", out_sig]), 0)
            self.assertTrue(os.path.exists(out_sig))
            with open(out_sig) as f:
                self.assertIn("+10.00%", f.read())

            self.assertEqual(self._run_main(["x", same, ref, "--output", out_none]), 0)
            self.assertFalse(os.path.exists(out_none))
        finally:
            for p in (cur, ref, same, out_sig, out_none):
                if os.path.exists(p):
                    os.unlink(p)

    def test_main_missing_reference_is_not_fatal(self):
        cur = _write_json({"d": {"sha": "n", "sizes": {"ls": 1000}}})
        try:
            self.assertEqual(self._run_main(["x", cur, "/nonexistent.json"]), 0)
        finally:
            os.unlink(cur)


if __name__ == "__main__":
    unittest.main()
