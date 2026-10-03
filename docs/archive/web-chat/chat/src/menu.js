/* Small popup menu anchored below a button or bubble; closes on any outside tap or scroll. */
import { h } from './dom.js';

let current = null;

export function closeMenu() {
  if (current && current.parentNode) current.parentNode.removeChild(current);
  current = null;
}

export function openMenu(anchor, entries) {
  closeMenu();
  const menu = h('div', { class: 'menu', role: 'menu' });
  for (const entry of entries) {
    const item = h('button', { type: 'button', role: 'menuitem', text: entry[0] });
    item.addEventListener('click', event => { event.stopPropagation(); closeMenu(); entry[1](); });
    menu.appendChild(item);
  }
  const host = anchor.parentNode;
  menu.style.top = (anchor.offsetTop + anchor.offsetHeight + 4) + 'px';
  if (anchor.classList.contains('bubble')) menu.style.right = '0';
  else menu.style.left = Math.max(0, anchor.offsetLeft) + 'px';
  host.appendChild(menu);
  current = menu;
}

document.addEventListener('click', event => {
  if (current && !current.contains(event.target)) closeMenu();
}, true);
window.addEventListener('scroll', closeMenu, { passive: true });
