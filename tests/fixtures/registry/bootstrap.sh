#!/bin/sh
# Bootstrap a synthetic ccgo registry for tests.
#
# Idempotent. Re-running reuses the existing LEAF_*.zip if one is already
# present in linkage/leaf/target/release/macos/ (the script rebuilds only
# when no zip exists), then refreshes the leaf-archive copy and re-inits
# the inner index git repo so commit history doesn't accumulate.
#
# Outputs (gitignored):
#   tests/fixtures/registry/index/                  (a real local git repo)
#   tests/fixtures/registry/index/index.json
#   tests/fixtures/registry/index/le/af/leaf.json
#   tests/fixtures/registry/leaf-archive/leaf-1.0.0.zip
set -eu

HERE="$(cd -- "$(dirname -- "$0")" && pwd)"
ARCHIVE_DIR="$HERE/leaf-archive"
INDEX_DIR="$HERE/index"
LINKAGE_LEAF="$HERE/../linkage/leaf"

mkdir -p "$ARCHIVE_DIR"
# PackageIndex::package_index_path uses cargo-index sharding:
# 4-char names land at <[0:2]>/<[2:4]>/<name>.json — for "leaf": le/af/leaf.json.
# No `packages/` prefix.
mkdir -p "$INDEX_DIR/le/af"

# Locate the ccgo binary. Prefer the cargo target dir build (so the test
# uses the freshly-built binary), then fall back to PATH.
REPO_ROOT="$(cd "$HERE/../../.." && pwd)"
CCGO_BIN=""
for candidate in \
    "$REPO_ROOT/target/debug/ccgo" \
    "$REPO_ROOT/target/release/ccgo"
do
    if [ -x "$candidate" ]; then
        CCGO_BIN="$candidate"
        break
    fi
done
if [ -z "$CCGO_BIN" ]; then
    if command -v ccgo >/dev/null 2>&1; then
        CCGO_BIN="$(command -v ccgo)"
    fi
fi

# Build the leaf fixture if no merged-package zip exists yet. We need the
# `target/release/package/LEAF_CCGO_PACKAGE-*.zip` artifact specifically:
# only that one carries an embedded `CCGO.toml` at the archive root, which
# `ccgo fetch` extracts as the consumer-side package manifest. The
# per-platform `target/release/<plat>/LEAF_*.zip` files do not include
# CCGO.toml and are unsuitable as a dep archive.
LATEST_ZIP=""
if [ -d "$LINKAGE_LEAF/target/release/package" ]; then
    LATEST_ZIP="$(ls -t "$LINKAGE_LEAF"/target/release/package/LEAF_CCGO_PACKAGE-*.zip 2>/dev/null | head -1 || true)"
fi
if [ -z "$LATEST_ZIP" ] || [ ! -f "$LATEST_ZIP" ]; then
    if [ -z "$CCGO_BIN" ]; then
        echo "bootstrap.sh: ccgo binary not found; build with 'cargo build' first" >&2
        exit 1
    fi
    ( cd "$LINKAGE_LEAF" && "$CCGO_BIN" package --release )
    LATEST_ZIP="$(ls -t "$LINKAGE_LEAF"/target/release/package/LEAF_CCGO_PACKAGE-*.zip | head -1)"
fi

cp "$LATEST_ZIP" "$ARCHIVE_DIR/leaf-1.0.0.zip"

CHECKSUM_RAW="$(shasum -a 256 "$ARCHIVE_DIR/leaf-1.0.0.zip" | awk '{print $1}')"
CHECKSUM="sha256:$CHECKSUM_RAW"
ARCHIVE_URL="file://$ARCHIVE_DIR/leaf-1.0.0.zip"

# Re-init the index repo from scratch so this script stays idempotent and
# we never accumulate a long commit history under tests/.
rm -rf "$INDEX_DIR/.git"
mkdir -p "$INDEX_DIR/le/af"

cat > "$INDEX_DIR/index.json" <<EOF
{
  "name": "test-registry",
  "version": "1",
  "packages_count": 1,
  "last_updated": "2026-04-29T00:00:00Z"
}
EOF

cat > "$INDEX_DIR/le/af/leaf.json" <<EOF
{
  "name": "leaf",
  "description": "Synthetic leaf for ccgo registry-resolution tests",
  "repository": "$ARCHIVE_URL",
  "license": "MIT",
  "platforms": ["macos"],
  "versions": [
    {
      "version": "1.0.0",
      "tag": "v1.0.0",
      "checksum": "$CHECKSUM",
      "archive_url": "$ARCHIVE_URL",
      "archive_format": "zip"
    }
  ]
}
EOF

(
    cd "$INDEX_DIR"
    git init -q
    git config user.email "test@example.com"
    git config user.name "ccgo-registry-test"
    git add -A
    # --no-verify insulates the inner repo from any user-side commit-msg /
    # pre-commit hooks (linthis, conventional-commits enforcement, etc.) —
    # the inner repo is a test fixture, not real source control.
    git commit -m "init" -q --no-verify
)

echo "[bootstrap] index:    $INDEX_DIR"
echo "[bootstrap] archive:  $ARCHIVE_DIR/leaf-1.0.0.zip"
echo "[bootstrap] checksum: $CHECKSUM"
