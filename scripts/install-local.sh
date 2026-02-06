#!/usr/bin/env bash
set -euo pipefail

cargo build --release
mkdir -p "$HOME/.local/bin"
cp ./target/release/scrubby "$HOME/.local/bin/scrubby"

echo "Installed to $HOME/.local/bin/scrubby"
