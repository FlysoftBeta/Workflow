/* Items of a turn's process area: messages, compact activity rows with lazy details, group rows. */
import { h, button, clear, show, syncChildren } from './dom.js';
import { icon } from './icons.js';
import { post } from './bridge.js';
import { schedule } from './frame.js';
import { imageNode } from './links.js';
import { MessageView, renderMarkdown } from './markdown.js';
import { S } from './strings.js';

export const GROUPABLE = { command: true, files: true, search: true, tool: true, agent: true };
const ROW = { command: true, files: true, search: true, tool: true, agent: true, reasoning: true, plan: true, unknown: true };
const PREVIEW_LINES = 12;

const str = value => (value == null ? '' : String(value));
const category = d => (d.kind === 'command' ? (d.category === 'read' || d.category === 'list' || d.category === 'search' ? d.category : 'run') : d.kind);
const isDone = step => /^(completed|done)$/i.test(str(step.status));

function rowIcon(d) {
  switch (d.kind) {
    case 'command': return { read: 'description', list: 'description', search: 'search' }[category(d)] || 'terminal';
    case 'files': return 'edit_document';
    case 'search': return 'search';
    case 'tool': return 'data_object';
    case 'agent': return 'fork_right';
    case 'reasoning': return 'lightbulb';
    case 'plan': return 'checklist';
    default: return 'info';
  }
}

function rowLabel(d) {
  const running = d.status === 'running';
  if (d.kind === 'reasoning') return running ? S.thinking : S.thought + (d.seconds != null ? ' ' + Math.round(d.seconds) + 's' : '');
  if (d.kind === 'plan') {
    const steps = Array.isArray(d.steps) ? d.steps : [];
    return S.plan + ' ' + steps.filter(isDone).length + '/' + steps.length;
  }
  let label = str(d.title).trim();
  if (!label) {
    const files = Array.isArray(d.files) ? d.files : [];
    const object = {
      command: d.command, search: d.query, agent: d.text && str(d.text).split('\n')[0],
      files: files.length === 1 ? files[0].path : S.fileCount(files.length), tool: d.server
    }[d.kind];
    const verb = S.verb[category(d)];
    label = d.kind === 'unknown' ? S.unknown : d.kind === 'agent' ? S.subagent + ' ' + str(object) : (verb ? verb[running ? 0 : 1] + ' ' : '') + str(object);
  }
  const status = S.itemStatus[d.status];
  return status ? label + ' · ' + status : label;
}

function canOpen(d) {
  switch (d.kind) {
    case 'command': return !!(d.command || d.output);
    case 'files': return Array.isArray(d.files) && d.files.length > 0;
    case 'search': return !!(d.query || d.url);
    case 'tool': return !!(d.args || d.result);
    case 'agent': case 'reasoning': return !!d.text;
    case 'plan': return Array.isArray(d.steps) && d.steps.length > 0;
    case 'unknown': return !!d.detail;
    default: return false;
  }
}

function monoBlock(text, cls) {
  return h('pre', { class: 'mono-block' + (cls ? ' ' + cls : ''), text: str(text) });
}

function diffBlock(text) {
  const pre = h('pre', { class: 'diff' });
  for (const line of str(text).replace(/\n$/, '').split('\n')) {
    const c = /^\+(?!\+\+)/.test(line) ? 'a' : /^-(?!--)/.test(line) ? 'd' : /^@@/.test(line) ? 'hunk' : null;
    pre.appendChild(h('span', { class: c }, line + '\n'));
  }
  return pre;
}

/** One changed file: dir (caption) + name + mono "+n −n"; tap toggles the diff, the icon opens the file. */
export function fileRow(f, openSet) {
  const path = str(f.path);
  const name = str(f.name) || path.split('/').pop();
  let dir = f.dir != null ? str(f.dir) : path.slice(0, path.length - name.length);
  if (dir && dir.charAt(dir.length - 1) !== '/') dir += '/';
  const openFile = () => post({ type: 'openPath', path, line: null, col: null });
  const diff = h('div', { class: 'file-diff', hidden: true });
  const main = button('file-main', null, () => {
    if (!f.diff) { openFile(); return; }
    const open = diff.hasAttribute('hidden');
    if (open && !diff.firstChild) diff.appendChild(diffBlock(f.diff));
    show(diff, open);
    main.setAttribute('aria-expanded', String(open));
    if (open) openSet.add(path); else openSet.delete(path);
  });
  main.appendChild(h('span', { class: 'file-path' }, h('span', { class: 'file-dir', text: dir }), h('span', { class: 'file-name', text: name })));
  main.appendChild(h('span', { class: 'stat' },
    h('span', { class: 'add', text: '+' + (Number(f.added) || 0) }), h('span', { class: 'del', text: '−' + (Number(f.removed) || 0) })));
  const openBtn = button('icon-btn file-open', S.openFile, openFile);
  openBtn.appendChild(icon('open_in_new'));
  const row = h('div', { class: 'file' }, h('div', { class: 'file-row' }, main, openBtn), diff);
  if (f.diff && openSet.has(path)) main.click();
  return row;
}

