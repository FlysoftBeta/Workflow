// Builds the offline chat transcript page into app/src/main/assets/web/chat/ (or the directory given
// as the first argument). marked, DOMPurify, KaTeX and core-js are not bundled: chat.html loads the
// vendored globals from ../vendor/ (see vendor.mjs).
import { build } from 'esbuild';
import { copyFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.dirname(fileURLToPath(import.meta.url));
export const defaultOutdir = path.resolve(root, '../app/src/main/assets/web/chat');

export async function buildChat(outdir = defaultOutdir) {
  await mkdir(outdir, { recursive: true });
  await build({
    entryPoints: [path.join(root, 'chat/src/main.js')],
    outfile: path.join(outdir, 'chat.js'),
    bundle: true,
    format: 'iife',
    target: 'chrome66',
    minify: true,
    legalComments: 'inline',
    logLevel: 'warning'
  });
  for (const file of ['chat.html', 'chat.css']) await copyFile(path.join(root, 'chat', file), path.join(outdir, file));
  return outdir;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const outdir = await buildChat(process.argv[2] ? path.resolve(process.argv[2]) : defaultOutdir);
  console.log(`Built chat page into ${outdir}`);
}
