/* Link and image policy. The WebView never navigates: links become openUrl/openPath posts, images
 * load only from data: or the same-origin /workspace/ handler, remote images become link chips. */
import { post } from './bridge.js';
import { h, button } from './dom.js';
import { icon } from './icons.js';
import { S } from './strings.js';

let prefixes = [];

export function setPaths(paths) {
  prefixes = (Array.isArray(paths) ? paths : [])
    .filter(p => typeof p === 'string' && p.length > 1)
    .map(p => (p.charAt(p.length - 1) === '/' ? p : p + '/'));
}

function decode(text) {
  try { return decodeURIComponent(text); } catch (e) { return text; }
}

const SCHEME = /^[a-z][a-z0-9+.-]*:/i;
const stripFile = href => href.replace(/^file:(\/\/[^/]*)?/i, '');

/** Returns an openUrl/openPath message for a link target, or null when the target is ignored. */
export function classifyHref(raw) {
  const href = String(raw || '').trim();
  if (!href || href.charAt(0) === '#') return null;
  if (/^(https?|mailto):/i.test(href)) return { type: 'openUrl', url: href };
  let path;
  if (/^file:/i.test(href)) path = stripFile(href);
  else if (SCHEME.test(href) && !/^[^:/]+\.[^:/]*:\d+(:\d+)?$/.test(href)) return null; // keeps "Main.kt:12"
  else path = href;
  path = decode(path);
  let line = null;
  let col = null;
  let m = /#L(\d+)(?:C(\d+))?(?:-L?\d+(?:C\d+)?)?$/i.exec(path) || /:(\d+)(?::(\d+))?$/.exec(path);
  if (m) {
    line = Number(m[1]);
    col = m[2] ? Number(m[2]) : null;
    path = path.slice(0, m.index);
  }
  path = path.replace(/#.*$/, '');
  return path ? { type: 'openPath', path, line, col } : null;
}

/** {url} loadable image, {remote, host} http(s) image, {path} file outside the workspace, or null. */
export function resolveImage(raw) {
  const src = String(raw || '').trim();
  if (/^data:image\//i.test(src)) return { url: src };
  if (/^https?:/i.test(src)) {
    let parsed = null;
    try { parsed = new URL(src); } catch (e) { return null; }
    if (parsed.host === location.host && parsed.pathname.indexOf('/workspace/') === 0) return { url: parsed.pathname };
    return { remote: src, host: parsed.host };
  }
  let path = /^file:/i.test(src) ? stripFile(src) : src;
  if (!path || SCHEME.test(path)) return null;
  path = decode(path).replace(/[?#].*$/, '');
  let rel = null;
  for (const prefix of prefixes) {
    if (path.indexOf(prefix) === 0) { rel = path.slice(prefix.length); break; }
  }
  if (rel == null && path.indexOf('/workspace/') === 0) rel = path.slice(11);
  if (rel == null && path.charAt(0) !== '/') rel = path.replace(/^(\.\/)+/, '');
  if (rel == null) return { path };
  const parts = rel.split('/');
  if (!rel || parts.some(p => p === '..')) return null;
  return { url: '/workspace/' + parts.map(encodeURIComponent).join('/') };
}

/** Image element (or chip / text fallback) for a src under the policy above. */
export function imageNode(src, alt, cls) {
  const r = resolveImage(src);
  const name = alt || String(src || '').split('/').pop() || S.image;
  if (r && r.url) return h('img', { src: r.url, alt: alt || '', class: cls });
  if (r && r.remote) return chip(S.image + ' · ' + r.host, () => post({ type: 'openUrl', url: r.remote }));
  if (r && r.path) return chip(S.image + ' · ' + name, () => post({ type: 'openPath', path: r.path, line: null, col: null }));
  return document.createTextNode(alt || '');
}

export function chip(label, onClick) {
  const el = button('chip', label, onClick);
  el.appendChild(icon('image'));
  el.appendChild(h('span', { text: label }));
  return el;
}

/** Delegated click handler for content links. */
export function onLinkClick(event) {
  const target = event.target;
  const link = target && target.closest ? target.closest('a') : null;
  if (!link) return;
  event.preventDefault();
  if (!link.closest('.md')) return;
  const msg = classifyHref(link.getAttribute('href'));
  if (msg) post(msg);
}
