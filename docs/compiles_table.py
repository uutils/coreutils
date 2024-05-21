#!/usr/bin/env python3
import multiprocessing
import subprocess
import argparse
import csv
import sys
from collections import defaultdict
from pathlib import Path

# third party dependencies
from tqdm import tqdm

# spell-checker:ignore (libs) tqdm imap ; (shell/mac) xcrun ; (vars) nargs retcode csvfile

BINS_PATH = Path("../src/uu")
CACHE_PATH = Path("compiles_table.csv")
TARGETS = [
    # Linux - GNU
    "aarch64-unknown-linux-gnu",
    "i686-unknown-linux-gnu",
    "powerpc64-unknown-linux-gnu",
    "riscv64gc-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    # Windows
    "aarch64-pc-windows-msvc",
    "i686-pc-windows-gnu",
    "i686-pc-windows-msvc",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
    # Apple
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-apple-ios",
    "x86_64-apple-ios",
    # BSDs
    "x86_64-unknown-freebsd",
    "x86_64-unknown-netbsd",
    # Android
    "aarch64-linux-android",
    "x86_64-linux-android",
    # Solaris
    "x86_64-sun-solaris",
    # Illumos
    "x86_64-unknown-illumos",
    # WASM
    "wasm32-wasi",
    # Redox
    "x86_64-unknown-redox",
    # Fuchsia
    "aarch64-fuchsia",
    "x86_64-fuchsia",
]


class Target(str):
    def __new__(cls, content):
        obj = super().__new__(cls, content)
        obj.arch, obj.platform, obj.os = Target.parse(content)
        return obj

    @staticmethod
    def parse(s):
        elem = s.split("-")
        if len(elem) == 2:
            arch, platform, os = elem[0], "n/a", elem[1]
        else:
            arch, platform, os = elem[0], elem[1], "-".join(elem[2:])
        if os == "ios":
            os = "apple IOS"
        if os == "darwin":
            os = "apple MacOS"
        return (arch, platform, os)

    @staticmethod
    def get_heading():
        return ["OS", "ARCH"]

    def get_row_heading(self):
        return [self.os, self.arch]

    def requires_nightly(self):
        return "redox" in self

    # Perform the 'it-compiles' check
    def check(self, binary):
        if self.requires_nightly():
            args = [
                "cargo",
                "+nightly",
                "check",
                "-p",
                f"uu_{binary}",
                "--bin",
                binary,
                f"--target={self}",
            ]
        else:
            args = [
                "cargo",
                "check",
                "-p",
                f"uu_{binary}",
                "--bin",
                binary,
                f"--target={self}",
            ]

        res = subprocess.run(args, capture_output=True)
        return res.returncode

    # Validate that the dependencies for running this target are met
    def is_installed(self):
        # check IOS sdk is installed, raise exception otherwise
        if "ios" in self:
            res = subprocess.run(["which", "xcrun"], capture_output=True)
            if len(res.stdout) == 0:
                raise Exception(
                    "Error: IOS sdk does not seem to be installed. Please do that manually"
                )
        if not self.requires_nightly():
            # check std toolchains are installed
            toolchains = subprocess.run(
                ["rustup", "target", "list"], capture_output=True
            )
            toolchains = toolchains.stdout.decode("utf-8").split("\n")
            if "installed" not in next(filter(lambda x: self in x, toolchains)):
                raise Exception(
                    f"Error: the {self} target is not installed. Please do that manually"
                )
        else:
            # check nightly toolchains are installed
            toolchains = subprocess.run(
                ["rustup", "+nightly", "target", "list"], capture_output=True
            )
            toolchains = toolchains.stdout.decode("utf-8").split("\n")
            if "installed" not in next(filter(lambda x: self in x, toolchains)):
                raise Exception(
                    f"Error: the {self} nightly target is not installed. Please do that manually"
                )
        return True


def install_targets():
    cmd = ["rustup", "target", "add"] + TARGETS
    print(" ".join(cmd))
    ret = subprocess.run(cmd)
    assert ret.returncode == 0


def get_all_bins():
    bins = map(lambda x: x.name, BINS_PATH.iterdir())
    return sorted(list(bins))


