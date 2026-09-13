# EZ Booth Downloaded Build

This folder is ready to run without Python, Node.js, or any extra setup.

## Quick Start

### Windows

1. Extract the downloaded archive.
2. Double-click `ez-booth.exe`.
3. Your browser opens automatically.

### macOS

1. Extract the downloaded archive.
2. Open Terminal in this folder.
3. Run `./ez-vend-macos`.
4. If macOS warns about an unidentified developer, allow it in System Settings > Privacy & Security and run it again.

### Linux

1. Extract the downloaded archive.
2. Open a terminal in this folder.
3. Run `./ez-vend-linux`.

## What The Launcher Does

- starts a local server on `127.0.0.1` using ports `8080` through `8089`
- opens your default browser automatically
- keeps all booth data on your device in that browser profile
- prevents two EZ Booth instances from running at the same time on the same device
- serves files only from this extracted folder and adds browser security headers for the local session

If the browser does not open, copy the printed URL into your browser manually.

## Keep These Files Together

- Keep the launcher binary and all extracted files in the same folder.
- Do not move `index.html`, `.wasm`, `.js`, or `.css` files away from the launcher.
- Use a current Chrome, Edge, Firefox, or Safari release.
- Use the launcher only for local extracted builds; if you deploy EZ Booth behind a normal web server, that server must provide its own asset and security configuration.

## Security Notes

- The launcher only listens on `127.0.0.1`, so it is intended for the same device that starts it.
- The app bundle is loaded from the extracted folder next to the launcher, not from arbitrary filesystem paths.
- Single-instance protection relies on a lock file in your user config directory. If the launcher crashes, it may need to clean up that stale file the next time it starts.

## Troubleshooting

### Another instance is already running

If no other launcher window is open, remove the lock file and start again.

- Windows: `%APPDATA%\ez-booth\launcher.lock`
- macOS: `~/Library/Application Support/ez-booth/launcher.lock`
- Linux: `~/.config/ez-booth/launcher.lock`

```bash
# macOS
rm ~/Library/Application\ Support/ez-booth/launcher.lock

# Linux
rm ~/.config/ez-booth/launcher.lock
```

```powershell
# Windows PowerShell
Remove-Item "$env:APPDATA\ez-booth\launcher.lock"
```

### Permission denied on macOS or Linux

Mark the launcher as executable and retry.

```bash
chmod +x ez-vend-macos
chmod +x ez-vend-linux
```

### macOS says the app is from an unidentified developer

This build is unsigned. Use one of these options:

1. Open System Settings > Privacy & Security and choose `Open Anyway`.
2. Or remove the quarantine flag:

```bash
xattr -d com.apple.quarantine ez-vend-macos
./ez-vend-macos
```

### Windows protected your PC

Windows SmartScreen may show a warning because the binary is unsigned.

1. Click `More info`.
2. Click `Run anyway`.

### Browser does not open automatically

Open the printed local URL manually, for example `http://127.0.0.1:8080`.

### All ports 8080-8089 are busy

Another local app may already be using those ports.

```bash
# macOS or Linux
lsof -i :8080-8089
```

```powershell
# Windows PowerShell
netstat -ano | Select-String 8080
```

Stop the conflicting process, then launch EZ Booth again.

### Blank page or missing file errors

- make sure every extracted file is still in the same folder
- re-extract the archive if files were moved or removed
- start the app with the included launcher, not by opening `index.html` directly
