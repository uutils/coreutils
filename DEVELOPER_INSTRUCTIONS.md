Code Coverage Report Generation
---------------------------------

Code coverage report can be generated using [grcov](https://github.com/mozilla/grcov).

### Using Nightly Rust

To generate [gcov-based](https://github.com/mozilla/grcov#example-how-to-generate-gcda-files-for-cc) coverage report

```bash
$ export CARGO_INCREMENTAL=0
$ export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
$ export RUSTDOCFLAGS="-Cpanic=abort"
$ cargo build <options...> # e.g., --features feat_os_unix
$ cargo test <options...> # e.g., --features feat_os_unix test_pathchk
$ grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing --ignore build.rs --excl-br-line "^\s*((debug_)?assert(_eq|_ne)?\#\[derive\()" -o ./target/debug/coverage/
$ # open target/debug/coverage/index.html in browser
```

if changes are not reflected in the report then run `cargo clean`  and run the above commands.

### Using Stable Rust

If you are using stable version of Rust that doesn't enable code coverage instrumentation by default 
then add `-Z-Zinstrument-coverage` flag to `RUSTFLAGS` env variable specified above.


pre-commit hooks
----------------

A configuration for `pre-commit` is provided in the repository. It allows automatically checking every git commit you make to ensure it compiles, and passes `clippy` and `rustfmt` without warnings.

To use the provided hook:

1. [Install `pre-commit`](https://pre-commit.com/#install)
2. Run `pre-commit install` while in the repository directory

Your git commits will then automatically be checked. If a check fails, an error message will explain why, and your commit will be canceled. You can then make the suggested changes, and run `git commit ...` again.
