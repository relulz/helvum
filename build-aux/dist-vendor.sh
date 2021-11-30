#!/usr/bin/env bash

set -euo pipefail

export DIST="$1"
export SOURCE_ROOT="$2"

cd "$SOURCE_ROOT"
mkdir "$DIST"/.cargo
cargo vendor > $DIST/.cargo/config
# Move vendor into dist tarball directory
mv vendor "$DIST"
