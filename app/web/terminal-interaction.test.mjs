import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { terminal, assets } from './terminal-harness.mjs';

const lines = count => Array.from({ length: count }, (_, i) => `line-${String(i).padStart(3, '0')}`).join('\r\n');
const selection = h => JSON.parse(JSON.stringify(h.term.getSelectionPosition()));

test('packaged entrypoint stays offline and vendors exactly the pinned manifest bytes', async () => {
  const html = await readFile(new URL('terminal.html', assets), 'utf8');
  assert.match(html, /connect-src 'none'/);
  for (const [, path] of html.matchAll(/(?:src|href)="([^"]+)"/g)) {
    assert.doesNotMatch(path, /^(?:https?:|\/\/)/);
    assert.ok((await readFile(new URL(path, assets))).length);
  }
  const manifest = JSON.parse(await readFile(new URL('vendor/manifest.json', assets), 'utf8'));
  for (const entry of manifest) {
    const bytes = await readFile(new URL('vendor/' + entry.file, assets));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.sha256);
  }
});

test('wrapped URL taps refresh current geometry and deliver exactly one callback', async t => {
  const h = await terminal(t, { cols: 16 });
  await h.write('https://example.com/a?x=1&y=2');
  assert.ok(h.host.querySelectorAll('.workflow-link').length >= 2);
  assert.equal(h.tap(4, 1).defaultPrevented, true);
  h.mouse('click', h.position(4, 1)); // Old WebViews may omit sourceCapabilities.
  assert.deepEqual(h.calls.urls, ['https://example.com/a?x=1&y=2']);
  const button = h.host.querySelector('button.workflow-link');
  assert.match(button.getAttribute('aria-label'), /Open https:/);
  button.click(); // Keyboard/AT activation uses the target, not synthetic (0,0) coordinates.
  assert.equal(h.calls.urls.length, 2);
});

test('scroll gestures, cancellation, viewport changes and replaced text never activate a stale link', async t => {
  const h = await terminal(t);
  await h.write('https://example.com\r\n' + lines(20));
  h.term.scrollToTop();
  const p = h.position(4);
  h.touch('touchstart', p);
  assert.equal(h.touch('touchmove', { x: p.x, y: p.y + 30 }).defaultPrevented, false);
  h.touch('touchend', { x: p.x, y: p.y + 30 });
  h.touch('touchstart', p); h.touch('touchcancel', p); h.touch('touchend', p);
  h.touch('touchstart', p); h.term.scrollLines(2); h.touch('touchend', p);
  h.term.scrollToTop();
  h.touch('touchstart', p);
  await h.reset('https://different.example');
  h.touch('touchend', p);
  assert.deepEqual(h.calls.urls, []);
  h.tap(4);
  assert.deepEqual(h.calls.urls, ['https://different.example']);
});

test('path checks are deduplicated, context-bound, strict booleans and safe against late replies', async t => {
  const h = await terminal(t);
  await h.write('src/Main.kt:12:5');
  assert.equal(h.calls.checks.length, 1);
  const first = h.calls.checks[0];
  h.tap(3); h.tap(3);
  assert.equal(h.calls.checks.length, 1);
  assert.deepEqual(h.calls.paths, []);
  h.api.invalidateLinks();
  await h.settle();
  const second = h.calls.checks.at(-1);
  assert.notEqual(second.id, first.id);
  h.api.linksChecked(first.id, [true]);
  h.tap(3);
  assert.deepEqual(h.calls.paths, []);
  h.api.linksChecked(second.id, ['true']);
  h.tap(3);
  assert.deepEqual(h.calls.paths, []);
  h.api.invalidateLinks(); await h.settle();
  h.api.linksChecked(h.calls.checks.at(-1).id, [true]);
  h.tap(3);
  assert.deepEqual(h.calls.paths, ['src/Main.kt:12:5']);
  h.api.invalidateLinks(); await h.settle();
  const pending = h.calls.checks.at(-1);
  await h.reset('src/Main.kt:12:5');
  h.api.linksChecked(pending.id, [true]); h.tap(3);
  assert.equal(h.calls.paths.length, 1);
});

