import { test } from 'node:test';
import assert from 'node:assert/strict';
import { terminal } from './terminal-harness.mjs';

// Touch scrolling, link taps and selection chrome with the packaged xterm parser, buffer and selection.
const lines = count => Array.from({ length: count }, (_, i) => `line-${String(i).padStart(3, '0')}`).join('\r\n');
const viewport = h => h.term.buffer.active.viewportY;

test('a vertical swipe moves the scrollback 1:1 with the finger and never activates the link under it', async t => {
  const h = await terminal(t, { rows: 8 });
  await h.write(Array.from({ length: 100 }, (_, i) => `https://example.com/${i}`).join('\r\n'));
  const bottom = viewport(h);
  // Down by four rows plus the tap slop and a fraction: older output, held still before release.
  const end = await h.swipe(h.position(4, 1), 22 * 4 + 15, { pause: 80 });
  assert.equal(end.defaultPrevented, true);
  assert.equal(viewport(h), bottom - 4);
  await h.sleep(80);
  assert.equal(viewport(h), bottom - 4, 'a finger that stopped before release does not fling');
  await h.swipe(h.position(4, 6), -(22 * 2 + 15), { pause: 80 });
  assert.equal(viewport(h), bottom - 2);
  // A horizontal pan is neither a scroll nor a tap.
  await h.swipe(h.position(4, 3), 0, { dx: 60, pause: 80 });
  assert.equal(viewport(h), bottom - 2);
  assert.deepEqual(h.calls.urls, []);
});

test('a released swipe flings with decaying momentum; a tap stops the fling without activating', async t => {
  const h = await terminal(t, { rows: 8 });
  await h.write(Array.from({ length: 400 }, (_, i) => `https://example.com/${i}`).join('\r\n'));
  await h.swipe(h.position(4, 1), 120, { steps: 4, stepMs: 12 });
  const released = viewport(h);
  await h.sleep(90);
  const coasting = viewport(h);
  assert.ok(coasting < released - 2, `momentum continues after release (${released} -> ${coasting})`);
  h.tap(4, 3);
  const stopped = viewport(h);
  await h.sleep(90);
  assert.equal(viewport(h), stopped);
  assert.deepEqual(h.calls.urls, [], 'the tap that stops a fling does not open the link below it');
  h.tap(4, 3);
  assert.equal(h.calls.urls.length, 1);
  // A fling ends at the oldest row.
  h.term.scrollToLine(6);
  await h.settle();
  await h.swipe(h.position(4, 1), 160, { steps: 3, stepMs: 8 });
  await h.sleep(150);
  assert.equal(viewport(h), 0);
});

