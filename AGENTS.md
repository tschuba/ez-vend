# AGENTS.md

This file gives coding agents the repo-specific rules and commands needed to work safely in `ez-vend-rs`.

## Repository Snapshot

- Rust 2021 Cargo workspace with domain, storage, UI, app, and launcher crates.
- Primary app flow: `domain` -> `ez-vend-storage` -> `ez-vend-ui` -> `ez-vend-app`.
- UI is a client-side Leptos app compiled to WASM and bundled with Trunk.
- Storage uses IndexedDB in the browser.
- Launcher is a native Rust binary that serves the built WASM bundle locally.
- Money and fee calculations are business-critical; preserve exact decimal behavior.

## Important Docs

- `README.md` for product scope, build flow, and delivery expectations.
- `TESTING.md` for unit, browser, and manual validation details.
- `docs/validation/VALIDATION_WORKFLOW.md` for when manual validation is required.
- `docs/BRANCH_STRATEGY.md` for branch naming, PR expectations, and squash-merge policy.
- `docs/validation/SAFARI_VALIDATION_CHECKLIST.md` and `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html` for manual operator validation.

## Setup Prerequisites

- Install Rust via `rustup`.
- Install Trunk: `cargo install trunk`.
- Install `wasm-pack` for browser tests: `cargo install wasm-pack`.
- Add the WASM target: `rustup target add wasm32-unknown-unknown`.
- Install LLVM with WebAssembly support for the SQLite migration feature:
  - macOS: `brew install llvm direnv`, then `cp .envrc.example .envrc && direnv allow`
  - Linux: system `clang` usually sufficient
  - Windows: install LLVM from `releases.llvm.org` (untested)
- In `crates/ez-vend-app`, install frontend tooling before Trunk builds: `npm ci`.
- Safari browser tests require one-time enablement: `sudo safaridriver --enable`.
- Activate pre-commit lint/format hooks: `git config core.hooksPath .githooks`.

## Build Commands

- Full workspace build: `cargo build --workspace --locked`
- WASM target build: `cargo build -p ez-vend-app --target wasm32-unknown-unknown --locked`
- Standalone launcher build: `cargo build --release -p ez-vend-launcher --locked`
- App dev server: `trunk serve` from `crates/ez-vend-app`
- Production bundle: `trunk build --release` from `crates/ez-vend-app`
- Tailwind CSS only: `npm run build:css` from `crates/ez-vend-app`

## Release Commands

- Validate release readiness: `./scripts/validate-release.sh`
- Create a release PR from local `main`: `./scripts/create-release.sh 0.1.0`
- Create the release tag after the PR is merged: `./scripts/tag-release.sh 0.1.0`
- Release workflow trigger: push a stable semantic version tag like `v0.1.0`

## Lint And Format Commands

- Format check: `cargo fmt --all --check`
- Apply formatting: `cargo fmt --all`
- Clippy: `cargo clippy --workspace --all-targets --locked`
- There is no repo-specific `rustfmt.toml` or `clippy.toml`; default tool behavior applies.

## Test Commands

- Fast local suite: `./run-tests.sh`
- Unit tests only: `cargo test --workspace --lib --locked`
- Chrome browser suite: `./run-tests.sh --chrome`
- Safari browser suite: `./run-tests.sh --safari`
- Full automated suite: `./run-tests.sh --chrome --safari`

## Single-Crate And Single-Test Commands

- Domain crate tests: `cargo test -p domain`
- UI unit tests: `cargo test -p ez-vend-ui --lib`
- Storage browser tests in Chrome: `wasm-pack test --headless --chrome crates/storage`
- Storage browser tests in Safari: `wasm-pack test --headless --safari crates/storage`
- UI browser tests in Chrome: `wasm-pack test --headless --chrome crates/ez-vend-ui`
- UI browser tests in Safari: `wasm-pack test --headless --safari crates/ez-vend-ui`
- Single Rust test by exact name: `cargo test -p <crate> <test_name> -- --exact`
- Single browser test by exact name: `wasm-pack test --headless --chrome crates/<crate> <test_name> -- --exact`
- Example single UI unit test: `cargo test -p ez-vend-ui test_to_booth_valid_data -- --exact`
- Example single browser test: `wasm-pack test --headless --chrome crates/storage test_save_booth -- --exact`
- Watch mode example: `cargo watch -x "test -p ez-vend-ui --lib"`

## Manual Validation Expectations

- If you change operator-facing flows, reporting, recovery behavior, print output, or Safari-sensitive behavior, do more than unit tests.
- Start the app with `trunk serve` from `crates/ez-vend-app` for manual validation.
- Use the smallest validation set that proves safety, then note what you ran.
- Update validation docs in `docs/validation/` when workflows or acceptance coverage change.

## Architecture Guidance

- Keep domain rules in `crates/domain`; do not bury core business validation in UI-only code.
- Keep persistence concerns in `crates/storage`.
- Keep rendering, interaction, and translation wiring in `crates/ez-vend-ui`.
- Keep `crates/ez-vend-app` thin; it should mostly initialize logging, panic hooks, and mount the app.
- Keep `crates/ez-vend-launcher` focused on packaging and local serving concerns.

## Code Style: General

- Follow existing Rust 2021 idioms and let `rustfmt` drive layout.
- Prefer small helper functions over deeply nested event handlers or validation branches.
- Match surrounding style in a file before introducing a new pattern.
- Use 4-space indentation; do not manually align columns.
- Prefer expressive names over abbreviations.
- Keep comments sparse and only for non-obvious business or browser constraints.

