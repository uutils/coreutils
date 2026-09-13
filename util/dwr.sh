#!/usr/bin/env bash

# `dwr` - delete workflow runs (by DJ Adams)
# ref: <https://github.com/qmacro/dotfiles/blob/230c6df494f239e9d1762794943847816e1b7c32/scripts/dwr>
# ref: [Mass deletion of GitHub Actions workflow runs](https://qmacro.org/autodidactics/2021/03/26/mass-deletion-of-github-actions-workflow-runs) @@ <https://archive.is/rxdCY>

# LICENSE: "Feel free to steal, modify, or make fun of" (from <https://github.com/qmacro/dotfiles/blob/230c6df494f239e9d1762794943847816e1b7c32/README.md>)

# spell-checker:ignore (options) multi ; (people) DJ Adams * qmacro ; (words) gsub

# Given an "owner/repo" name, such as "qmacro/thinking-aloud",
# retrieve the workflow runs for that repo and present them in a
# list. Selected runs will be deleted. Uses the GitHub API.

# Requires gh (GitHub CLI) and jq (JSON processor)

# First version

set -o errexit
set -o pipefail

declare repo=${1:?No owner/repo specified}

jq_script() {

    cat <<EOF
    def symbol:
        sub("skipped"; "SKIP") |
        sub("success"; "GOOD") |
        sub("failure"; "FAIL");

    def tz:
        gsub("[TZ]"; " ");


    .workflow_runs[]
        | [
            (.conclusion | symbol),
            (.created_at | tz),
            .id,
            .event,
            .name
        ]
        | @tsv
EOF

}

select_runs() {

    gh api --paginate "/repos/$repo/actions/runs" |
        jq -r -f <(jq_script) |
        fzf --multi

}

delete_run() {

    local run id result
    run=$1
    id="$(cut -f 3 <<<"$run")"
    gh api -X DELETE "/repos/$repo/actions/runs/$id"
    # shellcheck disable=SC2181
    [[ $? = 0 ]] && result="OK!" || result="BAD"
    printf "%s\t%s\n" "$result" "$run"

}

delete_runs() {

    local id
    while read -r run; do
        delete_run "$run"
        sleep 0.25
    done

}

main() {

    select_runs | delete_runs

}

main
