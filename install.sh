#!/usr/bin/env bash

set -e

if [ ! -f "Cargo.toml" ] || ! grep -q "name = \"nlsh-rs\"" Cargo.toml; then
    TEMP_DIR=$(mktemp -d)
    git clone https://github.com/uwuclxdy/nlsh-rs "$TEMP_DIR/nlsh-rs"
    cd "$TEMP_DIR/nlsh-rs"
fi

cargo build --release

cargo install nlsh-rs --force --path .

rm -rf "$TEMP_DIR"

nlsh-rs
