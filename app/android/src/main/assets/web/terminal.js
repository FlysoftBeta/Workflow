/*
 * Offline xterm adapter. Android owns navigation/clipboard UI; Engine resolves files.
 * Bridge: ready, input, key, resize, openUrl, openPath, checkLinks(id,json), selection(text,x,y).
 * reset(text) starts a new retained output window. invalidateLinks() retires cwd/generation checks.
 */
(function () {
  'use strict';
  var host = document.getElementById('terminal');
  var term = new Terminal({
    cursorBlink: true, fontSize: 13, lineHeight: 1.1, scrollback: 5000,
    fontFamily: '"JetBrains Mono NL", "Noto Sans Mono CJK SC", monospace',
    allowProposedApi: false, macOptionIsMeta: false, rightClickSelectsWord: false,
    screenReaderMode: true, smoothScrollDuration: 0,
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
  function scanViewport() {
    var buffer = term.buffer.active;
    var links = [];
    var seen = Object.create(null);
    for (var y = buffer.viewportY; y < buffer.viewportY + term.rows && y < buffer.length; y++) {
      var line = logicalLine(y);
      if (seen[line.first]) continue;
      seen[line.first] = true;
      WorkflowTerminalLinks.find(line.text).forEach(function (link) {
        var segments = segmentsFor(line, link.start, link.end);
        links.push({ text: link.text, kind: link.kind, segments: segments,
          key: linkEpoch + ':' + link.kind + ':' + link.text + ':' + JSON.stringify(segments) });
      });
      // A long wrapped line must be decoded once, not once for every visible row.
      y = line.last;
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
    var now = Date.now();
    Object.keys(linkCache).forEach(function (text) {
      var cached = linkCache[text];
      if (now - cached.at >= (cached.pending ? CHECK_MS : CACHE_MS)) delete linkCache[text];
    });
    Object.keys(pendingChecks).forEach(function (id) {
      if (now - pendingChecks[id].at >= CHECK_MS) delete pendingChecks[id];
    });
    var candidates = scanViewport();
    var unknown = [];
    if (hasBridge('checkLinks') && hasBridge('openPath')) candidates.forEach(function (link) {
      if (link.kind !== 'path') return;
      var cached = linkCache[link.text];
      if (cached && ((cached.pending && now - cached.at < CHECK_MS) ||
          (!cached.pending && now - cached.at < CACHE_MS))) return;
      if (unknown.length < 128 && unknown.indexOf(link.text) < 0) unknown.push(link.text);
    });
    if (unknown.length) {
      var id = ++requestSeq;
      pendingChecks[id] = { texts: unknown, epoch: linkEpoch, at: now };
      unknown.forEach(function (text) { linkCache[text] = { ok: false, at: now, pending: true }; });
      if (!callBridge('checkLinks', [id, JSON.stringify(unknown)])) delete pendingChecks[id];
    }
    visibleLinks = candidates.filter(function (link) {
      var cached = linkCache[link.text];
      return link.kind === 'url' ? hasBridge('openUrl') :
        hasBridge('openPath') && cached && !cached.pending && cached.ok && now - cached.at < CACHE_MS;
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
  function invalidateLinks() {
    linkEpoch++;
    linkCache = Object.create(null);
    pendingChecks = Object.create(null);
    visibleLinks = [];
    pressedLink = null;
    if (touch) touch.link = null;
    drawLinks();
    scheduleRefresh();
  }

  // ---- Selection in xterm buffer coordinates ---------------------------------------------------
  var handles = null;
  var selectionReported = false;
  var selectionMarker = null;
  var reportedText = '';
  var selectionAnchor = { x: 0, y: 0 };
  var touch = null;
  var drag = null;
  var edgeTimer = 0;
  var suppressMouseUntil = 0;
  var TAP_SLOP = 10;
  function flat(cell) { return cell.row * term.cols + cell.col; }
  function currentSelection() {
    var range = term.getSelectionPosition();
    return range ? { start: { row: range.start.y, col: range.start.x }, end: { row: range.end.y, col: range.end.x } } : null;
  }
  function rowMarker(row) {
    var buffer = term.buffer.active;
    return buffer.type === 'normal' ? term.registerMarker(row - buffer.baseY - buffer.cursorY) : null;
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
    drawSelection();
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
  function reportSelection(x, y) {
    selectionReported = term.hasSelection();
    reportedText = term.getSelection() || '';
    selectionAnchor = { x: Math.round(x), y: Math.round(y) };
    callBridge('selection', [reportedText, selectionAnchor.x, selectionAnchor.y]);
  }
  function hideToolbar() { callBridge('selection', ['', 0, 0]); }
  function clearSelection() {
    releaseSelectionMarker();
    term.clearSelection();
    selectionReported = false;
    hideToolbar();
    drawSelection();
  }
  function drawSelection() {
    if (!handles) return;
    // xterm's programmatic start+length selection can survive trimming its start
    // row and accidentally select newer text. A public row marker fails closed.
    if (selectionMarker && selectionMarker.isDisposed) {
      selectionMarker = null;
      cancelTouch(); endDrag(true); clearSelection(); return;
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
      if (selectionReported && !drag && !(touch && touch.selecting) && term.getSelection() !== reportedText) {
        reportSelection(selectionAnchor.x, selectionAnchor.y);
      }
    }
    if (!range && selectionReported) { selectionReported = false; hideToolbar(); }
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
    } else if (touch && touch.selecting) {
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
      var point = drag && drag.kind !== 'scroll' ? drag.point : touch && touch.selecting ? touch.point : null;
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
    var metrics = scrollMetrics();
    scrollbar.hidden = !metrics.max;
    scrollbar.setAttribute('aria-valuemax', metrics.max);
    scrollbar.setAttribute('aria-valuenow', term.buffer.active.viewportY);
    scrollbar.setAttribute('aria-valuetext', 'Row ' + (term.buffer.active.viewportY + 1) + ' of ' + (metrics.max + 1));
    thumb.style.height = metrics.thumb + 'px';
    thumb.style.top = (metrics.max ? term.buffer.active.viewportY / metrics.max * metrics.travel : 0) + 'px';
  }
  function moveScrollbar(point) {
    var metrics = drag.metrics;
    // Freeze the row/track mapping for one drag so streaming cannot move its target.
    if (metrics.travel) term.scrollToLine(Math.round(clamp((point.y - metrics.top - drag.grab) / metrics.travel, 0, 1) * metrics.max));
    drawScrollbar();
    scheduleRefresh();
  }
  function controlTarget(target) {
    return target && target.closest ? target.closest('[data-workflow-control]') : null;
  }
  function startDrag(node, point, event, source) {
    if (drag || interactionSuspended) return;
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
  function endDrag(cancelled, event) {
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
    if (ended.kind !== 'scroll' && !cancelled && ended.node._anchor) reportSelection(ended.node._anchor.x, ended.node._anchor.y);
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
        if (drag && event.pointerId !== drag.pointerId) { endDrag(true, event); return; }
        var node = controlTarget(event.target);
        if (node && event.button === 0 && event.isPrimary !== false) startDrag(node, { x: event.clientX, y: event.clientY }, event, 'pointer');
      }, true);
      document.addEventListener('pointermove', function (event) {
        if (drag && drag.source === 'pointer' && drag.pointerId === event.pointerId) moveDrag({ x: event.clientX, y: event.clientY }, event);
      }, true);
      ['pointerup', 'pointercancel', 'lostpointercapture'].forEach(function (name) {
        document.addEventListener(name, function (event) {
          if (drag && drag.source === 'pointer' && drag.pointerId === event.pointerId) endDrag(name !== 'pointerup', event);
        }, true);
      });
    } else {
      // API 28 WebViews without PointerEvent retain the drag through document listeners.
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
    window.addEventListener('blur', function () { cancelTouch(); endDrag(true); });
  }

  // ---- Tap-versus-scroll arbitration and long press --------------------------------------------
  function cancelTouch() {
    if (touch) clearTimeout(touch.timer);
    if (touch && touch.wordMarker) touch.wordMarker.dispose();
    touch = null;
    stopEdgeScroll();
    if (pressedLink) { pressedLink = null; drawLinks(); }
  }
  host.addEventListener('touchstart', function (event) {
    if (interactionSuspended) return;
    if (event.touches.length !== 1) { cancelTouch(); endDrag(true); return; }
    if (controlTarget(event.target)) return;
    cancelTouch();
    var point = event.touches[0];
    var link = linkAt(point.clientX, point.clientY);
    touch = { x: point.clientX, y: point.clientY, link: link, moved: false, selecting: false,
      viewport: term.buffer.active.viewportY, hadSelection: term.hasSelection(), timer: 0 };
    if (link) { pressedLink = link; drawLinks(); }
    touch.timer = setTimeout(function () {
      if (!touch || touch.moved || touch.viewport !== term.buffer.active.viewportY) return;
      var cell = cellAt(touch.x, touch.y);
      if (!cell) return;
      touch.selecting = true;
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
    }, 450);
  }, { capture: true, passive: true });
  host.addEventListener('touchmove', function (event) {
    if (!touch || drag) return;
    if (event.touches.length !== 1) { cancelTouch(); return; }
    var point = event.touches[0];
    if (touch.selecting) {
      event.preventDefault(); event.stopPropagation();
      hideToolbar();
      moveSelection({ x: point.clientX, y: point.clientY });
      startEdgeScroll();
    } else if (Math.abs(point.clientX - touch.x) > TAP_SLOP || Math.abs(point.clientY - touch.y) > TAP_SLOP) {
      touch.moved = true;
      clearTimeout(touch.timer);
      pressedLink = null;
      drawLinks();
    }
  }, { capture: true, passive: false });
  host.addEventListener('touchend', function (event) {
    if (!touch || drag) return;
    var ended = touch;
    var point = event.changedTouches[0];
    cancelTouch();
    suppressMouseUntil = Date.now() + 800;
    if (!point) return;
    if (ended.selecting) {
      event.preventDefault(); event.stopPropagation();
      reportSelection(point.clientX, point.clientY);
      return;
    }
    if (ended.moved || ended.viewport !== term.buffer.active.viewportY ||
        Math.abs(point.clientX - ended.x) > TAP_SLOP || Math.abs(point.clientY - ended.y) > TAP_SLOP) return;
    if (ended.hadSelection) { event.preventDefault(); clearSelection(); return; }
    var released = linkAt(point.clientX, point.clientY);
    if (ended.link && released && released.key === ended.link.key) {
      event.preventDefault(); event.stopPropagation(); activate(released);
    }
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
    cancelTouch(); endDrag(true); clearSelection(); invalidateLinks();
  }
  function open() {
    term.open(host);
    if (!window.WorkflowAndroidInput.install(term)) console.error('Android terminal input adapter is unavailable');
    installControls();
    term.onData(function (data) { if (data.indexOf('\r') >= 0) invalidateLinks(); callBridge('input', [data]); });
    term.onResize(function (size) { resetInteraction(); callBridge('resize', [size.cols, size.rows]); scheduleRefresh(); });
    term.onRender(scheduleRefresh);
    term.onScroll(scheduleRefresh);
    term.onSelectionChange(scheduleRefresh);
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
      term.scrollToBottom(); callBridge('key', [data]);
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
      var now = Date.now();
      pending.texts.forEach(function (text, i) { linkCache[text] = { ok: Array.isArray(results) && results[i] === true, at: now }; });
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
    clearSelection: function () { cancelTouch(); endDrag(true); clearSelection(); },
    fit: fitViewport
  };
  var fontsReady = document.fonts && document.fonts.load ? document.fonts.load('13px "JetBrains Mono NL"') : Promise.resolve();
  fontsReady.then(open, open);
})();
