#!/usr/bin/env python3

"""
Compare the current results to the last results gathered from the main branch to highlight
if a PR is making the results better/worse
"""

import json
import sys
from os import environ

REPO_DEFAULT_BRANCH = environ.get("REPO_DEFAULT_BRANCH", "main")

NEW = json.load(open("gnu-result.json"))
OLD = json.load(open("main-gnu-result.json"))

# Extract the specific results from the dicts
last = OLD[list(OLD.keys())[0]]
current = NEW[list(NEW.keys())[0]]


pass_d = int(current["pass"]) - int(last["pass"])
fail_d = int(current["fail"]) - int(last["fail"])
error_d = int(current["error"]) - int(last["error"])
skip_d = int(current["skip"]) - int(last["skip"])

# Get an annotation to highlight changes
print(
    f"::warning ::Changes from '{REPO_DEFAULT_BRANCH}': PASS {pass_d:+d} / FAIL {fail_d:+d} / ERROR {error_d:+d} / SKIP {skip_d:+d} "
)

# If results are worse fail the job to draw attention
if pass_d < 0:
    print(
        f"::error ::PASS count is reduced from '{REPO_DEFAULT_BRANCH}': PASS {pass_d:+d} "
    )
    sys.exit(1)
