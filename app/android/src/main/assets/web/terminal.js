/*
 * Offline xterm adapter. Android owns navigation, clipboard and the selection chrome; Engine resolves files.
 * Bridge: ready, input, key, resize, openUrl, openPath, checkLinks(id,json), selection(text,x,y), selectionState(json).
 * reset(text) starts a new retained output window. invalidateLinks() retires cwd/generation checks.
 */
(function () {
  'use strict';
  var host = document.getElementById('terminal');
  // A touch long press dispatches contextmenu. xterm's right-click handler would move its hidden textarea under
  // the finger and select() it, which focuses it, opens the soft keyboard and resizes the page. Never let it run.
  host.addEventListener('contextmenu', function (event) { event.preventDefault(); event.stopPropagation(); }, true);
  var term = new Terminal({
    cursorBlink: true, fontSize: 13, lineHeight: 1.1, scrollback: 5000,
    fontFamily: '"JetBrains Mono NL", "Noto Sans Mono CJK SC", monospace',
    allowProposedApi: false, macOptionIsMeta: false, rightClickSelectsWord: false,
    screenReaderMode: true, smoothScrollDuration: 0,
    // OSC 8 hyperlinks are activated by this adapter's own hit regions; xterm's default would call confirm()/window.open.
    linkHandler: { activate: function () {}, allowNonHttpProtocols: false },
    theme: { background: '#F4FBF8', foreground: '#171D1B', cursor: '#286B57' }
  });
  var fit = new FitAddon.FitAddon();
  term.loadAddon(fit);

  function hasBridge(name) { return !!window.Workflow && typeof window.Workflow[name] === 'function'; }
  function callBridge(name, args) {
    if (!hasBridge(name)) return false;
    try { window.Workflow[name].apply(window.Workflow, args || []); return true; }
    catch (_) { return false; } // Detached WebViews and unavailable handlers must not break input.
  }
  function clamp(value, low, high) { return Math.max(low, Math.min(high, value)); }
  function now() { return window.performance && performance.now ? performance.now() : Date.now(); }
  function screen() { return host.querySelector('.xterm-screen'); }
  function cellSize() {
    var element = screen();
    if (!element || !term.cols || !term.rows) return null;
    var rect = element.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return null;
    return { left: rect.left, top: rect.top, width: rect.width / term.cols,
      height: rect.height / term.rows, right: rect.right, bottom: rect.bottom };
  }
  var fitting = false;
  function fitViewport() {
    if (fitting) return;
    fitting = true;
    requestAnimationFrame(function () {
      fitting = false;
      var height = Math.max(1, window.innerHeight) + 'px';
      [document.documentElement, document.body, host].forEach(function (element) {
        if (element.style.height !== height) element.style.height = height;
      });
      try { fit.fit(); } catch (_) { /* Not opened yet. */ }
      scheduleRefresh();
    });
  }

  // A UTF-16 index maps to a complete xterm cell, never to half an emoji/wide glyph.
  function logicalLine(y) {
    var buffer = term.buffer.active;
    var first = y;
    while (first > 0 && buffer.getLine(first) && buffer.getLine(first).isWrapped) first--;
    var text = '';
    var cells = [];
    for (var row = first; row < buffer.length; row++) {
      var line = buffer.getLine(row);
      if (!line || (row > first && !line.isWrapped)) break;
      var next = buffer.getLine(row + 1);
      for (var col = 0; col < term.cols; col++) {
        var cell = line.getCell(col);
        if (!cell) break;
        var width = cell.getWidth();
        if (width === 0) continue;
        // xterm leaves an empty padding cell when a wide glyph wraps at the margin.
        if (col === term.cols - 1 && !cell.getChars() && next && next.isWrapped &&
            next.getCell(0) && next.getCell(0).getWidth() === 2) continue;
        var chars = cell.getChars() || ' ';
        for (var k = 0; k < chars.length; k++) cells.push({ row: row, col: col, width: width });
        text += chars;
      }
    }
    return { first: first, last: row - 1, text: text.replace(/\s+$/, ''), cells: cells };
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

  // ---- Link candidates: plain text, hard-wrapped URLs and OSC 8 hyperlinks -------------------------
  var RAW_URL = /^https?:\/\/[^\s"'`<>]+/i;
  var CONTINUATION = /^[^\s"'`<>]+/;
  // The last visible character of a logical line sits in the last column: a program may have broken a long token.
  function reachesMargin(line) {
    var last = line.cells[line.text.length - 1];
    return !!last && last.col + last.width >= term.cols;
  }
  // TUIs print long URLs as several rows joined by cursor movement rather than soft wraps. A URL that
  // fills its row to the margin continues with the next row's leading delimiter-free run, starting in column 0.
  function stitchUrl(line, link) {
    var raw = RAW_URL.exec(line.text.slice(link.start));
    if (!raw) return null;
    var parts = [{ line: line, start: link.start, text: raw[0] }];
    var current = line;
    var piece = parts[0];
    while (parts.length < 8 && piece.start + piece.text.length === current.text.length && reachesMargin(current)) {
      var next = logicalLine(current.last + 1);
      if (next.first !== current.last + 1 || !next.text) break;
      var match = CONTINUATION.exec(next.text);
      if (!match || RAW_URL.test(match[0])) break;
      piece = { line: next, start: 0, text: match[0] };
      parts.push(piece);
      current = next;
    }
    if (parts.length === 1) return null;
    var joined = WorkflowTerminalLinks.find(parts.map(function (part) { return part.text; }).join(''))
      .filter(function (found) { return found.kind === 'url' && found.start === 0; })[0];
    if (!joined) return null;
    var remaining = joined.end;
    var segments = [];
    parts.forEach(function (part) {
      var take = Math.min(part.text.length, Math.max(0, remaining));
      if (take > 0) segments = segments.concat(segmentsFor(part.line, part.start, part.start + take));
      remaining -= part.text.length;
    });
    return { text: joined.text, segments: segments, parts: parts };
  }
  // Start above the viewport when its first rows continue a URL that began offscreen.
  function scanStart(top) {
    var first = logicalLine(top).first;
    var candidate = first;
    for (var guard = 0; guard < 8 && candidate > 0; guard++) {
      if (!CONTINUATION.test(logicalLine(candidate).text)) break;
      var previous = logicalLine(candidate - 1);
      if (!reachesMargin(previous)) break;
      if (/https?:\/\/[^\s"'`<>]*$/i.test(previous.text)) return previous.first;
      if (/[\s"'`<>]/.test(previous.text)) break;
      candidate = previous.first;
    }
    return first;
  }
  // OSC 8 ranges come from xterm's own link provider, so its scheme filter and cell attributes stay authoritative.
  // The provider service is internal; a missing or changed shape disables OSC 8 targets, never plain links.
  function oscRanges(row) {
    var service = term._core && term._core._linkProviderService;
    var providers = service && service.linkProviders;
    var found = [];
    if (!providers || typeof providers.forEach !== 'function') return found;
    providers.forEach(function (provider) {
      if (!provider || typeof provider.provideLinks !== 'function') return;
      try {
        provider.provideLinks(row + 1, function (links) {
          (links || []).forEach(function (link) {
            var range = link && link.range;
            if (!range || typeof link.text !== 'string' || range.start.y !== row + 1 || range.end.y !== row + 1) return;
            found.push({ text: link.text, segment: { row: row, col: range.start.x - 1, len: range.end.x - range.start.x + 1 } });
          });
        });
      } catch (_) { /* Plain-text links remain available. */ }
    });
    return found;
  }
  function overlaps(segments, others) {
    return segments.some(function (a) {
      return others.some(function (b) { return a.row === b.row && a.col < b.col + b.len && b.col < a.col + a.len; });
    });
  }

  // ---- Validated links -------------------------------------------------------------------------
  var overlay = null;
  var linkCache = Object.create(null);
  var pendingChecks = Object.create(null);
  var requestSeq = 0;
  var linkEpoch = 0;
  var visibleLinks = [];
  var pressedLink = null;
  var refreshFrame = 0;
  var CACHE_MS = 10000;
  var CHECK_MS = 5000;
  var interactionSuspended = false;
  function linkRecord(kind, text, segments) {
    return { text: text, kind: kind, segments: segments,
      key: linkEpoch + ':' + kind + ':' + text + ':' + JSON.stringify(segments) };
  }
  function scanViewport() {
    var buffer = term.buffer.active;
    var bottom = Math.min(buffer.viewportY + term.rows, buffer.length);
    var links = [];
    var consumed = Object.create(null);
    for (var y = scanStart(buffer.viewportY); y < bottom; ) {
      var line = logicalLine(y);
      var skip = consumed[line.first] || 0;
      var osc = [];
      for (var row = line.first; row <= line.last; row++) osc = osc.concat(oscRanges(row));
      var oscSegments = osc.map(function (item) { return item.segment; });
      WorkflowTerminalLinks.find(line.text).forEach(function (link) {
        if (link.start < skip) return; // Already part of the previous row's URL.
        var segments = segmentsFor(line, link.start, link.end);
        if (overlaps(segments, oscSegments)) return; // An explicit hyperlink wins over its label text.
        var stitched = link.kind === 'url' ? stitchUrl(line, link) : null;
        if (stitched) {
          stitched.parts.slice(1).forEach(function (part) { consumed[part.line.first] = part.text.length; });
          links.push(linkRecord('url', stitched.text, stitched.segments));
        } else links.push(linkRecord(link.kind, link.text, segments));
      });
      osc.forEach(function (item) {
        var previous = links[links.length - 1];
        var tail = previous && previous.osc && previous.segments[previous.segments.length - 1];
        if (tail && previous.text === item.text && tail.row + 1 === item.segment.row) {
          previous.segments.push(item.segment);
          previous.key = linkEpoch + ':osc:' + item.text + ':' + JSON.stringify(previous.segments);
        } else {
          var record = linkRecord('url', item.text, [item.segment]);
          record.osc = true;
          record.key = linkEpoch + ':osc:' + item.text + ':' + JSON.stringify(record.segments);
          links.push(record);
        }
      });
      // A long wrapped line must be decoded once, not once for every visible row.
      y = line.last + 1;
    }
    return links;
  }
  function scheduleRefresh() {
    if (refreshFrame) return;
    refreshFrame = requestAnimationFrame(function () {
      refreshFrame = 0;
      refreshLinks();
      drawSelection();
      drawScrollbar();
    });
  }
  function refreshLinks() {
    if (interactionSuspended) { visibleLinks = []; drawLinks(); return; }
    var time = Date.now();
    Object.keys(linkCache).forEach(function (text) {
      var cached = linkCache[text];
      if (time - cached.at >= (cached.pending ? CHECK_MS : CACHE_MS)) delete linkCache[text];
    });
    Object.keys(pendingChecks).forEach(function (id) {
      if (time - pendingChecks[id].at >= CHECK_MS) delete pendingChecks[id];
    });
    var candidates = scanViewport();
    var unknown = [];
    if (hasBridge('checkLinks') && hasBridge('openPath')) candidates.forEach(function (link) {
      if (link.kind !== 'path') return;
      var cached = linkCache[link.text];
      if (cached && ((cached.pending && time - cached.at < CHECK_MS) ||
          (!cached.pending && time - cached.at < CACHE_MS))) return;
      if (unknown.length < 128 && unknown.indexOf(link.text) < 0) unknown.push(link.text);
    });
    if (unknown.length) {
      var id = ++requestSeq;
      pendingChecks[id] = { texts: unknown, epoch: linkEpoch, at: time };
      unknown.forEach(function (text) { linkCache[text] = { ok: false, at: time, pending: true }; });
      if (!callBridge('checkLinks', [id, JSON.stringify(unknown)])) delete pendingChecks[id];
    }
    visibleLinks = candidates.filter(function (link) {
      var cached = linkCache[link.text];
      return link.kind === 'url' ? hasBridge('openUrl') :
        hasBridge('openPath') && cached && !cached.pending && cached.ok && time - cached.at < CACHE_MS;
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
    var used = [];
    visibleLinks.forEach(function (link) {
      var first = true;
      link.segments.forEach(function (segment, index) {
        var row = segment.row - top;
        if (row < 0 || row >= term.rows || !size) return;
        var key = link.key + ':' + index;
        var node = Array.prototype.find.call(overlay.children, function (child) { return child._linkKey === key; });
        if (node && (node.tagName === 'BUTTON') !== first) { overlay.removeChild(node); node = null; }
        if (!node) {
          node = document.createElement(first ? 'button' : 'span');
          node._linkKey = key;
          if (first) {
            node.type = 'button';
            node.setAttribute('aria-label', 'Open ' + link.text);
            node.addEventListener('click', function (event) {
              event.preventDefault(); event.stopPropagation();
              if (!term.hasSelection()) activate(node._link);
            });
          } else node.setAttribute('aria-hidden', 'true');
          overlay.appendChild(node);
        }
        first = false;
        node._link = link;
        node.className = 'workflow-link' + (pressedLink && pressedLink.key === link.key ? ' pressed' : '');
        node.style.left = segment.col * size.width + 'px';
        node.style.top = row * size.height + 'px';
        node.style.width = segment.len * size.width + 'px';
        node.style.height = size.height + 'px';
        used.push(node);
      });
    });
    Array.prototype.slice.call(overlay.children).forEach(function (node) {
      if (used.indexOf(node) < 0) overlay.removeChild(node);
    });
  }
  function linkAt(x, y) {
    // A render/scroll/async validation can precede the next animation frame.
    refreshLinks();
    var size = cellSize();
    if (!size || x < size.left || x >= size.right || y < size.top || y >= size.bottom) return null;
    var col = Math.floor((x - size.left) / size.width);
    var row = Math.floor((y - size.top) / size.height) + term.buffer.active.viewportY;
    for (var i = 0; i < visibleLinks.length; i++) {
      if (visibleLinks[i].segments.some(function (s) { return s.row === row && col >= s.col && col < s.col + s.len; })) return visibleLinks[i];
    }
    return null;
  }
  function activate(link) {
    if (!link) return;
    if (link.kind === 'url' && /^https?:\/\//i.test(link.text)) callBridge('openUrl', [link.text]);
    else if (link.kind === 'path') callBridge('openPath', [link.text]);
  }
  // A pressed link is identified by its text and first buffer cell. A public row marker follows that row
  // while output streams or old rows are trimmed, so a tap survives a moving viewport.
  function identify(link) {
    if (!link) return null;
    var first = link.segments[0];
    return { kind: link.kind, text: link.text, row: first.row, col: first.col, cols: term.cols,
      epoch: linkEpoch, marker: rowMarker(first.row) };
  }
  function releasePress(press) { if (press && press.marker) press.marker.dispose(); }
  function resolvePress(press) {
    if (!press || press.epoch !== linkEpoch || press.cols !== term.cols || interactionSuspended) return null;
    if (press.marker && press.marker.isDisposed) return null;
    var row = press.marker ? press.marker.line : press.row;
    refreshLinks();
    for (var i = 0; i < visibleLinks.length; i++) {
      var link = visibleLinks[i];
      if (link.kind === press.kind && link.text === press.text && link.segments[0].row === row &&
          link.segments[0].col === press.col) return link;
    }
    return null;
  }
  function invalidateLinks() {
    linkEpoch++;
    linkCache = Object.create(null);
    pendingChecks = Object.create(null);
    visibleLinks = [];
    pressedLink = null;
    if (touch) { releasePress(touch.press); touch.press = null; }
    drawLinks();
    scheduleRefresh();
  }

  // ---- Selection in xterm buffer coordinates ---------------------------------------------------
  var handles = null;
  var toolbarShown = false;
  var selectionDirty = false;
  var selectionMarker = null;
  var savedSelection = null;
  var restoringSelection = false;
  var reportedText = '';
  var selectionAnchor = { x: 0, y: 0 };
  var chromeState = '';
  var touch = null;
  var drag = null;
  var edgeTimer = 0;
  var suppressMouseUntil = 0;
  var TAP_SLOP = 10;
  var LONG_PRESS_MS = 450;
  function flat(cell) { return cell.row * term.cols + cell.col; }
  function currentSelection() {
    var range = term.getSelectionPosition();
    return range ? { start: { row: range.start.y, col: range.start.x }, end: { row: range.end.y, col: range.end.x } } : null;
  }
  function rowMarker(row) {
    var buffer = term.buffer.active;
    if (buffer.type !== 'normal') return null;
    return term.registerMarker(row - buffer.baseY - buffer.cursorY) || null;
  }
  function releaseSelectionMarker() {
    if (selectionMarker) selectionMarker.dispose();
    selectionMarker = null;
  }
  function trackSelection() {
    releaseSelectionMarker();
    var range = currentSelection();
    if (range) selectionMarker = rowMarker(range.start.row);
  }
  function cellAt(x, y) {
    var size = cellSize();
    if (!size) return null;
    return { col: clamp(Math.floor((x - size.left) / size.width), 0, term.cols - 1),
      row: clamp(Math.floor((y - size.top) / size.height) + term.buffer.active.viewportY,
        term.buffer.active.viewportY, Math.min(term.buffer.active.viewportY + term.rows - 1, term.buffer.active.length - 1)) };
  }
  function snapBoundary(value, after) {
    value = clamp(value, 0, term.buffer.active.length * term.cols);
    var row = Math.floor(value / term.cols);
    var col = value % term.cols;
    var line = term.buffer.active.getLine(row);
    if (line && col && line.getCell(col) && line.getCell(col).getWidth() === 0) value += after ? 1 : -1;
    return value;
  }
  function selectBetween(a, b) {
    var start = snapBoundary(Math.min(a, b), false);
    var end = snapBoundary(Math.max(a, b), true);
    if (end === start) end = snapBoundary(Math.min(start + 1, term.buffer.active.length * term.cols), true);
    if (end > start) term.select(start % term.cols, Math.floor(start / term.cols), end - start);
    trackSelection();
    savedSelection = term.hasSelection() ? { start: start, end: end, row: Math.floor(start / term.cols), cols: term.cols } : null;
    selectionDirty = true;
    drawSelection();
  }
  // xterm clears its selection whenever the row count changes (an IME or extra-keys row appearing). The
  // buffer does not reflow when only rows change, so the saved range is still exact; the start-row marker
  // accounts for rows trimmed in the meantime.
  function restoreSelection() {
    var saved = savedSelection;
    if (!saved || term.hasSelection() || saved.cols !== term.cols) return;
    if (!selectionMarker || selectionMarker.isDisposed) { savedSelection = null; return; }
    var shift = (selectionMarker.line - saved.row) * term.cols;
    restoringSelection = true;
    try { term.select((saved.start + shift) % term.cols, Math.floor((saved.start + shift) / term.cols), saved.end - saved.start); }
    finally { restoringSelection = false; }
    selectionDirty = true;
  }
  function selectWord(cell) {
    var line = logicalLine(cell.row);
    var index = -1;
    for (var i = 0; i < line.cells.length; i++) {
      var mapped = line.cells[i];
      if (mapped.row === cell.row && cell.col >= mapped.col && cell.col < mapped.col + mapped.width) { index = i; break; }
    }
    if (index < 0 || /\s/.test(line.text.charAt(index) || ' ')) {
      selectBetween(flat(cell), flat(cell) + 1); return;
    }
    var start = index;
    var end = index;
    while (start > 0 && !/[\s"'`<>|;]/.test(line.text.charAt(start - 1))) start--;
    while (end < line.text.length - 1 && !/[\s"'`<>|;]/.test(line.text.charAt(end + 1))) end++;
    selectBetween(flat(line.cells[start]), flat(line.cells[end]) + line.cells[end].width);
  }
  // The settled selection text and toolbar anchor; Android copies this text.
  function reportSelection(x, y) {
    toolbarShown = term.hasSelection();
    selectionDirty = false;
    reportedText = term.getSelection() || '';
    selectionAnchor = { x: Math.round(x), y: Math.round(y) };
    callBridge('selection', [reportedText, selectionAnchor.x, selectionAnchor.y]);
    publishChrome();
  }
  function hideToolbar() {
    toolbarShown = false;
    callBridge('selection', ['', 0, 0]);
    publishChrome();
  }
  function clearSelection() {
    releaseSelectionMarker();
    savedSelection = null;
    term.clearSelection();
    hideToolbar();
    drawSelection();
  }
  function round(value) { return Math.round(value * 10) / 10; }
  // Geometry for Android's native handles and floating toolbar, in CSS pixels of the layout viewport.
  function chromeGeometry() {
    var range = currentSelection();
    var size = cellSize();
    if (!range || !size || interactionSuspended) return { active: false };
    var top = term.buffer.active.viewportY;
    function endpoint(cell, edge) {
      var row = cell.row;
      var col = cell.col;
      if (edge === 'end' && col === 0 && row > 0) { row--; col = term.cols; }
      return { row: row, x: round(size.left + col * size.width), y: round(size.top + (row - top + 1) * size.height),
        visible: row >= top && row < top + term.rows };
    }
    var start = endpoint(range.start, 'start');
    var end = endpoint(range.end, 'end');
    var first = Math.max(start.row, top);
    var last = Math.min(end.row, top + term.rows - 1);
    var single = start.row === end.row;
    return {
      active: true, toolbar: toolbarShown && !drag && !(touch && touch.mode === 'select'),
      start: { x: start.x, y: start.y, visible: start.visible }, end: { x: end.x, y: end.y, visible: end.visible },
      rect: first > last ? null : { left: single ? start.x : round(size.left), right: single ? end.x : round(size.right),
        top: round(size.top + (first - top) * size.height), bottom: round(size.top + (last - top + 1) * size.height) },
      line: round(size.height), width: window.innerWidth
    };
  }
  function publishChrome() {
    if (!hasBridge('selectionState')) return;
    var state = JSON.stringify(chromeGeometry());
    if (state === chromeState) return;
    chromeState = state;
    callBridge('selectionState', [state]);
  }
  function drawSelection() {
    if (!handles) return;
    // xterm's programmatic start+length selection can survive trimming its start
    // row and accidentally select newer text. A public row marker fails closed.
    if (selectionMarker && selectionMarker.isDisposed) {
      selectionMarker = null;
      cancelTouch(); endDrag(true, null, true); clearSelection(); return;
    }
    var range = currentSelection();
    var size = cellSize();
    var hostRect = host.getBoundingClientRect();
    ['start', 'end'].forEach(function (edge) {
      var node = handles[edge];
      if (!range || !size) { node.hidden = true; return; }
      var endpoint = { row: range[edge].row, col: range[edge].col };
      if (edge === 'end' && endpoint.col === 0 && endpoint.row > 0) { endpoint.row--; endpoint.col = term.cols; }
      var row = endpoint.row - term.buffer.active.viewportY;
      // Retain both handles at the nearest edge when an endpoint is offscreen.
      var x = size.left + endpoint.col * size.width;
      var y = size.top + (clamp(row, 0, term.rows - 1) + 1) * size.height;
      node._anchor = { x: x, y: y };
      node.style.left = clamp(x - hostRect.left - (edge === 'start' ? 44 : 0), 0, Math.max(0, hostRect.width - 44)) + 'px';
      node.style.top = clamp(y - hostRect.top, 0, Math.max(0, hostRect.height - 44)) + 'px';
      node.setAttribute('aria-label', 'Selection ' + edge + ', row ' + (endpoint.row + 1) + ', column ' + (endpoint.col + 1));
      node.hidden = false;
    });
    if (range && size) {
      var a = handles.start.getBoundingClientRect();
      var b = handles.end.getBoundingClientRect();
      // Near a screen corner the clamped 44px targets can overlap. Keep both
      // independently reachable, while their actual buffer endpoints stay fixed.
      if (Math.abs(a.left - b.left) < 44 && Math.abs(a.top - b.top) < 44) {
        if (b.top - hostRect.top >= 44) handles.start.style.top = b.top - hostRect.top - 44 + 'px';
        else if (hostRect.height - (a.bottom - hostRect.top) >= 44) handles.end.style.top = a.bottom - hostRect.top + 'px';
        else if (hostRect.width >= 88) {
          handles.start.style.left = clamp(a.left - hostRect.left, 0, hostRect.width - 88) + 'px';
          handles.end.style.left = parseFloat(handles.start.style.left) + 44 + 'px';
        }
      }
      if (toolbarShown && selectionDirty && !drag && !(touch && touch.mode === 'select')) {
        selectionDirty = false;
        if (term.getSelection() !== reportedText) reportSelection(selectionAnchor.x, selectionAnchor.y);
      }
    }
    if (!range && toolbarShown) hideToolbar();
    publishChrome();
  }
  function boundaryAt(point) {
    var size = cellSize();
    if (!size) return null;
    var cell = cellAt(point.x, point.y - 1);
    cell.col = clamp(Math.round((point.x - size.left) / size.width), 0, term.cols);
    return flat(cell);
  }
  function moveSelection(point) {
    if (drag && drag.kind !== 'scroll') {
      var target = { x: point.x - drag.offsetX, y: point.y - drag.offsetY };
      drag.point = target;
      var boundary = boundaryAt(target);
      var range = currentSelection();
      if (boundary === null || !range) return;
      // Read the opposite endpoint afresh: xterm shifts its selection when old rows expire.
      var fixed = flat(range[drag.edge === 'start' ? 'end' : 'start']);
      selectBetween(fixed, boundary);
      if (boundary !== fixed) drag.edge = boundary < fixed ? 'start' : 'end';
    } else if (touch && touch.mode === 'select') {
      if (touch.wordMarker && touch.wordMarker.isDisposed) { cancelTouch(); clearSelection(); return; }
      touch.point = point;
      var cell = cellAt(point.x, point.y);
      if (cell) {
        var value = flat(cell);
        var shift = touch.wordMarker ? (touch.wordMarker.line - touch.wordRow) * term.cols : 0;
        selectBetween(Math.min(touch.wordStart + shift, value), Math.max(touch.wordEnd + shift, value + 1));
      }
    }
  }
  function startEdgeScroll() {
    if (edgeTimer) return;
    edgeTimer = setInterval(function () {
      var point = drag && drag.kind !== 'scroll' ? drag.point : touch && touch.mode === 'select' ? touch.point : null;
      var size = cellSize();
      if (!point || !size) return;
      var zone = Math.min(28, (size.bottom - size.top) / 4);
      var amount = point.y < size.top + zone ? -Math.ceil((size.top + zone - point.y) / 6) :
        point.y > size.bottom - zone ? Math.ceil((point.y - size.bottom + zone) / 6) : 0;
      if (!amount) return;
      term.scrollLines(clamp(amount, -8, 8));
      if (drag) moveSelection({ x: point.x + drag.offsetX, y: point.y + drag.offsetY });
      else moveSelection(point);
      scheduleRefresh();
    }, 50);
  }
  function stopEdgeScroll() { clearInterval(edgeTimer); edgeTimer = 0; }

  // ---- Touch scrolling with momentum -------------------------------------------------------------
  // xterm 6 has no touch scrolling. A vertical swipe moves the scrollback 1:1 with the finger, then decays.
  var scrollRemainder = 0;
  var fling = null;
  var FLING_DECAY_MS = 325;
  var MIN_FLING = 0.05; // px per ms: Android's minimum fling velocity of 50 dp/s
  var MAX_FLING = 8; // px per ms: Android's maximum fling velocity of 8000 dp/s
  function wheelReportsToApp() {
    var mode = term.modes && term.modes.mouseTrackingMode;
    return mode === 'vt200' || mode === 'drag' || mode === 'any';
  }
  // Returns the rows actually moved. Alternate screens and mouse-reporting programs receive the same
  // line-mode wheel input as a desktop mouse; xterm turns it into wheel reports or cursor keys.
  function scrollRows(rows, point) {
    if (!rows) return 0;
    var buffer = term.buffer.active;
    if (buffer.type === 'normal' && !wheelReportsToApp()) {
      var before = buffer.viewportY;
      term.scrollLines(rows);
      return term.buffer.active.viewportY - before;
    }
    var target = screen();
    var size = cellSize();
    if (!target || !size || typeof WheelEvent !== 'function') return 0;
    var x = point ? point.x : (size.left + size.right) / 2;
    var y = point ? point.y : (size.top + size.bottom) / 2;
    for (var i = 0; i < Math.abs(rows); i++) {
      target.dispatchEvent(new WheelEvent('wheel', { deltaY: rows < 0 ? -1 : 1, deltaMode: 1,
        clientX: x, clientY: y, bubbles: true, cancelable: true }));
    }
    return rows;
  }
  // Positive pixels move toward newer output. Returns false once the scrollback end stops the movement.
  function scrollPixels(pixels, point) {
    var size = cellSize();
    if (!size || !pixels) return !!size;
    scrollRemainder += pixels / size.height;
    var rows = scrollRemainder < 0 ? Math.ceil(scrollRemainder) : Math.floor(scrollRemainder);
    if (!rows) return true;
    scrollRemainder -= rows;
    if (scrollRows(rows, point) === rows) return true;
    scrollRemainder = 0;
    return false;
  }
  function stopFling() {
    if (!fling) return false;
    cancelAnimationFrame(fling.frame);
    fling = null;
    return true;
  }
  function startFling(velocity, point) {
    stopFling();
    velocity = clamp(velocity, -MAX_FLING, MAX_FLING);
    if (Math.abs(velocity) < MIN_FLING) return;
    var state = { velocity: velocity, point: point, last: 0, frame: 0 };
    function step(time) {
      if (fling !== state) return;
      var elapsed = state.last ? clamp(time - state.last, 0, 64) : 16;
      state.last = time;
      var moving = !interactionSuspended && scrollPixels(state.velocity * elapsed, state.point);
      state.velocity *= Math.exp(-elapsed / FLING_DECAY_MS);
      if (!moving || Math.abs(state.velocity) < 0.02) { fling = null; return; }
      state.frame = requestAnimationFrame(step);
    }
    fling = state;
    state.frame = requestAnimationFrame(step);
  }
  // Velocity over the last 100 ms, like Android's tracker; a finger that paused before release does not fling.
  function releaseVelocity(samples, time) {
    var recent = samples.filter(function (sample) { return time - sample.t <= 100; });
    if (recent.length < 2 || time - recent[recent.length - 1].t > 50) return 0;
    var first = recent[0];
    var last = recent[recent.length - 1];
    return last.t > first.t ? (first.y - last.y) / (last.t - first.t) : 0;
  }

  // ---- A large captured scrollbar, independent of xterm's desktop thumb -------------------------
  var scrollbar = null;
  var thumb = null;
  function scrollMetrics() {
    if (!scrollbar) return null;
    var rect = scrollbar.getBoundingClientRect();
    var max = term.buffer.active.baseY;
    var height = Math.min(rect.height, Math.max(48, rect.height * term.rows / (max + term.rows)));
    return { top: rect.top, height: rect.height, thumb: height, travel: Math.max(0, rect.height - height), max: max };
  }
  function drawScrollbar() {
    if (!scrollbar) return;
    // A hidden control has no DOM geometry. Reveal it before measuring the first scrollback frame.
    scrollbar.hidden = !term.buffer.active.baseY;
    var metrics = scrollMetrics();
    scrollbar.setAttribute('aria-valuemax', metrics.max);
    scrollbar.setAttribute('aria-valuenow', term.buffer.active.viewportY);
    scrollbar.setAttribute('aria-valuetext', 'Row ' + (term.buffer.active.viewportY + 1) + ' of ' + (metrics.max + 1));
    thumb.style.height = metrics.thumb + 'px';
    thumb.style.top = (metrics.max ? term.buffer.active.viewportY / metrics.max * metrics.travel : 0) + 'px';
  }
  function moveScrollbar(point) {
    var metrics = drag.metrics;
    drag.point = point;
    // Freeze the row/track mapping for one drag so streaming cannot move its target.
    if (metrics.travel) term.scrollToLine(Math.round(clamp((point.y - metrics.top - drag.grab) / metrics.travel, 0, 1) * metrics.max));
    drawScrollbar();
    scheduleRefresh();
  }
  // A resize (soft keyboard, extra-keys row, split) changes the track while the finger is down: remap
  // without ending the drag, keeping the finger's offset inside the thumb.
  function remapScrollDrag() {
    var metrics = scrollMetrics();
    if (!metrics || !metrics.height) return;
    drag.grab = clamp(drag.grab, 0, metrics.thumb);
    drag.metrics = metrics;
    if (drag.point) moveScrollbar(drag.point);
  }
  function controlTarget(target) {
    return target && target.closest ? target.closest('[data-workflow-control]') : null;
  }
  function startDrag(node, point, event, source) {
    if (drag || interactionSuspended) return;
    stopFling();
    cancelTouch();
    suppressMouseUntil = Date.now() + 800;
    var kind = node.getAttribute('data-workflow-control');
    drag = { kind: kind, node: node, source: source, pointerId: event.pointerId };
    if (kind === 'scroll') {
      var metrics = scrollMetrics();
      if (!metrics.max || !metrics.travel) { drag = null; return; }
      var thumbTop = term.buffer.active.viewportY / metrics.max * metrics.travel;
      var relative = point.y - metrics.top;
      drag.metrics = metrics;
      drag.grab = relative >= thumbTop && relative <= thumbTop + metrics.thumb ? relative - thumbTop : metrics.thumb / 2;
      moveScrollbar(point);
    } else {
      if (!currentSelection() || !node._anchor) { drag = null; return; }
      drag.edge = kind;
      drag.offsetX = point.x - node._anchor.x;
      drag.offsetY = point.y - node._anchor.y;
      hideToolbar();
    }
    node.classList.add('dragging');
    if (source === 'pointer' && node.setPointerCapture) {
      try { node.setPointerCapture(event.pointerId); } catch (_) { /* Document listeners retain capture. */ }
    }
    event.preventDefault(); event.stopPropagation();
  }
  function moveDrag(point, event) {
    if (!drag) return;
    event.preventDefault(); event.stopPropagation();
    if (drag.kind === 'scroll') moveScrollbar(point);
    else { moveSelection(point); startEdgeScroll(); }
  }
  // A cancelled handle drag keeps its selection and brings the toolbar back; `silent` is used when the
  // caller is about to discard the selection anyway.
  function endDrag(cancelled, event, silent) {
    if (!drag) return;
    var ended = drag;
    drag = null;
    stopEdgeScroll();
    ended.node.classList.remove('dragging');
    if (ended.source === 'pointer' && ended.node.releasePointerCapture) {
      try { ended.node.releasePointerCapture(ended.pointerId); } catch (_) { /* Already released. */ }
    }
    suppressMouseUntil = Date.now() + 800;
    if (event) { event.preventDefault(); event.stopPropagation(); }
    if (ended.kind !== 'scroll' && !silent && term.hasSelection()) {
      drawSelection();
      var anchor = handles[ended.edge] && handles[ended.edge]._anchor || ended.node._anchor;
      if (anchor) reportSelection(anchor.x, anchor.y);
    }
    scheduleRefresh();
  }
  function installControls() {
    scrollbar = document.createElement('div');
    scrollbar.className = 'workflow-scrollbar';
    scrollbar.setAttribute('data-workflow-control', 'scroll');
    scrollbar.setAttribute('role', 'scrollbar');
    scrollbar.setAttribute('aria-label', 'Terminal scrollback');
    scrollbar.setAttribute('aria-controls', 'terminal');
    scrollbar.setAttribute('aria-orientation', 'vertical');
    scrollbar.setAttribute('aria-valuemin', '0');
    scrollbar.tabIndex = 0;
    thumb = document.createElement('div');
    thumb.className = 'workflow-scroll-thumb';
    scrollbar.appendChild(thumb);
    host.appendChild(scrollbar);
    handles = {};
    ['start', 'end'].forEach(function (edge) {
      var node = document.createElement('button');
      node.type = 'button';
      node.hidden = true;
      node.className = 'workflow-selection-handle ' + edge;
      node.setAttribute('data-workflow-control', edge);
      node.setAttribute('aria-label', 'Selection ' + edge);
      handles[edge] = node;
      host.appendChild(node);
    });
    // Android draws platform teardrop handles over the page; these buttons remain for keyboards and accessibility.
    if (hasBridge('selectionState')) host.classList.add('native-selection-chrome');
    host.addEventListener('keydown', function (event) {
      var node = controlTarget(event.target);
      if (!node) return;
      var kind = node.getAttribute('data-workflow-control');
      var amount = event.key === 'ArrowUp' ? -1 : event.key === 'ArrowDown' ? 1 :
        event.key === 'PageUp' ? -term.rows : event.key === 'PageDown' ? term.rows : 0;
      if (kind === 'scroll') {
        if (event.key === 'Home') term.scrollToTop();
        else if (event.key === 'End') term.scrollToBottom();
        else if (amount) term.scrollLines(amount);
        else return;
      } else {
        var range = currentSelection();
        if (event.key === 'Escape') { clearSelection(); term.focus(); }
        else if (range) {
          if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') amount = event.key === 'ArrowLeft' ? -1 : 1;
          else amount *= term.cols;
          if (!amount) return;
          selectBetween(flat(range[kind === 'start' ? 'end' : 'start']), flat(range[kind]) + amount);
          reportSelection(node._anchor.x, node._anchor.y);
        } else return;
      }
      event.preventDefault(); event.stopPropagation(); scheduleRefresh();
    }, true);
    if (window.PointerEvent) {
      host.addEventListener('pointerdown', function (event) {
        if (drag && drag.source === 'pointer' && event.pointerId !== drag.pointerId) { endDrag(true, event); return; }
        var node = controlTarget(event.target);
        if (node && event.button === 0 && event.isPrimary !== false) startDrag(node, { x: event.clientX, y: event.clientY }, event, 'pointer');
      }, true);
      document.addEventListener('pointermove', function (event) {
        if (drag && drag.source === 'pointer' && drag.pointerId === event.pointerId) moveDrag({ x: event.clientX, y: event.clientY }, event);
      }, true);
      document.addEventListener('pointerup', function (event) {
        if (drag && drag.source === 'pointer' && drag.pointerId === event.pointerId) endDrag(false, event);
      }, true);
      document.addEventListener('pointercancel', function (event) {
        if (drag && drag.source === 'pointer' && drag.pointerId === event.pointerId) endDrag(true, event);
      }, true);
      // Only the capturing control losing capture ends a drag. Capture moving from the pressed thumb to
      // its track (or any other bubbling capture change) must not cancel it.
      document.addEventListener('lostpointercapture', function (event) {
        if (drag && drag.source === 'pointer' && drag.pointerId === event.pointerId && event.target === drag.node) endDrag(true, event);
      }, true);
    } else {
      // WebViews without PointerEvent retain the drag through document listeners.
      host.addEventListener('touchstart', function (event) {
        var node = controlTarget(event.target);
        if (node && event.touches.length === 1) startDrag(node, { x: event.touches[0].clientX, y: event.touches[0].clientY }, event, 'touch');
      }, { capture: true, passive: false });
      document.addEventListener('touchmove', function (event) {
        if (drag && drag.source === 'touch') {
          if (event.touches.length !== 1) endDrag(true, event);
          else moveDrag({ x: event.touches[0].clientX, y: event.touches[0].clientY }, event);
        }
      }, { capture: true, passive: false });
      ['touchend', 'touchcancel'].forEach(function (name) {
        document.addEventListener(name, function (event) { if (drag && drag.source === 'touch') endDrag(name !== 'touchend', event); }, { capture: true, passive: false });
      });
      host.addEventListener('mousedown', function (event) {
        var node = controlTarget(event.target);
        if (node && event.button === 0 && Date.now() >= suppressMouseUntil) startDrag(node, { x: event.clientX, y: event.clientY }, event, 'mouse');
      }, true);
      document.addEventListener('mousemove', function (event) {
        if (drag && drag.source === 'mouse') moveDrag({ x: event.clientX, y: event.clientY }, event);
      }, true);
      document.addEventListener('mouseup', function (event) { if (drag && drag.source === 'mouse') endDrag(false, event); }, true);
    }
    // Touch streams are cancelled by Android itself. A blur only ends what cannot otherwise finish:
    // a pending press, a fling and a mouse drag whose release happens outside the window.
    window.addEventListener('blur', function () {
      stopFling();
      if (touch && touch.mode === 'press') cancelTouch();
      if (drag && drag.source === 'mouse') endDrag(true);
    });
  }

  // ---- Touch arbitration: tap, vertical scroll and long-press selection ------------------------
  function cancelTouch() {
    if (touch) {
      clearTimeout(touch.timer);
      if (touch.wordMarker) touch.wordMarker.dispose();
      releasePress(touch.press);
    }
    touch = null;
    stopEdgeScroll();
    if (pressedLink) { pressedLink = null; drawLinks(); }
  }
  host.addEventListener('touchstart', function (event) {
    var stoppedFling = stopFling();
    if (interactionSuspended) return;
    if (event.touches.length !== 1) { cancelTouch(); if (drag && drag.source !== 'native') endDrag(true); return; }
    if (controlTarget(event.target)) return;
    cancelTouch();
    var point = event.touches[0];
    // A tap that stops a fling only stops it, as on Android.
    var link = stoppedFling ? null : linkAt(point.clientX, point.clientY);
    touch = { x: point.clientX, y: point.clientY, lastY: point.clientY, mode: 'press', press: identify(link),
      stoppedFling: stoppedFling, hadSelection: term.hasSelection(), samples: [{ t: now(), y: point.clientY }], timer: 0 };
    if (link) { pressedLink = link; drawLinks(); }
    touch.timer = setTimeout(function () {
      if (!touch || touch.mode !== 'press') return;
      var cell = cellAt(touch.x, touch.y);
      if (!cell) return;
      touch.mode = 'select';
      releasePress(touch.press);
      touch.press = null;
      pressedLink = null;
      drawLinks();
      selectWord(cell);
      var range = currentSelection();
      if (!range) return;
      touch.wordStart = flat(range.start);
      touch.wordEnd = flat(range.end);
      touch.wordRow = range.start.row;
      touch.wordMarker = rowMarker(range.start.row);
      reportSelection(touch.x, touch.y);
    }, LONG_PRESS_MS);
  }, { capture: true, passive: true });
  host.addEventListener('touchmove', function (event) {
    if (!touch || drag) return;
    if (event.touches.length !== 1) { cancelTouch(); return; }
    var point = event.touches[0];
    var x = point.clientX;
    var y = point.clientY;
    if (touch.mode === 'select') {
      event.preventDefault(); event.stopPropagation();
      if (toolbarShown) hideToolbar();
      moveSelection({ x: x, y: y });
      startEdgeScroll();
      return;
    }
    if (touch.mode === 'press') {
      var dx = x - touch.x;
      var dy = y - touch.y;
      if (Math.abs(dy) > TAP_SLOP && Math.abs(dy) >= Math.abs(dx)) {
        touch.mode = 'scroll';
        // Start from the slop boundary so content does not jump by the slop distance.
        touch.lastY = touch.y + (dy > 0 ? TAP_SLOP : -TAP_SLOP);
      } else if (Math.abs(dx) > TAP_SLOP) touch.mode = 'pan';
      if (touch.mode !== 'press') {
        clearTimeout(touch.timer);
        releasePress(touch.press);
        touch.press = null;
        if (pressedLink) { pressedLink = null; drawLinks(); }
      }
    }
    if (touch.mode === 'scroll') {
      event.preventDefault();
      var time = now();
      touch.samples.push({ t: time, y: y });
      while (touch.samples.length > 2 && time - touch.samples[0].t > 100) touch.samples.shift();
      scrollPixels(touch.lastY - y, { x: x, y: y });
      touch.lastY = y;
    }
  }, { capture: true, passive: false });
  host.addEventListener('touchend', function (event) {
    if (!touch || drag) return;
    var ended = touch;
    var point = event.changedTouches[0];
    var still = !!point && ended.mode === 'press' &&
      Math.abs(point.clientX - ended.x) <= TAP_SLOP && Math.abs(point.clientY - ended.y) <= TAP_SLOP;
    var released = still && !ended.stoppedFling && !ended.hadSelection ? resolvePress(ended.press) : null;
    cancelTouch();
    suppressMouseUntil = Date.now() + 800;
    if (!point) return;
    if (ended.mode === 'select') {
      event.preventDefault(); event.stopPropagation();
      reportSelection(point.clientX, point.clientY);
      return;
    }
    if (ended.mode === 'scroll') {
      // No compatibility mouse events: a swipe never focuses xterm or opens the keyboard.
      event.preventDefault();
      startFling(releaseVelocity(ended.samples, now()), { x: point.clientX, y: point.clientY });
      return;
    }
    if (!still) return;
    if (ended.stoppedFling) { event.preventDefault(); return; }
    if (ended.hadSelection) { event.preventDefault(); clearSelection(); return; }
    if (released) { event.preventDefault(); event.stopPropagation(); activate(released); }
  }, { capture: true, passive: false });
  host.addEventListener('touchcancel', cancelTouch, { capture: true, passive: true });
  host.addEventListener('mousedown', function (event) {
    if (!controlTarget(event.target)) releaseSelectionMarker();
  }, true);
  host.addEventListener('click', function (event) {
    if (controlTarget(event.target) || (event.target.closest && event.target.closest('.workflow-link'))) return;
    if (Date.now() < suppressMouseUntil || (event.sourceCapabilities && event.sourceCapabilities.firesTouchEvents)) {
      event.preventDefault(); return;
    }
    var link = linkAt(event.clientX, event.clientY);
    if (link && !term.hasSelection()) { event.preventDefault(); event.stopPropagation(); activate(link); }
  }, true);

  // Serialize writes and resets. An older queued xterm write may finish, but cannot land
  // after a new Engine generation/retention reset. xterm itself owns streaming row anchors.
  var outputQueue = [];
  var outputBusy = false;
  var resetSerial = 0;
  function completeOutput(next) {
    if ((next.reset || next.clear) && next.serial === resetSerial) interactionSuspended = false;
    scheduleRefresh();
    drainOutput();
  }
  function drainOutput() {
    if (outputBusy || !outputQueue.length) return;
    var next = outputQueue.shift();
    if (next.reset) term.reset();
    if (next.clear) term.clear();
    if (!next.text) { completeOutput(next); return; }
    outputBusy = true;
    term.write(next.text, function () { outputBusy = false; completeOutput(next); });
  }
  function resetInteraction() {
    stopFling(); cancelTouch(); endDrag(true, null, true); clearSelection(); invalidateLinks();
  }
  // Only a column change reflows the buffer. A rows-only resize keeps gestures, the pending tap and the
  // selection; a reflow keeps only scrolling, because cell geometry under the finger has changed.
  var geometryCols = 0;
  function handleResize(size) {
    var reflowed = size.cols !== geometryCols;
    geometryCols = size.cols;
    if (reflowed) {
      if (touch && touch.mode !== 'scroll') cancelTouch();
      if (drag && drag.kind !== 'scroll') endDrag(true, null, true);
      clearSelection();
    } else Promise.resolve().then(restoreSelection); // xterm's own resize listener clears it after this one.
    if (drag && drag.kind === 'scroll') remapScrollDrag();
    callBridge('resize', [size.cols, size.rows]);
    scheduleRefresh();
  }
  function open() {
    term.open(host);
    if (!window.WorkflowAndroidInput.install(term)) console.error('Android terminal input adapter is unavailable');
    installControls();
    geometryCols = term.cols;
    term.onData(function (data) {
      stopFling();
      if (data.indexOf('\r') >= 0) invalidateLinks();
      callBridge('input', [data]);
    });
    term.onResize(handleResize);
    term.onRender(scheduleRefresh);
    term.onScroll(scheduleRefresh);
    term.onSelectionChange(function () {
      selectionDirty = true;
      // xterm clears the selection inside a rows-only resize, and onResize restores it in a microtask. Forget
      // the saved range only when nothing restores it in the same task (input, buffer switch, reset).
      if (!term.hasSelection() && savedSelection && !restoringSelection) {
        var candidate = savedSelection;
        Promise.resolve().then(function () {
          if (savedSelection === candidate && !term.hasSelection()) { savedSelection = null; releaseSelectionMarker(); }
        });
      }
      scheduleRefresh();
    });
    term.buffer.onBufferChange(resetInteraction);
    if (window.ResizeObserver) new ResizeObserver(fitViewport).observe(host);
    window.addEventListener('resize', fitViewport);
    fitViewport();
    callBridge('ready');
  }
  window.WorkflowTerminal = {
    write: function (text) { outputQueue.push({ text: text }); drainOutput(); },
    reset: function (text) {
      interactionSuspended = true; resetInteraction();
      outputQueue = [{ reset: true, text: text, serial: ++resetSerial }]; drainOutput();
    },
    clear: function () {
      interactionSuspended = true; resetInteraction();
      outputQueue.push({ clear: true, serial: ++resetSerial }); drainOutput();
    },
    focus: function () { term.focus(); },
    blur: function () { term.blur(); },
    key: function (normal, application) {
      var data = application && term.modes.applicationCursorKeysMode ? application : normal;
      stopFling(); term.scrollToBottom(); callBridge('key', [data]);
    },
    paste: function (text) { term.paste(text); },
    theme: function (theme, linkColor) {
      term.options.theme = theme;
      document.body.style.background = theme.background;
      if (linkColor) document.documentElement.style.setProperty('--link', linkColor);
      scheduleRefresh();
    },
    font: function (size) { term.options.fontSize = size; fitViewport(); },
    linksChecked: function (id, results) {
      var pending = pendingChecks[id];
      delete pendingChecks[id];
      if (!pending || pending.epoch !== linkEpoch || Date.now() - pending.at >= CHECK_MS) return;
      var time = Date.now();
      pending.texts.forEach(function (text, i) { linkCache[text] = { ok: Array.isArray(results) && results[i] === true, at: time }; });
      scheduleRefresh();
    },
    invalidateLinks: invalidateLinks,
    selectAll: function () {
      if (interactionSuspended) return;
      term.selectAll();
      var range = currentSelection();
      // Select the current buffer snapshot instead of expanding with later output.
      if (range) selectBetween(flat(range.start), flat(range.end));
      reportSelection(0, 0);
    },
    clearSelection: function () { cancelTouch(); endDrag(true, null, true); clearSelection(); },
    // A native selection handle drag: x/y is the handle's hotspot (the endpoint at its row's bottom) in CSS px.
    dragHandle: function (edge, phase, x, y) {
      var point = { x: Number(x), y: Number(y) };
      if (phase === 'start') {
        if (drag || interactionSuspended || !handles || !handles[edge] || !currentSelection()) return false;
        stopFling(); cancelTouch();
        drag = { kind: edge, edge: edge, node: handles[edge], source: 'native', offsetX: 0, offsetY: 0, point: point };
        handles[edge].classList.add('dragging');
        hideToolbar();
        return true;
      }
      if (!drag || drag.source !== 'native') return false;
      if (phase === 'move') { moveSelection(point); startEdgeScroll(); }
      else endDrag(phase !== 'end');
      return true;
    },
    fit: fitViewport
  };
  var fontsReady = document.fonts && document.fonts.load ? document.fonts.load('13px "JetBrains Mono NL"') : Promise.resolve();
  fontsReady.then(open, open);
})();
