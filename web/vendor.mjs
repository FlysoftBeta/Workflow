import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { transform } from 'esbuild';

const root = path.dirname(fileURLToPath(import.meta.url));
const target = path.resolve(root, '../app/src/main/assets/web/vendor');
const packages = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8')).dependencies;
const assets = {
  '@xterm/xterm': ['lib/xterm.js', 'css/xterm.css', 'LICENSE'],
  '@xterm/addon-fit': ['lib/addon-fit.js', 'LICENSE'],
  '@xterm/addon-web-links': ['lib/addon-web-links.js', 'LICENSE'],
  'core-js-bundle': ['minified.js', 'LICENSE']
};
const manifest = [];
for (const [name, files] of Object.entries(assets)) {
  const destination = path.join(target, name.replace('@', '').replaceAll('/', '-'));
  await mkdir(destination, { recursive: true });
  for (const file of files) {
    const source = path.join(root, 'node_modules', name, file);
    const output = path.join(destination, path.basename(file));
    await cp(source, output, { recursive: true });
    if (file.endsWith('.js')) {
      const result = await transform(await readFile(source, 'utf8'), { target: 'chrome66', minify: true, legalComments: 'inline' });
      await writeFile(output, result.code);
    }
    if (!file.endsWith('fonts')) {
      manifest.push({ package: name, version: packages[name], file: path.relative(target, output), sha256: createHash('sha256').update(await readFile(output)).digest('hex'), upstreamSha256: createHash('sha256').update(await readFile(source)).digest('hex') });
    }
  }
}
await writeFile(path.join(target, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log(`Vendored ${Object.keys(assets).length} packages into ${target}`);
