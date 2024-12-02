#!/usr/bin/env bash

# spell-checker:ignore (shell) OSTYPE
# spell-checker:ignore (utils) cksum coreutils dircolors hashsum mkdir mktemp printenv printf readlink realpath grealpath rmdir shuf tsort unexpand
# spell-checker:ignore (jq) deps startswith

# Use GNU version for realpath on *BSD
case "$OSTYPE" in
    *bsd*)
        REALPATH="grealpath"
        ;;
    *)
        REALPATH="realpath"
        ;;
esac

ME="${0}"
ME_dir="$(dirname -- "${ME}")"
ME_parent_dir="$(dirname -- "${ME_dir}")"
ME_parent_dir_abs="$("${REALPATH}" -mP -- "${ME_parent_dir}" || "${REALPATH}" -- "${ME_parent_dir}")"

# refs: <https://forge.rust-lang.org/release/platform-support.html> , <https://docs.rs/platforms/0.2.1/platforms/platform/tier1/index.html>

# default ("Tier 1" cross-platform) utility list
default_utils="base32 base64 basename cat cksum comm cp cut date dircolors dirname echo env expand expr factor false fmt fold hashsum head join link ln ls mkdir mktemp more mv nl od paste printenv printf ptx pwd readlink realpath rm rmdir seq shred shuf sleep sort split sum tac tail tee test tr true truncate tsort unexpand uniq wc yes"

project_main_dir="${ME_parent_dir_abs}"
# printf 'project_main_dir="%s"\n' "${project_main_dir}"
cd "${project_main_dir}" &&

    # `jq` available?
    if ! jq --version 1>/dev/null 2>&1; then
        echo "WARN: missing \`jq\` (install with \`sudo apt install jq\`); falling back to default (only fully cross-platform) utility list" 1>&2
        echo "$default_utils"
    else
    # Find 'coreutils' id with regex
    # with cargo v1.76.0, id = "coreutils 0.0.26 (path+file://<coreutils local directory>)"
    # with cargo >= v1.77.0
    # - if local path != '<...>/coreutils' id = "path+file://<coreutils local directory>#coreutils@0.0.26"
    # - if local path == '<...>/coreutils' id = "path+file://<parent directory>/coreutils#0.0.26"
        cargo metadata "$@" --format-version 1 | jq -r '[.resolve.nodes[] | select(.id|match(".*coreutils[ |@|#]\\d+\\.\\d+\\.\\d+")) | .deps[] | select(.pkg|match("uu_")) | .name | sub("^uu_"; "")] | sort | join(" ")'
    fi
