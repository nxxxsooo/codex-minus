#!/usr/bin/env bash
#
# Bump the version everywhere it is written, then commit.
#
#   scripts/release.sh 0.4.5
#
# Deliberately plain shell: any person, any editor, and any agent can run this, and it needs no
# service, plugin, or account. It stops before pushing — opening the PR, waiting for CI, merging,
# and tagging stay explicit acts (see scripts/release-tag.sh and docs/workflow.md).

set -euo pipefail

version="${1-}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() { printf '%s\n' "release: $*" >&2; exit 1; }

[[ -n "$version" ]] || fail "usage: scripts/release.sh <version>   e.g. 0.4.5"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "'$version' is not MAJOR.MINOR.PATCH"

cd "$root"

# A dirty tree would fold unrelated edits into the release commit, where nobody looks for them.
[[ -z "$(git status --porcelain)" ]] || fail "working tree is dirty; commit or stash first"

branch="$(git rev-parse --abbrev-ref HEAD)"
[[ "$branch" != "master" ]] || fail "release from a branch, not master; master is protected"

current="$(node -p "require('./package.json').version")"
[[ "$version" != "$current" ]] || fail "already at $version"

# BOARD.md is written by hand — what changed, why, how it was verified. No script can produce that,
# and a release without it is a version number nobody can explain later.
grep -q "^### $(date +%Y-%m-%d)$" BOARD.md \
  || fail "BOARD.md has no '### $(date +%Y-%m-%d)' section; write the entry first"

printf 'release: %s -> %s\n' "$current" "$version"

node -e '
  const fs = require("fs");
  const [version] = process.argv.slice(1);
  for (const file of ["package.json", "src-tauri/tauri.conf.json"]) {
    const value = JSON.parse(fs.readFileSync(file, "utf8"));
    value.version = version;
    fs.writeFileSync(file, JSON.stringify(value, null, 2) + "\n");
  }
' "$version"

# Only the first `version =` — the [package] one. Later ones belong to dependencies.
perl -0pi -e "s/^version = \"[^\"]+\"/version = \"$version\"/m" src-tauri/Cargo.toml

# Nothing but cargo writes Cargo.lock, which is exactly why it is the one that gets forgotten.
cargo update -p codex-minus --manifest-path src-tauri/Cargo.toml --quiet

npm test --silent >/dev/null || fail "tests fail at $version; not committing"

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock BOARD.md
git commit -q -m "chore: release $version"

cat <<EOF

committed $(git rev-parse --short HEAD)  chore: release $version

next:
  git push -u origin $branch
  gh pr create --base master --head $branch --title "..." --body "..."
  gh pr checks <n> --watch
  gh pr merge <n> --merge
  scripts/release-tag.sh $version
EOF
