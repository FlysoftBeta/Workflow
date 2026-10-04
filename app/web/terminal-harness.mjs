import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { JSDOM } from 'jsdom';

export const assets = new URL('./build/test-assets/', import.meta.url);
const scripts = await Promise.all([
  'vendor/xterm-xterm/xterm.js', 'android-input.js', 'terminal-links.js', 'terminal.js'
].map(file => readFile(new URL(file, assets), 'utf8')));
const css = await readFile(new URL('terminal.css', assets), 'utf8');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

// Real packaged xterm, parser, buffer, selection, bridge adapter and DOM listeners.
// JSDOM has no layout: the only substitutes below are font/cell measurement, fit
// (resize is explicit), canvas paint and optional PointerEvent dispatch.
export async function terminal(t, { cols = 40, rows = 8, bridge = {}, pointer = false } = {}) {
  const dom = new JSDOM('<!doctype html><head></head><body><div id="terminal"></div></body>', {
    runScripts: 'outside-only', pretendToBeVisual: true
  });
  const w = dom.window;
  const style = w.document.createElement('style');
  style.textContent = css;
  w.document.head.appendChild(style);
  w.matchMedia = () => ({ matches: false, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {} });
  w.ResizeObserver = class { observe() {} disconnect() {} };
  w.HTMLCanvasElement.prototype.getContext = () => ({
    createLinearGradient() { return { addColorStop() {} }; }, fillRect() {},
    measureText() { return { width: 10 }; }, getImageData() { return { data: new Uint8ClampedArray(4) }; }
  });
  Object.defineProperty(w.HTMLElement.prototype, 'offsetWidth', { get() { return this.classList.contains('xterm-char-measure-element') ? 320 : parseFloat(this.style.width) || cols * 10; } });
  Object.defineProperty(w.HTMLElement.prototype, 'offsetHeight', { get() { return this.classList.contains('xterm-char-measure-element') ? 20 : parseFloat(this.style.height) || rows * 22; } });
  const rect = (x, y, width, height) => ({ x, y, left: x, top: y, width, height, right: x + width, bottom: y + height });
  let term;
  w.HTMLElement.prototype.getBoundingClientRect = function () {
    if (this.hidden) return rect(0, 0, 0, 0);
    const columns = term?.cols ?? cols;
    const lines = term?.rows ?? rows;
    if (this.id === 'terminal') return rect(0, 0, columns * 10 + 48, lines * 22 + 8);
    if (this.classList.contains('workflow-scrollbar')) return rect(columns * 10 + 4, 4, 44, lines * 22);
    if (this.classList.contains('workflow-selection-handle')) return rect(parseFloat(this.style.left) || 0, parseFloat(this.style.top) || 0, 44, 44);
    return rect(4, 4, columns * 10, lines * 22);
  };
  if (pointer) w.PointerEvent = w.MouseEvent;
  else w.PointerEvent = undefined;
  const calls = { urls: [], paths: [], checks: [], selections: [], states: [], input: [], keys: [], resize: [], ready: 0 };
  w.Workflow = {
    ready() { calls.ready++; }, input(text) { calls.input.push(text); }, key(text) { calls.keys.push(text); },
    resize(...size) { calls.resize.push(size); }, openUrl(url) { calls.urls.push(url); }, openPath(path) { calls.paths.push(path); },
    checkLinks(id, json) { calls.checks.push({ id, texts: JSON.parse(json) }); }, selection(...args) { calls.selections.push(args); },
    selectionState(json) { calls.states.push(JSON.parse(json)); }, ...bridge
  };
  w.eval(scripts[0]);
  const RealTerminal = w.Terminal;
  w.Terminal = function (options) { term = new RealTerminal({ ...options, cols, rows, cursorBlink: false }); return term; };
  w.FitAddon = { FitAddon: class { activate() {} dispose() {} fit() {} } };
  scripts.slice(1).forEach(source => w.eval(source));
  const settle = async () => { await sleep(25); await sleep(25); };
  await settle();
  assert.equal(calls.ready, 1);
  const host = w.document.getElementById('terminal');
  const screen = host.querySelector('.xterm-screen');
  const api = w.WorkflowTerminal;
  const position = (col, row = 0) => ({ x: 4 + (col + 0.5) * 10, y: 4 + (row + 0.5) * 22 });
  function touch(type, point, target = screen, count = 1) {
    const event = new w.Event(type, { bubbles: true, cancelable: true });
    const value = { clientX: point.x, clientY: point.y, identifier: 7 };
    Object.defineProperties(event, {
      touches: { value: type === 'touchend' || type === 'touchcancel' ? [] : Array(count).fill(value) },
      changedTouches: { value: [value] }
    });
    target.dispatchEvent(event);
    return event;
  }
  function mouse(type, point, target = screen) {
    const event = new w.MouseEvent(type, { bubbles: true, cancelable: true, clientX: point.x, clientY: point.y, button: 0 });
    target.dispatchEvent(event); return event;
  }
  function pointerEvent(type, point, target = screen, pointerId = 1) {
    const event = new w.MouseEvent(type, { bubbles: true, cancelable: true, clientX: point.x, clientY: point.y, button: 0 });
    Object.defineProperties(event, { pointerId: { value: pointerId }, isPrimary: { value: true } });
    target.dispatchEvent(event); return event;
  }
  const tap = (col, row = 0) => { const p = position(col, row); touch('touchstart', p); return touch('touchend', p); };
  const longPress = async (col, row = 0) => { const p = position(col, row); touch('touchstart', p); await sleep(480); touch('touchend', p); await settle(); };
  const handle = edge => host.querySelector('.workflow-selection-handle.' + edge);
  async function dragHandle(edge, dx, dy = 0) {
    const node = handle(edge);
    assert.equal(node.hidden, false);
    const r = node.getBoundingClientRect();
    const p = { x: r.left + 22, y: r.top + 22 };
    touch('touchstart', p, node);
    touch('touchmove', { x: p.x + dx * 10, y: p.y + dy * 22 }, w.document);
    touch('touchend', { x: p.x + dx * 10, y: p.y + dy * 22 }, w.document);
    await settle();
  }
  // A one-finger swipe with real elapsed time between moves; `pause` ms before release suppresses a fling.
  async function swipe(start, dy, { steps = 6, stepMs = 16, pause = 0, dx = 0 } = {}) {
    touch('touchstart', start);
    let last = start;
    for (let step = 1; step <= steps; step++) {
      await sleep(stepMs);
      last = { x: start.x + dx * step / steps, y: start.y + dy * step / steps };
      touch('touchmove', last);
    }
    if (pause) await sleep(pause);
    return touch('touchend', last);
  }
  function key(target, value) {
    const event = new w.KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true });
    target.dispatchEvent(event); return event;
  }
  const text = () => Array.from({ length: term.buffer.active.length }, (_, index) => term.buffer.active.getLine(index)?.translateToString(true) ?? '').join('\n');
  t.after(() => { w.dispatchEvent(new w.Event('blur')); api.clearSelection(); term.dispose(); w.close(); });
  return { w, term, api, host, screen, calls, settle, sleep, position, touch, tap, mouse, pointerEvent, key, swipe,
    longPress, handle, dragHandle, text, scrollbar: () => host.querySelector('.workflow-scrollbar'),
    async write(value) { api.write(value); await settle(); }, async reset(value = '') { api.reset(value); await settle(); } };
}
