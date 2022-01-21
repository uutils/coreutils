# Simple script to create the correct SUMMARY.md and other files
# for the mdbook documentation.
# Note: This will overwrite the existing files!

import os

with open('src/SUMMARY.md', 'w') as summary:
    summary.write("# Summary\n\n")
    summary.write("[Introduction](index.md)\n")
    summary.write("* [Contributing](contributing.md)\n")
    for d in sorted(os.listdir('../src/uu')):
        summary.write(f"* [{d}](utils/{d}.md)\n")