test('in the alternate screen a swipe is xterm wheel input: cursor keys, or wheel reports when requested', async t => {
  const h = await terminal(t, { rows: 8 });
  await h.write('\x1b[?1049hfull screen program');
  await h.swipe(h.position(5, 2), 22 * 2 + 15, { pause: 80 });
  assert.deepEqual(h.calls.input, ['\x1b[A', '\x1b[A']);
  h.calls.input.length = 0;
  await h.write('\x1b[?1000h\x1b[?1006h');
  await h.swipe(h.position(5, 5), -(22 + 15), { pause: 80 });
  assert.equal(h.calls.input.length, 1);
  assert.match(h.calls.input[0], /^\x1b\[<65;\d+;\d+M$/);
});

test('a thumb drag survives capture moving to the track and a keyboard resize, mapping the whole track', async t => {
  const h = await terminal(t, { rows: 8, pointer: true });
  await h.write(lines(200));
  const bar = h.scrollbar();
  const thumb = bar.querySelector('.workflow-scroll-thumb');
  bar.setPointerCapture = () => {};
  bar.releasePointerCapture = () => {};
  const r = bar.getBoundingClientRect();
  const max = h.term.buffer.active.baseY;
  const travel = r.height - parseFloat(thumb.style.height);
  const grab = { x: r.left + 22, y: r.top + parseFloat(thumb.style.top) + 10 };
  h.pointerEvent('pointerdown', grab, thumb, 3);
  assert.equal(viewport(h), max, 'pressing the thumb does not jump');
  // Chromium's implicit touch capture on the thumb moves to the scrollbar: a bubbling lostpointercapture.
  h.pointerEvent('lostpointercapture', grab, thumb, 3);
  h.pointerEvent('pointermove', { x: grab.x, y: grab.y - travel / 2 }, h.w.document, 3);
  assert.equal(viewport(h), Math.round(max / 2));
  // The soft keyboard takes two rows mid-drag. The drag continues on the remapped, shorter track.
  h.term.resize(h.term.cols, 6);
  await h.settle();
  h.pointerEvent('pointermove', { x: grab.x, y: r.top - 80 }, h.w.document, 3);
  assert.equal(viewport(h), 0);
  h.pointerEvent('pointermove', { x: grab.x, y: r.top + r.height + 80 }, h.w.document, 3);
  assert.equal(viewport(h), h.term.buffer.active.baseY);
  h.pointerEvent('pointerup', grab, h.w.document, 3);
  h.pointerEvent('pointermove', { x: grab.x, y: r.top }, h.w.document, 3);
  assert.equal(viewport(h), h.term.buffer.active.baseY);
  // Losing capture on the scrollbar itself does end a drag.
  h.pointerEvent('pointerdown', { x: grab.x, y: r.top + 1 }, bar, 4);
  h.pointerEvent('lostpointercapture', grab, bar, 4);
  const ended = viewport(h);
  h.pointerEvent('pointermove', { x: grab.x, y: r.top + r.height }, h.w.document, 4);
  assert.equal(viewport(h), ended);
});

test('a rows-only resize keeps the selection and its copy text; input and reflow clear it', async t => {
  const h = await terminal(t, { cols: 30, rows: 8 });
  await h.write(lines(30) + '\r\nalpha beta gamma');
  await h.longPress(7, 7);
  assert.equal(h.term.getSelection(), 'beta');
  h.term.resize(30, 5);
  await h.settle();
  assert.equal(h.term.getSelection(), 'beta');
  h.term.resize(30, 9);
  await h.settle();
  assert.equal(h.term.getSelection(), 'beta');
  assert.equal(h.calls.selections.at(-1)[0], 'beta');
  assert.equal(h.handle('end').hidden, false);
  h.term.resize(20, 9);
  await h.settle();
  assert.equal(h.term.hasSelection(), false);
  assert.deepEqual(h.calls.selections.at(-1), ['', 0, 0]);
  // Typing clears xterm's selection; a later keyboard resize must not resurrect it.
  await h.longPress(7, h.term.rows - 1);
  assert.ok(h.term.hasSelection());
  h.term.textarea.dispatchEvent(new h.w.KeyboardEvent('keydown', { key: 'x', keyCode: 88, which: 88, bubbles: true, cancelable: true }));
  await h.settle();
  h.term.resize(20, 7);
  await h.settle();
  assert.equal(h.term.hasSelection(), false);
});

test('a link tap survives output streaming under the finger, but not the link leaving the screen', async t => {
  const h = await terminal(t, { rows: 8 });
  await h.write(lines(20) + '\r\nhttps://example.com/streaming\r\nprompt');
  const p = h.position(4, 6);
  h.touch('touchstart', p);
  await h.write('\r\nnew output');
  assert.equal(h.touch('touchend', p).defaultPrevented, true);
  assert.deepEqual(h.calls.urls, ['https://example.com/streaming']);
  const q = h.position(4, 5);
  h.touch('touchstart', q);
  await h.write('\r\n' + lines(10));
  h.touch('touchend', q);
  assert.equal(h.calls.urls.length, 1);
  // A rows-only resize between press and release keeps the pending tap.
  h.term.scrollToTop();
  await h.write('');
  await h.reset('https://example.com/resized\r\n');
  h.touch('touchstart', h.position(4));
  h.term.resize(h.term.cols, 6);
  await h.settle();
  h.touch('touchend', h.position(4));
  assert.deepEqual(h.calls.urls.at(-1), 'https://example.com/resized');
});

test('hard-wrapped URLs that fill a row continue on the next row, even when the start is above the viewport', async t => {
  const h = await terminal(t, { cols: 20, rows: 4 });
  // A TUI prints each row itself: no soft wrap joins these lines.
  await h.write('https://example.com/\r\npath/a?x=1 tail\r\nhttps://no.example/a\r\n indented');
  assert.equal(h.term.buffer.active.getLine(1).isWrapped, false);
  h.tap(2, 1);
  h.tap(5, 0);
  h.tap(5, 2);
  assert.deepEqual(h.calls.urls, ['https://example.com/path/a?x=1', 'https://example.com/path/a?x=1', 'https://no.example/a']);
  assert.ok(h.calls.checks.every(batch => !batch.texts.some(text => text.includes('path/a'))), 'the continuation is not a path candidate');
  await h.write('\r\nmore\r\nrows');
  h.term.scrollToLine(1);
  await h.settle();
  h.tap(2, 0);
  assert.equal(h.calls.urls.at(-1), 'https://example.com/path/a?x=1');
});

test('OSC 8 hyperlinks open their target rather than the label, and non-web targets stay inactive', async t => {
  const h = await terminal(t, { cols: 60 });
  await h.write('\x1b]8;;https://example.com/osc\x07click here\x1b]8;;\x07 then \x1b]8;;file:///etc/passwd\x07secret\x1b]8;;\x07' +
    ' \x1b]8;;https://target.example/\x07https://label.example/\x1b]8;;\x07');
  h.tap(3);
  h.tap(18);
  h.tap(30);
  assert.deepEqual(h.calls.urls, ['https://example.com/osc', 'https://target.example/']);
  assert.ok(h.host.querySelector('button.workflow-link[aria-label="Open https://example.com/osc"]'));
});

test('a long-press contextmenu never reaches xterm: no textarea focus, keyboard or lost selection', async t => {
  const h = await terminal(t);
  await h.write('alpha beta');
  const p = h.position(7);
  h.touch('touchstart', p);
  await h.sleep(480);
  const menu = new h.w.MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: p.x, clientY: p.y, button: 2 });
  h.screen.dispatchEvent(menu);
  assert.equal(menu.defaultPrevented, true);
  assert.notEqual(h.w.document.activeElement, h.term.textarea);
  assert.notEqual(h.term.textarea.style.zIndex, '1000');
  h.touch('touchend', p);
  await h.settle();
  assert.equal(h.term.getSelection(), 'beta');
  assert.equal(h.calls.selections.at(-1)[0], 'beta');
});

