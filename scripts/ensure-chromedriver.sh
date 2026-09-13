#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCAL_DIR="${ROOT_DIR}/.local/chromedriver"

chrome_version() {
    local candidates=(
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        "google-chrome"
        "chromium"
        "chromium-browser"
    )

    for candidate in "${candidates[@]}"; do
        if command -v "$candidate" >/dev/null 2>&1; then
            "$candidate" --version 2>/dev/null | awk '{print $3}' | head -n1
            return 0
        elif [ -x "$candidate" ]; then
            "$candidate" --version 2>/dev/null | awk '{print $3}' | head -n1
            return 0
        fi
    done

    return 1
}

find_cached_matching_driver() {
    local chrome_major="$1"
    python3 - "$chrome_major" <<'PY'
import glob, subprocess, sys
major = sys.argv[1]
for path in glob.glob('/Users/thomas/Library/Caches/.wasm-pack/chromedriver-*/chromedriver'):
    try:
        out = subprocess.check_output([path, '--version'], text=True).strip()
    except Exception:
        continue
    version = out.split()[1]
    if version.split('.')[0] == major:
        print(path)
        raise SystemExit(0)
raise SystemExit(1)
PY
}

activate_matching_driver_in_wasm_pack_cache() {
    local matching_driver="$1"
    local cache_root="/Users/thomas/Library/Caches/.wasm-pack"
    local candidate

    for candidate in "$cache_root"/chromedriver-*/chromedriver; do
        [ -e "$candidate" ] || continue
        if [ "$(python3 - <<'PY' "$candidate" "$matching_driver"
import os,sys
print('same' if os.path.realpath(sys.argv[1]) == os.path.realpath(sys.argv[2]) else 'different')
PY
)" = "same" ]; then
            continue
        fi

        rm -f "$candidate"
        ln -s "$matching_driver" "$candidate"
    done
}

download_matching_driver() {
    local chrome_major="$1"
    mkdir -p "$LOCAL_DIR"

    local version url zip_path extract_dir driver_path
    version="$(curl -fsSL "https://googlechromelabs.github.io/chrome-for-testing/LATEST_RELEASE_${chrome_major}")"
    url="https://storage.googleapis.com/chrome-for-testing-public/${version}/mac-arm64/chromedriver-mac-arm64.zip"
    zip_path="${LOCAL_DIR}/chromedriver-${version}.zip"
    extract_dir="${LOCAL_DIR}/chromedriver-${version}"

    curl -fsSL "$url" -o "$zip_path"
    rm -rf "$extract_dir"
    unzip -q -o "$zip_path" -d "$extract_dir"
    driver_path="${extract_dir}/chromedriver-mac-arm64/chromedriver"
    chmod +x "$driver_path"
    printf '%s\n' "$driver_path"
}

main() {
    local chrome_ver chrome_major driver_path
    chrome_ver="$(chrome_version)" || {
        echo "Could not determine local Chrome version" >&2
        exit 1
    }

    chrome_major="${chrome_ver%%.*}"

    if driver_path="$(find_cached_matching_driver "$chrome_major" 2>/dev/null)"; then
        activate_matching_driver_in_wasm_pack_cache "$driver_path"
        printf '%s\n' "$driver_path"
        exit 0
    fi

    driver_path="$(download_matching_driver "$chrome_major")"
    activate_matching_driver_in_wasm_pack_cache "$driver_path"
    printf '%s\n' "$driver_path"
}

main "$@"
