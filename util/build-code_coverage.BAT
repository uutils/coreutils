@setLocal
@echo off
set "ERRORLEVEL="

@rem ::# spell-checker:ignore (abbrevs/acronyms) gcno
@rem ::# spell-checker:ignore (CMD) COMSPEC ERRORLEVEL
@rem ::# spell-checker:ignore (jargon) toolchain
@rem ::# spell-checker:ignore (rust) Ccodegen Cinline Coverflow Cpanic RUSTC RUSTDOCFLAGS RUSTFLAGS RUSTUP Zpanic
@rem ::# spell-checker:ignore (utils) genhtml grcov lcov sccache uutils

@rem ::# ref: https://github.com/uutils/coreutils/pull/1476

set "FEATURES_OPTION=--features feat_os_windows"

cd "%~dp0.."
call echo [ "%CD%" ]

for /f "tokens=*" %%G in ('%~dp0\show-utils.BAT %FEATURES_OPTION%') do set UTIL_LIST=%%G
REM echo UTIL_LIST=%UTIL_LIST%
set "CARGO_INDIVIDUAL_PACKAGE_OPTIONS="
for %%H in (%UTIL_LIST%) do (
    if DEFINED CARGO_INDIVIDUAL_PACKAGE_OPTIONS call set "CARGO_INDIVIDUAL_PACKAGE_OPTIONS=%%CARGO_INDIVIDUAL_PACKAGE_OPTIONS%% "
    call set "CARGO_INDIVIDUAL_PACKAGE_OPTIONS=%%CARGO_INDIVIDUAL_PACKAGE_OPTIONS%%-puu_%%H"
)
REM echo CARGO_INDIVIDUAL_PACKAGE_OPTIONS=%CARGO_INDIVIDUAL_PACKAGE_OPTIONS%

REM call cargo clean

set "CARGO_INCREMENTAL=0"
set "RUSTC_WRAPPER="    &@REM ## NOTE: RUSTC_WRAPPER=='sccache' breaks code coverage calculations (uu_*.gcno files are not created during build)
@REM set "RUSTFLAGS=-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads"
set "RUSTFLAGS=-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
set "RUSTDOCFLAGS=-Cpanic=abort"
set "RUSTUP_TOOLCHAIN=nightly-gnu"
call cargo build %FEATURES_OPTION%
call cargo test --no-run %FEATURES_OPTION%
call cargo test --quiet %FEATURES_OPTION%
call cargo test --quiet %FEATURES_OPTION% %CARGO_INDIVIDUAL_PACKAGE_OPTIONS%

if NOT DEFINED COVERAGE_REPORT_DIR set COVERAGE_REPORT_DIR=target\debug\coverage-win
call rm -r "%COVERAGE_REPORT_DIR%" 2>NUL

set GRCOV_IGNORE_OPTION=--ignore build.rs --ignore "/*" --ignore "[A-Za-z]:/*" --ignore "C:/Users/*"
set GRCOV_EXCLUDE_OPTION=--excl-br-line "^\s*((debug_)?assert(_eq|_ne)?!|#\[derive\()"
@rem ::# * build LCOV coverage file
REM echo call grcov . --output-type lcov --output-path "%COVERAGE_REPORT_DIR%/../lcov.info" --branch %GRCOV_IGNORE_OPTION% %GRCOV_EXCLUDE_OPTION%
call grcov . --output-type lcov --output-path "%COVERAGE_REPORT_DIR%/../lcov.info" --branch %GRCOV_IGNORE_OPTION% %GRCOV_EXCLUDE_OPTION%
@rem ::# * build HTML
@rem ::# -- use `genhtml` if available for display of additional branch coverage information
set "ERRORLEVEL="
call genhtml --version 2>NUL 1>&2
if NOT ERRORLEVEL 1 (
    echo call genhtml target/debug/lcov.info --prefix "%CD%" --output-directory "%COVERAGE_REPORT_DIR%" --branch-coverage --function-coverage ^| grep ": [0-9]"
    call genhtml target/debug/lcov.info --prefix "%CD%" --output-directory "%COVERAGE_REPORT_DIR%" --branch-coverage --function-coverage | grep ": [0-9]"
) else (
    echo call grcov . --output-type html --output-path "%COVERAGE_REPORT_DIR%" --branch %GRCOV_IGNORE_OPTION%
    call grcov . --output-type html --output-path "%COVERAGE_REPORT_DIR%" --branch %GRCOV_IGNORE_OPTION%
)
if ERRORLEVEL 1 goto _undefined_ 2>NUL || @for %%G in ("%COMSPEC%") do @title %%nG & @"%COMSPEC%" /d/c exit %ERRORLEVEL%