test('a changed cwd between press and release cancels file activation', async t => {
  const h = await terminal(t);
  await h.write('README.md');
  h.api.linksChecked(h.calls.checks[0].id, [true]);
  const p = h.position(2);
  h.touch('touchstart', p);
  h.api.invalidateLinks();
  h.touch('touchend', p);
  assert.deepEqual(h.calls.paths, []);
});

test('detached, partial and throwing bridges do not prevent terminal input or selection', async t => {
  const h = await terminal(t, { bridge: { openUrl: undefined, checkLinks: undefined } });
  await h.write('https://example.com README.md');
  assert.equal(h.host.querySelectorAll('.workflow-link').length, 0);
  h.tap(2);
  h.w.Workflow.openUrl = () => { throw new Error('No activity handler'); };
  assert.doesNotThrow(() => h.tap(2));
  h.api.key('\t', null);
  assert.deepEqual(h.calls.keys, ['\t']);
  h.w.Workflow = null;
  assert.doesNotThrow(() => { h.api.invalidateLinks(); h.api.key('\r'); h.api.selectAll(); h.api.clearSelection(); });
});

test('long press selects wrapped surrogate, combining and wide cells without duplicated cell lengths', async t => {
  const h = await terminal(t, { cols: 10 });
  const word = 'ab🚀e\u0301中文xyz';
  await h.write('go ' + word + ' tail');
  let wide;
  for (let y = 0; y < h.term.buffer.active.length; y++) {
    for (let x = 0; x < h.term.cols; x++) {
      if (h.term.buffer.active.getLine(y)?.getCell(x)?.getChars() === '中') wide = { x, y };
    }
  }
  assert.ok(wide);
  await h.longPress(wide.x + 1, wide.y); // The second physical cell of a CJK glyph.
  assert.equal(h.term.getSelection(), word);
  assert.equal(h.handle('start').hidden, false);
  assert.equal(h.handle('end').hidden, false);
  assert.equal(h.calls.selections.at(-1)[0], word);
  assert.deepEqual(h.calls.urls, []);
});

test('each visible handle drags independently and copy callbacks contain the final exact selection', async t => {
  const h = await terminal(t, { cols: 30 });
  await h.write('one two three four');
  await h.longPress(5);
  assert.equal(h.term.getSelection(), 'two');
  const initial = selection(h);
  await h.dragHandle('start', -4);
  assert.equal(h.term.getSelection(), 'one two');
  assert.deepEqual(selection(h).end, initial.end);
  await h.dragHandle('end', 6);
  assert.equal(h.term.getSelection(), 'one two three');
  assert.equal(h.calls.selections.at(-1)[0], 'one two three');
  assert.ok(h.calls.selections.some(([text, x, y]) => text === '' && x === 0 && y === 0));
  h.api.clearSelection();
  assert.equal(h.handle('start').hidden, true);
  assert.equal(h.handle('end').hidden, true);
});

test('selection handle edge drag scrolls, then cancellation stops autoscroll', async t => {
  const h = await terminal(t, { cols: 20, rows: 6 });
  await h.write(lines(40)); h.term.scrollToLine(10); await h.settle();
  await h.longPress(3, 2);
  const node = h.handle('end');
  const r = node.getBoundingClientRect();
  const p = { x: r.left + 22, y: r.top + 22 };
  h.touch('touchstart', p, node);
  h.touch('touchmove', { x: p.x, y: p.y + 250 }, h.w.document);
  await h.sleep(130);
  assert.ok(h.term.buffer.active.viewportY > 10);
  h.touch('touchcancel', { x: p.x, y: p.y + 250 }, h.w.document);
  const stopped = h.term.buffer.active.viewportY;
  await h.sleep(130);
  assert.equal(h.term.buffer.active.viewportY, stopped);
  assert.ok(h.term.getSelection().includes('line-'));
});

