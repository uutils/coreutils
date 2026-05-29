"""
Extract the GNU logs into a JSON file.
"""

import json
import re
import sys
from pathlib import Path

out = {}

if len(sys.argv) != 2:
    print("Usage: python gnu-json-result.py <gnu_test_directory>")
    sys.exit(1)

test_dir = Path(sys.argv[1])
if not test_dir.is_dir():
    print(f"Directory {test_dir} does not exist.")
    sys.exit(1)

# Test all the logs from the test execution
for filepath in test_dir.glob("**/*.log"):
    path = Path(filepath)
    current = out
    for key in path.parent.relative_to(test_dir).parts:
        if key not in current:
            current[key] = {}
        current = current[key]
    try:
        with open(path, errors="ignore") as f:
            content = f.read()
            result = re.search(
                r"(PASS|FAIL|SKIP|ERROR) [^ ]+ \(exit status: \d+\)$", content
            )
            if result:
                current[path.name] = result.group(1)
    except Exception as e:
        print(f"Error processing file {path}: {e}", file=sys.stderr)

print(json.dumps(out, indent=2, sort_keys=True))
