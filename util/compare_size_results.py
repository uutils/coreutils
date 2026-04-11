#!/usr/bin/env python3
"""
Compare individual binary sizes between current run and reference (main branch)
to flag PRs that introduce a significant change in the size of standalone
(non-multicall) binaries.

This is the size-tracking counterpart of compare_test_results.py.

Both inputs are JSON files in the format produced by `make.yml` (and stored in
the `uutils/coreutils-tracking` repo as `individual-size-result.json`):

    {
        "Mon, 17 Apr 2023 06:42:22 +0000": {
            "sha": "<commit sha>",
            "sizes": {
                "ls": 1064,
                "date": 856,
                ...
            }
        }
    }

Sizes are in 1K blocks (the value of `du -s`), so deltas are also in KB.

A change is considered "significant" when the relative delta exceeds the
configured threshold (default: 5%, matching the previous shell-based check
in make.yml). A small absolute floor is also applied to suppress 1KB rounding
noise that would otherwise show up for tiny binaries.
"""

import argparse
import json
import sys

# Default thresholds: 5% relative AND >= 4 KB absolute. The 5% matches the
# previous inline shell check; the 4 KB floor avoids reporting single-block
# rounding noise on tiny binaries (sizes are stored in 1K blocks).
DEFAULT_REL_THRESHOLD = 0.05
DEFAULT_ABS_THRESHOLD_KB = 4


def load_sizes(path):
    """Load a sizes JSON file and return (sha, {name: size_kb}).

    The file is expected to be a single-entry dict keyed by date, whose
    value is `{"sha": ..., "sizes": {...}}` (the format produced by
    make.yml). For robustness, a flat `{name: size}` mapping is also
    accepted.
    """
    with open(path, "r") as f:
        data = json.load(f)

    if not isinstance(data, dict):
        raise ValueError(f"Unexpected JSON structure in {path}")

    # date-keyed wrapper used in tracking JSON
    if data and all(isinstance(v, dict) and "sizes" in v for v in data.values()):
        # Take the (single) entry; if there are multiple, take the last one,
        # matching how compare_gnu_result.py picks list(d.keys())[0].
        entry = data[list(data.keys())[-1]]
        return entry.get("sha"), {k: int(v) for k, v in entry["sizes"].items()}

    # Flat mapping
    return None, {k: int(v) for k, v in data.items()}


def human_kb(size_kb):
    """Render a value already expressed in KB as a human-friendly string."""
    sign = "-" if size_kb < 0 else ""
    n = abs(float(size_kb))
    if n < 1024:
        return f"{sign}{n:.0f} KB"
    n /= 1024
    if n < 1024:
        return f"{sign}{n:.2f} MB"
    n /= 1024
    return f"{sign}{n:.2f} GB"


def compare(current, reference, rel_threshold, abs_threshold_kb):
    """Return (significant, added, removed, totals).

    `significant` is a list of dicts with name/old/new/delta/rel, sorted by
    descending |delta|.
    """
    significant = []
    added = []
    removed = []

    for name, new_size in current.items():
        if name not in reference:
            added.append((name, new_size))
            continue
        old_size = reference[name]
        delta = new_size - old_size
        if old_size == 0:
            rel = 0.0 if delta == 0 else float("inf")
        else:
            rel = delta / old_size
        if abs(delta) >= abs_threshold_kb and abs(rel) >= rel_threshold:
            significant.append(
                {
                    "name": name,
                    "old": old_size,
                    "new": new_size,
                    "delta": delta,
                    "rel": rel,
                }
            )

    for name, old_size in reference.items():
        if name not in current:
            removed.append((name, old_size))

    significant.sort(key=lambda c: abs(c["delta"]), reverse=True)

    common = [n for n in current if n in reference]
    totals = {
        "current": sum(current[n] for n in common),
        "reference": sum(reference[n] for n in common),
    }
    return significant, added, removed, totals


