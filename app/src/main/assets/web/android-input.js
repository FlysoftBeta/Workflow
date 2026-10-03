/*
 * Android keyCode 229 compatibility for Workflow's pinned @xterm/xterm 6.0.0.
 *
 * CompositionHelper's deferred textarea fallback reads the final textarea once
 * for every queued keydown. If several keys arrive before timers run, those
 * snapshots overlap. Finish one fallback at the next keydown/compositionstart
 * boundary, before xterm handles Enter or starts a composition; retain a timer
 * for the last key. This changes scheduling, not the text/del/replacement rules.
 * No onData filtering is used, so repeated characters remain separate input.
 *
 * This intentionally uses a private helper. Source fingerprints and shape checks
 * fail closed on a different bundle: install returns false and leaves xterm's
 * original behavior intact. A vendor upgrade requires reviewing these methods
 * and rerunning terminal-input.test.mjs plus the Android WebView input test.
 * Fingerprints are FNV-1a of Function#toString with whitespace removed; they are
 * compatibility checks, not security hashes. Vendor bundle SHA is recorded in
 * artifacts/physical-review/xterm-composition-fallback-location.json.
 *
 * The textarea delta rules below follow CompositionHelper, Copyright (c) 2016
 * The xterm.js authors, MIT (see vendor/xterm-xterm/LICENSE).
 */
(function (global) {
  'use strict';
  var installed = new WeakMap();

  function fingerprint(fn) {
    if (typeof fn !== 'function') return '';
    var source = Function.prototype.toString.call(fn).replace(/\s/g, '');
    var hash = 2166136261;
    for (var i = 0; i < source.length; i++) {
      hash = Math.imul(hash ^ source.charCodeAt(i), 16777619) >>> 0;
    }
    return source.length + ':' + hash.toString(16);
  }

  function install(term) {
    var helper = term && term._core && term._core._compositionHelper;
    if (!helper) return false;
    var existing = installed.get(helper);
    if (existing) return existing.matches();
    if (typeof term.loadAddon !== 'function' || helper._textarea !== term.textarea ||
        !helper._textarea || typeof helper._textarea.value !== 'string' ||
        helper._isComposing !== false || helper._isSendingComposition !== false ||
        typeof helper._dataAlreadySent !== 'string' ||
        !helper._coreService || typeof helper._coreService.triggerDataEvent !== 'function' ||
        fingerprint(helper._handleAnyTextareaChanges) !== '371:9061fd12' ||
        fingerprint(helper.keydown) !== '244:a7f63d12' ||
        fingerprint(helper.compositionstart) !== '208:627c4f70') return false;

    var originalChanges = helper._handleAnyTextareaChanges;
    var originalKeydown = helper.keydown;
    var originalStart = helper.compositionstart;
    var pending = null;
    var disposed = false;

    function cancelPending() {
      if (!pending) return;
      global.clearTimeout(pending.timer);
      pending = null;
    }

    function flush() {
      if (!pending || disposed) return;
      var oldValue = pending.oldValue;
      cancelPending();
      if (helper._isComposing) return;
      var newValue = helper._textarea.value;
      var diff = newValue.replace(oldValue, '');
      helper._dataAlreadySent = diff;
      if (newValue.length > oldValue.length) {
        helper._coreService.triggerDataEvent(diff, true);
      } else if (newValue.length < oldValue.length) {
        helper._coreService.triggerDataEvent('\x7f', true);
      } else if (newValue !== oldValue) {
        helper._coreService.triggerDataEvent(newValue, true);
      }
    }

    function changes() {
      if (disposed) return;
      flush();
      pending = { oldValue: helper._textarea.value, timer: null };
      pending.timer = global.setTimeout(flush, 0);
    }

    function keydown(event) {
      flush();
      return originalKeydown.call(this, event);
    }

    function compositionstart() {
      flush();
      return originalStart.apply(this, arguments);
    }

    function dispose() {
      if (disposed) return;
      disposed = true;
      cancelPending();
      if (helper._handleAnyTextareaChanges === changes) helper._handleAnyTextareaChanges = originalChanges;
      if (helper.keydown === keydown) helper.keydown = originalKeydown;
      if (helper.compositionstart === compositionstart) helper.compositionstart = originalStart;
      installed.delete(helper);
    }

    try {
      helper._handleAnyTextareaChanges = changes;
      helper.keydown = keydown;
      helper.compositionstart = compositionstart;
      // Use xterm's public addon lifetime so a pending final key cannot outlive
      // the terminal. This addon adds no DOM listeners or onData subscriptions.
      term.loadAddon({ activate: function () {}, dispose: dispose });
      installed.set(helper, { matches: function () {
        return !disposed && helper._handleAnyTextareaChanges === changes &&
          helper.keydown === keydown && helper.compositionstart === compositionstart;
      } });
      return true;
    } catch (_) {
      dispose();
      return false;
    }
  }

  global.WorkflowAndroidInput = { install: install };
})(window);
