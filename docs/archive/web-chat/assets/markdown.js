/* Parse math as Markdown tokens so backslashes survive and code stays literal. */
const mathRenderer = token => '<span class="workflow-math" data-formula="' + encodeURIComponent(token.formula) + '" data-display="' + token.display + '"></span>';
marked.use({ extensions: [
  { name: 'blockMath', level: 'block', start: source => source.search(/\$\$|\\\[/),
    tokenizer(source) {
      const match = /^(?:\$\$([\s\S]+?)\$\$|\\\[([\s\S]+?)\\\])(?:\n|$)?/.exec(source);
      if (match) return { type: 'blockMath', raw: match[0], formula: match[1] || match[2], display: true };
    }, renderer: mathRenderer },
  { name: 'inlineMath', level: 'inline', start: source => source.search(/\$|\\\(/),
    tokenizer(source) {
      const match = /^(?:\$(?!\$)((?:\\.|[^\n$\\])+?)\$|\\\(([\s\S]+?)\\\))/.exec(source);
      if (match) return { type: 'inlineMath', raw: match[0], formula: match[1] || match[2], display: false };
    }, renderer: mathRenderer }
] });
/* The only executable content is packaged code. Assistant HTML is always sanitized. */
window.WorkflowMarkdown = {
  render(text) {
    const element = document.getElementById('content');
    element.innerHTML = DOMPurify.sanitize(marked.parse(text || '', { gfm: true, breaks: false }), {
      USE_PROFILES: { html: true }, FORBID_TAGS: ['img', 'input', 'form', 'style'],
      FORBID_ATTR: ['style'], ALLOW_DATA_ATTR: false, ADD_ATTR: ['data-formula', 'data-display']
    });
    element.querySelectorAll('.workflow-math[data-formula]').forEach(node => {
      let formula;
      try { formula = decodeURIComponent(node.dataset.formula); } catch (_) { node.remove(); return; }
      katex.render(formula, node, {
        displayMode: node.dataset.display === 'true', throwOnError: false, trust: false, strict: 'ignore'
      });
    });
  }
};
document.addEventListener('click', event => {
  const link = event.target.closest('a');
  if (!link) return;
  event.preventDefault();
  if (window.Workflow) window.Workflow.openLink(link.getAttribute('href') || '');
});
new ResizeObserver(() => {
  if (window.Workflow) Workflow.height(Math.ceil(document.getElementById('content').getBoundingClientRect().height));
}).observe(document.getElementById('content'));
