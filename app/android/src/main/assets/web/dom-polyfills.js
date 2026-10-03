/*
 * DOM methods newer than the Android 9 system WebView (Chrome 69 on the API 28 emulator) that the pinned
 * xterm.js 6.0.0 calls. core-js covers the language built-ins; these are the DOM gaps.
 */
(function () {
  'use strict';
  function replaceChildren() {
    while (this.lastChild) this.removeChild(this.lastChild);
    if (arguments.length) this.append.apply(this, arguments);
  }
  [window.Element, window.Document, window.DocumentFragment].forEach(function (type) {
    if (type && type.prototype && !type.prototype.replaceChildren) {
      Object.defineProperty(type.prototype, 'replaceChildren', { value: replaceChildren, configurable: true, writable: true });
    }
  });
})();
