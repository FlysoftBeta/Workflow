import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { JSDOM } from 'jsdom';
const assets = new URL('../app/src/main/assets/web/', import.meta.url);

async function renderer() {
  const dom = new JSDOM('<!doctype html><article id="content"></article>', { runScripts: 'outside-only' });
  dom.window.ResizeObserver = class { observe() {} };
  for (const file of ['vendor/marked/marked.umd.js', 'vendor/dompurify/purify.min.js', 'vendor/katex/katex.min.js', 'markdown.js']) {
    dom.window.eval(await readFile(new URL(file, assets), 'utf8'));
  }
  return dom.window;
}

test('streaming Markdown accepts unfinished text then renders fenced code and all math delimiters', async () => {
  const window = await renderer();
  window.WorkflowMarkdown.render('Thinking about $x^');
  assert.match(window.document.body.textContent, /Thinking about/);
  window.WorkflowMarkdown.render('Inline $x^2$ and \\(y+1\\).\n\n$$\\frac{1}{2}$$\n\n\\[z^3\\]\n\n```js\nconst price = "$5$"\n```');
  assert.equal(window.document.querySelectorAll('.katex').length, 4);
  assert.match(window.document.querySelector('pre code').textContent, /\$5\$/);
  assert.equal(window.document.querySelectorAll('pre .katex').length, 0);
  window.close();
});

test('untrusted HTML, URLs and malformed formula attributes cannot execute or break rendering', async () => {
  const window = await renderer();
  window.WorkflowMarkdown.render('<script>Workflow.input("bad")</script><img src=x onerror="bad()"><a href="javascript:bad()">click</a><span class="workflow-math" data-formula="%XX">bad</span>\n\nSafe $x$');
  assert.equal(window.document.querySelectorAll('script,img,[onerror],a[href^="javascript:"]').length, 0);
  assert.equal(window.document.querySelectorAll('.katex').length, 1);
  window.close();
});

test('markdown links are handed to native navigation instead of opening in the privileged WebView', async () => {
  const window = await renderer();
  let selected;
  window.Workflow = { openLink: value => { selected = value; } };
  window.WorkflowMarkdown.render('[Docs](https://example.org/docs)');
  const event = new window.MouseEvent('click', { bubbles: true, cancelable: true });
  window.document.querySelector('a').dispatchEvent(event);
  assert.equal(selected, 'https://example.org/docs');
  assert.equal(event.defaultPrevented, true);
  window.close();
});
