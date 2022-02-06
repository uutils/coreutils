#!/bin/sh

# spell-checker:ignore (abbrevs/acronyms) HTML gcno llvm
# spell-checker:ignore (jargon) toolchain
# spell-checker:ignore (rust) Ccodegen Cinline Coverflow Cpanic RUSTC RUSTDOCFLAGS RUSTFLAGS RUSTUP Zpanic
# spell-checker:ignore (shell) OSID esac
# spell-checker:ignore (utils) genhtml grcov lcov readlink sccache shellcheck uutils

FEATURES_OPTION="--features feat_os_unix"

ME="${0}"
ME_dir="$(dirname -- "$(readlink -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

cd "${REPO_main_dir}" &&
    echo "[ \"$PWD\" ]"

#shellcheck disable=SC2086
UTIL_LIST=$("${ME_dir}"/show-utils.sh ${FEATURES_OPTION})
CARGO_INDIVIDUAL_PACKAGE_OPTIONS=""
for UTIL in ${UTIL_LIST}; do
    if [ -n "${CARGO_INDIVIDUAL_PACKAGE_OPTIONS}" ]; then CARGO_INDIVIDUAL_PACKAGE_OPTIONS="${CARGO_INDIVIDUAL_PACKAGE_OPTIONS} "; fi
    CARGO_INDIVIDUAL_PACKAGE_OPTIONS="${CARGO_INDIVIDUAL_PACKAGE_OPTIONS}-puu_${UTIL}"
done
# echo "CARGO_INDIVIDUAL_PACKAGE_OPTIONS=${CARGO_INDIVIDUAL_PACKAGE_OPTIONS}"

# cargo clean

export CARGO_INCREMENTAL=0
export RUSTC_WRAPPER="" ## NOTE: RUSTC_WRAPPER=='sccache' breaks code coverage calculations (uu_*.gcno files are not created during build)
# export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads"
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
export RUSTUP_TOOLCHAIN="nightly-gnu"
#shellcheck disable=SC2086
{
    cargo build ${FEATURES_OPTION}
    cargo test --no-run ${FEATURES_OPTION}
    cargo test --quiet ${FEATURES_OPTION}
    cargo test --quiet ${FEATURES_OPTION} ${CARGO_INDIVIDUAL_PACKAGE_OPTIONS}
}

export COVERAGE_REPORT_DIR
if [ -z "${COVERAGE_REPORT_DIR}" ]; then COVERAGE_REPORT_DIR="${REPO_main_dir}/target/debug/coverage-nix"; fi
rm -r "${COVERAGE_REPORT_DIR}" 2>/dev/null
mkdir -p "${COVERAGE_REPORT_DIR}"

## NOTE: `grcov` is not accepting environment variable contents as options for `--ignore` or `--excl_br_line`
# export GRCOV_IGNORE_OPTION="--ignore build.rs --ignore '/*' --ignore '[A-Za-z]:/*' --ignore 'C:/Users/*'"
# export GRCOV_EXCLUDE_OPTION="--excl-br-line '^\s*((debug_)?assert(_eq|_ne)?!|#\[derive\()'"
# * build LCOV coverage file
grcov . --output-type lcov --output-path "${COVERAGE_REPORT_DIR}/../lcov.info" --branch --ignore build.rs --ignore '/*' --ignore '[A-Za-z]:/*' --ignore 'C:/Users/*' --excl-br-line '^\s*((debug_)?assert(_eq|_ne)?!|#\[derive\()'
# * build HTML
# -- use `genhtml` if available for display of additional branch coverage information
if genhtml --version 2>/dev/null 1>&2; then
    genhtml "${COVERAGE_REPORT_DIR}/../lcov.info" --output-directory "${COVERAGE_REPORT_DIR}" --branch-coverage --function-coverage | grep ": [0-9]"
else
    grcov . --output-type html --output-path "${COVERAGE_REPORT_DIR}" --branch --ignore build.rs --ignore '/*' --ignore '[A-Za-z]:/*' --ignore 'C:/Users/*' --excl-br-line '^\s*((debug_)?assert(_eq|_ne)?!|#\[derive\()'
fi
# shellcheck disable=SC2181
if [ $? -ne 0 ]; then exit 1; fi
