/* Tiny DOM helpers. Only page code creates elements this way; content goes through markdown.js. */
export function h(tag, props) {
  const el = document.createElement(tag);
  if (props) {
    for (const key of Object.keys(props)) {
      const value = props[key];
      if (value == null || value === false) continue;
      if (key === 'class') el.className = value;
      else if (key === 'text') el.textContent = value;
      else if (key === 'onclick') el.addEventListener('click', value);
      else el.setAttribute(key, value === true ? '' : String(value));
    }
  }
  for (let i = 2; i < arguments.length; i++) add(el, arguments[i]);
  return el;
}

function add(el, kid) {
  if (kid == null || kid === false) return;
  if (Array.isArray(kid)) kid.forEach(k => add(el, k));
  else el.appendChild(typeof kid === 'string' ? document.createTextNode(kid) : kid);
}

export function button(cls, label, onClick) {
  const el = h('button', { class: cls, type: 'button' });
  if (label) el.setAttribute('aria-label', label);
  el.addEventListener('click', event => { event.stopPropagation(); onClick(event); });
  return el;
}

export function clear(el) {
  while (el.firstChild) el.removeChild(el.firstChild);
}

/** Make `parent`'s children exactly `nodes`, moving only nodes that are out of place. */
export function syncChildren(parent, nodes) {
  let cur = parent.firstChild;
  for (const node of nodes) {
    if (node === cur) { cur = cur.nextSibling; continue; }
    parent.insertBefore(node, cur);
  }
  while (cur) {
    const next = cur.nextSibling;
    parent.removeChild(cur);
    cur = next;
  }
}

export function show(el, visible) {
  if (visible) el.removeAttribute('hidden');
  else el.setAttribute('hidden', '');
}
