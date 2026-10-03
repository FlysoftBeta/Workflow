/* The page's only way out: WorkflowBridge.post(json) with a fixed set of message types. */
const TYPES = ['ready', 'layout', 'earlier', 'toggle', 'copy', 'openUrl', 'openPath', 'action'];

export function post(msg) {
  if (!msg || TYPES.indexOf(msg.type) < 0) return;
  const bridge = window.WorkflowBridge;
  if (!bridge || typeof bridge.post !== 'function') return;
  try { bridge.post(JSON.stringify(msg)); } catch (e) { console.error(e); }
}
