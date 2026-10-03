import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { JSDOM } from 'jsdom';
import { buildChat, defaultOutdir } from './build-chat.mjs';

const chatDir = new URL('../app/src/main/assets/web/chat/', import.meta.url);
const ALLOWED = new Set(['ready', 'layout', 'earlier', 'toggle', 'copy', 'openUrl', 'openPath', 'action']);
const allPosts = [];
const cache = new Map();
const load = async url => { const key = String(url); if (!cache.has(key)) cache.set(key, await readFile(url, 'utf8')); return cache.get(key); };

/** Loads the built chat.html, evaluates its scripts in order, stubs WorkflowBridge and scroll geometry. */
async function page() {
  const html = await load(new URL('chat.html', chatDir));
  const dom = new JSDOM(html, { url: 'https://appassets.androidplatform.net/assets/web/chat/chat.html', runScripts: 'outside-only', pretendToBeVisual: true });
  const w = dom.window;
  const posts = [];
  w.WorkflowBridge = { post: json => { const msg = JSON.parse(json); posts.push(msg); allPosts.push(msg); } };
  const geo = { top: 0, height: null, perTurn: 400 };
  const doc = w.document.documentElement;
  Object.defineProperty(doc, 'scrollHeight', { get: () => (geo.height != null ? geo.height : w.document.querySelectorAll('.turn').length * geo.perTurn) });
  Object.defineProperty(doc, 'scrollTop', {
    get: () => geo.top,
    set: value => { geo.top = Math.max(0, Math.min(value, doc.scrollHeight - w.innerHeight)); }
  });
  for (const [, src] of html.matchAll(/<script src="([^"]+)"><\/script>/g)) w.eval(await load(new URL(src, chatDir)));
  const frame = () => new Promise(resolve => w.requestAnimationFrame(() => w.requestAnimationFrame(resolve)));
  const send = ops => w.WorkflowChat.receive(JSON.stringify({ v: 1, ops }));
  const $ = selector => w.document.querySelector(selector);
  const $$ = selector => Array.from(w.document.querySelectorAll(selector));
  const click = el => el.dispatchEvent(new w.MouseEvent('click', { bubbles: true, cancelable: true }));
  const userPosts = () => posts.filter(p => p.type !== 'layout' && p.type !== 'ready');
  return { w, posts, geo, frame, send, $, $$, click, userPosts };
}

const turn = (id, extra = {}) => ({ id, status: 'completed', startedAt: null, durationMs: 5000, timeLabel: null, user: { text: 'hi', attachments: [] }, items: [], final: null, error: null, changes: null, canFork: false, canRetry: false, expanded: null, ...extra });
const message = (id, text, status = 'done', phase = 'final') => ({ id, kind: 'message', phase, status, text });
const mdTurn = text => turn('t', { items: [message('m', text)], final: 'm' });

after(() => {
  for (const p of allPosts) assert.ok(ALLOWED.has(p.type), 'unexpected post type ' + p.type);
});

test('ready is posted once the page loads and unknown ops / versions are ignored', async () => {
  const p = await page();
  await p.frame();
  assert.deepEqual(p.posts[0], { type: 'ready', v: 1 });
  assert.equal(p.w.WorkflowChat.receive('{"v":2,"ops":[{"op":"reset","turns":[]}]}'), false);
  assert.equal(p.w.WorkflowChat.receive('not json'), false);
  p.send([{ op: 'reset', turns: [mdTurn('one')], hasEarlier: false, paths: [] }, { op: 'explode', turn: 't' }, { op: 'item', turn: 'nope', item: message('x', 'y') }]);
  p.send([{ op: 'append', turns: [turn('u', { items: [message('m2', 'two')], final: 'm2' })] }]);
  assert.equal(p.$$('.turn').length, 2);
  assert.equal(p.$('[data-turn="u"] .final').textContent.trim(), 'two');
  assert.deepEqual(p.userPosts(), []);
  p.w.close();
});

const STREAM = 'Intro paragraph with **bold** text.\n\n$$\\frac{1}{2}$$\n\nInline \\(x\\) and $y^2$ here.\n\n- one\n- two\n\n```js\nconst a = "$$";\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nClosing $$\\fr' + 'ac{3}{4}$$ words and $z$ end.';

