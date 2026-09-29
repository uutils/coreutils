#!/usr/bin/env bash

set -eo pipefail
# spell-checker:ignore (jq) deps startswith

cd -- "$(dirname -- "$0")/.."

# `jq` available?
if ! jq --version 1>/dev/null 2>&1; then
    # refs: <https://forge.rust-lang.org/release/platform-support.html> , <https://docs.rs/platforms/0.2.1/platforms/platform/tier1/index.html>
    # default utility list
    default_utils=$(sed -n '/feat_common_core = \[/,/\]/p' Cargo.toml | sed '1d' | tr -d '],"\n')
    echo "WARN: missing \`jq\` (install with \`sudo apt install jq\`); falling back to default (only fully cross-platform) utility list" 1>&2
    echo "$default_utils"
else
    cargo metadata "$@" --format-version 1 | jq -r '[.resolve.nodes[] | select(.id|match(".*coreutils[ |@|#]\\d+\\.\\d+\\.\\d+")) | .deps[] | select(.pkg|match("uu_")) | .name | sub("^uu_"; "")] | sort | join(" ")'
fi
