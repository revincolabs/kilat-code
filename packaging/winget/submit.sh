#!/usr/bin/env bash
# Submit a Kilat Code version to the Windows Package Manager community repo
# (microsoft/winget-pkgs) as a pull request from your fork.
#
#   bash packaging/winget/submit.sh 0.2.0
#
# Needs: gh (logged in), the manifests in manifests/r/revincolabs/KilatCode/<version>/
# validated with:  winget validate --manifest <that folder>
set -euo pipefail

VERSION="${1:?usage: submit.sh <version>, e.g. 0.2.0}"
ID="revincolabs.KilatCode"
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$HERE/manifests/r/revincolabs/KilatCode/$VERSION"
DEST="manifests/r/revincolabs/KilatCode/$VERSION"
UPSTREAM="microsoft/winget-pkgs"

[ -d "$SRC" ] || { echo "Tidak ada manifest di $SRC" >&2; exit 1; }
ME="$(gh api user --jq .login)"
FORK="$ME/winget-pkgs"
BRANCH="$ID-$VERSION"

echo "• Fork $UPSTREAM ke $FORK (kalau belum ada)…"
gh repo fork "$UPSTREAM" --clone=false >/dev/null 2>&1 || true
for _ in $(seq 1 60); do
  gh api "repos/$FORK" >/dev/null 2>&1 && break
  sleep 3
done

echo "• Sinkronkan fork dengan upstream…"
gh api -X POST "repos/$FORK/merge-upstream" -f branch=master >/dev/null 2>&1 || true
BASE="$(gh api "repos/$FORK/git/ref/heads/master" --jq .object.sha)"

echo "• Buat branch $BRANCH…"
gh api -X POST "repos/$FORK/git/refs" -f ref="refs/heads/$BRANCH" -f sha="$BASE" >/dev/null

for f in "$SRC"/*.yaml; do
  name="$(basename "$f")"
  echo "• Unggah $name"
  gh api -X PUT "repos/$FORK/contents/$DEST/$name" \
    -f message="New package: $ID version $VERSION" \
    -f branch="$BRANCH" \
    -f content="$(base64 -w0 "$f")" >/dev/null
done

BODY="$(cat <<EOF
New package: **Kilat Code** $VERSION (\`$ID\`).

Lightweight Tauri desktop app that runs the real Claude Code and Codex terminals side by side with multiple accounts.
Homepage: https://kilat.revincolabs.com · Source: https://github.com/revincolabs/kilat-code

The installer is the NSIS (\`nullsoft\`) setup built by Tauri, per-user scope, silent with \`/S\`. It is not code-signed yet.

###### Checklist

- [x] This PR only modifies one (1) manifest
- [x] Have you validated your manifest locally with \`winget validate --manifest <path>\`?
- [x] Does your manifest conform to the [1.12 schema](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)?
EOF
)"

echo "• Buka pull request…"
gh pr create --repo "$UPSTREAM" --head "$ME:$BRANCH" --base master \
  --title "New package: $ID version $VERSION" --body "$BODY"
