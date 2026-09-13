#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

require_clean_main() {
    local branch
    branch="$(git branch --show-current)"

    if [[ "$branch" != "main" ]]; then
        echo "Release validation must run from the local main branch." >&2
        exit 1
    fi

    if [[ -n "$(git status --short)" ]]; then
        echo "Working tree must be clean before release validation." >&2
        exit 1
    fi

    git fetch origin

    local counts behind ahead
    counts="$(git rev-list --left-right --count main...origin/main)"
    behind="${counts%%$'\t'*}"
    ahead="${counts##*$'\t'}"

    if [[ "$behind" != "0" || "$ahead" != "0" ]]; then
        echo "Local main must match origin/main before release validation." >&2
        exit 1
    fi
}

run_step() {
    local label="$1"
    shift

    echo
    echo "==> $label"
    "$@"
}

main() {
    cd "$ROOT_DIR"

    require_clean_main

    run_step "Checking formatting" cargo fmt --all --check
    run_step "Running clippy" cargo clippy --workspace --all-targets --locked
    run_step "Running workspace unit tests" cargo test --workspace --lib --locked
    run_step "Building workspace" cargo build --workspace --locked
    run_step "Building launcher release binary" cargo build --release -p ez-vend-launcher --locked
    run_step "Installing frontend dependencies" npm ci --prefix crates/ez-vend-app
    run_step "Building production WASM bundle" bash -lc 'cd crates/ez-vend-app && trunk build --release'

    echo
    echo "Release validation passed."
}

main "$@"
