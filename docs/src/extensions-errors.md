<!-- spell-checker:ignore numfmt OPTS ariadne mydir mypipe mydev -->

# Error diagnostics

GNU coreutils reports every error as a single line on stderr. That line
answers *what* went wrong, but not *where*. For utilities whose arguments form
a small language of their own — a `test` expression, a `chmod` mode, a `sort`
key — the interesting question is usually which argument, or which character
inside an argument, broke the parse.

When stderr is a terminal, uutils renders these errors as a compiler-style
report instead: the command line is echoed back as a source line, and a caret
points at the part that is at fault, often with a line of advice. Everywhere
else — a script, a pipe, a test harness — the plain one-line message is kept,
so nothing that reads stderr can tell the difference.

## Before and after

Each utility below shows the plain one-line message first — what uutils
prints when stderr is not a terminal, and in most cases exactly what GNU
prints — then the report rendered on a terminal.

### `test`

```
$ test 7 -eq zap
test: invalid integer 'zap'
```

```
$ test 7 -eq zap
test: invalid integer 'zap'
   ╭─[ test:1:7 ]
   │
 1 │ 7 -eq zap
   │       ───
   │
   │ Help: -eq, -ne, -lt, -le, -gt and -ge compare integers; use =, !=, < or > to compare strings
   │       -eq equal, -ne not equal, -lt less than, -le less than or equal, -gt greater than, -ge greater than or equal
───╯
```

