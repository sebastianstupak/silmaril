/**
 * Renders icon-source.svg → src-tauri/icons/icon-source.png (1024x1024)
 * Then `cargo tauri icon` handles the rest.
 */
import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const { Resvg } = await import('@resvg/resvg-js');

const svg = readFileSync(resolve(__dirname, 'icon-source.svg'), 'utf-8');

const resvg = new Resvg(svg, {
  fitTo: { mode: 'width', value: 1024 },
});

const png = resvg.render().asPng();
const outPath = resolve(root, 'src-tauri', 'icons', 'icon-source.png');
writeFileSync(outPath, png);
console.log(`Written: ${outPath} (${png.byteLength} bytes)`);
