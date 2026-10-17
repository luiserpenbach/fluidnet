// Build a single-page artifact: inline app.js and app.css into one HTML file
// (no <html>/<head>/<body> wrapper), next to fluidnet.wasm and fluids/*.bin.
import fs from 'fs';
import path from 'path';
const ui = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const out = process.argv[2] ?? path.join(ui, 'artifact');
fs.mkdirSync(path.join(out, 'fluids'), { recursive: true });
const js = fs.readFileSync(path.join(ui, 'dist/app.js'), 'utf8').replace(/<\/script/gi, '<\\/script');
const css = fs.readFileSync(path.join(ui, 'dist/app.css'), 'utf8');
const html = `<title>fluidnet</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&display=swap">
<style>${css}</style>
<div id="root"></div>
<script type="module">${js}</script>
`;
fs.writeFileSync(path.join(out, 'fluidnet.html'), html);
fs.copyFileSync(path.join(ui, 'public/fluidnet.wasm'), path.join(out, 'fluidnet.wasm'));
for (const f of fs.readdirSync(path.join(ui, 'public/fluids'))) fs.copyFileSync(path.join(ui, 'public/fluids', f), path.join(out, 'fluids', f));
console.log('artifact written to', out, (html.length / 1024).toFixed(0), 'KB html');
