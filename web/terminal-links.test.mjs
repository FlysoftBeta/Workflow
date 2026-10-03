import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const source = await readFile(new URL('../app/src/main/assets/web/terminal-links.js', import.meta.url), 'utf8');
const context = { window: {} };
vm.runInNewContext(source, context);
// JSON round trip: arrays from the vm realm have another prototype.
const find = line => JSON.parse(JSON.stringify(context.window.WorkflowTerminalLinks.find(line).map(l => [l.kind, l.text, line.slice(l.start, l.end)])));

test('relative paths from ls / echo output are candidates (term_link.png)', () => {
  assert.deepEqual(find('./settings.gradle.kts'), [['path', './settings.gradle.kts', './settings.gradle.kts']]);
  assert.deepEqual(find('README.md  app  build.gradle.kts').map(l => l[1]), ['README.md', 'build.gradle.kts']);
  assert.deepEqual(find('qa.md'), [['path', 'qa.md', 'qa.md']]);
});

test('line and column suffixes stay part of the link, trailing punctuation does not', () => {
  assert.deepEqual(find('src/Main.kt:12:5: error: unresolved').map(l => l[1]), ['src/Main.kt:12:5']);
  assert.deepEqual(find('at foo (lib/a.js:3:9)').map(l => l[1]), ['lib/a.js:3:9']);
  assert.deepEqual(find('see "docs/ui.md".').map(l => l[1]), ['docs/ui.md']);
  assert.deepEqual(find('Main.java(12,4)').map(l => l[1]), ['Main.java(12,4)']);
  assert.deepEqual(find('Makefile:20').map(l => l[1]), ['Makefile:20']);
});

test('URLs are recognised with balanced parentheses and without trailing punctuation', () => {
  assert.deepEqual(find('open https://example.com/a?b=1.'), [['url', 'https://example.com/a?b=1', 'https://example.com/a?b=1']]);
  assert.deepEqual(find('(https://en.wikipedia.org/wiki/Foo_(bar))').map(l => l[1]), ['https://en.wikipedia.org/wiki/Foo_(bar)']);
  assert.deepEqual(find('x=https://a.b/c,').map(l => l[1]), ['https://a.b/c']);
});

test('non-path tokens are not candidates', () => {
  assert.deepEqual(find('version 1.2.3 at 10:30, ratio ../ and / only'), []);
  assert.deepEqual(find('$ ls -la'), []);
  assert.deepEqual(find('~/notes/todo.txt /workspace/a b').map(l => l[1]), ['~/notes/todo.txt', '/workspace/a']);
});

test('indices map back into the original line, including CJK text before the link', () => {
  const line = '中文 docs/说明.md 结束';
  const [link] = context.window.WorkflowTerminalLinks.find(line);
  assert.equal(line.slice(link.start, link.end), 'docs/说明.md');
});