test('streaming tiny chunks equals a one-shot render, freezes finished blocks and coalesces per frame', async () => {
  const one = await page();
  one.send([{ op: 'reset', turns: [mdTurn(STREAM)], hasEarlier: false, paths: [] }]);
  const expected = one.$('.msg');
  const expectedText = expected.textContent;
  const expectedKatex = expected.querySelectorAll('.katex').length;
  assert.equal(expectedKatex, 5);
  one.w.close();

  const p = await page();
  let renders = 0;
  const lexer = p.w.marked.Marked.prototype.lexer;
  p.w.marked.Marked.prototype.lexer = function (...args) { renders++; return lexer.apply(this, args); };
  const katexRender = p.w.katex.render;
  let half = 0;
  p.w.katex.render = function (formula, ...rest) { if (formula === '\\frac{1}{2}') half++; return katexRender.call(this, formula, ...rest); };
  p.send([{ op: 'reset', turns: [turn('t', { status: 'running', startedAt: Date.now(), durationMs: null, items: [message('m', '', 'running')] })], hasEarlier: false, paths: [] }]);
  await p.frame();
  const chunks = [];
  for (let i = 0; i < STREAM.length; i += 3) chunks.push(STREAM.slice(i, i + 3));
  let frozenFormula = null;
  let halfAtFreeze = 0;
  for (const chunk of chunks) {
    p.send([{ op: 'text', turn: 't', id: 'm', append: chunk }]);
    await p.frame();
    if (!frozenFormula) {
      frozenFormula = p.$('.md-frozen .math-block');
      halfAtFreeze = half;
    }
  }
  const msg = p.$('.msg');
  assert.ok(frozenFormula && frozenFormula.isConnected, 'the display formula was frozen and kept');
  assert.equal(half, halfAtFreeze, 'a frozen formula is never rendered again');
  assert.ok(p.$('.caret'), 'streaming caret is shown');
  assert.equal(msg.textContent, expectedText);
  assert.equal(msg.querySelectorAll('.katex').length, expectedKatex);

  renders = 0;
  p.send(Array.from({ length: 40 }, () => ({ op: 'text', turn: 't', id: 'm', append: ' more' })));
  p.send([{ op: 'text', turn: 't', id: 'm', append: '!' }]);
  await p.frame();
  assert.equal(renders, 1, 'many text ops in a frame cause one tail render');

  const node = p.$('.msg');
  p.send([{ op: 'finalize', turn: 't', status: 'completed', durationMs: 72000, final: 'm', error: null }]);
  assert.equal(p.$('.final .msg'), node, 'final message node is reused');
  assert.equal(p.$('.caret'), null);
  assert.equal(node.querySelectorAll('.katex').length, expectedKatex);
  assert.ok(node.textContent.trim().endsWith(' more more!'));
  p.w.close();
});

test('split formulas render only once closed', async () => {
  const p = await page();
  p.send([{ op: 'reset', turns: [turn('t', { status: 'running', items: [message('m', '', 'running')] })], hasEarlier: false, paths: [] }]);
  p.send([{ op: 'text', turn: 't', id: 'm', append: 'a $$\\fr' }]);
  await p.frame();
  assert.equal(p.$$('.msg .katex').length, 0);
  assert.match(p.$('.msg').textContent, /\$\$\\fr/);
  p.send([{ op: 'text', turn: 't', id: 'm', append: 'ac{1}{2}$$ b \\(' }]);
  await p.frame();
  assert.equal(p.$$('.msg .katex').length, 1);
  p.send([{ op: 'text', turn: 't', id: 'm', append: 'x\\)' }]);
  await p.frame();
  assert.equal(p.$$('.msg .katex').length, 2);
  p.w.close();
});

test('all four math delimiters, code stays literal and currency is text', async () => {
  const p = await page();
  p.send([{ op: 'reset', turns: [mdTurn('Inline $x^2$ and \\(y+1\\).\n\n$$\\frac{1}{2}$$\n\n\\[z^3\\]\n\n```js\nconst price = "$5$"\n```\n\nCosts $5 and $10 today, `$x$` too.')], hasEarlier: false, paths: [] }]);
  assert.equal(p.$$('.katex').length, 4);
  assert.equal(p.$$('.math-block').length, 2);
  assert.equal(p.$$('pre .katex, code .katex').length, 0);
  assert.match(p.$('.msg').textContent, /Costs \$5 and \$10 today/);
  p.w.close();
});

