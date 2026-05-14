#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  ./scripts/sync-version.sh <version>
  ./scripts/sync-version.sh --check <version>
EOF
}

CHECK_ONLY=0
if [[ "${1:-}" == "--check" ]]; then
    CHECK_ONLY=1
    shift
fi

if [[ $# -ne 1 ]]; then
    usage >&2
    exit 2
fi

VERSION="$1"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: version must look like 1.2.3" >&2
    exit 1
fi

extract_package_version() {
    perl -0ne 'print $1 if /\[package\]\n(?:(?!^\[).*\n)*?version = "([^"]+)"/ms' "$1"
}

extract_lock_version() {
    perl -0ne 'print $1 if /\[\[package\]\]\nname = "zot"\nversion = "([^"]+)"/ms' "$1"
}

assert_equals() {
    local label="$1"
    local expected="$2"
    local actual="$3"

    if [[ "$expected" != "$actual" ]]; then
        echo "error: $label is '$actual', expected '$expected'" >&2
        exit 1
    fi
}

if (( CHECK_ONLY )); then
    assert_equals "Cargo.toml version" \
        "$VERSION" \
        "$(extract_package_version "$ROOT_DIR/Cargo.toml")"
    assert_equals "Cargo.lock version" \
        "$VERSION" \
        "$(extract_lock_version "$ROOT_DIR/Cargo.lock")"
    echo "Release version fields already match $VERSION."
    exit 0
fi

VERSION="$VERSION" perl -0pi -e \
    's/(\[package\]\n(?:(?!^\[).*\n)*?version = ")[^"]+(")/$1.$ENV{VERSION}.$2/ems or die "failed to update Cargo.toml\n";' \
    "$ROOT_DIR/Cargo.toml"

VERSION="$VERSION" perl -0pi -e \
    's/(\[\[package\]\]\nname = "zot"\nversion = ")[^"]+(")/$1.$ENV{VERSION}.$2/ems or die "failed to update Cargo.lock\n";' \
    "$ROOT_DIR/Cargo.lock"

echo "Updated release version to $VERSION."
