import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { JSDOM } from 'jsdom';

const assets = new URL('./build/test-assets/', import.meta.url);
const [vendor, adapter] = await Promise.all([
  readFile(new URL('vendor/xterm-xterm/xterm.js', assets), 'utf8'),
  readFile(new URL('android-input.js', assets), 'utf8')
]);

// Run the shipped xterm bundle and its actual DOM listeners/CompositionHelper.
// Only browser layout APIs absent in JSDOM are stubbed; input logic is untouched.
function terminal(t, patched = true) {
  const dom = new JSDOM('<!doctype html><div id="terminal"></div>', {
    runScripts: 'outside-only', pretendToBeVisual: true
  });
  const w = dom.window;
  w.matchMedia = () => ({ matches: false, addListener() {}, removeListener() {},
    addEventListener() {}, removeEventListener() {} });
  w.ResizeObserver = class { observe() {} disconnect() {} };
  w.HTMLCanvasElement.prototype.getContext = () => ({
    createLinearGradient() { return { addColorStop() {} }; }, fillRect() {},
    measureText() { return { width: 10 }; },
    getImageData() { return { data: new Uint8ClampedArray(4) }; }
  });
  w.eval(vendor);
  const term = new w.Terminal();
  term.open(w.document.getElementById('terminal'));
  w.eval(adapter);
  if (patched) assert.equal(w.WorkflowAndroidInput.install(term), true);
  const chunks = [];
  term.onData(value => chunks.push(value));
  const textarea = term.textarea;
  const key = (type, code, name = 'Unidentified') => textarea.dispatchEvent(new w.KeyboardEvent(type, {
    key: name, keyCode: code, which: code, bubbles: true, composed: true, cancelable: true
  }));
  const input = (data, inputType = 'insertText') => textarea.dispatchEvent(new w.InputEvent('input', {
    data, inputType, bubbles: true, composed: true
  }));
  const change = (next, data, inputType = 'insertText') => {
    key('keydown', 229);
    textarea.dispatchEvent(new w.InputEvent('beforeinput', {
      data, inputType, bubbles: true, composed: true, cancelable: true
    }));
    textarea.value = next;
    input(data, inputType);
    key('keyup', 229);
  };
  const type = text => { for (const character of text) change(textarea.value + character, character); };
  const enter = () => { key('keydown', 13, 'Enter'); key('keyup', 13, 'Enter'); };
  const composition = (kind, data = '') => textarea.dispatchEvent(new w.CompositionEvent(kind, {
    data, bubbles: true, composed: true
  }));
  t.after(() => { term.dispose(); w.close(); });
  return { w, term, textarea, chunks, key, input, change, type, enter, composition,
    output: () => chunks.join(''), settle: () => new Promise(resolve => setTimeout(resolve, 20)) };
}

test('unmodified vendored Terminal reproduces the captured Android 229 burst duplication', async t => {
  const h = terminal(t, false);
  h.type('bookkeeper');
  await h.settle();
  assert.equal(h.output(), 'bookkeeperookkeeperokkeeperkkeeperkeepereepereperpererr');
});

test('Android 229 burst preserves repeated letters and flushes before immediate Enter', async t => {
  const h = terminal(t);
  h.type('bookkeeper');
  h.enter();
  assert.equal(h.output(), 'bookkeeper\r');
  assert.equal(h.textarea.value, '');
  await h.settle();
  assert.equal(h.output(), 'bookkeeper\r');
});

test('last fallback flushes by timer and slow input retains every repeated letter', async t => {
  const h = terminal(t);
  for (const character of 'bookkeeper') {
    h.type(character);
    await h.settle();
  }
  assert.equal(h.output(), 'bookkeeper');
  assert.deepEqual(h.chunks, [...'bookkeeper']);
});

test('burst backspaces emit one DEL per edit before Enter, with no content deduplication', async t => {
  const h = terminal(t);
  h.type('bookkeeper');
  for (let i = 0; i < 3; i++) h.change(h.textarea.value.slice(0, -1), null, 'deleteContentBackward');
  h.enter();
  await h.settle();
  assert.equal(h.output(), 'bookkeeper\x7f\x7f\x7f\r');
});