test('long press selects the word under the finger even while output moves the viewport', async t => {
  const h = await terminal(t, { rows: 4 });
  await h.write(lines(10) + '\r\nalpha beta');
  const p = h.position(7, 3);
  h.touch('touchstart', p);
  await h.write('\r\nnewest words');
  await h.sleep(450);
  h.touch('touchend', p);
  await h.settle();
  assert.equal(h.term.getSelection(), 'words');
});

test('native chrome receives handle geometry and toolbar state, and native handle drags edit the selection', async t => {
  const h = await terminal(t, { cols: 30 });
  await h.write('one two three four');
  assert.ok(h.host.classList.contains('native-selection-chrome'));
  const p = h.position(5);
  h.touch('touchstart', p);
  await h.sleep(480);
  assert.deepEqual([h.calls.states.at(-1).active, h.calls.states.at(-1).toolbar], [true, false], 'no toolbar while held');
  h.touch('touchend', p);
  await h.settle();
  const state = h.calls.states.at(-1);
  assert.deepEqual(state, { active: true, toolbar: true, start: { x: 44, y: 26, visible: true }, end: { x: 74, y: 26, visible: true },
    rect: { left: 44, right: 74, top: 4, bottom: 26 }, line: 22, width: h.w.innerWidth });
  assert.equal(h.api.dragHandle('end', 'start', state.end.x, state.end.y), true);
  assert.equal(h.calls.states.at(-1).toolbar, false);
  assert.deepEqual(h.calls.selections.at(-1), ['', 0, 0]);
  h.api.dragHandle('end', 'move', state.end.x + 60, state.end.y);
  h.api.dragHandle('end', 'end', state.end.x + 60, state.end.y);
  await h.settle();
  assert.equal(h.term.getSelection(), 'two three');
  assert.equal(h.calls.selections.at(-1)[0], 'two three');
  assert.equal(h.calls.states.at(-1).toolbar, true);
  // A cancelled native drag keeps what it selected and brings the toolbar back.
  h.api.dragHandle('start', 'start', state.start.x, state.start.y);
  h.api.dragHandle('start', 'move', state.start.x - 40, state.start.y);
  h.api.dragHandle('start', 'cancel', state.start.x - 40, state.start.y);
  await h.settle();
  assert.equal(h.term.getSelection(), 'one two three');
  assert.equal(h.calls.states.at(-1).toolbar, true);
  assert.equal(h.api.dragHandle('end', 'move', 0, 0), false, 'moves without a started drag are ignored');
  h.api.clearSelection();
  await h.settle();
  assert.deepEqual(h.calls.states.at(-1), { active: false });
  const plain = await terminal(t, { bridge: { selectionState: undefined } });
  assert.equal(plain.host.classList.contains('native-selection-chrome'), false);
});

test('selection chrome reports offscreen endpoints and an offscreen toolbar rectangle', async t => {
  const h = await terminal(t, { cols: 20, rows: 4 });
  await h.write(lines(20));
  h.api.selectAll();
  await h.settle();
  let state = h.calls.states.at(-1);
  assert.equal(state.start.visible, false);
  assert.equal(state.end.visible, true);
  assert.deepEqual(state.rect, { left: 4, right: 204, top: 4, bottom: 92 });
  h.api.clearSelection();
  await h.longPress(2, 0);
  await h.write('\r\n' + lines(8));
  state = h.calls.states.at(-1);
  assert.equal(state.active, true);
  assert.equal(state.start.visible, false);
  assert.equal(state.rect, null);
});