## Code Style: Imports

- Keep imports explicit and local to the file.
- Preserve the existing grouping/order style of the file you touch; import ordering is not perfectly uniform across crates.
- Avoid wildcard imports except where the file already uses them heavily, such as some Leptos view modules.
- Remove unused imports rather than leaving them for later cleanup.

## Code Style: Types And Data Modeling

- Use `rust_decimal::Decimal` for money, fees, payout math, and amount stepping.
- Use `chrono::DateTime<Utc>` for timestamps and `chrono::NaiveDate` for booth dates.
- Reuse domain model types such as `BoothId`, `VendorId`, and `PurchaseId` instead of raw strings when possible.
- Return `DomainResult<T>` or crate-specific result types instead of ad hoc `Result<T, String>` in core logic.
- Use `Arc` for shared repositories and services in UI/storage state where the existing code does.
- Prefer typed enums for mode/state (`ConflictStrategy`, `DraftLoadOutcome`, `Locale`) over free-form strings.

## Code Style: Naming

- Types, enums, and traits use `PascalCase`.
- Functions, methods, modules, and variables use `snake_case`.
- Constants use `SCREAMING_SNAKE_CASE`.
- Test names should describe the scenario and expected outcome, usually in full snake_case phrases.
- Translation keys remain dot-separated strings and should follow existing naming families like `checkout.errors.*`.

## Code Style: Validation And Business Rules

- Validate at the domain boundary first.
- Trim and normalize user input before persisting or validating when the existing flow does so.
- Preserve exact current fee and payout semantics; even small rounding changes are high risk.
- Treat amount stepping, vendor ID rules, omission rules, and reporting totals as regression-sensitive.
- Prefer adding targeted regression tests when changing money handling or recovery logic.

## Code Style: Error Handling

- Use `thiserror` enums for structured errors.
- Convert infrastructure errors into domain-facing errors at crate boundaries.
- In UI code, translate domain errors for operator-facing messages with `translate_domain_error` or i18n helpers.
- Prefer returning rich errors over panicking.
- Limit `unwrap()` and `expect()` to tests or truly impossible invariants; avoid introducing them in normal runtime paths.
- Log actionable failures with `log::{error, warn, info}` when they help diagnose browser/storage issues.

## Code Style: UI And Leptos

- Use signals, memos, and effects in the established Leptos style already present in `crates/ez-vend-ui`.
- Use `spawn_local` for async browser-side tasks.
- Keep persistent UI preferences in local storage through focused helper functions.
- Route visible strings through translations; do not hard-code new operator-facing copy if an i18n key is appropriate.
- Translation keys are dot-separated (e.g. `checkout.errors.amount_too_large`); add new keys to both `locales/de.json` and `locales/en.json`.
- Use the `t!("key")()` macro pattern — it returns a closure; call it inside `move || ...` for reactivity.
- Keep accessibility attributes when editing reusable components like buttons and dialogs.
- Preserve current Tailwind utility style rather than introducing a second styling system.

## Testing Conventions

- Inline unit tests commonly live under `#[cfg(test)]` in the same Rust module.
- Browser integration tests live in crate-level `tests/` directories and use `wasm-bindgen-test`.
- Async domain/service tests often use `#[tokio::test]` with lightweight mock repositories.
- When fixing a bug, add or update the narrowest regression test that proves the fix.
- For browser-storage or recovery fixes, prefer both automated coverage and a note about manual validation needs.

## Workflow Expectations For Agents

- Start from a focused branch, usually `feature/...` or `fix/...`, when branch work is part of the task.
- Keep changes scoped to one problem or milestone.
- Before finishing, run the smallest relevant command set from this file.
- In PR summaries, explain why the change exists, what validation ran, and any deferred follow-up.
- Do not plan on direct merges to `main`; the repo expects pull requests and prefers squash merges.
- For release automation changes, keep the tag and built source aligned; prefer validating version state in CI over mutating version files after a tag already exists.

## Safe Defaults For Agentic Changes

- If a change touches money math, reports, import/export, recovery, or deletion flows, be conservative and add tests.
- If a change affects operator workflows, review the relevant docs in `docs/` and update them when behavior changes.
- If a file already has local conventions, follow them instead of normalizing unrelated style.
- If browser behavior is uncertain, prefer `./run-tests.sh --chrome` and call out whether Safari validation is still needed.

## Tool Use Discipline For Agentic Models

- **Never announce a tool call in prose and then fail to invoke it.** Do not write "Here is the replace_in_file call:" or "I will now use the write_to_file tool:" as a sentence — just invoke the tool directly.
- **Do not describe what you are about to do as a substitute for doing it.** If the next step is to edit a file, edit it. If the next step is to run a command, run it.
- **One tool call per response turn when operations are dependent.** Wait for the result before proceeding to the next dependent step.
- **Use `write_to_file` as a fallback when `replace_in_file` fails three times.** Do not keep retrying the same failing SEARCH block — rewrite the whole file instead.
- **When a `replace_in_file` SEARCH block fails to match**, re-read the relevant section of the file first, then construct the SEARCH block from the actual file content, not from memory.
- **Do not repeat apologies in lieu of tool calls.** If a tool call failed, retry it immediately with a corrected invocation — do not write multiple sentences explaining the failure before acting.
- **Keep SEARCH blocks in `replace_in_file` small and precise.** Match only the lines that need to change plus a few lines of unique context. Large SEARCH blocks are fragile and likely to fail.
