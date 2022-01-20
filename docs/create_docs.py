# Simple script to create the correct SUMMARY.md and other files
# for the mdbook documentation.
# Note: This will overwrite the existing files!

import os

with open('src/utils/index.md', 'w') as index:
    with open('src/SUMMARY.md', 'w') as summary:
        summary.write("# Summary\n\n")
        summary.write("[Introduction](index.md)\n")
        summary.write("* [Contributing](contributing.md)\n")
        summary.write("* [Utils](utils/index.md)\n")
        index.write("# Utils\n\n")
        for d in sorted(os.listdir('../src/uu')):
            with open(f"src/utils/{d}.md", 'w') as f:
                f.write(f"# {d}\n\n")
                f.write(f"{{{{ #include ../../_generated/{d}-help.md }}}}\n")
                print(f"Created docs/src/utils/{d}.md")
            summary.write(f"  * [{d}](utils/{d}.md)\n")
            index.write(f"* [{d}](./{d}.md)\n")