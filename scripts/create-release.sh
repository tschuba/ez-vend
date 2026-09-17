#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
    cat <<'EOF'
Usage: ./scripts/create-release.sh [version]

Creates a release branch and pull request from an up-to-date local main branch.
After the PR is merged, run ./scripts/tag-release.sh to create and push the release tag.

Examples:
  ./scripts/create-release.sh
  ./scripts/create-release.sh 0.1.0
EOF
}

cleanup() {
    if [[ -n "${NOTES_FILE:-}" && -f "$NOTES_FILE" ]]; then
        rm -f "$NOTES_FILE"
    fi

    if [[ -n "${PR_BODY_FILE:-}" && -f "$PR_BODY_FILE" ]]; then
        rm -f "$PR_BODY_FILE"
    fi
}

require_clean_worktree() {
    if [[ -n "$(git status --short)" ]]; then
        echo "Working tree must be clean before creating a release." >&2
        exit 1
    fi
}

require_branch_main() {
    local current_branch
    current_branch="$(git branch --show-current)"

    if [[ "$current_branch" != "main" ]]; then
        echo "Release creation must start from the local main branch." >&2
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
        echo "Local main must match origin/main before creating a release." >&2
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

ensure_release_branch_absent() {
    local release_branch="$1"

    if git show-ref --verify --quiet "refs/heads/$release_branch"; then
        echo "Branch $release_branch already exists locally." >&2
        exit 1
    fi

    if git ls-remote --heads origin "$release_branch" | grep -q .; then
        echo "Branch $release_branch already exists on origin." >&2
        exit 1
    fi
}

update_workspace_version() {
    local version="$1"

    VERSION="$version" python3 - <<'PY'
from pathlib import Path
import os
import re

path = Path("Cargo.toml")
content = path.read_text()
updated, count = re.subn(
    r'(?m)^version = "[^"]+"$',
    f'version = "{os.environ["VERSION"]}"',
    content,
    count=1,
)
if count != 1:
    raise SystemExit("Failed to update workspace version in Cargo.toml")
path.write_text(updated)
PY
}

capture_notes() {
    local notes_file="$1"

    cat <<'EOF'
Enter optional release notes to include in the release PR.
You can reuse them later with ./scripts/tag-release.sh when the PR is merged.
Press Ctrl-D on a new line to finish, or just press Ctrl-D immediately to skip.
EOF

    cat > "$notes_file" || true
}

build_pr_body() {
    local version="$1"
    local notes_file="$2"
    local pr_body_file="$3"

    cat > "$pr_body_file" <<EOF
## Summary
- bump the workspace version to ${version} so the release tag can match Cargo.toml
- prepare the release branch for merge into main without pushing directly to the protected branch

## Validation
- not run by this helper; run ./scripts/validate-release.sh before merging if release readiness still needs confirmation

## After Merge
- run ./scripts/tag-release.sh ${version} from an up-to-date local main branch to create the annotated release tag and trigger the GitHub release workflow
EOF

    if [[ -s "$notes_file" ]]; then
        cat >> "$pr_body_file" <<EOF

## Release Notes Draft
$(<"$notes_file")
EOF
    fi
}

create_release_pr() {
    local version="$1"
    local release_branch="$2"
    local pr_body_file="$3"

    gh pr create \
        --base main \
        --head "$release_branch" \
        --title "chore: prepare release v${version}" \
        --body-file "$pr_body_file"
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

    local version tag_name commit_message release_branch pr_url original_branch
    version="$(prompt_version "${1:-}")"
    tag_name="v${version}"
    release_branch="release/${tag_name}"
    original_branch="$(git branch --show-current)"
    NOTES_FILE="$(mktemp)"
    PR_BODY_FILE="$(mktemp)"
    commit_message="chore: bump version to ${version}"

    trap cleanup EXIT

    ensure_tag_absent "$tag_name"
    ensure_release_branch_absent "$release_branch"
    capture_notes "$NOTES_FILE"

    git checkout -b "$release_branch"

    update_workspace_version "$version"

    if git diff --quiet -- Cargo.toml; then
        echo "Cargo.toml already uses version ${version}; no release PR created." >&2
        git checkout "$original_branch"
        git branch -D "$release_branch"
        exit 1
    fi

    git add Cargo.toml
    git commit -m "$commit_message"
    git push -u origin "$release_branch"

    build_pr_body "$version" "$NOTES_FILE" "$PR_BODY_FILE"
    pr_url="$(create_release_pr "$version" "$release_branch" "$PR_BODY_FILE")"

    git checkout "$original_branch"

    echo "Created release branch $release_branch and opened PR: $pr_url"
    echo "Next steps:"
    echo "1. Review and merge the PR into main."
    echo "2. Run ./scripts/tag-release.sh ${version} from an up-to-date local main branch."
    echo "3. Monitor the workflow at: https://github.com/tschuba/ez-vend/actions/workflows/release.yml"
}

main "$@"
