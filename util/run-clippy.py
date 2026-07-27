#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# spell-checker:ignore pcoreutils
"""Run cargo clippy with appropriate flags and emit GitHub Actions annotations."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys


def run_cmd(
    cmd: list[str],
    *,
    check: bool = False,
) -> subprocess.CompletedProcess[str]:
    """Run a command with UTF-8 encoding (avoids cp1252 issues on Windows)."""
    env = {**os.environ, "PYTHONUTF8": "1"}
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=check,
        env=env,
    )


def get_utility_list(features: str) -> list[str]:
    """Get list of utilities from cargo metadata."""
    if features == "all":
        cmd = ["cargo", "metadata", "--all-features", "--format-version", "1"]
    else:
        cmd = ["cargo", "metadata", "--features", features, "--format-version", "1"]
    result = run_cmd(cmd, check=True)
    metadata = json.loads(result.stdout)
    # Find the coreutils root node and collect uu_ dependencies
    utilities = []
    for node in metadata["resolve"]["nodes"]:
        if re.search(r"coreutils[ @#]\d+\.\d+\.\d+", node["id"]):
            for dep in node["deps"]:
                # The pkg field contains the crate name (uu_<util>),
                # while name is the renamed dependency alias
                pkg = dep["pkg"]
                match = re.search(r"uu_(\w+)[@#]", pkg)
                if match:
                    utilities.append(match.group(1))
            break
    return sorted(utilities)


def build_clippy_command(
    features: str,
    *,
    workspace: bool,
    target: str | None,
) -> list[str]:
    """Build the cargo clippy command line."""
    cmd = ["cargo", "clippy"]

    extra = []
    if features == "all":
        extra.append("--all-features")
    else:
        extra.extend(["--features", features])

    if workspace:
        extra.append("--workspace")

    if target:
        extra.extend(["--no-default-features", "--target", target])
        # For cross-compilation targets, just check -pcoreutils
        # (show-utils.sh over-resolves due to default features)
        extra.append("-pcoreutils")
    else:
        extra.extend(["--all-targets", "--tests", "--benches", "-pcoreutils"])
        utilities = get_utility_list(features)
        extra.extend(f"-puu_{u}" for u in utilities)

    cmd.extend(extra)
    cmd.extend(["--", "-D", "warnings"])
    return cmd


# Pattern to match clippy/rustc errors for GHA annotations
ERROR_PATTERN = re.compile(
    r"^error:\s+(.*)\n\s+-->\s+(.*):(\d+):(\d+)",
    re.MULTILINE,
)


def emit_annotations(output: str, fault_type: str) -> None:
    """Emit GitHub Actions annotations from cargo clippy errors."""
    fault_prefix = fault_type.upper()
    for m in ERROR_PATTERN.finditer(output):
        message, file, line, col = m.groups()
        print(
            f"::{fault_type} file={file},line={line},col={col}"
            f"::{fault_prefix}: `cargo clippy`: {message} (file:'{file}', line:{line})",
        )


def main() -> int:
    """Run cargo clippy and emit GHA annotations on failure."""
    parser = argparse.ArgumentParser(description="Run cargo clippy for CI")
    parser.add_argument("--features", required=True, help="Feature set to use")
    parser.add_argument(
        "--workspace",
        action="store_true",
        help="Include --workspace flag",
    )
    parser.add_argument("--target", default=None, help="Cross-compilation target")
    parser.add_argument(
        "--fault-type",
        default="warning",
        choices=["warning", "error"],
        help="GHA annotation type",
    )
    parser.add_argument(
        "--fail-on-fault",
        action="store_true",
        help="Exit with error code on clippy failures",
    )
    args = parser.parse_args()

    cmd = build_clippy_command(
        args.features,
        workspace=args.workspace,
        target=args.target,
    )
    print(f"Running: {' '.join(cmd)}", file=sys.stderr)

    result = run_cmd(cmd)
    output = result.stdout + result.stderr

    # Always print the full output
    print(output)

    if result.returncode != 0:
        emit_annotations(output, args.fault_type)
        if args.fail_on_fault:
            return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
