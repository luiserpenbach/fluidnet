// Copy the WebAssembly engine and fluid tables into public/ for dev and build.
import fs from 'fs';
import zlib from 'zlib';
import path from 'path';
import { fileURLToPath } from 'url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const wasm = path.join(root, 'target/wasm32-unknown-unknown/release/fluidnet_wasm.wasm');
const pub = path.join(root, 'ui/public');
fs.mkdirSync(path.join(pub, 'fluids'), { recursive: true });
fs.copyFileSync(wasm, path.join(pub, 'fluidnet.wasm'));
for (const f of fs.readdirSync(path.join(root, 'fluids'))) {
  // gzip + base64 text: servable everywhere (static hosts, artifacts), decoded with DecompressionStream
  if (f.endsWith('.fntb')) {
    const gz = zlib.gzipSync(fs.readFileSync(path.join(root, 'fluids', f)), { level: 9 });
    fs.writeFileSync(path.join(pub, 'fluids', f.replace('.fntb', '.txt')), gz.toString('base64'));
  }
}
console.log('copied engine and fluid tables to', pub);
