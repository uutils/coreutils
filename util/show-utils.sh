#!/usr/bin/env bash

set -eo pipefail
# spell-checker:ignore (jq) deps startswith

# This script should not depend on external binaries to maximize portability if possible
dir="${0%/*}"
[ "$dir" = "$0" ] && dir="."
cd -- "$dir/.."

# `jq` available?
if ! jq --version 1>/dev/null 2>&1; then
    # refs: <https://forge.rust-lang.org/release/platform-support.html> , <https://docs.rs/platforms/0.2.1/platforms/platform/tier1/index.html>
    # default utility list
    default_utils=$(cargo tree --depth 1 --features feat_common_core --format "{lib}" --prefix none | sed -n 's/^uu_//p')
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
