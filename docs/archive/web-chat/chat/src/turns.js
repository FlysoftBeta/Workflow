/* One turn: time separator, user bubble, header, process (grouped), final message, changes card, actions. */
import { h, button, clear, show, syncChildren } from './dom.js';
import { icon } from './icons.js';
import { post } from './bridge.js';
import { schedule } from './frame.js';
import { imageNode, resolveImage, chip } from './links.js';
import { openMenu } from './menu.js';
import { ItemView, GroupView, GROUPABLE, fileRow } from './items.js';
import { S, duration } from './strings.js';

const USER_LINES = 10;
const FILE_PREVIEW = 3;
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

function flash(btn) {
  btn.replaceChild(icon('check'), btn.firstChild);
  setTimeout(() => btn.replaceChild(icon('content_copy'), btn.firstChild), 1200);
}

function iconButton(name, label, onClick) {
  const el = button('icon-btn', label, () => onClick(el));
  el.appendChild(icon(name));
  return el;
}

export class TurnView {
  constructor(data) {
    this.id = data.id;
    this.data = { status: 'running' };
    this.items = new Map();
    this.order = [];
    this.groups = new Map();
    this.openGroups = new Set();
    this.openFiles = new Set();
    this.showAllFiles = false;
    this.expanded = null;
    this.live = null;
    this.timeEl = null;
    this.el = h('section', { class: 'turn', 'data-turn': data.id });
    this.sep = h('div', { class: 'time-sep', hidden: true });
    this.user = h('div', { class: 'user', hidden: true });
    this.head = h('div', { class: 'turn-head' });
    this.process = h('div', { class: 'process' });
    this.final = h('div', { class: 'final' });
    this.changes = h('div', { class: 'changes', hidden: true });
    this.actions = h('div', { class: 'actions', hidden: true });
    [this.sep, this.user, this.head, this.process, this.final, this.changes, this.actions].forEach(el => this.el.appendChild(el));
    this.update(data, true);
    let after = null;
    for (const item of Array.isArray(data.items) ? data.items : []) {
      if (item && item.id != null) { this.upsertItem(item, after, true); after = item.id; }
    }
    this.layout();
  }

  running() { return this.data.status === 'running'; }
  isExpanded() { return this.expanded != null ? this.expanded : this.running(); }
  finalId() { return !this.running() && this.data.final != null && this.items.has(this.data.final) ? this.data.final : null; }

  /** Shell upsert; a transition out of "running" is the finalize path. */
  update(shell, initial) {
    const prev = this.data;
    const d = Object.assign({}, prev);
    for (const key of Object.keys(shell)) if (key !== 'items' && shell[key] !== undefined) d[key] = shell[key];
    this.data = d;
    const ended = !initial && prev.status === 'running' && d.status !== 'running';
    if (shell.expanded != null) this.expanded = !!shell.expanded;
    else if (ended) this.expanded = false;
    this.el.className = 'turn ' + d.status;
    this.sep.textContent = d.timeLabel || '';
    show(this.sep, !!d.timeLabel);
    if (initial || !same(prev.user, d.user)) this.renderUser();
    if (d.status === 'running' && !this.live) this.live = h('span', { class: 'sr', 'aria-live': 'polite' });
    if (ended) {
      this.items.forEach(v => v.finish());
      if (this.live && !this.live.textContent) {
        this.live.textContent = d.status === 'completed' ? S.announce.completed(duration(d.durationMs)) : S.announce[d.status] || S.done;
      }
    }
    this.items.forEach(v => v.syncCaret());
    this.renderHead();
    this.renderChanges(initial || !same(prev.changes, d.changes) || ended);
    this.renderActions();
    if (!initial) this.layout();
  }

  renderUser() {
    const u = this.data.user;
    clear(this.user);
    show(this.user, !!u);
    if (!u) return;
    const attachments = Array.isArray(u.attachments) ? u.attachments : [];
    if (attachments.length) {
      this.user.appendChild(h('div', { class: 'attachments' }, attachments.map(a => {
        const r = a.image && a.src ? resolveImage(a.src) : null;
        return r && r.url ? imageNode(a.src, a.name, 'thumb') : chip(String(a.name || S.attachment), () => {});
      })));
    }
    if (u.text == null || u.text === '') return;
    const text = h('div', { class: 'bubble-text', text: String(u.text) });
    const bubble = h('div', { class: 'bubble' }, text);
    bubble.addEventListener('contextmenu', event => {
      event.preventDefault();
      const entries = [[S.copy, () => post({ type: 'copy', turn: this.id, what: 'user' })]];
      if (this.data.canFork) entries.push([S.forkHere, () => post({ type: 'action', turn: this.id, name: 'fork' })]);
      openMenu(bubble, entries);
    });
    this.user.appendChild(bubble);
    const clamp = on => {
      bubble.classList.toggle('clamp', on);
      clear(more);
      more.appendChild(document.createTextNode(on ? S.showMore : S.showLess));
      more.appendChild(icon('keyboard_arrow_down', on ? null : 'flip'));
    };
    const more = button('bubble-more', null, () => clamp(!bubble.classList.contains('clamp')));
    if (String(u.text).split('\n').length > USER_LINES) {
      bubble.appendChild(more);
      clamp(true);
    } else {
      bubble.classList.add('clamp');
      schedule(() => {
        if (text.scrollHeight > text.clientHeight + 1) { bubble.appendChild(more); clamp(true); } else bubble.classList.remove('clamp');
      });
    }
  }

