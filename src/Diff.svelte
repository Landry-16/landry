<script>
  import { t } from './lib/i18n.svelte.js';
  import { invoke } from '@tauri-apps/api/core';

  let { onclose } = $props();
  let a = $state('');
  let b = $state('');
  let lines = $state([]);
  let first = $state();
  let seq = 0;

  const counts = $derived({
    ins: lines.filter((l) => l.tag === 'ins').length,
    del: lines.filter((l) => l.tag === 'del').length,
  });

  $effect(() => {
    first?.focus();
  });

  $effect(() => {
    const s = ++seq;
    const [left, right] = [a, b];
    invoke('diff_texts', { a: left, b: right }).then((r) => {
      if (s === seq) lines = r;
    });
  });

  function onkeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<div class="diff" role="dialog" aria-modal="true" aria-label={t('Comparer deux textes')} tabindex="-1" {onkeydown}>
  <header>
    <h1>{t('Comparer deux textes')}</h1>
    <p><span class="del">−{counts.del}</span> <span class="ins">+{counts.ins}</span> {t('lignes')}</p>
    <button onclick={onclose} aria-label={t('Fermer')}>{t('Échap')}</button>
  </header>
  <div class="inputs">
    <label>{t('Avant')}<textarea bind:this={first} bind:value={a} spellcheck="false"></textarea></label>
    <label>{t('Après')}<textarea bind:value={b} spellcheck="false"></textarea></label>
  </div>
  <ol class="result" aria-label={t('Différences')}>
    {#each lines as l}
      <li class={l.tag}><span aria-hidden="true">{l.tag === 'ins' ? '+' : l.tag === 'del' ? '−' : ' '}</span>{l.text}</li>
    {/each}
  </ol>
</div>

<style>
  .diff {
    width: min(1080px, calc(100vw - 48px));
    height: min(720px, calc(100vh - 48px));
    display: grid;
    grid-template-rows: auto auto 1fr;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 18px;
    box-shadow: 0 32px 90px rgb(0 0 0 / 0.42);
    overflow: hidden;
    outline: none;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 16px;
    padding: 18px 24px 12px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0;
    font-size: 1.3em;
  }
  header p {
    margin: 0;
    flex: 1;
    color: var(--muted);
  }
  button {
    font: inherit;
    color: inherit;
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 9px;
    padding: 5px 12px;
  }
  .inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding: 12px 24px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.8em;
    color: var(--muted);
  }
  textarea {
    height: 160px;
    resize: vertical;
    font: 0.85rem/1.4 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--fg);
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
  }
  .result {
    list-style: none;
    margin: 0;
    padding: 0 24px 16px;
    overflow: auto;
    font: 0.82rem/1.5 'JetBrains Mono', 'Fira Code', monospace;
  }
  .result li {
    white-space: pre-wrap;
    padding: 0 8px;
    border-radius: 4px;
  }
  .result span {
    display: inline-block;
    width: 1.5em;
    color: var(--muted);
  }
  .ins {
    color: #3fb950;
  }
  li.ins {
    background: color-mix(in srgb, #3fb950 14%, transparent);
    color: var(--fg);
  }
  .del {
    color: #e5484d;
  }
  li.del {
    background: color-mix(in srgb, #e5484d 14%, transparent);
    color: var(--fg);
  }
</style>