def get_targets(selection):
    if "all" in selection:
        return list(map(Target, TARGETS))
    else:
        # preserve the same order as in TARGETS
        return list(map(Target, filter(lambda x: x in selection, TARGETS)))


def test_helper(tup):
    bin, target = tup
    retcode = target.check(bin)
    return (target, bin, retcode)


def test_all_targets(targets, bins):
    pool = multiprocessing.Pool()
    inputs = [(b, t) for b in bins for t in targets]

    outputs = list(tqdm(pool.imap(test_helper, inputs), total=len(inputs)))

    table = defaultdict(dict)
    for (t, b, r) in outputs:
        table[t][b] = r
    return table


def save_csv(file, table):
    targets = get_targets(table.keys())  # preserve order in CSV
    bins = list(list(table.values())[0].keys())
    with open(file, "w") as csvfile:
        header = ["target"] + bins
        writer = csv.DictWriter(csvfile, fieldnames=header)
        writer.writeheader()
        for t in targets:
            d = {"target": t}
            d.update(table[t])
            writer.writerow(d)


def load_csv(file):
    table = {}
    cols = []
    rows = []
    with open(file, "r") as csvfile:
        reader = csv.DictReader(csvfile)
        cols = list(filter(lambda x: x != "target", reader.fieldnames))
        for row in reader:
            t = Target(row["target"])
            rows += [t]
            del row["target"]
            table[t] = dict([k, int(v)] for k, v in row.items())
    return (table, rows, cols)


def merge_tables(old, new):
    from copy import deepcopy

    tmp = deepcopy(old)
    tmp.update(deepcopy(new))
    return tmp


def render_md(fd, table, headings: str, row_headings: Target):
    def print_row(lst, lens=[]):
        lens = lens + [0] * (len(lst) - len(lens))
        for e, lmd in zip(lst, lens):
            fmt = "|{}" if lmd == 0 else "|{:>%s}" % len(header[0])
            fd.write(fmt.format(e))
        fd.write("|\n")

    def cell_render(target, bin):
        return "y" if table[target][bin] == 0 else " "

    # add some 'hard' padding to specific columns
    lens = [
        max(map(lambda x: len(x.os), row_headings)) + 2,
        max(map(lambda x: len(x.arch), row_headings)) + 2,
    ]
    header = Target.get_heading()
    header[0] = ("{:#^%d}" % lens[0]).format(header[0])
    header[1] = ("{:#^%d}" % lens[1]).format(header[1])

    header += headings
    print_row(header)
    lines = list(map(lambda x: "-" * len(x), header))
    print_row(lines)

    for t in row_headings:
        row = list(map(lambda b: cell_render(t, b), headings))
        row = t.get_row_heading() + row
        print_row(row)


if __name__ == "__main__":
    # create the top-level parser
    parser = argparse.ArgumentParser(prog="compiles_table.py")
    subparsers = parser.add_subparsers(
        help="sub-command to execute", required=True, dest="cmd"
    )
    # create the parser for the "check" command
    parser_a = subparsers.add_parser(
        "check", help="run cargo check on specified targets and update csv cache"
    )
    parser_a.add_argument(
        "targets",
        metavar="TARGET",
        type=str,
        nargs="+",
        choices=["all"] + TARGETS,
        help="target-triple to check, as shown by 'rustup target list'",
    )
    # create the parser for the "render" command
    parser_b = subparsers.add_parser("render", help="print a markdown table to stdout")
    parser_b.add_argument(
        "--equidistant",
        action="store_true",
        help="NOT IMPLEMENTED: render each column with an equal width (in plaintext)",
    )
    args = parser.parse_args()

    if args.cmd == "render":
        table, targets, bins = load_csv(CACHE_PATH)
        render_md(sys.stdout, table, bins, targets)

    if args.cmd == "check":
        targets = get_targets(args.targets)
        bins = get_all_bins()

        assert all(map(Target.is_installed, targets))
        table = test_all_targets(targets, bins)

        prev_table, _, _ = load_csv(CACHE_PATH)
        new_table = merge_tables(prev_table, table)
        save_csv(CACHE_PATH, new_table)
