# Contributing to ez-vend

Thanks for looking at `ez-vend`. This file is a short entry point — it links to
the detailed docs instead of repeating them, so nothing here goes stale on its
own.

## Getting Set Up

Follow the "Want to Build or Contribute?" section of the [README](README.md#want-to-build-or-contribute)
for prerequisites, the local lint hooks, and starting the dev server. For a
guided first run, see [Getting Started](docs/GETTING_STARTED.md).

## Branching and Pull Requests

See [docs/BRANCH_STRATEGY.md](docs/BRANCH_STRATEGY.md) for branch naming,
what a pull request description should cover, and the squash-merge policy.

## Testing

See [TESTING.md](TESTING.md) for unit, integration, and browser test suites,
and how to run them. Changes affecting operator-facing behavior should also
update the relevant files under `docs/validation/` — see
[docs/validation/VALIDATION_WORKFLOW.md](docs/validation/VALIDATION_WORKFLOW.md).

## Code Style

General naming, typing, and error-handling conventions are in
[AGENTS.md](AGENTS.md#code-style-general). One rule worth calling out on its
own:

- **Identifiers, comments, and internal/developer-facing strings (log
  messages, error messages, panics) are always English** — regardless of the
  language spoken by the person writing them.
- **User-facing product text** (labels, buttons, receipts, reports) stays
  German or English through the existing i18n system (`crates/ui/src/i18n.rs`,
  `crates/ui/locales/*.json`) — that's product content, not source code, and
  is expected to be German-first for this app's operators.

This keeps the codebase readable to any contributor regardless of which
language the app itself speaks to its users.
