const fs = require('fs');
const path = require('path');

const distDir = process.env.TRUNK_STAGING_DIR
    ? process.env.TRUNK_STAGING_DIR
    : path.join(__dirname, '..', 'dist');

const swPath = path.join(distDir, 'sw.js');

if (!fs.existsSync(swPath)) {
    process.exit(0);
}

const files = fs.readdirSync(distDir);
const wasmFile = files.find(function (f) {
    return f.endsWith('_bg.wasm');
});

let version;
if (wasmFile) {
    const match = wasmFile.match(/-([a-f0-9]+)_bg\.wasm$/);
    version = match ? match[1] : wasmFile.replace('_bg.wasm', '');
} else {
    version = Date.now().toString(36);
}

const sw = fs.readFileSync(swPath, 'utf8');
fs.writeFileSync(swPath, sw.replace('__CACHE_VERSION__', version));