test('code blocks have a language header and copy the exact code; tables scroll in a wrapper', async () => {
  const p = await page();
  const code = 'fn main() {\n    println!("<hi> & $5");\n}';
  p.send([{ op: 'reset', turns: [mdTurn('```rust\n' + code + '\n```\n\n| a | b |\n|:-:|---|\n| 1 | 2 |')], hasEarlier: false, paths: [] }]);
  assert.equal(p.$('.code .code-lang').textContent, 'rust');
  p.click(p.$('.code .icon-btn'));
  assert.deepEqual(p.userPosts(), [{ type: 'copy', text: code }]);
  const table = p.$('.msg table');
  assert.equal(table.parentNode.className, 'table-wrap');
  p.w.close();
});

test('untrusted content is inert: no scripts, handlers, frames, styles or bridge calls', async () => {
  const p = await page();
  const evil = '<script>WorkflowBridge.post("{\\"type\\":\\"action\\"}")</script>\n\n<img src=x onerror="bad()"><iframe src="https://evil.example"></iframe><style>body{display:none}</style>\n\n' +
    '[js](javascript:WorkflowBridge.post(1)) <a href="javascript:bad()">raw</a> <svg onload="bad()"></svg> <form><input></form>\n\n' +
    '![x](https://evil.example/track.png)\n\n![y](javascript:alert(1))';
  p.send([{ op: 'reset', turns: [turn('t', { user: { text: '<img src=x onerror=bad()>', attachments: [] }, items: [message('m', evil)], final: 'm' })], hasEarlier: false, paths: [] }]);
  await p.frame();
  const t = p.$('#transcript');
  assert.equal(t.querySelectorAll('script,iframe,style,form,input,svg:not(.ic),[onerror],[onload],[style*="display:none"]').length, 0);
  assert.equal(t.querySelectorAll('a[href^="javascript"], img[src^="https://"], img[src^="javascript"]').length, 0);
  assert.match(t.textContent, /<script>/);
  for (const a of t.querySelectorAll('a')) p.click(a);
  assert.deepEqual(p.userPosts(), []);
  p.w.close();
});

test('link clicks post openUrl / openPath with line and column, never navigate', async () => {
  const p = await page();
  const links = '[a](https://example.org/x) [m](mailto:a@b.c) [p](src/Main.kt:12:3) [q](/workspace/app/X.kt#L7) ' +
    '[f](file:///workspace/docs/a%20b.md) [n](Main.kt:9) [z](ftp://x.y/z) [h](#top) [d](data:text/html,hi)';
  p.send([{ op: 'reset', turns: [mdTurn(links)], hasEarlier: false, paths: ['/workspace/'] }]);
  const events = p.$$('.msg a').map(a => { const e = new p.w.MouseEvent('click', { bubbles: true, cancelable: true }); a.dispatchEvent(e); return e.defaultPrevented; });
  assert.ok(events.every(Boolean));
  assert.deepEqual(p.userPosts(), [
    { type: 'openUrl', url: 'https://example.org/x' },
    { type: 'openUrl', url: 'mailto:a@b.c' },
    { type: 'openPath', path: 'src/Main.kt', line: 12, col: 3 },
    { type: 'openPath', path: '/workspace/app/X.kt', line: 7, col: null },
    { type: 'openPath', path: '/workspace/docs/a b.md', line: null, col: null },
    { type: 'openPath', path: 'Main.kt', line: 9, col: null }
  ]);
  p.w.close();
});

test('remote images become chips, workspace images load from /workspace/', async () => {
  const p = await page();
  p.send([{ op: 'reset', turns: [mdTurn('![r](https://cdn.example.com/a.png) ![w](docs/shot.png) ![a](/data/ws/img/a%20b.png) ![o](/etc/x.png) ![d](data:image/png;base64,AAAA)')], hasEarlier: false, paths: ['/data/ws'] }]);
  const srcs = p.$$('.msg img').map(i => i.getAttribute('src'));
  assert.deepEqual(srcs, ['/workspace/docs/shot.png', '/workspace/img/a%20b.png', 'data:image/png;base64,AAAA']);
  const chips = p.$$('.msg .chip');
  assert.equal(chips[0].textContent, '图片 · cdn.example.com');
  chips.forEach(c => p.click(c));
  assert.deepEqual(p.userPosts(), [{ type: 'openUrl', url: 'https://cdn.example.com/a.png' }, { type: 'openPath', path: '/etc/x.png', line: null, col: null }]);
  p.w.close();
});

