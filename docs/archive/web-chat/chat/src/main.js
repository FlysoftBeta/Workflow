/* Transcript page entry: window.WorkflowChat.receive({v:1, ops:[…]}) is the only way in,
 * WorkflowBridge.post (bridge.js) the only way out. Protocol: docs/report/initial/w7-chat.md §1. */
import { post } from './bridge.js';
import { h, clear } from './dom.js';
import { requestFrame } from './frame.js';
import { setPaths, onLinkClick } from './links.js';
import { Scroller } from './scroll.js';
import { TurnView } from './turns.js';

const root = document.getElementById('transcript') || document.body.appendChild(h('main', { id: 'transcript' }));
const scroller = new Scroller();
const turns = new Map();
let ticker = 0;
let warned = false;

/* Sizes scaled by the theme op's fontScale (px). */
const SIZES = { '--wf-chat-size': 15, '--wf-chat-line': 24, '--wf-mono-size': 13 };
const sizes = Object.assign({}, SIZES);
let fontScale = 1;

function applyTheme(op) {
  const style = document.documentElement.style;
  const vars = op.vars && typeof op.vars === 'object' ? op.vars : {};
  for (const name of Object.keys(vars)) {
    const value = vars[name];
    if (!/^--wf-[a-z0-9-]+$/.test(name) || (typeof value !== 'string' && typeof value !== 'number')) continue;
    if (name in SIZES) { const px = parseFloat(value); if (px > 0) sizes[name] = px; } else style.setProperty(name, String(value));
  }
  if (typeof op.fontScale === 'number' && op.fontScale > 0) fontScale = Math.min(3, Math.max(0.5, op.fontScale));
  for (const name of Object.keys(SIZES)) style.setProperty(name, Math.round(sizes[name] * fontScale * 10) / 10 + 'px');
  if (typeof op.dark === 'boolean') document.documentElement.classList.toggle('dark', op.dark);
}

const list = value => (Array.isArray(value) ? value.filter(t => t && t.id != null) : []);

function insertTurn(data, before) {
  const old = turns.get(data.id);
  const view = new TurnView(data);
  if (old) root.replaceChild(view.el, old.el);
  else root.insertBefore(view.el, before || null);
  turns.set(data.id, view);
}

function dropTurn(id) {
  const view = turns.get(id);
  if (!view) return;
  turns.delete(id);
  if (view.el.parentNode) root.removeChild(view.el);
}

function withTurn(id, fn) {
  const view = turns.get(id);
  if (view) fn(view);
}

const OPS = {
  reset(op) {
    turns.clear();
    clear(root);
    setPaths(op.paths);
    list(op.turns).forEach(t => insertTurn(t, null));
    scroller.setEarlier(op.hasEarlier);
    scroller.toBottom();
  },
  theme: applyTheme,
  append(op) { list(op.turns).forEach(t => insertTurn(t, null)); },
  prepend(op) {
    scroller.keepAnchor(() => {
      const first = root.firstChild;
      list(op.turns).forEach(t => { if (turns.has(t.id)) insertTurn(t); else insertTurn(t, first); });
    });
    scroller.setEarlier(op.hasEarlier);
  },
  turn(op) {
    if (!op.turn || op.turn.id == null) return;
    const view = turns.get(op.turn.id);
    if (view) view.update(op.turn);
    else insertTurn(op.turn, null);
  },
  item(op) {
    if (op.item && op.item.id != null && op.item.kind) withTurn(op.turn, v => v.upsertItem(op.item, op.after));
  },
  text(op) { withTurn(op.turn, v => v.appendText(op.id, op.append)); },
  remove(op) {
    if (op.id != null) withTurn(op.turn, v => v.removeItem(op.id));
    else dropTurn(op.turn);
  },
  finalize(op) {
    withTurn(op.turn, v => v.update({ status: op.status || 'completed', durationMs: op.durationMs, final: op.final, error: op.error }));
  },
  collapse(op) { withTurn(op.turn, v => v.setExpanded(op.expanded)); },
  scroll(op) {
    if (op.to === 'bottom') scroller.toBottom();
    if (typeof op.follow === 'boolean') scroller.setFollow(op.follow);
  }
};

function syncTicker() {
  let running = false;
  turns.forEach(v => { running = running || v.running(); });
  if (running && !ticker) {
    ticker = setInterval(() => { const now = Date.now(); turns.forEach(v => v.tick(now)); }, 1000);
  } else if (!running && ticker) {
    clearInterval(ticker);
    ticker = 0;
  }
}

function receive(input) {
  let batch = input;
  if (typeof input === 'string') {
    try { batch = JSON.parse(input); } catch (e) { return false; }
  }
  if (!batch || typeof batch !== 'object') return false;
  if (batch.v !== 1) {
    if (!warned) { warned = true; console.warn('WorkflowChat: ignoring batch with v=' + batch.v); }
    return false;
  }
  for (const op of Array.isArray(batch.ops) ? batch.ops : []) {
    const name = op && (op.op || op.type);
    if (!Object.prototype.hasOwnProperty.call(OPS, name)) continue;
    try { OPS[name](op); } catch (e) { console.error(e); }
  }
  syncTicker();
  requestFrame();
  return true;
}

window.WorkflowChat = { receive };
document.addEventListener('click', onLinkClick);

function ready() {
  post({ type: 'ready', v: 1 });
  requestFrame();
}
if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', ready);
else ready();
