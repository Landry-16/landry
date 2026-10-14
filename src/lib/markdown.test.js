import { describe, expect, it } from 'vitest';
import { render } from './markdown.js';

describe('markdown', () => {
  it('renders headings, emphasis, lists and code', () => {
    const html = render('# Titre\n**gras** et *italique* et `code`\n- un\n- deux\n1. a\n```\nx < y\n```');
    expect(html).toContain('<h1>Titre</h1>');
    expect(html).toContain('<strong>gras</strong> et <em>italique</em> et <code>code</code>');
    expect(html).toContain('<ul>\n<li>un</li>\n<li>deux</li>\n</ul>');
    expect(html).toContain('<ol>\n<li>a</li>\n</ol>');
    expect(html).toContain('<pre><code>x &lt; y</code></pre>');
  });

  it('renders task items and links', () => {
    const html = render('- [x] fait\n- [ ] à faire\n[site](https://landry.dev)');
    expect(html).toContain('<li class="task done">fait</li>');
    expect(html).toContain('<li class="task">à faire</li>');
    expect(html).toContain('<a href="https://landry.dev" target="_blank" rel="noreferrer">site</a>');
  });

  it('never lets markup or javascript links through', () => {
    const html = render('<img src=x onerror=alert(1)>\n[x](javascript:alert(1))\n<script>alert(1)</script>');
    expect(html).not.toContain('<img');
    expect(html).not.toContain('<script');
    expect(html).not.toContain('href="javascript');
    expect(html).toContain('&lt;img');
  });
});