test('finalize moves the final message out of the collapsed process and shows card and actions', async () => {
  const p = await page();
  const files = ['a', 'b', 'c', 'd', 'e'].map((n, i) => ({ path: `app/src/${n}.kt`, dir: 'app/src/', name: `${n}.kt`, added: i, removed: 1, diff: i === 0 ? '@@ -1 +1 @@\n-old\n+new' : null }));
  p.send([{ op: 'reset', turns: [turn('t', { status: 'running', startedAt: Date.now() - 72000, durationMs: null, canFork: true, items: [
    message('c1', 'Looking around.', 'done', 'commentary'),
    { id: 'x1', kind: 'command', category: 'run', status: 'done', title: '已运行 git status --short', command: 'git status --short', exitCode: 0, output: '' },
    message('f', 'Final **answer**', 'running')
  ] })], hasEarlier: false, paths: [] }]);
  await p.frame();
  assert.match(p.$('.turn-head .head-time').textContent, /^1m 1\ds$/);
  assert.equal(p.$('.process').hidden, false);
  assert.equal(p.$('.actions').hidden, true);
  const node = p.$('[data-item="f"]');
  p.send([{ op: 'turn', turn: { id: 't', changes: { files } } }]);
  assert.equal(p.$('.changes').hidden, true, 'card waits for the end of the turn');
  p.send([{ op: 'finalize', turn: 't', status: 'completed', durationMs: 1478000, final: 'f', error: null }]);
  await p.frame();
  assert.equal(p.$('.final [data-item="f"]'), node);
  assert.equal(p.$('.process').hidden, true);
  assert.equal(p.$('.turn-head .head-btn').textContent, '用时 24m 38s');
  assert.equal(p.$('.turn-head .sr').textContent, '已完成，用时 24m 38s');
  assert.equal(p.$('.changes-head').textContent, '已编辑 5 个文件');
  assert.equal(p.$$('.changes .file').length, 3);
  assert.equal(p.$('.changes-more').textContent, '再显示 2 个文件');
  p.click(p.$('.changes .file-main'));
  assert.match(p.$('.changes .diff').textContent, /\+new/);
  p.click(p.$('.changes .file-open'));
  p.click(p.$('.changes-more'));
  assert.equal(p.$$('.changes .file').length, 5);
  p.click(p.$('.turn-head .head-btn'));
  assert.equal(p.$('.process').hidden, false);
  const [copy, fork, more] = p.$$('.actions .icon-btn');
  p.click(copy);
  p.click(fork);
  p.click(more);
  p.click(p.$('.menu button'));
  assert.deepEqual(p.userPosts(), [
    { type: 'openPath', path: 'app/src/a.kt', line: null, col: null },
    { type: 'toggle', turn: 't', expanded: true },
    { type: 'copy', turn: 't', what: 'final' },
    { type: 'action', turn: 't', name: 'fork' },
    { type: 'copy', turn: 't', what: 'markdown' }
  ]);
  p.w.close();
});

test('interrupted and failed headers, retry only when allowed', async () => {
  const p = await page();
  p.send([{ op: 'reset', turns: [
    turn('a', { status: 'interrupted', durationMs: 3000 }),
    turn('b', { status: 'failed', error: { message: '连接已断开' }, canRetry: true }),
    turn('c', { status: 'failed', error: { message: 'x' }, canRetry: false })
  ], hasEarlier: false, paths: [] }]);
  assert.equal(p.$('[data-turn="a"] .head-btn').textContent, '已停止 · 3s');
  assert.equal(p.$('[data-turn="b"] .head-error').textContent, '连接已断开');
  assert.equal(p.$('[data-turn="c"] .text-btn'), null);
  p.click(p.$('[data-turn="b"] .text-btn'));
  assert.deepEqual(p.userPosts(), [{ type: 'action', turn: 'b', name: 'retry' }]);
  p.w.close();
});

