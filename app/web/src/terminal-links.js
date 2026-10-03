/*
 * Offline terminal link candidates, with UTF-16 string offsets. HTTP(S) URLs open
 * through Android; file candidates are validated and resolved by the Engine.
 * This scanner never probes a filesystem or guesses a workspace root.
 */
(function (global) {
  'use strict';
  var TOKEN = /[^\s"'`<>|;&{}]+/g;
  var URL = /https?:\/\/[^\s"'`<>]+/gi;
  var QUOTED = /(["'])([^\r\n]*?)\1/g;
  var EXTENSION = /\.[A-Za-z0-9_+-]{1,12}(?::\d+(?::\d+)?|\(\d+(?:,\d+)?\))?$/;
  var LINE_SUFFIX = /^[^\s:]+:\d+(?::\d+)?$/;
  var LEADING = '([{\'"';
  var TRAILING = ')]}.,;:!?\'"';

  function balancedClose(token, open, close) {
    return token.split(open).length >= token.split(close).length;
  }

  function trim(token, start) {
    while (token.length && LEADING.indexOf(token.charAt(0)) >= 0) { token = token.substring(1); start++; }
    while (token.length) {
      var last = token.charAt(token.length - 1);
      if (TRAILING.indexOf(last) < 0) break;
      if (last === ')' && balancedClose(token, '(', ')')) break;
      if (last === ']' && balancedClose(token, '[', ']')) break;
      token = token.substring(0, token.length - 1);
    }
    return { text: token, start: start };
  }

  function isPath(text) {
    if (text.length < 2 || text.length > 1024) return false;
    if (/^file:\/\//i.test(text)) return true;
    if (/^[A-Za-z][A-Za-z0-9+.-]*:\/\//.test(text) || /^(?:javascript|data|mailto):/i.test(text)) return false;
    if (/^[.\/~:0-9-]+$/.test(text)) return false;
    if (text.indexOf('/') >= 0) return /[^\s.\/~:0-9-]/.test(text);
    return EXTENSION.test(text) || LINE_SUFFIX.test(text);
  }

  function find(line) {
    var links = [];
    var occupied = [];
    var match;
    function overlaps(start, end) {
      return occupied.some(function (range) { return start < range.end && end > range.start; });
    }
    function add(text, start, kind, limitStart, limitEnd) {
      links.push({ start: start, end: start + text.length, text: text, kind: kind });
      occupied.push({ start: limitStart, end: limitEnd });
    }
    // Scan URLs before shell token delimiters: '&' and ';' belong to query strings.
    URL.lastIndex = 0;
    while ((match = URL.exec(line)) !== null) {
      var url = trim(match[0], match.index);
      if (/^https?:\/\/.+/i.test(url.text)) add(url.text, url.start, 'url', match.index, URL.lastIndex);
    }
    QUOTED.lastIndex = 0;
    while ((match = QUOTED.exec(line)) !== null) {
      if (!overlaps(match.index, QUOTED.lastIndex) && isPath(match[2])) {
        add(match[2], match.index + 1, 'path', match.index, QUOTED.lastIndex);
      }
    }
    TOKEN.lastIndex = 0;
    while ((match = TOKEN.exec(line)) !== null) {
      if (overlaps(match.index, TOKEN.lastIndex)) continue;
      var token = trim(match[0], match.index);
      if (isPath(token.text)) add(token.text, token.start, 'path', match.index, TOKEN.lastIndex);
    }
    return links.sort(function (a, b) { return a.start - b.start; });
  }

  global.WorkflowTerminalLinks = { find: find };
})(typeof window !== 'undefined' ? window : globalThis);
