#!/usr/bin/env bash
# This file is part of the uutils coreutils package.
#
# For the full copyright and license information, please view the LICENSE
# file that was distributed with this source code.

set -euo pipefail

if [[ $# != 1 || ! -x $1 ]]; then
    echo "Usage: bash util/test-logind.sh /absolute/path/to/tests-binary" >&2
    exit 1
fi

logind_test_binary=$(realpath "$1")
logind_utmp_fixture=$(mktemp)
trap 'rm -f "$logind_utmp_fixture"' EXIT

logind_privilege=()
if [[ $EUID != 0 ]]; then
    logind_privilege=(sudo)
fi

# Hide utmp only in the test service's mount namespace, leaving the host's
# records intact. The test creates and stops its own PAM/TTY session.
"${logind_privilege[@]}" systemd-run --quiet --wait --pipe --collect \
    "--property=BindReadOnlyPaths=$logind_utmp_fixture:/run/utmp" \
    "$logind_test_binary" --ignored --exact \
    test_who::logind::sessions_match_loginctl --nocapture