test('large scrollbar maps full-track touch drags to rows and preserves the viewport during streaming', async t => {
  const h = await terminal(t, { rows: 8 });
  await h.write(lines(100));
  const bar = h.scrollbar();
  const r = bar.getBoundingClientRect();
  const thumb = bar.querySelector('.workflow-scroll-thumb');
  assert.equal(bar.hidden, false);
  assert.ok(parseFloat(thumb.style.height) >= 48);
  assert.equal(h.w.getComputedStyle(bar).width, '44px');
  const max = h.term.buffer.active.baseY;
  const p = { x: r.left + 22, y: r.top + r.height / 2 };
  h.touch('touchstart', p, bar);
  assert.equal(h.term.buffer.active.viewportY, Math.round(max / 2));
  await h.write('\r\n' + lines(10));
  assert.equal(h.term.buffer.active.viewportY, Math.round(max / 2));
  h.touch('touchmove', { x: p.x - 500, y: r.top + r.height + 100 }, h.w.document);
  assert.equal(h.term.buffer.active.viewportY, max); // Mapping was frozen at drag start.
  h.touch('touchend', p, h.w.document);
  h.key(bar, 'Home'); assert.equal(h.term.buffer.active.viewportY, 0);
  h.key(bar, 'PageDown'); assert.equal(h.term.buffer.active.viewportY, h.term.rows);
  h.key(bar, 'End'); assert.equal(h.term.buffer.active.viewportY, h.term.buffer.active.baseY);
  await h.write('\r\nnext');
  assert.equal(h.term.buffer.active.viewportY, h.term.buffer.active.baseY);
});

test('pointer capture survives leaving the scrollbar and cancellation releases it', async t => {
  const h = await terminal(t, { pointer: true });
  await h.write(lines(70));
  const bar = h.scrollbar();
  const capture = [];
  bar.setPointerCapture = id => capture.push(['set', id]);
  bar.releasePointerCapture = id => capture.push(['release', id]);
  const p = { x: 430, y: 90 };
  h.pointerEvent('pointerdown', p, bar, 12);
  h.pointerEvent('pointermove', { x: -100, y: -100 }, h.w.document, 12);
  assert.equal(h.term.buffer.active.viewportY, 0);
  h.pointerEvent('pointercancel', p, h.w.document, 12);
  h.pointerEvent('pointermove', { x: 500, y: 500 }, h.w.document, 12);
  assert.equal(h.term.buffer.active.viewportY, 0);
  assert.deepEqual(capture, [['set', 12], ['release', 12]]);
});

test('reset retires queued old output, selection, scrollbar anchor and pending path checks', async t => {
  const h = await terminal(t);
  await h.write(lines(80));
  h.term.scrollToLine(5); h.api.selectAll();
  h.api.write('old-running\r\n');
  h.api.write('old-queued\r\n');
  h.api.reset('new-generation\r\n');
  h.api.write('new-tail');
  await h.settle();
  assert.match(h.text(), /new-generation\nnew-tail/);
  assert.doesNotMatch(h.text(), /old-|line-/);
  assert.equal(h.term.hasSelection(), false);
  assert.equal(h.term.buffer.active.viewportY, 0);
  assert.equal(h.handle('start').hidden, true);
  assert.equal(h.handle('end').hidden, true);
  assert.equal(h.scrollbar().hidden, true);
});

test('resize reflows links, resets invalid selection geometry and preserves keyboard input', async t => {
  const h = await terminal(t, { cols: 40 });
  await h.write('https://example.com/after-resize\r\n');
  h.api.selectAll();
  h.term.resize(12, 8); await h.settle();
  assert.equal(h.term.hasSelection(), false);
  h.tap(2, 1);
  assert.deepEqual(h.calls.urls, ['https://example.com/after-resize']);
  h.term.textarea.dispatchEvent(new h.w.KeyboardEvent('keydown', { key: 'a', keyCode: 65, which: 65, bubbles: true, cancelable: true }));
  assert.deepEqual(h.calls.input, ['a']);
  h.api.key('\x1b[A', '\x1bOA');
  await h.write('\x1b[?1h');
  h.api.key('\x1b[A', '\x1bOA');
  assert.deepEqual(h.calls.keys, ['\x1b[A', '\x1bOA']);
});

