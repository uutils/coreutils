<!-- spell-checker:ignore numfmt OPTS ariadne mydir mypipe mydev ucase iflag oflag nocreat notrunc fdatasync noatime noctty -->
<!-- markdownlint-disable MD033 -->

# Error diagnostics

GNU coreutils reports every error as a single line on stderr. That line
answers *what* went wrong, but Not *where*. For utilities whose arguments form
a small language of their own - a `test` expression, a `chmod` mode, a `sort`
key - the interesting question is usually which argument, or which character
inside an argument, broke the parse.

When stderr is a terminal, uutils renders these errors as a compiler-style
report instead: the command line is echoed back as a source line, and a caret
points at the part that is at fault, often with a line of advice. Everywhere
else - a script, a pipe, a test harness - the plain one-line message is kept,
so nothing that reads stderr can tell the difference.

<link rel="stylesheet" href="extensions-errors.css">

## Before and after

Each utility below shows the plain one-line message first - what uutils
prints when stderr is not a terminal, and in most cases exactly what GNU
prints - then the report rendered on a terminal, colors included.

### `test`

Before:

<pre class="diag"><span class="a-p">$</span> test 7 -eq zap
test: invalid integer &#x27;zap&#x27;</pre>

After:

<pre class="diag"><span class="a-p">$</span> test 7 -eq zap
test: invalid integer &#x27;zap&#x27;
   <span class="a-d">╭─[</span> test:1:7 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">7 -eq </span><span class="a-e">zap</span>
 <span class="a-f">  │</span>       <span class="a-e">───</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: -eq, -ne, -lt, -le, -gt and -ge compare integers; use =, !=, &lt; or &gt; to compare strings
 <span class="a-f">  │</span>       -eq equal, -ne not equal, -lt less than, -le less than or equal, -gt greater than, -ge greater than or equal
<span class="a-d">───╯</span></pre>

[Try it in the playground](https://uutils.org/playground/?cmd=test+7+-eq+zap).

### `cut`

A list of ranges is often long, and only one item in it is wrong:

Before:

<pre class="diag"><span class="a-p">$</span> cut -f 1,4-2,9-12 notes.txt
cut: invalid decreasing range</pre>

After:

<pre class="diag"><span class="a-p">$</span> cut -f 1,4-2,9-12 notes.txt
cut: invalid decreasing range
   <span class="a-d">╭─[</span> cut:1:10 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">cut -f 1,</span><span class="a-e">4-2</span><span class="a-s">,9-12 notes.txt</span>
 <span class="a-f">  │</span>          <span class="a-e">─┬─</span>
 <span class="a-f">  │</span>           <span class="a-e">╰───</span> this range ends before it starts
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a list is N, N-M, N- or -M, separated by commas, as in -f1,4-6,9-
<span class="a-d">───╯</span></pre>

### `expr`

Before:

<pre class="diag"><span class="a-p">$</span> expr 9 + foo
expr: non-integer argument</pre>

After:

<pre class="diag"><span class="a-p">$</span> expr 9 + foo
expr: non-integer argument
   <span class="a-d">╭─[</span> expr:1:5 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">9 + </span><span class="a-e">foo</span>
 <span class="a-f">  │</span>     <span class="a-e">───</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: arithmetic operators need integers; use = or != to compare strings instead
<span class="a-d">───╯</span></pre>

[Try it in the playground](https://uutils.org/playground/?cmd=expr+9+%2B+foo).

### `chmod`

The caret can point *inside* an argument, at the exact character that broke
the parse:

Before:

<pre class="diag"><span class="a-p">$</span> chmod &#x27;g+rw?x&#x27; notes.txt
chmod: invalid operator (expected +, -, or =, but found ?)</pre>

After:

<pre class="diag"><span class="a-p">$</span> chmod &#x27;g+rw?x&#x27; notes.txt
chmod: invalid operator (expected +, -, or =, but found ?)
   <span class="a-d">╭─[</span> chmod:1:5 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">g+rw</span><span class="a-e">?</span><span class="a-s">x notes.txt</span>
 <span class="a-f">  │</span>     <span class="a-e">─</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a mode is either octal, as in 644, or clauses such as u+rwx,go-w
<span class="a-d">───╯</span></pre>

### `tr`

Before:

<pre class="diag"><span class="a-p">$</span> tr &#x27;qw[y-b]&#x27; x
tr: range-endpoints of &#x27;y-b&#x27; are in reverse collating sequence order</pre>

After:

<pre class="diag"><span class="a-p">$</span> tr &#x27;qw[y-b]&#x27; x
tr: range-endpoints of &#x27;y-b&#x27; are in reverse collating sequence order
   <span class="a-d">╭─[</span> tr:1:7 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">tr qw[</span><span class="a-e">y-b</span><span class="a-s">] x</span>
 <span class="a-f">  │</span>       <span class="a-e">─┬─</span>
 <span class="a-f">  │</span>        <span class="a-e">╰───</span> did you mean &#x27;b-y&#x27;?
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a range goes from the lower character to the higher one, as in a-z
<span class="a-d">───╯</span></pre>

[Try it in the playground](https://uutils.org/playground/?cmd=tr+%27qw%5By-b%5D%27+x).

### `csplit`

A pattern is a small language of its own, and the regex engine already knows
which character of it it choked on - so the caret can say so too:

Before:

<pre class="diag"><span class="a-p">$</span> csplit notes.txt &#x27;/a{2,1}/&#x27;
csplit: &#x27;/a{2,1}/&#x27;: invalid pattern</pre>

After:

<pre class="diag"><span class="a-p">$</span> csplit notes.txt &#x27;/a{2,1}/&#x27;
csplit: &#x27;/a{2,1}/&#x27;: invalid pattern
   <span class="a-d">╭─[</span> csplit:1:20 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">csplit notes.txt /a</span><span class="a-e">{2,1}</span><span class="a-s">/</span>
 <span class="a-f">  │</span>                    <span class="a-e">──┬──</span>
 <span class="a-f">  │</span>                      <span class="a-e">╰──── </span>invalid repetition count range, the start must be &lt;= the end
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a pattern is a line number N, /REGEXP/[OFFSET] or %REGEXP%[OFFSET], each optionally followed by {N} or {*}
<span class="a-d">───╯</span></pre>

The label is quoted from the regex engine rather than translated, since that
is the only place the wording exists.

### `sort`

Before:

<pre class="diag"><span class="a-p">$</span> sort -k2.3x notes.txt
sort: stray character in field spec: invalid field specification &#x27;2.3x&#x27;</pre>

After:

<pre class="diag"><span class="a-p">$</span> sort -k2.3x notes.txt
sort: stray character in field spec: invalid field specification &#x27;2.3x&#x27;
   <span class="a-d">╭─[</span> sort:1:11 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">sort -k2.3</span><span class="a-e">x</span><span class="a-s"> notes.txt</span>
 <span class="a-f">  │</span>           <span class="a-e">─</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a key is FIELD[.CHAR][OPTS][,FIELD[.CHAR][OPTS]], as in -k2.3,4nr
<span class="a-d">───╯</span></pre>

[Try it in the playground](https://uutils.org/playground/?cmd=sort+-k2.3x+fruits.txt).

### `numfmt`

Before:

<pre class="diag"><span class="a-p">$</span> numfmt --format=%q 1000
numfmt: invalid format &#x27;%q&#x27;, directive must be %[0][&#x27;][-][N][.][N]f</pre>

After:

<pre class="diag"><span class="a-p">$</span> numfmt --format=%q 1000
numfmt: invalid format &#x27;%q&#x27;, directive must be %[0][&#x27;][-][N][.][N]f
   <span class="a-d">╭─[</span> numfmt:1:18 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">numfmt --format=%</span><span class="a-e">q</span><span class="a-s"> 1000</span>
 <span class="a-f">  │</span>                  <span class="a-e">┬</span>
 <span class="a-f">  │</span>                  <span class="a-e">╰──</span> f is the only conversion numfmt has; %d, %e, %g and the other C conversions are not accepted
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a format is [PREFIX]%[0][&#x27;][-][WIDTH][.PRECISION]f[SUFFIX], as in &quot;%&#x27;-10.2f&quot;
<span class="a-d">───╯</span></pre>

[Try it in the playground](https://uutils.org/playground/?cmd=numfmt+--format%3D%25q+1000).

### `printf`

Before:

<pre class="diag"><span class="a-p">$</span> printf %5.2c q
printf: %5.2c: invalid conversion specification</pre>

After:

<pre class="diag"><span class="a-p">$</span> printf %5.2c q
printf: %5.2c: invalid conversion specification
   <span class="a-d">╭─[</span> printf:1:8 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">printf </span><span class="a-e">%5.2c</span><span class="a-s"> q</span>
 <span class="a-f">  │</span>        <span class="a-e">─────</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: %d, %s, %x, %f and the other C conversions are accepted, plus %b and %q; a literal % is written %%
<span class="a-d">───╯</span></pre>

The same goes for a broken escape: `printf 'a\xzb'` puts the caret under the
`\x` that is missing its hexadecimal digits.

[Try it in the playground](https://uutils.org/playground/?cmd=printf+%255.2c+q).

### `env`

`env -S` takes a whole command line and splits it as a shell would. Its
messages name an offset, which is precisely the thing a caret can show
instead:

Before:

<pre class="diag"><span class="a-p">$</span> env -S &#x27;echo ${1FOO}&#x27;
env: only ${VARNAME} expansion is supported, error at: ${1FOO}</pre>

After:

<pre class="diag"><span class="a-p">$</span> env -S &#x27;echo ${1FOO}&#x27;
env: only ${VARNAME} expansion is supported, error at: ${1FOO}
   <span class="a-d">╭─[</span> env:1:14 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">env -S &#x27;echo </span><span class="a-e">${1</span><span class="a-s">FOO}&#x27;</span>
 <span class="a-f">  │</span>              <span class="a-e">─┬─</span>
 <span class="a-f">  │</span>               <span class="a-e">╰───</span> a variable name cannot start with a digit
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: only $NAME and ${NAME} are expanded; the other shell forms are not
<span class="a-d">───╯</span></pre>

Note that the `-S` string holds spaces, so it is echoed back quoted - and the
caret still points inside it.

### `head`

A SIZE is a number and a unit, and the caret says which of the two was
rejected:

Before:

<pre class="diag"><span class="a-p">$</span> head -c 1fb notes.txt
head: invalid number of bytes: &#x27;1fb&#x27;</pre>

After:

<pre class="diag"><span class="a-p">$</span> head -c 1fb notes.txt
head: invalid number of bytes: &#x27;1fb&#x27;
   <span class="a-d">╭─[</span> head:1:10 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">head -c 1</span><span class="a-e">fb</span><span class="a-s"> notes.txt</span>
 <span class="a-f">  │</span>          <span class="a-e">─┬</span>
 <span class="a-f">  │</span>           <span class="a-e">╰──</span> not a known unit
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a size is a number and an optional unit: K, M, G and so on for 1024, KB, MB, GB for 1000
<span class="a-d">───╯</span></pre>

### `dd`

Every `dd` operand is a `KEY=VALUE` pair, and a value can be a comma-separated
list of flags, so there are three things the caret can pick out: the key, the
whole value, or one flag inside the list. A flag is underlined where it sits
in the list rather than wherever its text first turns up, and the advice names
the flags that operand accepts:

Before:

<pre class="diag"><span class="a-p">$</span> dd conv=ucase,zap
dd: invalid conversion: &#x27;zap&#x27;</pre>

After:

<pre class="diag"><span class="a-p">$</span> dd conv=ucase,zap
dd: invalid conversion: &#x27;zap&#x27;
   <span class="a-d">╭─[</span> dd:1:15 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">dd conv=ucase,</span><span class="a-e">zap</span>
 <span class="a-f">  │</span>               <span class="a-e">─┬─</span>
 <span class="a-f">  │</span>                <span class="a-e">╰───</span> not a known conversion
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: conv= is one of ascii, ebcdic, ibm, lcase, ucase, block, unblock, swab, sync, noerror, sparse, excl, nocreat, notrunc, fdatasync or fsync
<span class="a-d">───╯</span></pre>

`iflag=` and `oflag=` are reported apart, so an output flag is no longer
blamed on the input, and each lists its own flags:

Before:

<pre class="diag"><span class="a-p">$</span> dd oflag=zap
dd: invalid output flag: &#x27;zap&#x27;</pre>

After:

<pre class="diag"><span class="a-p">$</span> dd oflag=zap
dd: invalid output flag: &#x27;zap&#x27;
   <span class="a-d">╭─[</span> dd:1:10 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">dd oflag=</span><span class="a-e">zap</span>
 <span class="a-f">  │</span>          <span class="a-e">─┬─</span>
 <span class="a-f">  │</span>           <span class="a-e">╰───</span> not a known output flag
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: oflag= is one of direct, directory, dsync, sync, nocache, nonblock, noatime, noctty, nofollow, append or seek_bytes
<span class="a-d">───╯</span></pre>

An unknown key is underlined without its value, since the value is not what
was rejected:

Before:

<pre class="diag"><span class="a-p">$</span> dd zap=1
dd: unrecognized operand &#x27;zap=1&#x27;</pre>

After:

<pre class="diag"><span class="a-p">$</span> dd zap=1
dd: unrecognized operand &#x27;zap=1&#x27;
   <span class="a-d">╭─[</span> dd:1:4 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">dd </span><span class="a-e">zap</span><span class="a-s">=1</span>
 <span class="a-f">  │</span>    <span class="a-e">───</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: an operand is KEY=VALUE, as in if=file bs=4k count=10
<span class="a-d">───╯</span></pre>

A number that does not fit is rejected rather than quietly clamped, and the
caret covers the whole value:

Before:

<pre class="diag"><span class="a-p">$</span> dd count=99999999999999999999999
dd: invalid number: &#x27;99999999999999999999999&#x27;: Value too large for defined data type</pre>

After:

<pre class="diag"><span class="a-p">$</span> dd count=99999999999999999999999
dd: invalid number: &#x27;99999999999999999999999&#x27;: Value too large for defined data type
   <span class="a-d">╭─[</span> dd:1:10 <span class="a-d">]</span>
   <span class="a-d">│</span>
 <span class="a-d">1 │</span> <span class="a-s">dd count=</span><span class="a-e">99999999999999999999999</span>
 <span class="a-f">  │</span>          <span class="a-e">───────────────────────</span>
 <span class="a-f">  │</span>
 <span class="a-f">  │</span> <span class="a-h">Help</span>: a number may be followed by a multiplier: c, w, b, then K, M, G and so on for 1024, kB, MB, GB for 1000
<span class="a-d">───╯</span></pre>

[Try it in the playground](https://uutils.org/playground/?cmd=dd+conv%3Ducase%2Czap).

## Compatibility

This is strictly an interactive nicety; nothing that reads our output can tell
the difference:

- Reports are only rendered when **stderr is a terminal**, unless `UUTILS_DIAG`
  says otherwise (see below). In a script, a pipe, or a test harness, the
  utility keeps printing its plain one-line message, so existing scripts that
  match on stderr keep working.
- A report is drawn only when the error can be **tied to something on the
  command line**. When it cannot - the error is not about any one argument, or
  the operand it names was rewritten or consumed before it could be located -
  the plain one-line message is printed instead, even at a terminal. A utility
  in the table below therefore reports the parse errors it can place, not
  every error it has.
- Exit codes are unchanged, and so is the `Try '... --help'` hint: an error
  that is a usage error still prints it under the report.
- Colors follow the usual conventions: they are used only on a terminal, and
  [`NO_COLOR`](https://no-color.org/) disables them.
- All messages, labels and help lines are localized like the rest of uutils
  (see [Localization](l10n.md)).
- The rendering can be compiled out entirely: it sits behind the
  `feat_diagnostics` cargo feature, which is on by default. Building with
  `--no-default-features` (plus a `feat_os_*` selection) drops the renderer
  and its `ariadne` dependency, and every utility keeps its plain one-line
  messages.

## Turning it on and off

The default keys off stderr being a terminal, and nothing else - which is
usually what you want, but not always. `UUTILS_DIAG` overrides it:

| Value | Effect |
| ----- | ------ |
| `always` | Draw the report even when stderr is a file or a pipe. |
| `never` | Keep the plain one-line message even at a terminal. |
| `auto`, unset, anything else | Decide from stderr, as above. |

An unrecognized value is deliberately not an error - this is the kind of
variable that gets exported from a shell profile once and forgotten, and no
spelling of it should be able to make a utility fail.

`always` is the one to reach for when the error has to leave the terminal
it happened in - a CI log, or a report to paste into a bug:

```
$ UUTILS_DIAG=always sort -k2.3x notes.txt 2> parse.log
```

Colors are a separate question, and one the terminal still answers: a report
forced into a file is written without them, so nothing has to strip escape
sequences back out. [`NO_COLOR`](https://no-color.org/) is the middle setting
at a terminal - the report is still drawn, just in plain text.

Without the variable, both directions are still one command away. To get the
plain line while sitting at a terminal, send stderr somewhere that is not one:

```
$ sort -k2.3x notes.txt 2>&1 | cat
sort: stray character in field spec: invalid field specification '2.3x'
```

And to get a report out of a command that has to run under a real terminal,
give it a pty with `script -qec "..." /dev/null`, or `unbuffer` from expect.

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
| `stat`   | the failing directive of a `-c`/`--format` or `--printf` format | [`stat -c %d%.3 fruits.txt`](https://uutils.org/playground/?cmd=stat+-c+%25d%25.3+fruits.txt) |
| `env`    | the failing part of a `-S`/`--split-string` string | [`env -S 'echo ${1FOO}'`](https://uutils.org/playground/?cmd=env+-S+%27echo+%24%7B1FOO%7D%27) |
| `dd`     | the failing key, value or flag of a `KEY=VALUE` operand | [`dd conv=ucase,zap`](https://uutils.org/playground/?cmd=dd+conv%3Ducase%2Czap) |
| `join`   | the failing field of the output format given to `-o` | [`join -o 1.2,2.x fruits.txt fruits.txt`](https://uutils.org/playground/?cmd=join+-o+1.2%2C2.x+fruits.txt+fruits.txt) |
| `cut`    | the failing range in the list given to `-b`, `-c`, `-f` or `-F` | [`cut -f 1,4-2 fruits.txt`](https://uutils.org/playground/?cmd=cut+-f+1%2C4-2+fruits.txt) |
| `csplit` | the failing pattern operand, the character of its regex that broke, or the format given to `-b`/`-n` | [`csplit fruits.txt '/a(b/'`](https://uutils.org/playground/?cmd=csplit+fruits.txt+%27%2Fa%28b%2F%27) |
| `split`  | the failing part of the SIZE given to `-b`, `-C` or `-l` | [`split -b 7zq fruits.txt`](https://uutils.org/playground/?cmd=split+-b+7zq+fruits.txt) |
| `shred`  | the failing part of the SIZE given to `-s`/`--size` | [`shred -s 4vv fruits.txt`](https://uutils.org/playground/?cmd=shred+-s+4vv+fruits.txt) |
| `head`   | the failing part of the SIZE given to `-c` or `-n` | [`head -c 1fb fruits.txt`](https://uutils.org/playground/?cmd=head+-c+1fb+fruits.txt) |
| `tail`   | the failing part of the SIZE given to `-c` or `-n` | [`tail -c 1fb fruits.txt`](https://uutils.org/playground/?cmd=tail+-c+1fb+fruits.txt) |
| `truncate` | the failing part of the SIZE given to `-s`/`--size` | [`truncate -s 10fb fruits.txt`](https://uutils.org/playground/?cmd=truncate+-s+10fb+fruits.txt) |
| `od`     | the failing part of the SIZE given to `-j`, `-N`, `-S` or `-w` | [`od -N 3zz fruits.txt`](https://uutils.org/playground/?cmd=od+-N+3zz+fruits.txt) |
| `du`     | the failing part of the SIZE given to `-B`/`--block-size` or `-t`/`--threshold` | [`du -B 1fb`](https://uutils.org/playground/?cmd=du+-B+1fb) |
| `df`     | the failing part of the SIZE given to `-B`/`--block-size` | [`df -B 1fb`](https://uutils.org/playground/?cmd=df+-B+1fb) |
| `ls`     | the failing part of the SIZE given to `--block-size` (also `dir` and `vdir`) | [`ls --block-size=1fb`](https://uutils.org/playground/?cmd=ls+--block-size%3D1fb) |
| `stdbuf` | the failing part of the buffering mode given to `-i`, `-o` or `-e` | [`stdbuf -o 6pq head`](https://uutils.org/playground/?cmd=stdbuf+-o+6pq+head) |

## How it works

The rendering lives in `uucore::features::diagnostics` and is built on
[ariadne](https://crates.io/crates/ariadne). A utility that opts in:

1. Takes a `Snapshot` of its argument list. The snapshot joins the arguments
   into one line (quoting where needed, and rendering non-UTF-8 arguments the
   way the shell would) and remembers the byte range of each argument.
2. Maps its own error type to a position, and optionally a label and a line
   of advice, in a small per-utility `diagnostics.rs` module. A label is only
   used when it adds something the message does not say - an expectation, or
   a fix such as tr's `did you mean 'b-y'?` - never to restate it; with no
   label the span is drawn as a bare underline. Everything user-facing is
   passed in already localized.
3. Locates the argument the operand came from - with
   `Snapshot::index_of_value` for an option's value in whatever spelling it
   was given (`-k 2.3x`, `-k2.3x`, `-rk2.3x`, `--key 2.3x`, `--key=2.3x`),
   `Snapshot::index_of_positional` for a positional operand, or an index the
   utility tracked itself - and calls `Snapshot::render` to point at the
   whole argument, or `Snapshot::render_inside_at` to point at a byte range
   *inside* the operand it carries. Because the argument is named rather than
   searched for, a file, another option, or the program name that happens to
   share the operand's text can never take the caret.
   `Snapshot::render_option_value` does the two steps in one, which is what
   most option values want.

An argument holding a space is echoed back quoted, and the caret still points
inside it: the quotes only wrap the operand, so its bytes are found where they
were printed and offsets count from there. An argument that could not be
printed as-is - a non-UTF-8 one, or one whose quoting had to be broken up - is
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
  `split`, `shred`, `stdbuf`, `sort`, `od`, `du`, `df` and `ls` (with `dir`
  and `vdir`) today, and available to the other callers of the parser.
  `ParseSizeError::span` works out from the operand which of its two parts -
  the number or the unit - was rejected, so the error type keeps the shape its
  callers build by hand.

Taking modes as the worked example: the parser reports errors as a structured
`ModeError` carrying the byte range at fault, and its rendering places the
caret for every utility that takes a mode - `ModeError::render_option_value`
finds the mode as the value of `-m`/`--mode` for `mkdir`, `mkfifo`, `mknod`
and `install`, while `chmod`, which accepts modes clap cannot see (such as
`chmod -w -r file`), tracks where each mode operand sat and passes the index
to `ModeError::render_at`.
