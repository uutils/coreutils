# Contributing to coreutils

Contributions are very welcome, and should target Rust's master branch until the
standard libraries are stabilized. You may *claim* an item on the to-do list by
following these steps:

1. Open an issue named "Implement [the utility of your choice]", e.g. "Implement ls"
2. State that you are working on this utility.
3. Develop the utility.
4. Add integration tests.
5. Add the reference to your utility into Cargo.toml and Makefile.
6. Remove utility from the to-do list in the README.
7. Submit a pull request and close the issue.

The steps above imply that, before starting to work on a utility, you should
search the issues to make sure no one else is working on it.

## Commit messages

To help the project maintainers review pull requests from contributors across
numerous utilites, the team has settled on conventions for commit messages.

From http://git-scm.com/book/ch5-2.html:

```
Short (50 chars or less) summary of changes

More detailed explanatory text, if necessary.  Wrap it to about 72
characters or so.  In some contexts, the first line is treated as the
subject of an email and the rest of the text as the body.  The blank
line separating the summary from the body is critical (unless you omit
the body entirely); tools like rebase can get confused if you run the
two together.

Further paragraphs come after blank lines.

  - Bullet points are okay, too

  - Typically a hyphen or asterisk is used for the bullet, preceded by a
    single space, with blank lines in between, but conventions vary here
```

Furthermore, here are a few examples for a summary line:

* commit for a single utility

```
nohup: cleanup and refactor
```

* commit for a utility's tests

```
tests/rm: test new feature
```

Beyond changes to an individual utility or its tests, other summary
lines for non-utility modules include:

```
README: add help
```

```
travis: fix build
```

```
uucore: add new modules
```

```
uutils: add new utility
```

```
gitignore: add temporary files
```
