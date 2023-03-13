"""
Extract the GNU logs into a JSON file.
"""

import json
import re
import sys
from pathlib import Path

out = {}

test_dir = Path(sys.argv[1])
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
    except:
        pass

print(json.dumps(out, indent=2, sort_keys=True))
