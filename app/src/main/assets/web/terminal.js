/*
 * Workflow terminal page: xterm.js + the Android input adapter + touch links and selection.
 * Bridge (window.Workflow, provided by the app): ready(), input(data), key(data), resize(cols, rows),
 * openUrl(url), openPath(text), checkLinks(id, json), selection(text, x, y).
 * API for the app: window.WorkflowTerminal (write, reset, clear, focus, blur, key, paste, theme, font,
 * linksChecked, invalidateLinks, selectAll, clearSelection, fit).
 */
(function () {
  'use strict';
  var bridge = window.Workflow || {
    ready: function () {}, input: function () {}, key: function () {}, resize: function () {},
    openUrl: function () {}, openPath: function () {}, checkLinks: function () {}, selection: function () {}
  };
  var host = document.getElementById('terminal');
  var term = new Terminal({
    cursorBlink: true, fontSize: 13, lineHeight: 1.1, scrollback: 5000,
    fontFamily: '"JetBrains Mono NL", "Noto Sans Mono CJK SC", monospace',
    allowProposedApi: false, macOptionIsMeta: false, rightClickSelectsWord: false,
    theme: { background: '#F4FBF8', foreground: '#171D1B', cursor: '#286B57' }
  });
  var fit = new FitAddon.FitAddon();
  term.loadAddon(fit);

  var fitting = false;
  function fitViewport() {
    if (fitting) return;
    fitting = true;
    requestAnimationFrame(function () {
      fitting = false;
      // Some Android WebView embeddings expose a real innerHeight while CSS percentage/vh heights
      // still use a zero auto-size layout viewport.
      var height = Math.max(1, window.innerHeight) + 'px';
      [document.documentElement, document.body, host].forEach(function (element) {
        if (element.style.height !== height) element.style.height = height;
      });
      try { fit.fit(); } catch (e) { /* not opened yet */ }
      scheduleLinks();
    });
  }

  // ---- Links -------------------------------------------------------------------------------------
  var overlay = null;
  var linkCache = {};          // text -> { ok: bool, at: ms }
  var pendingChecks = {};      // request id -> [texts]
  var requestSeq = 0;
  var visibleLinks = [];       // [{text, kind, segments: [{row, col, len}]}]
  var linkTimer = 0;
  var pressedLink = null;
  var CACHE_MS = 10000;

  function screen() { return host.querySelector('.xterm-screen'); }

  function cellSize() {
    var element = screen();
    if (!element || !term.cols || !term.rows) return null;
    var rect = element.getBoundingClientRect();
    return { left: rect.left, top: rect.top, width: rect.width / term.cols, height: rect.height / term.rows };
  }

  /** The logical (unwrapped) line containing buffer row y: text plus index -> {row, col} cells. */
  function logicalLine(y) {
    var buffer = term.buffer.active;
    var first = y;
    while (first > 0 && buffer.getLine(first) && buffer.getLine(first).isWrapped) first--;
    var text = '';
    var cells = [];
    for (var row = first; row < buffer.length; row++) {
      var line = buffer.getLine(row);
      if (!line || (row > first && !line.isWrapped)) break;
      for (var col = 0; col < term.cols; col++) {
        var cell = line.getCell(col);
        if (!cell) break;
        var width = cell.getWidth();
        if (width === 0) continue;
        var chars = cell.getChars() || ' ';
        for (var k = 0; k < chars.length; k++) cells.push({ row: row, col: col, width: width });
        text += chars;
      }
    }
    return { first: first, text: text.replace(/\s+$/, ''), cells: cells };
  }

  function segmentsFor(line, start, end) {
    var segments = [];
    var current = null;
    for (var i = start; i < end && i < line.cells.length; i++) {
      var cell = line.cells[i];
      if (current && current.row === cell.row && current.col + current.len === cell.col) current.len += cell.width;
      else if (!current || current.row !== cell.row || cell.col >= current.col + current.len) {
        current = { row: cell.row, col: cell.col, len: cell.width };
        segments.push(current);
      }
    }
    return segments;
  }

  function scanViewport() {
    var buffer = term.buffer.active;
    var top = buffer.viewportY;
    var links = [];
    var seen = {};
    for (var y = top; y < top + term.rows && y < buffer.length; y++) {
      var line = logicalLine(y);
      if (seen[line.first]) continue;
      seen[line.first] = true;
      WorkflowTerminalLinks.find(line.text).forEach(function (link) {
        links.push({ text: link.text, kind: link.kind, segments: segmentsFor(line, link.start, link.end) });
      });
    }
    return links;
  }

  function scheduleLinks() {
    if (linkTimer) return;
    linkTimer = setTimeout(function () { linkTimer = 0; refreshLinks(); }, 120);
  }

  function refreshLinks() {
    var now = Date.now();
    var candidates = scanViewport();
    var unknown = [];
    candidates.forEach(function (link) {
      if (link.kind !== 'path') return;
      var cached = linkCache[link.text];
      if ((!cached || now - cached.at > CACHE_MS) && unknown.indexOf(link.text) < 0) unknown.push(link.text);
    });
    if (unknown.length) {
      var id = ++requestSeq;
      pendingChecks[id] = unknown;
      unknown.forEach(function (text) { if (!linkCache[text]) linkCache[text] = { ok: false, at: 0, pending: true }; });
      bridge.checkLinks(id, JSON.stringify(unknown.slice(0, 200)));
    }
    visibleLinks = candidates.filter(function (link) {
      return link.kind === 'url' || (linkCache[link.text] && linkCache[link.text].ok);
    });
    drawLinks();
  }

  function drawLinks() {
    var element = screen();
    if (!element) return;
    if (!overlay || overlay.parentNode !== element) {
      overlay = document.createElement('div');
      overlay.className = 'workflow-links';
      element.appendChild(overlay);
    }
    var size = cellSize();
    var top = term.buffer.active.viewportY;
    var html = '';
    visibleLinks.forEach(function (link, index) {
      link.segments.forEach(function (segment) {
        var row = segment.row - top;
        if (row < 0 || row >= term.rows || !size) return;
        html += '<div class="workflow-link' + (pressedLink === link ? ' pressed' : '') + '" data-link="' + index +
          '" style="left:' + (segment.col * size.width) + 'px;top:' + ((row + 1) * size.height - 2) +
          'px;width:' + (segment.len * size.width) + 'px"></div>';
      });
    });
    overlay.innerHTML = html;
  }

  function linkAt(clientX, clientY) {
    var size = cellSize();
    if (!size) return null;
    var col = Math.floor((clientX - size.left) / size.width);
    var row = Math.floor((clientY - size.top) / size.height) + term.buffer.active.viewportY;
    for (var i = 0; i < visibleLinks.length; i++) {
      var hit = visibleLinks[i].segments.some(function (s) { return s.row === row && col >= s.col && col < s.col + s.len; });
      if (hit) return visibleLinks[i];
    }
    return null;
  }

  function activate(link) {
    if (link.kind === 'url') bridge.openUrl(link.text); else bridge.openPath(link.text);
  }

  // ---- Touch: tap links, long-press selection ---------------------------------------------------------
  var touch = null;    // { x, y, at, link, moved, timer, selecting, anchor }
  var TAP_SLOP = 10;

  function cellAt(clientX, clientY) {
    var size = cellSize();
    if (!size) return null;
    return {
      col: Math.max(0, Math.min(term.cols - 1, Math.floor((clientX - size.left) / size.width))),
      row: Math.max(0, Math.floor((clientY - size.top) / size.height)) + term.buffer.active.viewportY
    };
  }

  function selectWord(cell) {
    var line = logicalLine(cell.row);
    var index = -1;
    for (var i = 0; i < line.cells.length; i++) {
      if (line.cells[i].row === cell.row && line.cells[i].col === cell.col) { index = i; break; }
    }
    if (index < 0 || /\s/.test(line.text.charAt(index) || ' ')) { term.select(cell.col, cell.row, 1); return; }
    var start = index;
    var end = index;
    while (start > 0 && !/[\s"'`<>|;]/.test(line.text.charAt(start - 1))) start--;
    while (end < line.text.length - 1 && !/[\s"'`<>|;]/.test(line.text.charAt(end + 1))) end++;
    var from = line.cells[start];
    var length = 0;
    for (var j = start; j <= end; j++) length += line.cells[j].width;
    term.select(from.col, from.row, length);
  }

  function reportSelection(x, y) {
    bridge.selection(term.getSelection() || '', Math.round(x), Math.round(y));
  }

  host.addEventListener('touchstart', function (event) {
    if (event.touches.length !== 1) { cancelTouch(); return; }
    var point = event.touches[0];
    var link = linkAt(point.clientX, point.clientY);
    touch = { x: point.clientX, y: point.clientY, at: Date.now(), link: link, moved: false, selecting: false, timer: 0 };
    if (link) { pressedLink = link; drawLinks(); }
    touch.timer = setTimeout(function () {
      if (!touch || touch.moved) return;
      touch.selecting = true;
      if (pressedLink) { pressedLink = null; drawLinks(); }
      touch.anchor = cellAt(touch.x, touch.y);
      if (touch.anchor) selectWord(touch.anchor);
      reportSelection(touch.x, touch.y);
    }, 450);
  }, { capture: true, passive: true });

  host.addEventListener('touchmove', function (event) {
    if (!touch) return;
    var point = event.touches[0];
    if (touch.selecting) {
      event.preventDefault();
      event.stopPropagation();
      var cell = cellAt(point.clientX, point.clientY);
      var anchor = touch.anchor;
      if (cell && anchor) {
        var a = anchor.row * term.cols + anchor.col;
        var b = cell.row * term.cols + cell.col;
        var from = Math.min(a, b);
        term.select(from % term.cols, Math.floor(from / term.cols), Math.abs(b - a) + 1);
      }
      return;
    }
    if (Math.abs(point.clientX - touch.x) > TAP_SLOP || Math.abs(point.clientY - touch.y) > TAP_SLOP) {
      touch.moved = true;
      clearTimeout(touch.timer);
      if (pressedLink) { pressedLink = null; drawLinks(); }
    }
  }, { capture: true, passive: false });

  host.addEventListener('touchend', function (event) {
    if (!touch) return;
    var ended = touch;
    touch = null;
    clearTimeout(ended.timer);
    if (ended.selecting) {
      event.preventDefault();
      var point = event.changedTouches[0];
      reportSelection(point.clientX, point.clientY);
      return;
    }
    if (pressedLink) { pressedLink = null; drawLinks(); }
    if (ended.link && !ended.moved) {
      // Suppress the emulated mouse events (no cursor move, no keyboard) and open the link.
      event.preventDefault();
      activate(ended.link);
      return;
    }
    if (!ended.moved && term.hasSelection()) { term.clearSelection(); bridge.selection('', 0, 0); }
  }, { capture: true, passive: false });

  host.addEventListener('touchcancel', function () { cancelTouch(); }, { capture: true, passive: true });

  function cancelTouch() {
    if (touch) clearTimeout(touch.timer);
    touch = null;
    if (pressedLink) { pressedLink = null; drawLinks(); }
  }

  // Hardware mouse (Chromebooks, DeX): click opens links too.
  host.addEventListener('click', function (event) {
    if (event.sourceCapabilities && event.sourceCapabilities.firesTouchEvents) return;
    var link = linkAt(event.clientX, event.clientY);
    if (link && !term.hasSelection()) { event.preventDefault(); activate(link); }
  }, true);

  // ---- Boot ----------------------------------------------------------------------------------------
  function open() {
    term.open(host);
    if (!window.WorkflowAndroidInput.install(term)) console.error('Android terminal input adapter is unavailable');
    term.onData(function (data) {
      if (data.indexOf('\r') >= 0) linkCache = {};
      bridge.input(data);
    });
    term.onResize(function (size) { bridge.resize(size.cols, size.rows); });
    term.onRender(scheduleLinks);
    term.onScroll(scheduleLinks);
    new ResizeObserver(fitViewport).observe(host);
    window.addEventListener('resize', fitViewport);
    fitViewport();
    bridge.ready();
  }

  window.WorkflowTerminal = {
    write: function (text) { term.write(text); },
    reset: function (text) { term.reset(); if (text) term.write(text); linkCache = {}; },
    clear: function () { term.clear(); },
    focus: function () { term.focus(); },
    blur: function () { term.blur(); },
    /** Extra-keys row: [normal] or, in DECCKM application cursor mode, [application]. */
    key: function (normal, application) {
      var data = application && term.modes.applicationCursorKeysMode ? application : normal;
      term.scrollToBottom();
      bridge.key(data);
    },
    paste: function (text) { term.paste(text); },
    theme: function (theme, linkColor) {
      term.options.theme = theme;
      document.body.style.background = theme.background;
      if (linkColor) document.documentElement.style.setProperty('--link', linkColor);
      scheduleLinks();
    },
    font: function (size) { term.options.fontSize = size; fitViewport(); },
    linksChecked: function (id, results) {
      var texts = pendingChecks[id];
      delete pendingChecks[id];
      if (!texts) return;
      var now = Date.now();
      texts.forEach(function (text, i) { linkCache[text] = { ok: !!results[i], at: now }; });
      refreshLinks();
    },
    invalidateLinks: function () { linkCache = {}; scheduleLinks(); },
    selectAll: function () { term.selectAll(); reportSelection(0, 0); },
    clearSelection: function () { term.clearSelection(); },
    fit: fitViewport
  };

  var fontsReady = document.fonts && document.fonts.load ? document.fonts.load('13px "JetBrains Mono NL"') : Promise.resolve();
  fontsReady.then(open, open);
})();
