#!/usr/bin/env python3
# spell-checker:ignore debuginfo
import subprocess
from itertools import product
import shutil
import os
from collections import defaultdict
from pprint import pprint

# Set to false if you've only changed the analysis and do not want to recompile
# the binaries.
RECOMPILE = True

STRIP_VALS = ["none", "debuginfo", "symbols"]
PANIC_VALS = ["unwind", "abort"]
OPT_LEVEL_VALS = [3, "s", "z"]
LTO_VALS = ["off", "thin", "fat"]


def config(name, val):
    return ["--config", f"profile.release.{name}={val!r}"]


sizes = {}

for (strip, panic, opt, lto) in product(
    STRIP_VALS, PANIC_VALS, OPT_LEVEL_VALS, LTO_VALS
):
    if RECOMPILE:
        cmd = [
            "cargo",
            "build",
            "--release",
            "--features=unix",
            *config("strip", strip),
            *config("panic", panic),
            *config("opt-level", opt),
            *config("lto", lto),
        ]
        print("RUN:", " ".join(cmd))
        res = subprocess.call(cmd)

        shutil.copyfile(
            "target/release/coreutils",
            "-".join(["coreutils", strip, panic, str(opt), lto]),
        )
        print(res)

    sizes[(strip, panic, opt, lto)] = os.path.getsize(
        "-".join(["coreutils", strip, panic, str(opt), lto])
    )

changes_absolute = defaultdict(list)
changes_percent = defaultdict(list)


def with_val_at_idx(val, idx, other):
    other = list(other)
    other.insert(idx, val)
    return tuple(other)


def collect_diff(idx, name):
    all_params = [STRIP_VALS, PANIC_VALS, OPT_LEVEL_VALS, LTO_VALS]
    vals = all_params.pop(idx)
    for other in product(*all_params):
        baseline = sizes[with_val_at_idx(vals[0], idx, other)]
        for val in vals[1:]:
            changes = sizes[with_val_at_idx(val, idx, other)] - baseline
            changes_absolute[f"{name}={val}"].append(changes)
            changes_percent[f"{name}={val}"].append(changes / baseline * 100)


collect_diff(0, "strip")
collect_diff(1, "panic")
collect_diff(2, "opt-level")
collect_diff(3, "lto")


def analyze(l):
    return f"MIN: {float(min(l)):.3}, AVG: {float(sum(l)/len(l)):.3}, MAX: {float(max(l)):.3}"


print("Absolute changes")
pprint({k: analyze(v) for k, v in changes_absolute.items()})
print()
print("Percent changes")
pprint({k: analyze(v) for k, v in changes_percent.items()})

print()
print(changes_percent["opt-level=s"])
print(changes_percent["opt-level=z"])
