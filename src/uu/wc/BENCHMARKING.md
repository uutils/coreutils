# Benchmarking wc

<!-- spell-checker:ignore (words) uuwc uucat largefile somefile Mshortlines moby lwcm cmds tablefmt -->

Much of what makes wc fast is avoiding unnecessary work. It has multiple strategies, depending on which data is requested.

## Strategies

### Counting bytes

In the case of `wc -c` the content of the input doesn't have to be inspected at all, only the size has to be known. That enables a few optimizations.

#### File size

If it can, wc reads the file size directly. This is not interesting to benchmark, except to see if it still works. Try `wc -c largefile`.

#### `splice()`

On Linux `splice()` is used to get the input's length while discarding it directly.

The best way I've found to generate a fast input to test `splice()` is to pipe the output of uutils `cat` into it. Note that GNU `cat` is slower and therefore less suitable, and that if a file is given as its input directly (as in `wc -c < largefile`) the first strategy kicks in. Try `uucat somefile | wc -c`.

### Counting lines

In the case of `wc -l` or `wc -cl` the input doesn't have to be decoded. It's read in chunks and the `bytecount` crate is used to count the newlines.

It's useful to vary the line length in the input. GNU wc seems particularly bad at short lines.

### Processing unicode

This is the most general strategy, and it's necessary for counting words, characters, and line lengths. Individual steps are still switched on and off depending on what must be reported.

Try varying which of the `-w`, `-m`, `-l` and `-L` flags are used. (The `-c` flag is unlikely to make a difference.)

Passing no flags is equivalent to passing `-wcl`. That case should perhaps be given special attention as it's the default.

## Generating files

To generate a file with many very short lines, run `yes | head -c50000000 > 25Mshortlines`.

To get a file with less artificial contents, download a book from Project Gutenberg and concatenate it a lot of times:

```
wget https://www.gutenberg.org/files/2701/2701-0.txt -O moby.txt
cat moby.txt moby.txt moby.txt moby.txt > moby4.txt
cat moby4.txt moby4.txt moby4.txt moby4.txt > moby16.txt
cat moby16.txt moby16.txt moby16.txt moby16.txt > moby64.txt
```

And get one with lots of unicode too:

```
wget https://www.gutenberg.org/files/30613/30613-0.txt -O odyssey.txt
cat odyssey.txt odyssey.txt odyssey.txt odyssey.txt > odyssey4.txt
cat odyssey4.txt odyssey4.txt odyssey4.txt odyssey4.txt > odyssey16.txt
cat odyssey16.txt odyssey16.txt odyssey16.txt odyssey16.txt > odyssey64.txt
cat odyssey64.txt odyssey64.txt odyssey64.txt odyssey64.txt > odyssey256.txt
```

Finally, it's interesting to try a binary file. Look for one with `du -sh /usr/bin/* | sort -h`. On my system `/usr/bin/docker` is a good candidate as it's fairly large.

## Running benchmarks

Use [`hyperfine`](https://github.com/sharkdp/hyperfine) to compare the performance. For example, `hyperfine 'wc somefile' 'uuwc somefile'`.

If you want to get fancy and exhaustive, generate a table:

|                        |   moby64.txt |   odyssey256.txt |   25Mshortlines |   /usr/bin/docker |
|------------------------|--------------|------------------|-----------------|-------------------|
| `wc <FILE>`            |       1.3965 |           1.6182 |          5.2967 |            2.2294 |
| `wc -c <FILE>`         |       0.8134 |           1.2774 |          0.7732 |            0.9106 |
| `uucat <FILE> | wc -c` |       2.7760 |           2.5565 |          2.3769 |            2.3982 |
| `wc -l <FILE>`         |       1.1441 |           1.2854 |          2.9681 |            1.1493 |
| `wc -L <FILE>`         |       2.1087 |           1.2551 |          5.4577 |            2.1490 |
| `wc -m <FILE>`         |       2.7272 |           2.1704 |          7.3371 |            3.4347 |
| `wc -w <FILE>`         |       1.9007 |           1.5206 |          4.7851 |            2.8529 |
| `wc -lwcmL <FILE>`     |       1.1687 |           0.9169 |          4.4092 |            2.0663 |

Beware that:
- Results are fuzzy and change from run to run
- You'll often want to check versions of uutils wc against each other instead of against GNU
- This takes a lot of time to generate
- This only shows the relative speedup, not the absolute time, which may be misleading if the time is very short

Created by the following Python script:
```python
import json
import subprocess

from tabulate import tabulate

bins = ["wc", "uuwc"]
files = ["moby64.txt", "odyssey256.txt", "25Mshortlines", "/usr/bin/docker"]
cmds = [
    "{cmd} {file}",
    "{cmd} -c {file}",
    "uucat {file} | {cmd} -c",
    "{cmd} -l {file}",
    "{cmd} -L {file}",
    "{cmd} -m {file}",
    "{cmd} -w {file}",
    "{cmd} -lwcmL {file}",
]

table = []
for cmd in cmds:
    row = ["`" + cmd.format(cmd="wc", file="<FILE>") + "`"]
    for file in files:
        subprocess.run(
            [
                "hyperfine",
                cmd.format(cmd=bins[0], file=file),
                cmd.format(cmd=bins[1], file=file),
                "--export-json=out.json",
            ],
            check=True,
        )
        with open("out.json") as f:
            res = json.load(f)["results"]
        row.append(round(res[0]["mean"] / res[1]["mean"], 4))
    table.append(row)
print(tabulate(table, [""] + files, tablefmt="github"))
```
(You may have to adjust the `bins` and `files` variables depending on your setup, and please do add other interesting cases to `cmds`.)