function toggleBlock(label, body) {
  const box = h('div', { class: 'sub' });
  const t = button('sub-toggle', null, () => {
    const open = body.hasAttribute('hidden');
    show(body, open);
    box.classList.toggle('open', open);
  });
  t.appendChild(h('span', { text: label }));
  t.appendChild(icon('chevron_right', 'chev'));
  show(body, false);
  box.appendChild(t);
  box.appendChild(body);
  return box;
}

function commandDetail(v) {
  const d = v.data;
  const status = d.status === 'running' ? h('span', { class: 'spinner sm' })
    : d.exitCode == null ? null : icon(d.exitCode === 0 ? 'check' : 'close', d.exitCode === 0 ? 'ok' : 'bad');
  const box = h('div', { class: 'cmd' },
    h('div', { class: 'cmd-line' }, status, h('code', { text: str(d.command || d.title) }),
      d.exitCode != null && d.exitCode !== 0 ? h('span', { class: 'cmd-exit', text: S.exitCode(d.exitCode) }) : null));
  const output = str(d.output).replace(/\n$/, '');
  if (output) {
    const lines = output.split('\n');
    const full = v.showAll || lines.length <= PREVIEW_LINES;
    box.appendChild(h('pre', { class: 'out', text: full ? output : lines.slice(0, PREVIEW_LINES).join('\n') }));
    if (!full) {
      const more = button('link-btn', null, () => { v.showAll = true; v.refresh(); });
      more.textContent = S.showAll(lines.length);
      box.appendChild(more);
    }
  }
  if (d.truncated) box.appendChild(h('div', { class: 'caption', text: S.truncated }));
  return box;
}

function buildDetail(v) {
  const d = v.data;
  switch (d.kind) {
    case 'command': return commandDetail(v);
    case 'files': return h('div', { class: 'file-list' }, d.files.map(f => fileRow(f, v.openFiles)));
    case 'search': {
      const link = /^https?:/i.test(str(d.url)) ? button('link-btn', null, () => post({ type: 'openUrl', url: d.url })) : null;
      if (link) link.textContent = d.url;
      return h('div', { class: 'search' }, d.query ? h('div', { class: 'mono-line', text: str(d.query) }) : null, link);
    }
    case 'tool': return h('div', { class: 'tool' },
      d.args ? toggleBlock(S.args, monoBlock(d.args)) : null,
      d.result ? h('div', { class: 'caption', text: S.result }) : null,
      d.result ? monoBlock(d.result) : null);
    case 'agent': return h('div', { class: 'md' }, renderMarkdown(d.text));
    case 'reasoning': {
      const el = h('div', { class: 'md reasoning-md' });
      v.md = new MessageView(el);
      v.md.set(d.text, d.status !== 'running');
      return el;
    }
    case 'plan': return h('div', null,
      h('ul', { class: 'plan' }, d.steps.map(step => {
        const mark = isDone(step) ? icon('check', 'ok') : /progress|running|active/i.test(str(step.status)) ? h('span', { class: 'spinner sm' }) : h('span', { class: 'dot' });
        return h('li', { class: isDone(step) ? 'done' : null }, mark, h('span', { text: str(step.text) }));
      })),
      d.text ? h('div', { class: 'md' }, renderMarkdown(d.text)) : null);
    default: return monoBlock(d.detail, 'json');
  }
}

export class ItemView {
  constructor(turn, data) {
    this.turn = turn;
    this.data = data;
    this.kind = data.kind;
    this.open = false;
    this.showAll = false;
    this.openFiles = new Set();
    this.refreshTask = () => this.refresh();
    if (this.kind === 'message') {
      this.el = h('div', { class: 'item msg md', 'data-item': data.id });
      this.md = new MessageView(this.el);
    } else if (ROW[this.kind]) {
      this.iconSlot = h('span', { class: 'ic-slot' });
      this.label = h('span', { class: 'label' });
      this.chev = icon('chevron_right', 'chev');
      this.row = button('row', null, () => this.toggle());
      this.row.setAttribute('aria-expanded', 'false');
      this.row.appendChild(this.iconSlot);
      this.row.appendChild(this.label);
      this.row.appendChild(this.chev);
      this.detail = h('div', { class: 'detail', hidden: true });
      this.el = h('div', { class: 'item act', 'data-item': data.id }, this.row, this.detail);
    } else {
      this.el = h('div', { class: 'item ' + this.kind, 'data-item': data.id });
    }
    this.update(data);
  }

