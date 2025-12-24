#!/bin/sh

# spell-checker:ignore (utils) gitsome jq jaq ; (gh) repos

# ME="${0}"
# ME_dir="$(dirname -- "${ME}")"
# ME_parent_dir="$(dirname -- "${ME_dir}")"
# ME_parent_dir_abs="$(realpath -mP -- "${ME_parent_dir}")"

# ref: <https://stackoverflow.com/questions/57927115/anyone-know-a-way-to-delete-a-workflow-from-github-actions>

# note: requires `gh` and `jq`

## tools available?

# * `gh` available?
GH=$(command -v gh)
"${GH}" --version || (echo "ERR!: missing \`gh\` (see install instructions at <https://github.com/cli/cli>)"; exit 1)
# * `jq` or fallback available?
: ${JQ:=$(command -v jq || command -v jaq)}
"${JQ}" --version || (echo "ERR!: missing \`jq\` (install with \`sudo apt install jq\`)"; exit 1)

case "${dry_run}" in
    '0' | 'f' | 'false' | 'no' | 'never' | 'none') unset dry_run ;;
    *) dry_run="true" ;;
esac

USER_NAME="${USER_NAME:-uutils}"
REPO_NAME="${REPO_NAME:-coreutils}"
WORK_NAME="${WORK_NAME:-GNU}"

# * `--paginate` retrieves all pages
# gh api --paginate "repos/${USER_NAME}/${REPO_NAME}/actions/runs" | jq -r ".workflow_runs[] | select(.name == \"${WORK_NAME}\") | (.id)" | xargs -n1 sh -c "for arg do { echo gh api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; if [ -z "$dry_run" ]; then gh api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; fi ; } ; done ;" _
"${GH}" api "repos/${USER_NAME}/${REPO_NAME}/actions/runs" |
    "${JQ}" -r ".workflow_runs[] | select(.name == \"${WORK_NAME}\") | (.id)" |
    xargs -n1 sh -c "for arg do { echo ${GH} api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; if [ -z \"${dry_run}\" ]; then ${GH} api repos/${USER_NAME}/${REPO_NAME}/actions/runs/\${arg} -X DELETE ; fi ; } ; done ;" _
