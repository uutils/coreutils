#!/usr/bin/env python3
# This script lists the GNU failing tests by size
# Just like with util/run-gnu-test.sh, we expect the gnu sources
# to be in ../
import urllib.request

import urllib
import os
import glob
import json

base = "../gnu/tests/"
urllib.request.urlretrieve(
    "https://raw.githubusercontent.com/uutils/coreutils-tracking/main/gnu-full-result.json",
    "result.json",
)

types = ("/*/*.sh", "/*/*.pl", "/*/*.xpl")

tests = []
for files in types:
    tests.extend(glob.glob(base + files))
# sort by size
list_of_files = sorted(tests, key=lambda x: os.stat(x).st_size)

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
            list_of_files.remove(a)

# Remove the factor tests and reverse the list (bigger first)
tests = list(filter(lambda k: "factor" not in k, list_of_files))

for f in reversed(tests):
    print("%s: %s" % (f, os.stat(f).st_size))
print("")
print("%s tests remaining" % len(tests))
