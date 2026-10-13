<script>
  import { invoke } from '@tauri-apps/api/core';

  const EMPTY = { id: 0, name: '', keyword: '', body: '' };

  let list = $state([]);
  let draft = $state({ ...EMPTY });
  let message = $state('');
  let failure = $state('');
  let confirmDelete = $state(false);

  async function load() {
    list = await invoke('snippets_list');
  }

  $effect(() => {
    load().catch((e) => (failure = String(e)));
  });

  function edit(s) {
    draft = { ...s };
    message = '';
    failure = '';
    confirmDelete = false;
  }

  async function save() {
    message = '';
    failure = '';
    try {
      const id = await invoke('snippet_save', { snippet: draft });
      draft.id = id;
      await load();
      message = 'Extrait enregistré.';
    } catch (e) {
      failure = String(e);
    }
  }

  async function remove() {
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    try {
      await invoke('snippet_delete', { id: draft.id });
      await load();
      edit({ ...EMPTY });
      message = 'Extrait supprimé.';
    } catch (e) {
      failure = String(e);
    }
  }

  function insert(token) {
    draft.body = `${draft.body}${token}`;
  }
</script>

<div class="snippets">
  <nav aria-label="Extraits">
    <button class="new" onclick={() => edit({ ...EMPTY })}>Nouvel extrait</button>
    <ul>
      {#each list as s (s.id)}
        <li>
          <button class:current={s.id === draft.id} onclick={() => edit(s)}>
            <span>{s.name}</span>
            {#if s.keyword}<small>{s.keyword}</small>{/if}
          </button>
        </li>
      {:else}
        <li class="empty">Aucun extrait pour l'instant.</li>
      {/each}
    </ul>
  </nav>

  <div class="editor">
    <div class="row">
      <label>Nom <input bind:value={draft.name} placeholder="Relance devis" /></label>
      <label>Mot-clé (facultatif) <input bind:value={draft.keyword} placeholder="relance" spellcheck="false" /></label>
    </div>
    <label for="snippet-body">Texte</label>
    <textarea id="snippet-body" bind:value={draft.body} rows="10" placeholder={'Bonjour {champ:Prénom},\n\nSuite à notre échange du {date}…'}></textarea>
    <div class="tokens" aria-label="Variables">
      {#each ['{date}', '{heure}', '{date:%A %d %B}', '{presse-papiers}', '{curseur}', '{champ:Nom}'] as t}
        <button class="token" onclick={() => insert(t)}>{t}</button>
      {/each}
    </div>
    <p class="hint">
      <code>{'{champ:…}'}</code> est demandé au moment de l'insertion. <code>{'{curseur}'}</code> place le curseur à cet endroit après le collage.
      Dans la barre : Entrée colle l'extrait dans l'application d'où tu viens, Ctrl+K pour le copier seulement.
    </p>
    <div class="buttons">
      {#if message}<p class="ok" role="status">{message}</p>{/if}
      {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
      {#if draft.id}<button class="danger" onclick={remove}>{confirmDelete ? 'Confirmer la suppression' : 'Supprimer'}</button>{/if}
      <button class="primary" onclick={save} disabled={!draft.name.trim() || !draft.body}>Enregistrer</button>
    </div>
  </div>
</div>

<style>
  .snippets {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 16px;
    min-height: 100%;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    border: 0;
    background: transparent;
    padding: 8px 10px;
  }
  li button.current {
    background: var(--selection);
  }
  li.empty {
    color: var(--muted);
    font-size: 0.85em;
    padding: 8px 10px;
  }
  small {
    color: var(--muted);
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85em;
    color: var(--muted);
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
    padding: 6px 8px;
  }
  textarea {
    resize: vertical;
    line-height: 1.45;
  }
  button {
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 9px;
    padding: 6px 12px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .new {
    border-style: dashed;
  }
  .tokens {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .token {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.78em;
    padding: 3px 8px;
    border-radius: 999px;
    color: var(--accent);
  }
  .hint {
    margin: 0;
    font-size: 0.8em;
    color: var(--muted);
    line-height: 1.5;
  }
  code {
    color: var(--fg);
  }
  .buttons {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .buttons p {
    margin: 0;
    flex: 1;
    font-size: 0.85em;
  }
  button.primary {
    background: var(--accent);
    color: var(--bg);
    border-color: var(--accent);
    font-weight: 600;
  }
  button.danger {
    color: #e5484d;
  }
  .ok {
    color: var(--accent);
  }
  .bad {
    color: #e5484d;
  }
</style>
