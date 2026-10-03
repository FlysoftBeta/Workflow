/* One requestAnimationFrame per frame: queued render tasks run first (each at most once), then the
 * frame-end hooks (scroll follow + layout report) see the final DOM of that frame. */
const tasks = new Set();
const endHooks = [];
let scheduled = false;

export function schedule(task) {
  tasks.add(task);
  requestFrame();
}

export function onFrameEnd(hook) {
  endHooks.push(hook);
}

export function requestFrame() {
  if (scheduled) return;
  scheduled = true;
  window.requestAnimationFrame(run);
}

function run() {
  scheduled = false;
  const list = Array.from(tasks);
  tasks.clear();
  for (const task of list) {
    try { task(); } catch (e) { console.error(e); }
  }
  for (const hook of endHooks) {
    try { hook(); } catch (e) { console.error(e); }
  }
}