[Try it in the playground](https://uutils.org/playground/?cmd=test+7+-eq+zap).

### `expr`

```
$ expr 9 + foo
expr: non-integer argument
```

```
$ expr 9 + foo
expr: non-integer argument
   ╭─[ expr:1:5 ]
   │
 1 │ 9 + foo
   │     ───
   │
   │ Help: arithmetic operators need integers; use = or != to compare strings instead
───╯
```

[Try it in the playground](https://uutils.org/playground/?cmd=expr+9+%2B+foo).

### `chmod`

The caret can point *inside* an argument, at the exact character that broke
the parse:

```
$ chmod g+rw?x notes.txt
chmod: invalid operator (expected +, -, or =, but found ?)
```

```
$ chmod g+rw?x notes.txt
chmod: invalid operator (expected +, -, or =, but found ?)
   ╭─[ chmod:1:5 ]
   │
 1 │ g+rw?x notes.txt
   │     ─
   │
   │ Help: a mode is either octal, as in 644, or clauses such as u+rwx,go-w
───╯
```

### `tr`

```
$ tr 'qw[y-b]' x
tr: range-endpoints of 'y-b' are in reverse collating sequence order
```

```
$ tr 'qw[y-b]' x
tr: range-endpoints of 'y-b' are in reverse collating sequence order
   ╭─[ tr:1:7 ]
   │
 1 │ tr qw[y-b] x
   │       ─┬─
   │        ╰─── did you mean 'b-y'?
   │
   │ Help: a range goes from the lower character to the higher one, as in a-z
───╯
```

[Try it in the playground](https://uutils.org/playground/?cmd=tr+%27qw%5By-b%5D%27+x).

### `sort`

```
$ sort -k2.3x notes.txt
sort: stray character in field spec: invalid field specification '2.3x'
```

```
$ sort -k2.3x notes.txt
sort: stray character in field spec: invalid field specification '2.3x'
   ╭─[ sort:1:11 ]
   │
 1 │ sort -k2.3x notes.txt
   │           ─
   │
   │ Help: a key is FIELD[.CHAR][OPTS][,FIELD[.CHAR][OPTS]], as in -k2.3,4nr
───╯
```

[Try it in the playground](https://uutils.org/playground/?cmd=sort+-k2.3x+fruits.txt).

### `numfmt`

```
$ numfmt --format=%q 1000
numfmt: invalid format '%q', directive must be %[0]['][-][N][.][N]f
```

```
$ numfmt --format=%q 1000
numfmt: invalid format '%q', directive must be %[0]['][-][N][.][N]f
   ╭─[ numfmt:1:18 ]
   │
 1 │ numfmt --format=%q 1000
   │                  ─
   │
   │ Help: a format is [PREFIX]%[0]['][-][WIDTH][.PRECISION]f[SUFFIX], as in "%'-10.2f"
───╯
```

[Try it in the playground](https://uutils.org/playground/?cmd=numfmt+--format%3D%25q+1000).

### `printf`

```
$ printf %5.2c q
printf: %5.2c: invalid conversion specification
```

```
$ printf %5.2c q
printf: %5.2c: invalid conversion specification
   ╭─[ printf:1:8 ]
   │
 1 │ printf %5.2c q
   │        ─────
   │
   │ Help: %d, %s, %x, %f and the other C conversions are accepted, plus %b and %q; a literal % is written %%
───╯
```

The same goes for a broken escape: `printf 'a\xzb'` puts the caret under the
`\x` that is missing its hexadecimal digits.

[Try it in the playground](https://uutils.org/playground/?cmd=printf+%255.2c+q).

### `env`

`env -S` takes a whole command line and splits it as a shell would. Its
messages name an offset, which is precisely the thing a caret can show
instead:

```
$ env -S 'echo ${1FOO}'
env: only ${VARNAME} expansion is supported, error at: ${1FOO}
```

```
$ env -S 'echo ${1FOO}'
env: only ${VARNAME} expansion is supported, error at: ${1FOO}
   ╭─[ env:1:14 ]
   │
 1 │ env -S 'echo ${1FOO}'
   │              ─┬─
   │               ╰─── a variable name cannot start with a digit
   │
   │ Help: only $NAME and ${NAME} are expanded; the other shell forms are not
───╯
```

Note that the `-S` string holds spaces, so it is echoed back quoted — and the
caret still points inside it.

### `cut`

A list of ranges is often long, and only one item in it is wrong:

```
$ cut -f 1,4-2,9-12 notes.txt
cut: range '4-2' was invalid: high end of range less than low end
```

```
$ cut -f 1,4-2,9-12 notes.txt
cut: range '4-2' was invalid: high end of range less than low end
   ╭─[ cut:1:10 ]
   │
 1 │ cut -f 1,4-2,9-12 notes.txt
   │          ─┬─
   │           ╰─── this range ends before it starts
   │
   │ Help: a list is N, N-M, N- or -M, separated by commas, as in -f1,4-6,9-
───╯
```

### `head`

A SIZE is a number and a unit, and the caret says which of the two was
rejected:

```
$ head -c 1fb notes.txt
head: invalid number of bytes: '1fb'
```

```
$ head -c 1fb notes.txt
head: invalid number of bytes: '1fb'
   ╭─[ head:1:10 ]
   │
 1 │ head -c 1fb notes.txt
   │          ─┬
   │           ╰── not a known unit
   │
   │ Help: a size is a number and an optional unit: K, M, G and so on for 1024, KB, MB, GB for 1000
───╯
```

## Compatibility

This is strictly an interactive nicety; nothing that reads our output can tell
the difference:

- Reports are only rendered when **stderr is a terminal**. In a script, a
  pipe, or a test harness, the utility keeps printing its plain one-line
  message, so existing scripts that match on stderr keep working.
- Exit codes are unchanged.
- Colors follow the usual conventions: they are used only on a terminal, and
  [`NO_COLOR`](https://no-color.org/) disables them.
- All messages, labels and help lines are localized like the rest of uutils
  (see [Localization](l10n.md)).
- The rendering can be compiled out entirely: it sits behind the
  `feat_diagnostics` cargo feature, which is on by default. Building with
  `--no-default-features` (plus a `feat_os_*` selection) drops the renderer
  and its `ariadne` dependency, and every utility keeps its plain one-line
  messages.

## Supported utilities

| Utility  | What the caret points at | Try it |
| -------- | ------------------------ | ------ |
| `test`   | the argument that made the expression fail | [`test 7 -eq zap`](https://uutils.org/playground/?cmd=test+7+-eq+zap) |
| `expr`   | the argument that made the expression fail | [`expr 9 + foo`](https://uutils.org/playground/?cmd=expr+9+%2B+foo) |
| `chmod`  | the failing clause (or character) of an invalid symbolic or octal mode | [`chmod 'g+rw?x' fruits.txt`](https://uutils.org/playground/?cmd=chmod+%27g%2Brw%3Fx%27+fruits.txt) |
| `mkdir`  | the failing part of the mode given to `-m`/`--mode` | [`mkdir -m u+q mydir`](https://uutils.org/playground/?cmd=mkdir+-m+u%2Bq+mydir) |
| `mkfifo` | the failing part of the mode given to `-m`/`--mode` | [`mkfifo -m u+q mypipe`](https://uutils.org/playground/?cmd=mkfifo+-m+u%2Bq+mypipe) |
| `mknod`  | the failing part of the mode given to `-m`/`--mode` | [`mknod -m u+q mydev c 1 3`](https://uutils.org/playground/?cmd=mknod+-m+u%2Bq+mydev+c+1+3) |
| `install`| the failing part of the mode given to `-m`/`--mode` | [`install -m u+q fruits.txt dest`](https://uutils.org/playground/?cmd=install+-m+u%2Bq+fruits.txt+dest) |
| `tr`     | the part of a set that is at fault (bad class, backwards range, bad repeat count, …) | [`tr 'qw[y-b]' x`](https://uutils.org/playground/?cmd=tr+%27qw%5By-b%5D%27+x) |
| `sort`   | the failing part of a `-k`/`--key` or field specification, or of the SIZE given to `-S` | [`sort -k2.3x fruits.txt`](https://uutils.org/playground/?cmd=sort+-k2.3x+fruits.txt) |
| `numfmt` | the failing part of a `--field` or `--format` specification | [`numfmt --format=%q 1000`](https://uutils.org/playground/?cmd=numfmt+--format%3D%25q+1000) |
| `printf` | the failing conversion or escape in the format string | [`printf %5.2c q`](https://uutils.org/playground/?cmd=printf+%255.2c+q) |
| `seq`    | the failing conversion in the format given to `-f`/`--format` | [`seq -f %5.2c 1 3`](https://uutils.org/playground/?cmd=seq+-f+%255.2c+1+3) |
| `env`    | the failing part of a `-S`/`--split-string` string | [`env -S 'echo ${1FOO}'`](https://uutils.org/playground/?cmd=env+-S+%27echo+%24%7B1FOO%7D%27) |
| `cut`    | the failing range in the list given to `-b`, `-c`, `-f` or `-F` | [`cut -f 1,4-2 fruits.txt`](https://uutils.org/playground/?cmd=cut+-f+1%2C4-2+fruits.txt) |
| `split`  | the failing part of the SIZE given to `-b`, `-C` or `-l` | [`split -b 7zq fruits.txt`](https://uutils.org/playground/?cmd=split+-b+7zq+fruits.txt) |
| `shred`  | the failing part of the SIZE given to `-s`/`--size` | [`shred -s 4vv fruits.txt`](https://uutils.org/playground/?cmd=shred+-s+4vv+fruits.txt) |
| `head`   | the failing part of the SIZE given to `-c` or `-n` | [`head -c 1fb fruits.txt`](https://uutils.org/playground/?cmd=head+-c+1fb+fruits.txt) |
| `tail`   | the failing part of the SIZE given to `-c` or `-n` | [`tail -c 1fb fruits.txt`](https://uutils.org/playground/?cmd=tail+-c+1fb+fruits.txt) |
| `truncate` | the failing part of the SIZE given to `-s`/`--size` | [`truncate -s 10fb fruits.txt`](https://uutils.org/playground/?cmd=truncate+-s+10fb+fruits.txt) |
| `od`     | the failing part of the SIZE given to `-j`, `-N`, `-S` or `-w` | [`od -N 3zz fruits.txt`](https://uutils.org/playground/?cmd=od+-N+3zz+fruits.txt) |
| `stdbuf` | the failing part of the buffering mode given to `-i`, `-o` or `-e` | [`stdbuf -o 6pq head`](https://uutils.org/playground/?cmd=stdbuf+-o+6pq+head) |

## How it works

The rendering lives in `uucore::features::diagnostics` and is built on
[ariadne](https://crates.io/crates/ariadne). A utility that opts in:

1. Takes a `Snapshot` of its argument list. The snapshot joins the arguments
   into one line (quoting where needed, and rendering non-UTF-8 arguments the
   way the shell would) and remembers the byte range of each argument.
2. Maps its own error type to a position, and optionally a label and a line
   of advice, in a small per-utility `diagnostics.rs` module. A label is only
   used when it adds something the message does not say — an expectation, or
   a fix such as tr's `did you mean 'b-y'?` — never to restate it; with no
   label the span is drawn as a bare underline. Everything user-facing is
   passed in already localized.
3. Locates the argument the operand came from — with
   `Snapshot::index_of_value` for an option's value in whatever spelling it
   was given (`-k 2.3x`, `-k2.3x`, `-rk2.3x`, `--key 2.3x`, `--key=2.3x`),
   `Snapshot::index_of_positional` for a positional operand, or an index the
   utility tracked itself — and calls `Snapshot::render` to point at the
   whole argument, or `Snapshot::render_inside_at` to point at a byte range
   *inside* the operand it carries. Because the argument is named rather than
   searched for, a file, another option, or the program name that happens to
   share the operand's text can never take the caret.
   `Snapshot::render_option_value` does the two steps in one, which is what
   most option values want.

An argument holding a space is echoed back quoted, and the caret still points
inside it: the quotes only wrap the operand, so its bytes are found where they
were printed and offsets count from there. An argument that could not be
printed as-is — a non-UTF-8 one, or one whose quoting had to be broken up — is
underlined as a whole instead, since no offset into it would line up with what
the reader sees.

Callers check `diagnostics::enabled()` first, so the non-interactive path
costs nothing and keeps its GNU-compatible message.

Errors that come out of a shared parser are rendered by a shared helper, so
that a syntax common to several utilities is explained the same way in all of
them, and its labels live in the common locale strings rather than being
repeated per utility. Three parsers work this way:

- **Modes** (`uucore::mode`), for `chmod`, `mkdir`, `mkfifo`, `mknod` and
  `install`.
- **Range lists** (`uucore::ranges`), for `cut`'s `-b`, `-c` and `-f` and for
  `numfmt --field`. `Range::from_list` reports which item of the list failed
  and where it sat.
- **Sizes** (`uucore::parser::parse_size`), for `head`, `tail`, `truncate`,
  `split`, `shred`, `stdbuf`, `sort` and `od` today, and available to the other callers of the parser.
  `ParseSizeError::span` works out from the operand which of its two parts —
  the number or the unit — was rejected, so the error type keeps the shape its
  callers build by hand.

Taking modes as the worked example: the parser reports errors as a structured
`ModeError` carrying the byte range at fault, and its rendering places the
caret for every utility that takes a mode — `ModeError::render_option_value`
finds the mode as the value of `-m`/`--mode` for `mkdir`, `mkfifo`, `mknod`
and `install`, while `chmod`, which accepts modes clap cannot see (such as
`chmod -w -r file`), tracks where each mode operand sat and passes the index
to `ModeError::render_at`.