test('no navigation or selection can target the old screen while a reset waits for an in-flight write', async t => {
  const h = await terminal(t);
  await h.write('https://old.example\r\n');
  h.api.write('pending');
  h.api.reset('https://new.example');
  h.tap(3); h.api.selectAll();
  assert.deepEqual(h.calls.urls, []);
  assert.equal(h.term.hasSelection(), false);
  await h.settle();
  h.tap(3);
  assert.deepEqual(h.calls.urls, ['https://new.example']);
});

test('corner selections keep two separate touch targets and accessible keyboard adjustment', async t => {
  const h = await terminal(t);
  await h.write('a b');
  await h.longPress(0);
  const start = h.handle('start').getBoundingClientRect();
  const end = h.handle('end').getBoundingClientRect();
  assert.ok(start.right <= end.left || end.right <= start.left || start.bottom <= end.top || end.bottom <= start.top);
  assert.equal(h.key(h.handle('end'), 'ArrowRight').defaultPrevented, true);
  assert.equal(h.term.getSelection(), 'a ');
  h.key(h.handle('end'), 'ArrowRight');
  assert.equal(h.term.getSelection(), 'a b');
  assert.equal(h.calls.selections.at(-1)[0], 'a b');
  h.key(h.handle('start'), 'Escape');
  assert.equal(h.term.hasSelection(), false);
});

test('pointer handles can cross and continue dragging without moving the opposite endpoint', async t => {
  const h = await terminal(t, { pointer: true });
  await h.write('one two three');
  await h.longPress(5);
  const node = h.handle('start');
  const r = node.getBoundingClientRect();
  const p = { x: r.left + 22, y: r.top + 22 };
  h.pointerEvent('pointerdown', p, node, 5);
  h.pointerEvent('pointermove', { x: p.x + 80, y: p.y }, h.w.document, 5);
  assert.equal(h.term.getSelection(), ' thre');
  h.pointerEvent('pointermove', { x: p.x + 90, y: p.y }, h.w.document, 5);
  h.pointerEvent('pointerup', p, h.w.document, 5);
  assert.equal(h.term.getSelection(), ' three');
  assert.equal(h.calls.selections.at(-1)[0], ' three');
});

test('xterm scrollback trimming retains the visible row and exact selected text until that text expires', async t => {
  const h = await terminal(t, { rows: 6 });
  h.term.options.scrollback = 10;
  await h.write(lines(30));
  h.term.scrollToLine(4); await h.settle();
  const anchor = h.term.buffer.active.getLine(4).translateToString(true);
  await h.longPress(3, 2);
  const selected = h.term.getSelection();
  await h.write('\r\nadded-1\r\nadded-2');
  assert.equal(h.term.buffer.active.viewportY, 2);
  assert.equal(h.term.buffer.active.getLine(2).translateToString(true), anchor);
  assert.equal(h.term.getSelection(), selected);
  await h.write('\r\n' + lines(30));
  assert.equal(h.term.hasSelection(), false);
  assert.equal(h.handle('start').hidden, true);
  assert.deepEqual(h.calls.selections.at(-1), ['', 0, 0]);
});

test('visible path validation stays within the Engine 128-candidate limit', async t => {
  const h = await terminal(t, { cols: 40, rows: 180 });
  await h.write(Array.from({ length: 170 }, (_, i) => `file-${i}.txt`).join('\r\n'));
  assert.ok(h.calls.checks.length > 0);
  assert.ok(h.calls.checks.every(batch => batch.texts.length <= 128));
});
