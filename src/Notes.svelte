<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { render } from './lib/markdown.js';

  let { onclose, noteId = 0 } = $props();

  let notes = $state([]);
  let current = $state(null);
  let filter = $state('');
  let mode = $state('edit');
  let saved = $state('');
  let failure = $state('');
  let confirmDelete = $state(false);
  let editor = $state();
  let timer;

  const title = (body) => (body.split('\n').find((l) => l.trim()) ?? 'Note vide').replace(/^#+\s*/, '').trim();
  const visible = $derived(
    notes.filter((n) => !filter.trim() || n.body.toLowerCase().includes(filter.trim().toLowerCase())),
  );

  async function load(selectId) {
    notes = await invoke('notes_list');
    const pick = notes.find((n) => n.id === selectId) ?? (selectId === undefined ? current : null);
    if (pick) open(pick);
  }

  $effect(() => {
    const id = untrack(() => Number(noteId) || 0);
    load(id || undefined).then(() => {
      if (!current && notes.length) open(notes[0]);
      editor?.focus();
    });
  });

  function open(n) {
    flush();
    current = { ...n };
    confirmDelete = false;
    saved = '';
  }

  async function save() {
    if (!current) return;
    try {
      const id = await invoke('note_save', { id: current.id, body: current.body });
      if (!current.id) current.id = id;
      saved = 'Enregistré';
      notes = await invoke('notes_list');
    } catch (e) {
      failure = String(e);
    }
  }

  // Autosave half a second after the last keystroke.
  function oninput() {
    saved = '';
    clearTimeout(timer);
    timer = setTimeout(save, 500);
  }

  function flush() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
      save();
    }
  }

  function create() {
    flush();
    current = { id: 0, body: '# Nouvelle note\n\n', updated: 0 };
    mode = 'edit';
    setTimeout(() => editor?.focus(), 0);
  }

  async function remove() {
    if (!current?.id) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    await invoke('note_delete', { id: current.id });
    current = null;
    await load();
    if (notes.length) open(notes[0]);
  }

  function onkeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      flush();
      onclose();
    } else if (e.key === 'n' && e.ctrlKey) {
      e.preventDefault();
      create();
    } else if (e.key === 'e' && e.ctrlKey) {
      e.preventDefault();
      mode = mode === 'edit' ? 'preview' : 'edit';
    }
  }
</script>

<div class="notes" role="dialog" aria-modal="true" aria-label="Notes" tabindex="-1" {onkeydown}>
  <header>
    <h1>Notes</h1>
    <input class="search" bind:value={filter} placeholder="Chercher dans les notes" aria-label="Chercher dans les notes" />
    <button onclick={create} title="Ctrl+N">Nouvelle note</button>
    <button onclick={() => { flush(); onclose(); }} aria-label="Fermer">Échap</button>
  </header>
  <div class="columns">
    <ul class="list" aria-label="Liste des notes">
      {#each visible as n (n.id)}
        <li>
          <button class:current={current?.id === n.id} onclick={() => open(n)}>
            <span class="t">{title(n.body)}</span>
            <small>{new Date(n.updated * 1000).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })}</small>
          </button>
        </li>
      {:else}
        <li class="empty">{filter ? 'Aucune note ne correspond.' : 'Aucune note. Ctrl+N pour commencer.'}</li>
      {/each}
    </ul>
    <div class="editor">
      {#if current}
        <div class="toolbar">
          <div class="tabs" role="tablist">
            <button role="tab" aria-selected={mode === 'edit'} onclick={() => (mode = 'edit')}>Écrire</button>
            <button role="tab" aria-selected={mode === 'preview'} onclick={() => (mode = 'preview')}>Aperçu</button>
          </div>
          <span class="state">{saved}</span>
          {#if current.id}<button class="danger" onclick={remove}>{confirmDelete ? 'Confirmer la suppression' : 'Supprimer'}</button>{/if}
        </div>
        {#if mode === 'edit'}
          <textarea bind:this={editor} bind:value={current.body} {oninput} spellcheck="true" lang="fr" aria-label="Contenu de la note"></textarea>
        {:else}
          <!-- render() escapes the note before adding its own tags. -->
          <article class="md">{@html render(current.body)}</article>
        {/if}
      {/if}
      {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
    </div>
  </div>
</div>

<style>
  .notes {
    width: min(1080px, calc(100vw - 48px));
    height: min(720px, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
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
    align-items: center;
    gap: 10px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 8px 0 0;
    font-size: 1.3em;
  }
  .search {
    flex: 1;
  }
  .columns {
    display: grid;
    grid-template-columns: 260px 1fr;
    flex: 1;
    min-height: 0;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 8px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
  }
  .list button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    border: 0;
    background: transparent;
    padding: 8px 10px;
  }
  .list button.current {
    background: var(--selection);
  }
  .t {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  small,
  .empty,
  .state {
    color: var(--muted);
    font-size: 0.8em;
  }
  .empty {
    padding: 8px 10px;
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .state {
    flex: 1;
  }
  .tabs {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 2px;
  }
  .tabs button {
    border: 0;
    padding: 4px 12px;
  }
  .tabs button[aria-selected='true'] {
    background: var(--selection);
  }
  input,
  textarea,
  button {
    font: inherit;
    color: var(--fg);
  }
  input,
  textarea {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 10px;
  }
  textarea {
    flex: 1;
    resize: none;
    line-height: 1.55;
    padding: 12px;
  }
  button {
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 9px;
    padding: 6px 12px;
    cursor: pointer;
  }
  .danger,
  .bad {
    color: #e5484d;
  }
  .md {
    flex: 1;
    overflow-y: auto;
    line-height: 1.6;
    padding: 4px 8px;
  }
  .md :global(h1),
  .md :global(h2),
  .md :global(h3) {
    margin: 0.6em 0 0.3em;
    letter-spacing: -0.01em;
  }
  .md :global(code) {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.88em;
    background: color-mix(in srgb, var(--fg) 8%, transparent);
    padding: 1px 5px;
    border-radius: 5px;
  }
  .md :global(pre) {
    background: color-mix(in srgb, var(--fg) 6%, transparent);
    padding: 10px;
    border-radius: 8px;
    overflow-x: auto;
  }
  .md :global(pre code) {
    background: none;
    padding: 0;
  }
  .md :global(blockquote) {
    margin: 0.4em 0;
    padding-left: 12px;
    border-left: 3px solid var(--accent);
    color: var(--muted);
  }
  .md :global(a) {
    color: var(--accent);
  }
  .md :global(li.task) {
    list-style: '☐  ';
  }
  .md :global(li.task.done) {
    list-style: '☑  ';
    color: var(--muted);
    text-decoration: line-through;
  }
</style>
