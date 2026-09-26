#!/usr/bin/env python3

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

EXPECTED_ASSETS = {
    "coreutils-aarch64-apple-darwin.tar.gz",
    "coreutils-aarch64-pc-windows-msvc.zip",
    "coreutils-aarch64-unknown-linux-gnu.tar.gz",
    "coreutils-aarch64-unknown-linux-musl.tar.gz",
    "coreutils-arm-unknown-linux-gnueabihf.tar.gz",
    "coreutils-i686-pc-windows-msvc.zip",
    "coreutils-i686-unknown-linux-gnu.tar.gz",
    "coreutils-i686-unknown-linux-musl.tar.gz",
    "coreutils-wasm32-wasip1.tar.gz",
    "coreutils-wasm32-wasip2.tar.gz",
    "coreutils-x86_64-apple-darwin.tar.gz",
    "coreutils-x86_64-pc-windows-msvc.zip",
    "coreutils-x86_64-unknown-linux-gnu.tar.gz",
    "coreutils-x86_64-unknown-linux-musl.tar.gz",
    "docs.tar.zst",
    "coreutils-riscv64gc-unknown-linux-musl.tar.gz",
}


class ReleaseValidationError(ValueError):
    pass


def verify_release(data, sha, published=False):
    if not isinstance(data, dict):
        raise ReleaseValidationError("release data must be a JSON object")

    if len(sha) != 40 or any(character not in "0123456789abcdef" for character in sha):
        raise ReleaseValidationError("source SHA must be a full lowercase commit hash")

    expected_tag = f"main-{sha}"
    expected_fields = {
        "tagName": expected_tag,
        "targetCommitish": sha,
        "isDraft": not published,
        "isImmutable": published,
        "isPrerelease": True,
    }
    for field, expected in expected_fields.items():
        if data.get(field) != expected:
            raise ReleaseValidationError(
                f"{field} must be {expected!r}, got {data.get(field)!r}"
            )

    assets = data.get("assets")
    if not isinstance(assets, list) or any(
        not isinstance(asset, dict) or not isinstance(asset.get("name"), str)
        for asset in assets
    ):
        raise ReleaseValidationError("assets must be a list of named release assets")

    actual_assets = [asset["name"] for asset in assets]
    asset_counts = Counter(actual_assets)
    duplicates = sorted(name for name, count in asset_counts.items() if count > 1)
    if duplicates:
        raise ReleaseValidationError(f"duplicate assets: {', '.join(duplicates)}")

    actual_set = set(actual_assets)
    missing = sorted(EXPECTED_ASSETS - actual_set)
    unexpected = sorted(actual_set - EXPECTED_ASSETS)
    if missing or unexpected:
        details = []
        if missing:
            details.append(f"missing assets: {', '.join(missing)}")
        if unexpected:
            details.append(f"unexpected assets: {', '.join(unexpected)}")
        raise ReleaseValidationError("; ".join(details))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--published", action="store_true")
    arguments = parser.parse_args()

    try:
        with arguments.path.open(encoding="utf-8") as release_file:
            data = json.load(release_file)
        verify_release(data, arguments.sha, arguments.published)
    except (OSError, json.JSONDecodeError, ReleaseValidationError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