test('consecutive activity items group into one summary row; single rows keep their title', async () => {
  const p = await page();
  const cmd = (id, category, status = 'done') => ({ id, kind: 'command', category, status, title: `${category} ${id}`, command: `cmd ${id}`, exitCode: 0, output: 'l1\nl2' });
  p.send([{ op: 'reset', turns: [turn('t', { status: 'running', items: [
    cmd('r1', 'read'), cmd('r2', 'read'), cmd('x1', 'run'), cmd('r3', 'read'), cmd('x2', 'run'),
    message('m', 'between', 'done', 'commentary'),
    cmd('solo', 'run', 'running'),
    { id: 'th', kind: 'reasoning', status: 'running', text: '' },
    { id: 'pl', kind: 'plan', status: 'done', steps: [{ text: 'a', status: 'completed' }, { text: 'b', status: 'completed' }, { text: 'c', status: 'inProgress' }, { text: 'd', status: 'pending' }, { text: 'e', status: 'pending' }] },
    { id: 'n', kind: 'notice', level: 'warning', text: '额度即将用尽' },
    { id: 'u', kind: 'unknown', title: null, detail: '{\n  "a": 1\n}' }
  ] })], hasEarlier: false, paths: [] }]);
  const rows = p.$$('.process > .item > .row .label').map(l => l.textContent);
  assert.deepEqual(rows, ['已读取 3 个文件，运行了 2 条命令', 'run solo', '思考中…', '计划 2/5', '未识别的活动']);
  assert.equal(p.$('.group .group-items').hidden, true);
  p.click(p.$('.group > .row'));
  assert.equal(p.$('.group .group-items').hidden, false);
  assert.equal(p.$$('.group .group-items > .item').length, 5);
  p.click(p.$('[data-item="u"] > .row'));
  assert.equal(p.$('[data-item="u"] pre.json').textContent, '{\n  "a": 1\n}');
  assert.equal(p.$('.notice').textContent, '额度即将用尽');
  p.send([{ op: 'item', turn: 't', after: 'x2', item: { id: 's1', kind: 'search', status: 'done', query: 'q' } }]);
  assert.equal(p.$('.group > .row .label').textContent, '已读取 3 个文件，运行了 2 条命令，搜索了 1 次');
  p.send([{ op: 'item', turn: 't', after: 'pl', item: { id: 'th', kind: 'reasoning', status: 'done', seconds: 12, text: '**why**' } }]);
  assert.equal(p.$('[data-item="th"] .label').textContent, '已思考 12s');
  p.send([{ op: 'text', turn: 't', id: 'solo', append: Array.from({ length: 20 }, (_, i) => 'line' + i).join('\n') }]);
  p.click(p.$('[data-item="solo"] > .row'));
  assert.equal(p.$('[data-item="solo"] .out').textContent.split('\n').length, 12);
  p.click(p.$('[data-item="solo"] .link-btn'));
  assert.equal(p.$('[data-item="solo"] .out').textContent.split('\n').length, 21);
  p.send([{ op: 'remove', turn: 't', id: 'r1' }, { op: 'remove', turn: 't', id: 'r2' }, { op: 'remove', turn: 't', id: 'x1' }, { op: 'remove', turn: 't', id: 'r3' }, { op: 'remove', turn: 't', id: 's1' }]);
  assert.equal(p.$('.group'), null);
  assert.equal(p.$('[data-item="x2"] .label').textContent, 'run x2');
  assert.deepEqual(p.userPosts(), []);
  p.w.close();
});

