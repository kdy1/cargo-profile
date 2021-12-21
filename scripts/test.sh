#!/usr/bin/env bash
set -eux

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"


cargo install --debug --locked --path "$SCRIPT_DIR/.."
cargo profile $@