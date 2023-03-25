#!/usr/bin/env python3
# This script lists the GNU failing tests by size
# Just like with util/run-gnu-test.sh, we expect the gnu sources
# to be in ../
import urllib.request

import urllib
import os
import glob
import json
import sys

base = "../gnu/tests/"
urllib.request.urlretrieve(
    "https://raw.githubusercontent.com/uutils/coreutils-tracking/main/gnu-full-result.json",
    "result.json",
)

types = ("/*/*.sh", "/*/*.pl", "/*/*.xpl")

tests = []
error_or_skip_tests = []

for files in types:
    tests.extend(glob.glob(base + files))
# sort by size
list_of_files = sorted(tests, key=lambda x: os.stat(x).st_size)


def show_list(l):
    # Remove the factor tests and reverse the list (bigger first)
    tests = list(filter(lambda k: "factor" not in k, l))

    for f in reversed(tests):
        print("%s: %s" % (f, os.stat(f).st_size))
    print("")
    print("%s tests remaining" % len(tests))


with open("result.json", "r") as json_file:
    data = json.load(json_file)

for d in data:
    for e in data[d]:
        # Not all the tests are .sh files, rename them if not.
        script = e.replace(".log", ".sh")
        a = f"{base}{d}/{script}"
        if not os.path.exists(a):
            a = a.replace(".sh", ".pl")
            if not os.path.exists(a):
                a = a.replace(".pl", ".xpl")

        # the tests pass, we don't care anymore
        if data[d][e] == "PASS":
            try:
                list_of_files.remove(a)
            except ValueError:
                print("Could not find test '%s'. Maybe update the GNU repo?" % a)
                sys.exit(1)

        # if it is SKIP or ERROR, show it
        if data[d][e] == "SKIP" or data[d][e] == "ERROR":
            list_of_files.remove(a)
            error_or_skip_tests.append(a)


print("SKIP and ERROR tests:")
show_list(error_or_skip_tests)
print("")
print("===============")
print("FAIL tests:")
show_list(list_of_files)
