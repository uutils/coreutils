@setLocal
@echo off

@rem ::# spell-checker:ignore (abbrevs/acronyms) gcno
@rem ::# spell-checker:ignore (CMD) COMSPEC ERRORLEVEL
@rem ::# spell-checker:ignore (jargon) toolchain
@rem ::# spell-checker:ignore (rust) Ccodegen Cinline Coverflow RUSTC RUSTFLAGS RUSTUP
@rem ::# spell-checker:ignore (utils) genhtml grcov lcov sccache uutils

set BIN=uutils

set "FEATURES_OPTION=--features windows"

cd "%~dp0.."
call echo [ "%CD%" ]

call cargo clean

set CARGO_INCREMENTAL=0
set "RUSTC_WRAPPER="    &@REM ## NOTE: RUSTC_WRAPPER=='sccache' breaks code coverage calculations (uu_*.gcno files are not created during build)
set "RUSTFLAGS=-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads"
set RUSTUP_TOOLCHAIN=nightly-x86_64-pc-windows-gnu
call cargo build %FEATURES_OPTION%
call cargo test --no-run %FEATURES_OPTION%
call cargo test --quiet %FEATURES_OPTION%

set COVERAGE_REPORT_DIR=target\debug\coverage-win
call rm -r "%COVERAGE_REPORT_DIR%" 2>NUL

set GRCOV_IGNORE_OPTION=--ignore build.rs --ignore "/*" --ignore "[A-Za-z]:/*"
@rem ::# * build LCOV coverage file
call grcov . --output-type lcov --output-file "%COVERAGE_REPORT_DIR%/../lcov.info" --branch %GRCOV_IGNORE_OPTION%
@rem ::# * build HTML
@rem ::# -- use `genhtml` if available for display of additional branch coverage information
call genhtml --version 2>NUL 1>&2
if NOT ERRORLEVEL 1 (
    call genhtml target/debug/lcov.info --output-directory "%COVERAGE_REPORT_DIR%" --branch-coverage --function-coverage
) else (
    call grcov . --output-type html --output-file "%COVERAGE_REPORT_DIR%" --branch %GRCOV_IGNORE_OPTION%
)
if ERRORLEVEL 1 goto _undefined_ 2>NUL || @for %%G in ("%COMSPEC%") do @title %%nG & @"%COMSPEC%" /d/c exit %ERRORLEVEL%

call start "" "%COVERAGE_REPORT_DIR%"\index.html
