#!/usr/bin/env bash
#
# Tag a merged release and push the tag, which is what triggers the release build.
#
#   scripts/release-tag.sh 0.4.5
#
# Run this only after the release PR is merged. The tag is created on origin/master, never on the
# branch: a tag on an unmerged commit points at something that is not in the product's history, and
# the installers it builds could never be reproduced from master.

set -euo pipefail

version="${1-}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() { printf '%s\n' "release-tag: $*" >&2; exit 1; }

[[ -n "$version" ]] || fail "usage: scripts/release-tag.sh <version>   e.g. 0.4.5"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "'$version' is not MAJOR.MINOR.PATCH"

cd "$root"
git fetch --quiet origin master

# Read the version out of the commit that will actually be tagged, not out of the working tree —
# the working tree can be ahead, behind, or on a different branch entirely.
merged="$(git show origin/master:package.json | node -p "JSON.parse(require('fs').readFileSync(0,'utf8')).version")"
[[ "$merged" == "$version" ]] \
  || fail "origin/master is at $merged, not $version — is the release PR merged?"

! git rev-parse -q --verify "refs/tags/v$version" >/dev/null \
  || fail "tag v$version already exists locally"

git tag -a "v$version" origin/master -m "$version"
git push origin "v$version"

cat <<EOF

tagged v$version at $(git rev-parse --short origin/master)

the release build is running; when it finishes:
  gh release view v$version --json assets --jq '.assets[].name'
EOF
