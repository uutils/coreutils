<!-- spell-checker:ignore testfile -->

# Benchmarking to measure performance

To compare the performance of the `uutils` version of `split` with the
GNU version of `split`, you can use a benchmarking tool like
[hyperfine][0]. On Ubuntu 18.04 or later, you can install `hyperfine` by
running

    sudo apt-get install hyperfine

Next, build the `split` binary under the release profile:

    cargo build --release -p uu_split

Now, get a text file to test `split` on. The `split` program has three
main modes of operation: chunk by lines, chunk by bytes, and chunk by
lines with a byte limit. You may want to test the performance of `split`
with various shapes and sizes of input files and under various modes of
operation. For example, to test chunking by bytes on a large input file,
you can create a file named `testfile.txt` containing one million null
bytes like this:

    printf "%0.s\0" {1..1000000} > testfile.txt

For another example, to test chunking by bytes on a large real-world
input file, you could download a [database dump of Wikidata][1] or some
related files that the Wikimedia project provides. For example, [this
file][2] contains about 130 million lines.

Finally, you can compare the performance of the two versions of `split`
by running, for example,

    cd /tmp && hyperfine \
        --prepare 'rm x* || true' \
	"split -b 1000 testfile.txt" \
	"target/release/split -b 1000 testfile.txt"

Since `split` creates a lot of files on the filesystem, I recommend
changing to the `/tmp` directory before running the benchmark. The
`--prepare` argument to `hyperfine` runs a specified command before each
timing run. We specify `rm x* || true` so that the output files from the
previous run of `split` are removed before each run begins.

[0]: https://github.com/sharkdp/hyperfine
[1]: https://www.wikidata.org/wiki/Wikidata:Database_download
[2]: https://dumps.wikimedia.org/wikidatawiki/20211001/wikidatawiki-20211001-pages-logging.xml.gz