def format_report(significant, added, removed, totals, rel_threshold, abs_threshold_kb):
    lines = []

    total_delta = totals["current"] - totals["reference"]
    total_rel = total_delta / totals["reference"] if totals["reference"] else 0.0

    lines.append(
        "Individual binary size comparison VS main "
        f"(threshold: >={rel_threshold * 100:.0f}% AND >={abs_threshold_kb} KB)."
    )
    lines.append("")
    lines.append(
        f"Total size of compared binaries: {human_kb(totals['current'])} "
        f"({'+' if total_delta >= 0 else ''}{human_kb(total_delta)}, "
        f"{total_rel * 100:+.2f}%)"
    )

    if significant:
        lines.append("")
        lines.append("Significant per-binary changes:")
        name_w = max(len(c["name"]) for c in significant)
        for c in significant:
            sign = "+" if c["delta"] >= 0 else ""
            lines.append(
                f"  {c['name']:<{name_w}}  "
                f"{human_kb(c['old']):>10} -> {human_kb(c['new']):>10}  "
                f"({sign}{human_kb(c['delta'])}, {c['rel'] * 100:+.2f}%)"
            )

    if added:
        lines.append("")
        lines.append("New binaries:")
        for name, size in sorted(added):
            lines.append(f"  {name} ({human_kb(size)})")

    if removed:
        lines.append("")
        lines.append("Removed binaries:")
        for name, size in sorted(removed):
            lines.append(f"  {name} (was {human_kb(size)})")

    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(
        description=(
            "Compare individual binary sizes against a reference and flag "
            "significant changes"
        )
    )
    parser.add_argument(
        "current_json", help="Path to current run individual-size-result.json"
    )
    parser.add_argument(
        "reference_json",
        help="Path to reference (main branch) individual-size-result.json",
    )
    parser.add_argument(
        "--output",
        help="Path to output file for the GitHub PR comment body",
    )
    parser.add_argument(
        "--rel-threshold",
        type=float,
        default=DEFAULT_REL_THRESHOLD,
        help=f"Relative change threshold (default: {DEFAULT_REL_THRESHOLD})",
    )
    parser.add_argument(
        "--abs-threshold",
        type=int,
        default=DEFAULT_ABS_THRESHOLD_KB,
        help=(f"Absolute change threshold in KB (default: {DEFAULT_ABS_THRESHOLD_KB})"),
    )
    args = parser.parse_args()

    try:
        _, current = load_sizes(args.current_json)
    except (FileNotFoundError, json.JSONDecodeError, ValueError, KeyError) as e:
        sys.stderr.write(f"Error loading current sizes: {e}\n")
        return 1

    try:
        _, reference = load_sizes(args.reference_json)
    except (FileNotFoundError, json.JSONDecodeError, ValueError, KeyError) as e:
        sys.stderr.write(f"Error loading reference sizes: {e}\n")
        sys.stderr.write("Skipping comparison as reference is not available.\n")
        return 0

    significant, added, removed, totals = compare(
        current, reference, args.rel_threshold, args.abs_threshold
    )

    report = format_report(
        significant, added, removed, totals, args.rel_threshold, args.abs_threshold
    )
    print(report)

    # Emit GitHub workflow annotations so the changes are visible in the
    # Actions UI even without the PR comment. Growth is a warning,
    # shrinkage is an informational notice.
    for c in significant:
        msg = (
            f"Binary {c['name']} size changed: "
            f"{human_kb(c['old'])} -> {human_kb(c['new'])} "
            f"({'+' if c['delta'] >= 0 else ''}{human_kb(c['delta'])}, "
            f"{c['rel'] * 100:+.2f}%)"
        )
        level = "warning" if c["delta"] > 0 else "notice"
        print(f"::{level} ::{msg}", file=sys.stderr)

    # Only write the comment file when there is something worth saying so
    # downstream comment-posting workflows can skip empty bodies (the
    # GnuComment workflow uses a similar length check).
    if args.output and (significant or added or removed):
        with open(args.output, "w") as f:
            f.write(report)

    return 0


if __name__ == "__main__":
    sys.exit(main())
