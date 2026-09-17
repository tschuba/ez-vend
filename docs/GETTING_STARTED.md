---
title: Getting Started
nav_order: 2
---

# Getting Started

This guide helps you set up `ez-vend`, run it locally, and understand the first steps before using it for a real event.

## What You Need

Install these once:

- [Rust via rustup](https://rustup.rs/)
- `trunk`: `cargo install trunk`
- `wasm-pack`: `cargo install wasm-pack`
- WASM target: `rustup target add wasm32-unknown-unknown`

## Platform-Specific LLVM Setup

The SQLite migration feature adds a WASM build-time dependency on LLVM with WebAssembly target support. GitHub Actions already has the needed toolchain; this section is for local development.

### macOS

Apple clang does not provide the required WebAssembly target. Install Homebrew LLVM and use a project-local `CC` override:

```bash
brew install llvm direnv
cp .envrc.example .envrc
direnv allow
```

Add the `direnv` hook to your shell once:

```bash
echo 'eval "$(direnv hook zsh)"' >> ~/.zshrc
source ~/.zshrc
```

If you use bash, replace `zsh` with `bash`.

Verify the override in the project directory:

```bash
echo $CC
cargo check -p ez-vend-app --target wasm32-unknown-unknown
```

Expected `CC` value on Apple Silicon:

```bash
/opt/homebrew/opt/llvm/bin/clang
```

Intel Macs usually use:

```bash
/usr/local/opt/llvm/bin/clang
```

### Linux

Most Linux distributions already ship a `clang` build with WebAssembly support.

Verify with:

```bash
clang --version
```

If WASM builds fail, install LLVM explicitly:

```bash
sudo apt-get install llvm clang
```

### Windows

Install LLVM from [releases.llvm.org](https://releases.llvm.org/) and point `CC` at `clang.exe` if your local shell does not already pick it up.

Windows setup is currently untested in this repository.

Frontend tooling is also required for the app bundle:

```bash
cd crates/ez-vend-app
npm ci
```

If you want to run Safari browser tests later, enable Safari WebDriver once on the machine:

```bash
sudo safaridriver --enable
```

## Start the App in Development

From the app crate:

```bash
cd crates/ez-vend-app
trunk serve
```

Then open `http://127.0.0.1:8080` in your browser.

`trunk serve` watches for changes and rebuilds automatically.

## First Useful Checks

Before doing deeper work, these commands are the most useful quick validation steps:

```bash
./run-tests.sh
cargo build --workspace --locked
```

For browser validation options, see the [Testing Guide](https://github.com/tschuba/ez-vend/blob/main/TESTING.md) and [Validation Workflow](validation/VALIDATION_WORKFLOW.md).

## What the App Does

At a high level, the app lets operators:

- create and manage booth events
- configure participation fees, sales fees, and rounding
- record purchases for vendors
- generate booth and vendor reports
- export and import booth data for backup and recovery

## First Booth Walkthrough

Once the app is running:

1. create a booth with a description, event date, participation fee, sales fee percent, and rounding step
2. open the booth and start entering checkout items
3. verify vendor totals and fee calculations in the report views
4. export a backup so you have a recovery point outside the browser

If you are preparing for a real event, also review:

- [Fee Calculation Guide](user-guides/FEE_CALCULATION.md)
- [Data Backup Guide](user-guides/DATA_BACKUP_GUIDE.md)
- [Validation Workflow](validation/VALIDATION_WORKFLOW.md)

## Backup and Browser Storage Basics

`ez-vend` is offline-first and stores core booth data in the browser on the current device.

That means:

- the app works well without a backend for normal event use
- clearing browser storage can remove local booth data
- exported JSON backups are the durable recovery mechanism

Before real usage, make sure the operating team understands the backup flow:

- full export for all local data
- booth export for one event
- import with `Merge`, `Skip`, or `Replace`

See [Data Backup Guide](user-guides/DATA_BACKUP_GUIDE.md).

## Common Workflows

### Run the App Locally

```bash
cd crates/ez-vend-app
trunk serve
```

### Build the Full Workspace

```bash
cargo build --workspace --locked
```

### Build the Web App for Release

```bash
cd crates/ez-vend-app
trunk build --release
```

### Build the Standalone Launcher

```bash
cargo build --release -p ez-vend-launcher --locked
```

## Validation Paths

Choose the smallest validation set that matches your change:

- general local confidence: `./run-tests.sh`
- Chrome browser flow coverage: `./run-tests.sh --chrome`
- Safari-sensitive storage or print workflows: `./run-tests.sh --safari`
- full automated suite: `./run-tests.sh --chrome --safari`

Manual validation documents live in `docs/validation/`:

- `SAFARI_VALIDATION_CHECKLIST.md`
- `UAT_Ausfuehrungsplan_DE_EN.html`
- `PHASE2_M2_VALIDATION_RESULTS.md`

## Troubleshooting

### `trunk` or WASM build problems

- make sure `wasm32-unknown-unknown` is installed
- run `npm ci` again in `crates/ez-vend-app`
- check that `trunk` is installed from Cargo, not missing from your shell path

### `No available targets are compatible with triple "wasm32-unknown-unknown"`

- this usually means your C compiler does not include the WebAssembly target needed by the SQLite migration feature
- on macOS, run `brew install llvm direnv`, then `cp .envrc.example .envrc && direnv allow`
- verify with `echo $CC`; on Apple Silicon it should resolve to `/opt/homebrew/opt/llvm/bin/clang`
- if `direnv` is not active yet, test once with `export CC="/opt/homebrew/opt/llvm/bin/clang" && cargo check -p ez-vend-app --target wasm32-unknown-unknown`
- on Linux, install `llvm` and `clang` if your system compiler lacks WASM support
- on Windows, install LLVM and point `CC` at `clang.exe`; this path is documented but not yet validated in this repo

### Browser tests do not start

- install `wasm-pack`
- for Safari tests, run `sudo safaridriver --enable` once
- use the scripts in the repo rather than hand-assembling browser setup

### Data seems missing

- make sure you are using the same browser profile and device
- check whether browser storage was cleared
- restore from a JSON export if needed

## More Documentation

- [Architecture Overview](https://github.com/tschuba/ez-vend/blob/main/ARCHITECTURE.md)
- [Comparison to the Original App](COMPARISON_TO_ORIGINAL.md)
- [Redesign Summary](redesign/REDESIGN_SUMMARY.md)
- [Technical Docs](technical/)
- [Planning Docs](planning/)
