#!/usr/bin/env bash
set -eux

cargo install --debug --path .
cargo profile $@