  renderHead() {
    const d = this.data;
    const expanded = this.isExpanded();
    clear(this.head);
    const toggle = button('head-btn', null, () => this.toggle());
    toggle.setAttribute('aria-expanded', String(expanded));
    const took = d.durationMs != null ? duration(d.durationMs) : '';
    this.timeEl = null;
    if (d.status === 'running') {
      this.timeEl = h('span', { class: 'head-time' });
      toggle.appendChild(h('span', { class: 'spinner', 'aria-hidden': 'true' }));
      toggle.appendChild(h('span', { text: S.running }));
      toggle.appendChild(this.timeEl);
    } else if (d.status === 'failed') {
      toggle.appendChild(icon('error', 'err'));
      toggle.appendChild(h('span', { class: 'head-error', text: (d.error && d.error.message) || S.failed }));
    } else if (d.status === 'interrupted') {
      toggle.appendChild(h('span', { text: S.stopped + (took ? ' · ' + took : '') }));
    } else {
      toggle.appendChild(h('span', { text: took ? S.took + ' ' + took : S.done }));
    }
    toggle.appendChild(icon(expanded ? 'keyboard_arrow_down' : 'chevron_right', 'chev'));
    this.head.appendChild(toggle);
    if (d.status === 'failed' && d.canRetry) {
      const retry = button('text-btn', null, () => post({ type: 'action', turn: this.id, name: 'retry' }));
      retry.textContent = S.retry;
      this.head.appendChild(retry);
    }
    if (this.live) this.head.appendChild(this.live);
    this.tick(Date.now());
  }

  tick(now) {
    if (!this.timeEl) return;
    const d = this.data;
    const ms = d.startedAt != null ? now - d.startedAt : d.durationMs;
    this.timeEl.textContent = ms != null ? duration(ms) : '';
  }

  toggle() {
    this.expanded = !this.isExpanded();
    this.renderHead();
    this.layout();
    post({ type: 'toggle', turn: this.id, expanded: this.expanded });
  }

  setExpanded(expanded) {
    this.expanded = !!expanded;
    this.renderHead();
    this.layout();
  }

  renderChanges(changed) {
    const files = this.data.changes && Array.isArray(this.data.changes.files) ? this.data.changes.files : [];
    const visible = !this.running() && files.length > 0;
    show(this.changes, visible);
    if (!visible || !changed) return;
    clear(this.changes);
    this.changes.appendChild(h('div', { class: 'changes-head' }, icon('edit_document'), h('span', { text: S.editedFiles(files.length) })));
    const shown = this.showAllFiles ? files : files.slice(0, FILE_PREVIEW);
    this.changes.appendChild(h('div', { class: 'file-list' }, shown.map(f => fileRow(f, this.openFiles))));
    if (shown.length < files.length) {
      const more = button('changes-more', null, () => { this.showAllFiles = true; this.renderChanges(true); });
      more.appendChild(h('span', { text: S.moreFiles(files.length - shown.length) }));
      more.appendChild(icon('keyboard_arrow_down'));
      this.changes.appendChild(more);
    }
  }

  renderActions() {
    const d = this.data;
    show(this.actions, !this.running());
    clear(this.actions);
    if (this.running()) return;
    this.actions.appendChild(iconButton('content_copy', S.copy, btn => { post({ type: 'copy', turn: this.id, what: 'final' }); flash(btn); }));
    if (d.canFork) this.actions.appendChild(iconButton('fork_right', S.fork, () => post({ type: 'action', turn: this.id, name: 'fork' })));
    this.actions.appendChild(iconButton('more_horiz', S.more, btn => openMenu(btn, [
      [S.copyMarkdown, () => post({ type: 'copy', turn: this.id, what: 'markdown' })]
    ])));
  }

  upsertItem(data, after, deferLayout) {
    let view = this.items.get(data.id);
    if (view && view.kind !== data.kind) { this.removeItem(data.id, true); view = null; }
    if (view) {
      view.update(data);
    } else {
      view = new ItemView(this, data);
      this.items.set(data.id, view);
      let index = this.order.length;
      if (after === null) index = 0;
      else if (after !== undefined && this.order.indexOf(after) >= 0) index = this.order.indexOf(after) + 1;
      this.order.splice(index, 0, data.id);
    }
    if (!deferLayout) this.layout();
  }

  removeItem(id, deferLayout) {
    const view = this.items.get(id);
    if (!view) return;
    this.items.delete(id);
    this.order.splice(this.order.indexOf(id), 1);
    if (view.el.parentNode) view.el.parentNode.removeChild(view.el);
    if (!deferLayout) this.layout();
  }

  appendText(id, text) {
    const view = this.items.get(id);
    if (view && typeof text === 'string') view.appendText(text);
  }

  /** Place item nodes: process (runs of ≥ 2 groupable items become one group row), final area. */
  layout() {
    const finalId = this.finalId();
    const nodes = [];
    const used = new Set();
    let run = [];
    const flush = () => {
      if (run.length > 1) {
        const key = run[0].data.id;
        let group = this.groups.get(key);
        if (!group) { group = new GroupView(this, key); this.groups.set(key, group); }
        group.set(run);
        used.add(key);
        nodes.push(group.el);
      } else if (run.length) {
        nodes.push(run[0].el);
      }
      run = [];
    };
    for (const id of this.order) {
      if (id === finalId) continue;
      const view = this.items.get(id);
      if (GROUPABLE[view.kind]) run.push(view);
      else { flush(); nodes.push(view.el); }
    }
    flush();
    this.groups.forEach((group, key) => { if (!used.has(key)) this.groups.delete(key); });
    syncChildren(this.process, nodes);
    syncChildren(this.final, finalId != null ? [this.items.get(finalId).el] : []);
    show(this.process, this.isExpanded() && nodes.length > 0);
    show(this.final, finalId != null);
  }
}
