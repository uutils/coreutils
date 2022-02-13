#!/bin/sh

# spell-checker:ignore (vars) OSID binfmt

ME="${0}"
ME_dir="$(dirname -- "$(readlink -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

export COVERAGE_REPORT_DIR="${REPO_main_dir}/target/debug/coverage-nix"

if ! "${ME_dir}/build-code_coverage.sh"; then exit 1; fi

# WSL?
if [ -z "${OSID_tags}" ]; then
    if [ -e '/proc/sys/fs/binfmt_misc/WSLInterop' ] && (grep '^enabled$' '/proc/sys/fs/binfmt_misc/WSLInterop' >/dev/null); then
        __="wsl"
        case ";${OSID_tags};" in ";;") OSID_tags="$__" ;; *";$__;"*) ;; *) OSID_tags="$__;$OSID_tags" ;; esac
        unset __
        # Windows version == <major>.<minor>.<build>.<revision>
        # Release ID; see [Release ID/Version vs Build](https://winreleaseinfoprod.blob.core.windows.net/winreleaseinfoprod/en-US.html)[`@`](https://archive.is/GOj1g)
        OSID_wsl_build="$(uname -r | sed 's/^[0-9.][0-9.]*-\([0-9][0-9]*\)-.*$/\1/g')"
        OSID_wsl_revision="$(uname -v | sed 's/^#\([0-9.][0-9.]*\)-.*$/\1/g')"
        export OSID_wsl_build OSID_wsl_revision
    fi
fi

case ";${OSID_tags};" in
    *";wsl;"*) powershell.exe -c "$(wslpath -w "${COVERAGE_REPORT_DIR}"/index.html)" ;;
    *) xdg-open --version >/dev/null 2>&1 && xdg-open "${COVERAGE_REPORT_DIR}"/index.html || echo "report available at '\"${COVERAGE_REPORT_DIR}\"/index.html'" ;;
esac
