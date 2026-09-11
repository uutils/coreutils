---
name: code-review
description: Review a pull request or diff in uutils/coreutils. Checks GPL/GNU provenance, GNU behavior compatibility, tests, error handling, style, performance and commit hygiene, and produces short, actionable, line-level comments.
---

# Code review for uutils/coreutils

uutils coreutils is a cross-platform reimplementation of the GNU coreutils in Rust.
Reviews are about keeping it *original*, *GNU-compatible*, *tested*, and *small*.

## How to write the review

- One-line, actionable comments anchored to the specific line. Say what to change,
  not why it matters in three paragraphs.
- Push back on long-winded code, duplication, needless complexity, and changes
  without tests.
- Don't restate the diff back to the author, don't praise, don't pad.
- Keep the tone that of a human reviewer: plain, direct, no generated-sounding prose.
- If nothing is wrong, say so in one line rather than inventing findings.

## The one rule that cannot be bent

uutils is **original** code. It cannot contain code derived from GNU or any other
GPL/LGPL implementation, and a PR must not even link to the GNU source.

Flag as blocking:

- Code, structure, comments, variable names or file layout that mirror GNU's.
- Tests ported verbatim from the GNU test suite — including the *fixtures*: input
  strings, byte sequences, delimiters, field ranges and expected outputs. "It's only
  a few chars" is not an exception; GNU tests are GPLv3+.
- Links or references to GNU source/test paths in code comments, commit messages or
  the PR description.

Reading the GNU *manual* and man pages is fine, as are permissively-licensed
implementations (Apple file_cmds, OpenBSD). Be extra careful with AI-assisted
patches: assistants can reproduce GPL sources verbatim.

## Correctness and GNU compatibility

- Behavior, options, output and exit codes should match GNU, checked
  with `LC_ALL=C` (GNU messages are localized; the GNU tests run under `C`).
  Error messages can be different when better.
- A "GNU does X" claim in a PR should name the version it was checked
  against. And it should be a recent version.
- GNU compatibility must not regress (`util/run-gnu-test.sh`,
  `util/remaining-gnu-error.py`). A compatibility fix should mention the test it makes
  pass.
- Watch security-sensitive output that scripts parse: character classes
  (`[:graph:]`, `[:print:]`, `[:alnum:]`) in `tr`/`sort`/`cut`, real vs effective
  uid/gid in `id`/`groups`/`stat`/`whoami`, `--preserve-root` canonicalization,
  and flags that must work independently of each other (e.g. `ln --no-dereference`
  without `--force`).

## Safety

- **TOCTOU**: reject check-then-act on paths (stat/exists/permission check followed by
  a separate open/unlink/chmod). Prefer `openat()` / `O_NOFOLLOW` / fd-relative
  operations, especially in `cp`, `mv`, `rm`, `chown`, `chmod`, `chgrp`, `install`.
- Recursive traversal must not follow symlinks out of the intended tree.
- Untrusted input: no unbounded allocations, no integer overflow, no path-traversal
  foot-guns.

## Rust style

- No `panic!`, `.unwrap()`, `expect()` on fallible paths, or stray `println!`.
  `unreachable!` needs a comment justifying why the branch can't happen.
- Avoid `std::process::exit` in reusable utility logic; return `Result` and use `uucore::error`. Allow process-level entry points and platform/tooling paths that must terminate explicitly.
- `OsStr`/`Path` for paths, not `String`/`str`.
- Keep `unsafe` minimal; prefer FFI, allow rare documented non-FFI exceptions, and require a `// SAFETY:` comment.
- `thiserror`, not `quick-error`; `rustix` preferred over `nix`.
- Strip `(os error N)` from messages (`strip_errno`).
- Descriptive names, no cryptic abbreviations. Avoid single-call wrapper functions
  and gratuitous macros.
- Do not use `#[allow(...)]` to hide warnings without a documented, narrowly scoped reason; prefer `#[expect(..., reason = "...")]` for justified exceptions.
- Clippy runs with `all` + `cargo` + `pedantic` + `use_self`; CI fails on any warning.
  Common ones worth catching in review: `redundant_closure_for_method_calls`,
  `map_unwrap_or`, `needless_for_each`, `items_after_statements`, `unnecessary_wraps`,
  `assigning_clones`, `needless_continue`, `unreadable_literal`.

## Tests

- New or change of behavior comes with a test. Integration tests live in
  `tests/by-util/test_<utility>.rs`; unit tests next to the code.
- Tests must be original (see the provenance rule above) and pick values that are
  clearly distinct from GNU's fixtures.
- Use `.no_output()` rather than `.no_stdout().no_stderr()`.
- Anything that can't run under WASI needs `#[cfg_attr(wasi_runner, ignore)]`.
- If a GNU test was failing while the Rust suite passed, the PR should add a Rust
  test so the gap can't reopen.

## Docs, help and i18n

- New options or user-visible behavior should update the relevant `--help`/docs sources and generated man page as applicable, plus the appropriate `en-US.ftl` (utility, `uucore`, or shared crate).
- User-facing strings go through `translate!` with a namespaced key, not hardcoded.

## Performance and size

- Compare runtime changes with representative benchmarks, and apply the repository's established binary-size threshold rather than a blanket 3% limit.
- Extra memory is acceptable when it buys real speed or correctness.
- Watch for per-item work added to hot loops: new allocations, an extra pass over the
  data, newly non-inlined cross-crate calls, and — very common — error context
  (`format!`, `translate!`, `to_string()`) built eagerly on the *success* path.
  Error messages must be materialized only inside the `Err` arm.

## Scope, dependencies and commits

- Avoid code duplication at all costs. Look in uucore if the feature/function isn't there.
- Small and self-contained. Unrelated refactors, formatting-only churn, and
  dependency/lockfile bumps belong in their own PR. Pure code moves get their own commit.
- When a PR mixes several logical changes, ask for a split and point the author at the
  `gh stack` extension to keep the pieces reviewable in order.
- New dependencies must be discussed and justified: build time, binary size, audit
  surface, license compatibility.
- Descriptive title prefixed with the utility: `ls: fix version sort order`, not
  `Fix #1234`. Same for commits: `<program>: <description>`.

## Before approving

CI green (fmt, clippy `-D warnings`, tests) on Linux, macOS and Windows, with
`#[cfg(...)]` used for platform-specific code rather than breaking other targets.
