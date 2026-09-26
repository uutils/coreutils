#!/usr/bin/env python3

import re
import unittest
from pathlib import Path

from util.verify_release import EXPECTED_ASSETS, ReleaseValidationError, verify_release

SHA = "0123456789abcdef0123456789abcdef01234567"
WORKFLOW = Path(__file__).parents[1] / ".github/workflows/CICD.yml"


def release_data():
    return {
        "tagName": f"main-{SHA}",
        "targetCommitish": SHA,
        "isDraft": True,
        "isImmutable": False,
        "isPrerelease": True,
        "assets": [{"name": name} for name in sorted(EXPECTED_ASSETS)],
    }


class VerifyReleaseTests(unittest.TestCase):
    def test_rejects_non_object_release(self):
        with self.assertRaisesRegex(ReleaseValidationError, "JSON object"):
            verify_release([], SHA)

    def test_accepts_complete_draft(self):
        verify_release(release_data(), SHA)

    def test_accepts_complete_published_release(self):
        data = release_data()
        data["isDraft"] = False
        data["isImmutable"] = True
        verify_release(data, SHA, published=True)

    def test_rejects_incomplete_release(self):
        data = release_data()
        data["assets"].pop()
        with self.assertRaisesRegex(ReleaseValidationError, "missing assets"):
            verify_release(data, SHA)

    def test_rejects_unexpected_asset(self):
        data = release_data()
        data["assets"].append({"name": "unexpected.tar.gz"})
        with self.assertRaisesRegex(ReleaseValidationError, "unexpected assets"):
            verify_release(data, SHA)

    def test_rejects_duplicate_asset(self):
        data = release_data()
        data["assets"].append(data["assets"][0])
        with self.assertRaisesRegex(ReleaseValidationError, "duplicate assets"):
            verify_release(data, SHA)

    def test_rejects_wrong_commit(self):
        data = release_data()
        data["targetCommitish"] = "main"
        with self.assertRaisesRegex(ReleaseValidationError, "targetCommitish"):
            verify_release(data, SHA)

    def test_rejects_published_state_as_draft(self):
        data = release_data()
        data["isImmutable"] = True
        with self.assertRaisesRegex(ReleaseValidationError, "isImmutable"):
            verify_release(data, SHA)

    def test_expected_assets_match_publish_matrix(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        matrix = workflow.split("\n  build:\n", 1)[1].split(
            "\n  publish-main-release:\n", 1
        )[0]
        assets = set()
        for line in matrix.splitlines():
            match = re.match(r"\s+- \{.*\btarget: ([^ ,}]+)", line)
            if not match or "skip-publish: true" in line or "check-only: true" in line:
                continue
            target = match.group(1)
            suffix = ".zip" if "-pc-windows-" in target else ".tar.gz"
            assets.add(f"coreutils-{target}{suffix}")
        assets.add("docs.tar.zst")
        self.assertEqual(assets, EXPECTED_ASSETS)

    def test_rejects_short_commit(self):
        with self.assertRaisesRegex(ReleaseValidationError, "full lowercase"):
            verify_release(release_data(), SHA[:12])


if __name__ == "__main__":
    unittest.main()
