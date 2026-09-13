#!/bin/bash

# Safari Driver Management for wasm-pack browser tests.
# Safari includes safaridriver on macOS, but it must be enabled once with:
#   sudo safaridriver --enable

set -euo pipefail

check_safaridriver() {
    local version_output

    if ! command -v safaridriver >/dev/null 2>&1; then
        echo "❌ safaridriver not found. Install Safari 10 or newer to run Safari browser tests." >&2
        return 1
    fi

    if version_output="$(safaridriver --version 2>&1)"; then
        python3 -c 'import sys; print(sys.argv[1].splitlines()[0])' "$version_output"
        return 0
    fi

    echo "⚠️  safaridriver is installed but not enabled." >&2
    echo "" >&2
    echo "   One-time setup required on this machine:" >&2
    echo "   sudo safaridriver --enable" >&2
    echo "" >&2
    echo "   After enabling it, rerun:" >&2
    echo "   ./run-tests.sh --safari" >&2
    echo "" >&2
    echo "   This message is intentionally printed here so Safari tests are self-documenting" >&2
    echo "   when they are executed on another Mac." >&2
    return 1
}

main() {
    check_safaridriver
}

main "$@"
