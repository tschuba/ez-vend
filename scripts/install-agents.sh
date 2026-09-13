#!/usr/bin/env bash
# Usage: ./scripts/install-agents.sh
set -euo pipefail

REPO_URL="https://github.com/msitarzewski/agency-agents.git"
DEST="${HOME}/.claude/agents"

WHITELIST=(
  "engineering/engineering-software-architect.md"
  "engineering/engineering-backend-architect.md"
  "engineering/engineering-frontend-developer.md"
  "engineering/engineering-database-optimizer.md"
  "engineering/engineering-security-engineer.md"
  "engineering/engineering-senior-developer.md"
  "engineering/engineering-code-reviewer.md"
  "engineering/engineering-technical-writer.md"
  "engineering/engineering-minimal-change-engineer.md"
  "engineering/engineering-devops-automator.md"
  "design/design-ux-architect.md"
  "design/design-ui-designer.md"
  "design/design-brand-guardian.md"
  "testing/testing-accessibility-auditor.md"
  "testing/testing-performance-benchmarker.md"
  "testing/testing-reality-checker.md"
  "testing/testing-evidence-collector.md"
  "testing/testing-api-tester.md"
)

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "Cloning agency-agents..."
git clone --depth 1 "$REPO_URL" "$WORK_DIR/agency-agents" || { echo "ERROR: git clone failed" >&2; exit 1; }

mkdir -p "$DEST"

count=0
for file in "${WHITELIST[@]}"; do
  src="$WORK_DIR/agency-agents/$file"
  if [[ -f "$src" ]]; then
    cp "$src" "$DEST/"
    (( count++ )) || true
  else
    echo "WARNING: not found: $file" >&2
  fi
done

if [[ $count -eq 0 ]]; then
  echo "ERROR: no agents installed" >&2
  exit 1
fi

echo "Done. $count agents installed to $DEST"
