/*
 * Link candidates in terminal text (docs/ui.md §4.4): URLs, and local paths with an optional
 * :line[:col] or (line,col) suffix. Paths are only candidates here: the app resolves them against the
 * shell's current directory and underlines only those that exist. Pure function, no DOM; written for
 * the Android 9 WebView (no matchAll, optional chaining or replaceAll).
 */
(function (global) {
  'use strict';
  var TOKEN = /[^\s"'`<>|;&{}]+/g;
  var URL_PREFIX = /^(?:https?|ftp):\/\/./i;
  var EXTENSION = /\.[A-Za-z0-9_+-]{1,12}(?::\d+(?::\d+)?|\(\d+(?:,\d+)?\))?$/;
  var LINE_SUFFIX = /^[A-Za-z0-9_.+-]+:\d+(?::\d+)?$/;
  var LEADING = '([{\'"';
  var TRAILING = ')]}.,;:!?\'"';

  function balancedClose(token) {
    // Keep a ')' closing a "(line,col)" suffix or a URL parenthesis like /wiki/Foo_(bar).
    if (/\(\d+(?:,\d+)?\)$/.test(token)) return true;
    var open = token.split('(').length - 1;
    var close = token.split(')').length - 1;
    return open >= close;
  }

  function trim(token, start) {
    while (token.length && LEADING.indexOf(token.charAt(0)) >= 0) { token = token.substring(1); start++; }
    while (token.length) {
      var last = token.charAt(token.length - 1);
      if (TRAILING.indexOf(last) < 0) break;
      if (last === ')' && balancedClose(token)) break;
      token = token.substring(0, token.length - 1);
    }
    return { text: token, start: start };
  }

  function isPath(text) {
    if (text.length < 2 || text.length > 1024) return false;
    if (/^[.\/~:0-9-]+$/.test(text) && !/[A-Za-z]/.test(text)) return false; // "../", "1.2", "10:30"
    if (text.indexOf('/') >= 0) return /[A-Za-z0-9_]/.test(text);
    return EXTENSION.test(text) || LINE_SUFFIX.test(text);
  }

  /** Returns [{start, end, text, kind: 'url'|'path'}] with string indices (end exclusive). */
  function find(line) {
    var links = [];
    var match;
    TOKEN.lastIndex = 0;
    while ((match = TOKEN.exec(line)) !== null) {
      var trimmed = trim(match[0], match.index);
      var text = trimmed.text;
      if (!text) continue;
      var urlAt = text.search(/(?:https?|ftp):\/\//i);
      if (urlAt >= 0) {
        var url = text.substring(urlAt);
        if (URL_PREFIX.test(url)) links.push({ start: trimmed.start + urlAt, end: trimmed.start + urlAt + url.length, text: url, kind: 'url' });
        continue;
      }
      if (text.indexOf('file://') === 0) {
        links.push({ start: trimmed.start, end: trimmed.start + text.length, text: text, kind: 'path' });
        continue;
      }
      if (isPath(text)) links.push({ start: trimmed.start, end: trimmed.start + text.length, text: text, kind: 'path' });
    }
    return links;
  }

  global.WorkflowTerminalLinks = { find: find };
})(typeof window !== 'undefined' ? window : globalThis);
