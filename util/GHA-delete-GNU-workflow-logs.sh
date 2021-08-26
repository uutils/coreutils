#!/bin/sh

# spell-checker:ignore (utils) gitsome jq ; (gh) repos

ME="${0}"
ME_dir="$(dirname -- "${ME}")"
ME_parent_dir="$(dirname -- "${ME_dir}")"
ME_parent_dir_abs="$(realpath -mP -- "${ME_parent_dir}")"

# ref: <https://stackoverflow.com/questions/57927115/anyone-know-a-way-to-delete-a-workflow-from-github-actions>

# note: requires `gh` and `jq`

## tools available?

# * `gh` available?
unset GH
gh --version 1>/dev/null 2>&1
if [ $? -eq 0 ]; then export GH="gh"; fi

# * `jq` available?
unset JQ
jq --version 1>/dev/null 2>&1
if [ $? -eq 0 ]; then export JQ="jq"; fi

if [ -z "${GH}" ] || [ -z "${JQ}" ]; then
    if [ -z "${GH}" ]; then
        echo 'ERR!: missing `gh` (see install instructions at <https://github.com/cli/cli>)' 1>&2
    fi
    if [ -z "${JQ}" ]; then
        echo 'ERR!: missing `jq` (install with `sudo apt install jq`)' 1>&2
    fi
    exit 1
fi

dry_run=true

USER_NAME=uutils
REPO_NAME=coreutils
WORK_NAME=GNU

# * `--paginate` retrieves all pages
# gh api --paginate "repos/${USER_NAME}/${REPO_NAME}/actions/runs" | jq -r ".workflow_runs[] | select(.name == \"${WORK_NAME}\") | (.id)" | xargs -n1 sh -c "for arg do { echo gh api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; if [ -z "$dry_run" ]; then gh api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; fi ; } ; done ;" _
gh api "repos/${USER_NAME}/${REPO_NAME}/actions/runs" | jq -r ".workflow_runs[] | select(.name == \"${WORK_NAME}\") | (.id)" | xargs -n1 sh -c "for arg do { echo gh api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; if [ -z "$dry_run" ]; then gh api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; fi ; } ; done ;" _
