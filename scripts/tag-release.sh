#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
    cat <<'EOF'
Usage: ./scripts/tag-release.sh [version]

Creates and pushes the annotated release tag from an up-to-date local main branch.
Run this only after the matching release PR has been merged into main.

Examples:
  ./scripts/tag-release.sh
  ./scripts/tag-release.sh 0.1.0
EOF
}

cleanup() {
    if [[ -n "${NOTES_FILE:-}" && -f "$NOTES_FILE" ]]; then
        rm -f "$NOTES_FILE"
    fi
}

require_clean_worktree() {
    if [[ -n "$(git status --short)" ]]; then
        echo "Working tree must be clean before creating a release tag." >&2
        exit 1
    fi
}

require_branch_main() {
    local current_branch
    current_branch="$(git branch --show-current)"

    if [[ "$current_branch" != "main" ]]; then
        echo "Release tagging must start from the local main branch." >&2
        exit 1
    fi
}

require_synced_main() {
    git fetch origin

    local counts behind ahead
    counts="$(git rev-list --left-right --count main...origin/main)"
    behind="${counts%%$'\t'*}"
    ahead="${counts##*$'\t'}"

    if [[ "$behind" != "0" || "$ahead" != "0" ]]; then
        echo "Local main must match origin/main before creating a release tag." >&2
        echo "Run: git pull --ff-only" >&2
        exit 1
    fi
}

prompt_version() {
    local input_version="${1:-}"

    if [[ -z "$input_version" ]]; then
        read -r -p "Release version (for tag vX.Y.Z): " input_version
    fi

    if [[ ! "$input_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Version must use semantic versioning like 0.1.0." >&2
        exit 1
    fi

    printf '%s\n' "$input_version"
}

ensure_tag_absent() {
    local tag_name="$1"

    if git rev-parse "$tag_name" >/dev/null 2>&1; then
        echo "Tag $tag_name already exists locally." >&2
        exit 1
    fi

    if git ls-remote --tags origin "refs/tags/$tag_name" | grep -q .; then
        echo "Tag $tag_name already exists on origin." >&2
        exit 1
    fi
}

require_workspace_version() {
    local version="$1"
    local cargo_version
    cargo_version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)"

    if [[ -z "$cargo_version" ]]; then
        echo "Could not read workspace version from Cargo.toml." >&2
        exit 1
    fi

    if [[ "$cargo_version" != "$version" ]]; then
        echo "Cargo.toml version $cargo_version does not match requested tag version $version." >&2
        exit 1
    fi
}

capture_notes() {
    local notes_file="$1"

    cat <<'EOF'
Enter optional release notes for the annotated tag.
These notes are prepended to the auto-generated GitHub release notes.
Press Ctrl-D on a new line to finish, or just press Ctrl-D immediately to skip.
EOF

    cat > "$notes_file" || true
}

main() {
    if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
        usage
        exit 0
    fi

    cd "$ROOT_DIR"

    require_clean_worktree
    require_branch_main
    require_synced_main

    local version tag_name
    version="$(prompt_version "${1:-}")"
    tag_name="v${version}"
    NOTES_FILE="$(mktemp)"

    trap cleanup EXIT

    ensure_tag_absent "$tag_name"
    require_workspace_version "$version"
    capture_notes "$NOTES_FILE"

    if [[ -s "$NOTES_FILE" ]]; then
        git tag -a "$tag_name" -F "$NOTES_FILE"
    else
        git tag -a "$tag_name" -m "Release ${tag_name}"
    fi

    git push origin "$tag_name"

    echo "Created release tag $tag_name and pushed it to origin."
    echo "Monitor the workflow at: https://github.com/tschuba/ez-vend/actions/workflows/release.yml"
}

main "$@"
