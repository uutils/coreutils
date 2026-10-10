# AGENTS.md

Writing this file is not a statement for or against using AI coding agents.
People are using them in this repository either way, so this file exists to make
sure that when they do, the project's rules are actually followed.

If you are driving an agent here, you are responsible for its output. Read the
diff before you send it.

Responses to reviewers should be written by a human, not an agent.

## Start with an issue

Before making any code changes:

- Search open issues for one matching the requested work.
- Read the issue and confirm its scope covers the intended changes.
- Review related issues and PRs for context.
- If no matching issue exists, ask your human operator to provide one before
  proceeding.

If a PR already exists for the work, continue using it. Update its branch and
rebase as needed.

When opening a PR, reference the issue in its description. If the PR fully
resolves the issue, include Closes #XXXXX, using the actual issue number.

## Never read or copy GNU code

GNU coreutils is GPLv3; this project is MIT. Any code derived from it - even a
few lines, a helper structure, a test fixture or a comment - cannot be accepted.

## A PR needs tests

New behavior or a bug fix comes with a test, in `tests/by-util/test_<util>.rs`
or as a unit test next to the code. If a GNU test used to fail and now passes,
add a Rust test so it cannot silently regress.

No tests, no merge.

## Keep the PR description short

Describe the problem being solved and what changed. That is all.

- No generated walls of text, no bullet-point summaries of every hunk, no
  emoji-headed sections.
- Title: `<util>: <what changed>`, e.g. `ls: fix version sort order`.
- Write issue reports, PR descriptions and replies to reviewers in your own
  words. The point of review is to check that a human understands the change.

## Read the docs already in this repo

Before changing anything, look at the Markdown files here - they are the actual
rules, this file is only a pointer:

- `CONTRIBUTING.md` - licensing, commit hygiene, what a PR must contain
- `DEVELOPMENT.md` - build, test and environment setup
- `README.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`
- `docs/src/*.md` - platforms, performance, l10n, multicall, test coverage
- the per-utility `README.md` / `locales/*.ftl` files when touching a utility
