"""
Extract the GNU logs into a JSON file.
"""

import json
from pathlib import Path
import sys
from os import environ

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
        with open(path) as f:
            content = f.read()
            current[path.name] = content.split("\n")[-2].split(" ")[0]
    except:
        pass

print(json.dumps(out, indent=2, sort_keys=True))