/* Per-message incremental Markdown: marked (+ math extension) -> DOMPurify (strict, into an inert
 * fragment) -> page enhancements (code headers, scroll wrappers, KaTeX, image policy). */
import { h, button, clear } from './dom.js';
import { icon } from './icons.js';
import { post } from './bridge.js';
import { schedule } from './frame.js';
import { imageNode } from './links.js';
import { S } from './strings.js';

const escapeHtml = text => String(text).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

function mathHtml(token) {
  const span = '<span class="wf-math" data-display="' + token.display + '" data-formula="' + encodeURIComponent(token.formula) + '"></span>';
  return token.block ? '<div class="math-block">' + span + '</div>' : span;
}

const math = token => ({ type: token.type, raw: token.raw, formula: token.formula.trim(), display: token.display, block: token.block });

const md = new marked.Marked({ gfm: true, breaks: false });
md.use({
  extensions: [
    { // $$…$$ or \[…\] on their own lines
      name: 'blockMath', level: 'block',
      start(src) { const m = /(^|\n) {0,3}(\$\$|\\\[)/.exec(src); return m ? m.index + m[1].length : -1; },
      tokenizer(src) {
        const m = /^ {0,3}(?:\$\$([\s\S]+?)\$\$|\\\[([\s\S]+?)\\\])[ \t]*(?:\n|$)/.exec(src);
        if (m) return math({ type: 'blockMath', raw: m[0], formula: m[1] != null ? m[1] : m[2], display: true, block: true });
      },
      renderer: mathHtml
    },
    { // $…$ (pandoc rules, so "$5 and $10" stays text), \(…\), and display delimiters inside a line
      name: 'inlineMath', level: 'inline',
      start(src) { return src.search(/\$|\\[([]/); },
      tokenizer(src) {
        let m = /^\$\$((?:\\[\s\S]|[^\\$])+?)\$\$/.exec(src) || /^\\\[([\s\S]+?)\\\]/.exec(src);
        if (m) return math({ type: 'inlineMath', raw: m[0], formula: m[1], display: true });
        m = /^\\\(([\s\S]+?)\\\)/.exec(src);
        if (m) return math({ type: 'inlineMath', raw: m[0], formula: m[1], display: false });
        m = /^\$((?:\\[\s\S]|[^\\$\n])+?)\$(?!\d)/.exec(src);
        if (m && !/^\s|\s$|`/.test(m[1])) return math({ type: 'inlineMath', raw: m[0], formula: m[1], display: false });
      },
      renderer: mathHtml
    }
  ],
  renderer: {
    html: token => (token.block ? '<p>' + escapeHtml(token.text.trim()) + '</p>' : escapeHtml(token.text)),
    checkbox: token => '<span class="task">' + (token.checked ? '☑' : '☐') + '</span> '
  }
});

const purify = DOMPurify(window);
const PURIFY = {
  ALLOWED_TAGS: ['a', 'b', 'blockquote', 'br', 'code', 'del', 'div', 'em', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'hr', 'i', 'img',
    'kbd', 'li', 'ol', 'p', 'pre', 's', 'span', 'strong', 'sub', 'sup', 'table', 'tbody', 'td', 'th', 'thead', 'tr', 'ul'],
  ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class', 'align', 'start', 'data-formula', 'data-display'],
  ALLOW_DATA_ATTR: false,
  // http(s)/mailto/file, relative paths, and "Name.kt:12" (DOMPurify's default would read "Name.kt:" as a scheme)
  ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|file):|[^a-z]|[\w-]+\.[\w.-]*:\d+(?::\d+)?(?:$|#)|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i,
  ALLOW_ARIA_ATTR: false,
  RETURN_DOM_FRAGMENT: true
};
const CLASSES = /^(language-[\w+#.-]+|wf-math|math-block|task)$/;
purify.addHook('uponSanitizeAttribute', (node, data) => {
  if (data.attrName === 'class') data.attrValue = data.attrValue.split(/\s+/).filter(c => CLASSES.test(c)).join(' ');
  else if (data.attrName.indexOf('data-') === 0 && node.nodeName !== 'SPAN') data.keepAttr = false;
});

const all = (root, selector) => Array.prototype.slice.call(root.querySelectorAll(selector));

function copyButton(getText) {
  const el = button('icon-btn sm', S.copyCode, () => {
    post({ type: 'copy', text: getText() });
    el.replaceChild(icon('check'), el.firstChild);
    setTimeout(() => el.replaceChild(icon('content_copy'), el.firstChild), 1200);
  });
  el.appendChild(icon('content_copy'));
  return el;
}

/* Runs on the inert fragment before it is attached, so remote images are never requested. */
function enhance(root) {
  for (const pre of all(root, 'pre')) {
    const code = pre.querySelector('code') || pre;
    const lang = (/\blanguage-([\w+#.-]+)/.exec(code.className || '') || [])[1] || '';
    const block = h('div', { class: 'code' },
      h('div', { class: 'code-head' }, h('span', { class: 'code-lang', text: lang }), copyButton(() => code.textContent.replace(/\n$/, ''))));
    pre.parentNode.replaceChild(block, pre);
    block.appendChild(pre);
  }
  for (const table of all(root, 'table')) {
    const wrap = h('div', { class: 'table-wrap' });
    table.parentNode.replaceChild(wrap, table);
    wrap.appendChild(table);
  }
  for (const span of all(root, 'span.wf-math[data-formula]')) {
    let formula;
    try { formula = decodeURIComponent(span.getAttribute('data-formula')); } catch (e) { span.parentNode.removeChild(span); continue; }
    katex.render(formula, span, { displayMode: span.getAttribute('data-display') === 'true', throwOnError: false, trust: false, strict: 'ignore' });
  }
  for (const img of all(root, 'img')) img.parentNode.replaceChild(imageNode(img.getAttribute('src'), img.getAttribute('alt'), 'md-img'), img);
  return root;
}

function renderTokens(tokens) {
  return enhance(purify.sanitize(md.parser(tokens), PURIFY));
}

/** One-shot render (history, finished items, details). */
export function renderMarkdown(text) {
  return renderTokens(md.lexer(String(text || '')));
}

/* Freezing is only safe when no display math or fence is left open: [$$ count, \[ minus \], fence lines]. */
function openers(token) {
  if (token.type === 'code') return [0, 0, 0];
  const text = token.raw.replace(/`[^`\n]*`/g, '');
  const n = re => (text.match(re) || []).length;
  return [n(/\$\$/g), n(/\\\[/g) - n(/\\\]/g), n(/^ {0,3}(```|~~~)/gm)];
}

/**
 * Streaming state of one message: `source` and `stable` (end of the frozen prefix). Frozen blocks are
 * rendered once; each frame re-lexes only source.slice(stable). finish() is the single full render.
 */
export class MessageView {
  constructor(el) {
    this.el = el;
    this.source = '';
    this.stable = 0;
    this.complete = false;
    this.caretOn = false;
    this.cr = false;
    this.frozen = h('div', { class: 'md-frozen' });
    this.tail = h('div', { class: 'md-tail' });
    this.caret = h('span', { class: 'caret', 'aria-hidden': 'true' });
    el.appendChild(this.frozen);
    el.appendChild(this.tail);
    this.task = () => { if (!this.complete) this.renderTail(); };
  }

  normalize(text) {
    let s = String(text || '');
    if (this.cr && s.charAt(0) === '\n') s = s.slice(1);
    this.cr = s.charAt(s.length - 1) === '\r';
    return s.replace(/\r\n?/g, '\n');
  }

  /** Whole text from an item upsert: continue streaming when it extends what we have. */
  set(text, done) {
    const s = String(text || '').replace(/\r\n?/g, '\n');
    this.cr = false;
    if (done) {
      if (!this.complete || s !== this.source) { this.source = s; this.renderFull(); }
      return;
    }
    if (!this.complete && s.slice(0, this.source.length) === this.source) {
      if (s.length > this.source.length) this.push(s.slice(this.source.length));
      return;
    }
    this.source = s;
    this.stable = 0;
    this.complete = false;
    clear(this.frozen);
    schedule(this.task);
  }

  append(text) {
    const s = this.normalize(text);
    if (s) this.push(s);
  }

  push(s) {
    this.source += s;
    if (this.complete) { this.complete = false; this.stable = 0; clear(this.frozen); }
    schedule(this.task);
  }

  renderTail() {
    const src = this.source.slice(this.stable);
    const tokens = md.lexer(src);
    let last = tokens.length - 1;
    while (last >= 0 && tokens[last].type === 'space') last--;
    let count = 0;
    let length = 0;
    let raw = 0;
    const open = [0, 0, 0];
    for (let i = 0; i < last; i++) {
      const o = openers(tokens[i]);
      for (let k = 0; k < 3; k++) open[k] += o[k];
      raw += tokens[i].raw.length;
      if (tokens[i].type !== 'space' && open[0] % 2 === 0 && open[1] === 0 && open[2] % 2 === 0) { count = i + 1; length = raw; }
    }
    if (count && src.slice(0, length) === tokens.slice(0, count).map(t => t.raw).join('')) {
      this.frozen.appendChild(renderTokens(tokens.slice(0, count)));
      this.stable += length;
    } else {
      count = 0;
    }
    clear(this.tail);
    this.tail.appendChild(renderTokens(tokens.slice(count)));
    this.placeCaret();
  }

  renderFull() {
    clear(this.frozen);
    clear(this.tail);
    this.tail.appendChild(renderMarkdown(this.source));
    this.stable = 0;
    this.complete = true;
    this.placeCaret();
  }

  finish() {
    if (!this.complete) this.renderFull();
    this.setCaret(false);
  }

  setCaret(on) {
    if (this.caretOn === on) return;
    this.caretOn = on;
    this.placeCaret();
  }

  placeCaret() {
    if (this.caret.parentNode) this.caret.parentNode.removeChild(this.caret);
    if (!this.caretOn) return;
    let host = this.tail.lastElementChild || this.frozen.lastElementChild;
    while (host && /^(UL|OL|LI|BLOCKQUOTE)$/.test(host.tagName) && host.lastElementChild &&
      /^(UL|OL|LI|P|BLOCKQUOTE)$/.test(host.lastElementChild.tagName)) host = host.lastElementChild;
    if (!host || !/^(P|LI|H[1-6])$/.test(host.tagName)) host = this.tail;
    host.appendChild(this.caret);
  }
}
