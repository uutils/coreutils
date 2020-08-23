#!/bin/sh

# spell-checker:ignore (vars) OSID

ME_dir="$(dirname -- $(readlink -fm -- "$0"))"
REPO_main_dir="$(dirname -- "${ME_dir}")"

export COVERAGE_REPORT_DIR="${REPO_main_dir}/target/debug/coverage-nix"

"${ME_dir}/build-code_coverage.sh"
if [ $? -ne 0 ]; then exit 1 ; fi

case ";$OSID_tags;" in
    *";wsl;"* ) powershell.exe -c $(wslpath -w "${COVERAGE_REPORT_DIR}"/index.html) ;;
    * ) xdg-open --version >/dev/null 2>&1 && xdg-open "${COVERAGE_REPORT_DIR}"/index.html || echo "report available at '\"${COVERAGE_REPORT_DIR}\"/index.html'" ;;
esac ;
