// Small Markdown renderer for notes. HTML in the source is escaped first, so a note can never
// inject markup or scripts; only the patterns below produce tags.

const escape = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

function inline(s) {
  return s
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>')
    .replace(/~~([^~]+)~~/g, '<del>$1</del>')
    // Only http(s) links: no javascript: URLs.
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>');
}

export function render(src) {
  const lines = escape(src ?? '').split('\n');
  const out = [];
  let list = null;
  let code = null;
  const closeList = () => {
    if (list) out.push(`</${list}>`);
    list = null;
  };
  for (const line of lines) {
    if (line.startsWith('```')) {
      if (code === null) {
        closeList();
        code = [];
      } else {
        out.push(`<pre><code>${code.join('\n')}</code></pre>`);
        code = null;
      }
      continue;
    }
    if (code !== null) {
      code.push(line);
      continue;
    }
    const h = line.match(/^(#{1,3})\s+(.*)$/);
    const task = line.match(/^\s*[-*]\s+\[( |x)\]\s+(.*)$/i);
    const ul = line.match(/^\s*[-*]\s+(.*)$/);
    const ol = line.match(/^\s*\d+[.)]\s+(.*)$/);
    if (h) {
      closeList();
      out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`);
    } else if (task || ul || ol) {
      const kind = ol && !ul ? 'ol' : 'ul';
      if (list !== kind) {
        closeList();
        out.push(`<${kind}>`);
        list = kind;
      }
      if (task) out.push(`<li class="task${task[1].toLowerCase() === 'x' ? ' done' : ''}">${inline(task[2])}</li>`);
      else out.push(`<li>${inline((ul ?? ol)[1])}</li>`);
    } else if (line.startsWith('&gt; ')) {
      closeList();
      out.push(`<blockquote>${inline(line.slice(5))}</blockquote>`);
    } else if (line.trim() === '') {
      closeList();
    } else {
      closeList();
      out.push(`<p>${inline(line)}</p>`);
    }
  }
  closeList();
  if (code !== null) out.push(`<pre><code>${code.join('\n')}</code></pre>`);
  return out.join('\n');
}