  update(data) {
    const prev = this.data;
    this.data = data;
    const d = data;
    if (this.kind === 'message') {
      this.md.set(d.text, d.status !== 'running');
      this.syncCaret();
      return;
    }
    if (ROW[this.kind]) {
      this.el.classList.toggle('running', d.status === 'running');
      this.el.classList.toggle('failed', d.status === 'failed' || d.status === 'declined');
      if (!this.iconSlot.firstChild || rowIcon(prev) !== rowIcon(d)) { clear(this.iconSlot); this.iconSlot.appendChild(icon(rowIcon(d))); }
      this.label.textContent = rowLabel(d);
      show(this.chev, canOpen(d));
      if (this.kind === 'reasoning' && this.md) this.md.set(d.text, d.status !== 'running');
      else if (this.open) this.refresh();
      return;
    }
    clear(this.el);
    if (this.kind === 'notice') {
      const level = d.level === 'error' || d.level === 'warning' ? d.level : 'info';
      this.el.className = 'item notice ' + level;
      this.el.appendChild(h('span', { class: 'notice-text' }, icon(level), h('span', { text: str(d.text) })));
    } else if (this.kind === 'record') {
      this.el.appendChild(icon('check'));
      this.el.appendChild(h('span', { class: 'label', text: str(d.text) }));
    } else if (this.kind === 'marker') {
      this.el.textContent = str(d.text);
    } else if (this.kind === 'image') {
      this.el.appendChild(imageNode(d.src, d.caption, 'item-img'));
      if (d.caption) this.el.appendChild(h('div', { class: 'caption', text: str(d.caption) }));
    }
  }

  toggle() {
    if (!canOpen(this.data) && !this.open) return;
    this.open = !this.open;
    this.el.classList.toggle('open', this.open);
    this.row.setAttribute('aria-expanded', String(this.open));
    if (this.open && !this.detail.firstChild) this.refresh();
    show(this.detail, this.open);
  }

  refresh() {
    if (!this.detail) return;
    if (this.kind === 'reasoning' && this.md) return;
    clear(this.detail);
    this.detail.appendChild(buildDetail(this));
  }

  appendText(text) {
    const d = this.data;
    if (this.kind === 'message') { d.text = str(d.text) + text; this.md.append(text); return; }
    if (this.kind === 'command') {
      d.output = str(d.output) + text;
      if (this.open) schedule(this.refreshTask);
      else show(this.chev, true);
      return;
    }
    d.text = str(d.text) + text;
    if (this.md) this.md.append(text);
    else if (this.open) schedule(this.refreshTask);
    if (this.chev) show(this.chev, canOpen(d));
  }

  syncCaret() {
    if (this.md && this.kind === 'message') this.md.setCaret(this.data.status === 'running' && this.turn.data.status === 'running');
  }

  finish() {
    if (this.kind === 'message') this.md.finish();
  }
}

/** A run of ≥ 2 consecutive command/files/search/tool/agent items shown as one summary row. */
export class GroupView {
  constructor(turn, key) {
    this.key = key;
    this.iconSlot = h('span', { class: 'ic-slot' });
    this.label = h('span', { class: 'label' });
    this.row = button('row', null, () => {
      const open = !turn.openGroups.has(key);
      if (open) turn.openGroups.add(key); else turn.openGroups.delete(key);
      this.apply(open);
    });
    this.row.appendChild(this.iconSlot);
    this.row.appendChild(this.label);
    this.row.appendChild(icon('chevron_right', 'chev'));
    this.items = h('div', { class: 'group-items' });
    this.el = h('div', { class: 'item act group' }, this.row, this.items);
    this.apply(turn.openGroups.has(key));
  }

  apply(open) {
    this.el.classList.toggle('open', open);
    this.row.setAttribute('aria-expanded', String(open));
    show(this.items, open);
  }

  set(views) {
    const counts = {};
    const order = [];
    let running = false;
    for (const v of views) {
      const c = category(v.data);
      if (!counts[c]) { counts[c] = 0; order.push(c); }
      counts[c]++;
      running = running || v.data.status === 'running';
    }
    this.label.textContent = order.map(c => S.group[c](counts[c])).join(S.joiner);
    const name = rowIcon(views[0].data);
    if (this.iconName !== name) { clear(this.iconSlot); this.iconSlot.appendChild(icon(name)); this.iconName = name; }
    this.el.classList.toggle('running', running);
    syncChildren(this.items, views.map(v => v.el));
  }
}
