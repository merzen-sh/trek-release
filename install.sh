#!/usr/bin/env bash
set -euo pipefail

REPO="merzen-sh/trek-release"
VERSION="${1:-latest}"
DEST="${DESTDIR:-/usr/local/bin}"

if [ "$VERSION" = "latest" ]; then
  URL=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | jq -r '.assets[] | select(.name | endswith(".tar.gz")) | .browser_download_url')
else
  URL="https://github.com/${REPO}/releases/download/v${VERSION}/trek-release-v${VERSION}.tar.gz"
fi

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -sL "$URL" -o "$TMPDIR/trek-release.tar.gz"
tar xzf "$TMPDIR/trek-release.tar.gz" -C "$TMPDIR"
install -m 755 "$TMPDIR"/trek-release-v* "$DEST/trek-release"

echo "Installed trek-release to $DEST/trek-release"
