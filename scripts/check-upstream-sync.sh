#!/usr/bin/env bash
set -euo pipefail

TARGET_REF="${1:-upstream/main}"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "error: not inside a git repository"
  exit 2
fi

if [[ "${TARGET_REF}" != */* ]]; then
  echo "error: target ref must be in <remote>/<branch> form (got '${TARGET_REF}')"
  echo "example: scripts/check-upstream-sync.sh upstream/main"
  exit 2
fi

REMOTE="${TARGET_REF%%/*}"
BRANCH="${TARGET_REF#*/}"

if ! git remote get-url "${REMOTE}" >/dev/null 2>&1; then
  echo "error: git remote '${REMOTE}' not found"
  exit 2
fi

echo "sync-check: fetching ${REMOTE}/${BRANCH}"
git fetch "${REMOTE}" "${BRANCH}" --quiet

# ahead: commits on HEAD not in target
# behind: commits on target not in HEAD
ahead="$(git rev-list --count "${TARGET_REF}..HEAD")"
behind="$(git rev-list --count "HEAD..${TARGET_REF}")"

head_sha="$(git rev-parse --short HEAD)"
target_sha="$(git rev-parse --short "${TARGET_REF}")"

echo "sync-check: HEAD=${head_sha} target=${TARGET_REF}@${target_sha}"
echo "sync-check: ahead=${ahead} behind=${behind}"

if (( behind > 0 )); then
  echo "sync-check FAILED: branch is behind ${TARGET_REF} by ${behind} commit(s)."
  echo "action: git rebase ${TARGET_REF}"
  exit 3
fi

echo "sync-check OK: branch includes latest ${TARGET_REF}."
exit 0
