const fs = require('fs');
const path = require('path');

const indexPath = process.env.TRUNK_STAGING_DIR
  ? path.join(process.env.TRUNK_STAGING_DIR, 'index.html')
  : path.join(__dirname, '..', 'dist', 'index.html');

if (!fs.existsSync(indexPath)) {
  process.exit(0);
}

const html = fs.readFileSync(indexPath, 'utf8');
const updated = html.replace(/<link rel="preload"[^>]+_bg\.wasm[^>]*>/g, '');

if (updated !== html) {
  fs.writeFileSync(indexPath, updated);
}