test('follow mode: layout posts, user scroll leaves, bottom re-enters, scroll op jumps', async () => {
  const p = await page();
  p.geo.perTurn = 1000;
  p.send([{ op: 'reset', turns: [mdTurn('a')], hasEarlier: false, paths: [] }]);
  await p.frame();
  const layouts = () => p.posts.filter(m => m.type === 'layout');
  assert.deepEqual(layouts().at(-1), { type: 'layout', atBottom: true, height: 1000 });
  assert.equal(p.geo.top, 1000 - p.w.innerHeight);
  p.geo.top = 100;
  p.w.dispatchEvent(new p.w.Event('scroll'));
  await p.frame();
  assert.deepEqual(layouts().at(-1), { type: 'layout', atBottom: false, height: 1000 });
  const count = layouts().length;
  p.send([{ op: 'append', turns: [turn('b')] }]);
  await p.frame();
  assert.equal(p.geo.top, 100, 'not following: no jump');
  assert.equal(layouts().length, count + 1);
  await p.frame();
  assert.equal(layouts().length, count + 1, 'no repeated posts without changes');
  p.geo.top = 2000 - p.w.innerHeight;
  p.w.dispatchEvent(new p.w.Event('scroll'));
  await p.frame();
  assert.equal(layouts().at(-1).atBottom, true);
  p.send([{ op: 'append', turns: [turn('c')] }]);
  await p.frame();
  assert.equal(p.geo.top, 3000 - p.w.innerHeight, 'following: jump to the new bottom');
  p.geo.top = 0;
  p.w.dispatchEvent(new p.w.Event('scroll'));
  p.send([{ op: 'scroll', to: 'bottom' }]);
  await p.frame();
  assert.equal(p.geo.top, 3000 - p.w.innerHeight);
  assert.equal(layouts().at(-1).atBottom, true);
  p.w.close();
});

test('prepend keeps the visible anchor and near-top scrolling asks for earlier history once', async () => {
  const p = await page();
  p.send([{ op: 'reset', turns: [mdTurn('a'), turn('b'), turn('c')], hasEarlier: true, paths: [] }]);
  await p.frame();
  assert.equal(p.posts.filter(m => m.type === 'earlier').length, 0);
  p.geo.top = 100;
  p.w.dispatchEvent(new p.w.Event('scroll'));
  await p.frame();
  p.w.dispatchEvent(new p.w.Event('scroll'));
  await p.frame();
  assert.equal(p.posts.filter(m => m.type === 'earlier').length, 1);
  p.send([{ op: 'prepend', turns: [turn('y'), turn('z')], hasEarlier: false }]);
  assert.equal(p.geo.top, 900);
  assert.deepEqual(p.$$('.turn').map(t => t.getAttribute('data-turn')), ['y', 'z', 't', 'b', 'c']);
  p.w.close();
});

test('theme op sets variables, dark class and scaled sizes', async () => {
  const p = await page();
  p.send([{ op: 'theme', vars: { '--wf-primary': '#90D4BC', '--wf-chat-size': '15px', 'color': 'red' }, dark: true, fontScale: 1.2 }]);
  const style = p.w.document.documentElement.style;
  assert.equal(style.getPropertyValue('--wf-primary'), '#90D4BC');
  assert.equal(style.getPropertyValue('--wf-chat-size'), '18px');
  assert.equal(style.getPropertyValue('--wf-mono-size'), '15.6px');
  assert.equal(style.getPropertyValue('color'), '');
  assert.ok(p.w.document.documentElement.classList.contains('dark'));
  p.w.close();
});

test('user bubble collapses long text and offers copy/fork on long press', async () => {
  const p = await page();
  const text = Array.from({ length: 14 }, (_, i) => 'line ' + i).join('\n');
  p.send([{ op: 'reset', turns: [turn('t', { canFork: true, timeLabel: '星期日 6:48', user: { text, attachments: [{ name: 'a.png', image: true, src: '/workspace/a.png' }] } })], hasEarlier: false, paths: [] }]);
  assert.equal(p.$('.time-sep').textContent, '星期日 6:48');
  assert.equal(p.$('.thumb').getAttribute('src'), '/workspace/a.png');
  assert.ok(p.$('.bubble').classList.contains('clamp'));
  assert.equal(p.$('.bubble-more').textContent, '显示更多');
  p.click(p.$('.bubble-more'));
  assert.equal(p.$('.bubble').classList.contains('clamp'), false);
  p.$('.bubble').dispatchEvent(new p.w.MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  assert.deepEqual(p.$$('.menu button').map(b => b.textContent), ['复制', '从这里分叉']);
  p.click(p.$('.menu button'));
  assert.deepEqual(p.userPosts(), [{ type: 'copy', turn: 't', what: 'user' }]);
  p.w.close();
});

test('committed chat assets equal a fresh build', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'chat-build-'));
  try {
    await buildChat(dir);
    for (const file of ['chat.js', 'chat.html', 'chat.css']) {
      const fresh = await readFile(path.join(dir, file));
      const committed = await readFile(path.join(defaultOutdir, file));
      assert.ok(fresh.equals(committed), `${file} is stale: run npm run build:chat`);
    }
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