test('same-length replacement and no-change events retain upstream delta semantics', async t => {
  const h = terminal(t);
  h.type('a');
  h.change('b', 'b', 'insertReplacementText');
  h.change('b', 'b', 'insertReplacementText');
  h.enter();
  await h.settle();
  assert.equal(h.output(), 'ab\r');
});

test('pending Latin text commits before a Chinese composition starts, without duplicating its final text', async t => {
  const h = terminal(t);
  h.type('a');
  h.composition('compositionstart');
  assert.equal(h.output(), 'a');
  h.composition('compositionupdate', '你');
  h.textarea.value = 'a你';
  h.input('你', 'insertCompositionText');
  h.composition('compositionupdate', '你好');
  h.textarea.value = 'a你好';
  h.input('你好', 'insertCompositionText');
  await h.settle();
  assert.equal(h.output(), 'a');
  h.composition('compositionend', '你好');
  h.enter();
  await h.settle();
  assert.equal(h.output(), 'a你好\r');
});

test('229 starting an actual composition does not create a second fallback emission', async t => {
  const h = terminal(t);
  h.key('keydown', 229);
  h.composition('compositionstart');
  h.composition('compositionupdate', '中');
  h.textarea.value = '中';
  h.input('中', 'insertCompositionText');
  h.key('keyup', 229);
  await h.settle();
  assert.equal(h.output(), '');
  h.composition('compositionend', '中');
  await h.settle();
  h.enter();
  assert.equal(h.output(), '中\r');
});

test('consecutive Chinese compositions keep the original finalization boundaries', async t => {
  const h = terminal(t);
  h.composition('compositionstart');
  h.composition('compositionupdate', '你');
  h.textarea.value = '你';
  await h.settle();
  h.composition('compositionend', '你');
  h.composition('compositionstart');
  h.composition('compositionupdate', '好');
  h.textarea.value = '你好';
  await h.settle();
  assert.equal(h.output(), '你');
  h.composition('compositionend', '好');
  await h.settle();
  assert.equal(h.output(), '你好');
});

test('emoji-only input and normal keydown paths are not filtered by the adapter', async t => {
  const h = terminal(t);
  h.textarea.value = '🚀🚀';
  h.input('🚀🚀');
  h.key('keydown', 65, 'a');
  h.key('keyup', 65, 'a');
  h.key('keydown', 65, 'a');
  h.key('keyup', 65, 'a');
  await h.settle();
  assert.equal(h.output(), '🚀🚀aa');
});

test('install is idempotent and does not register another input producer', async t => {
  const h = terminal(t);
  assert.equal(h.w.WorkflowAndroidInput.install(h.term), true);
  assert.equal(h.w.WorkflowAndroidInput.install(h.term), true);
  h.type('aaa');
  h.enter();
  await h.settle();
  assert.equal(h.output(), 'aaa\r');
});

test('unknown helper implementation leaves all original methods untouched', t => {
  const h = terminal(t, false);
  const helper = h.term._core._compositionHelper;
  helper._handleAnyTextareaChanges = function unknownVersion() {};
  const originals = [helper._handleAnyTextareaChanges, helper.keydown, helper.compositionstart];
  assert.equal(h.w.WorkflowAndroidInput.install(h.term), false);
  assert.deepEqual([helper._handleAnyTextareaChanges, helper.keydown, helper.compositionstart], originals);
  assert.equal(h.w.WorkflowAndroidInput.install({}), false);
  assert.equal(h.w.WorkflowAndroidInput.install(null), false);
});

test('disposing the terminal cancels its pending fallback timer', async t => {
  const h = terminal(t);
  let calls = 0;
  const service = h.term._core._compositionHelper._coreService;
  const original = service.triggerDataEvent;
  service.triggerDataEvent = function (...args) { calls++; return original.apply(this, args); };
  h.type('a');
  h.term.dispose();
  await h.settle();
  assert.equal(calls, 0);
});
