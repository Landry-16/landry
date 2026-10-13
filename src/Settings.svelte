<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { styleString } from './lib/theme.js';
  import GeneralSettings from './GeneralSettings.svelte';
  import Accounts from './Accounts.svelte';

  let { active, systemDark, onclose } = $props();

  const MODULE_NAMES = {
    commands: 'Commandes Landry',
    apps: 'Applications',
    calc: 'Calcul, dates, montants',
    tools: 'Boîte à outils',
    projects: 'Projets git',
    files: 'Fichiers',
    system: 'Ports, SSH, Docker',
    cloud: 'GitHub et Vercel',
  };
  const COLOR_KEYS = [
    ['bg', 'Fond'],
    ['fg', 'Texte'],
    ['muted', 'Texte discret'],
    ['accent', 'Accent'],
    ['selection', 'Sélection'],
    ['border', 'Bordure'],
  ];

  let tab = $state('presets');
  let presets = $state([]);
  let selectedName = $state(untrack(() => active));
  let draft = $state(null);
  let original = $state('');
  let mode = $state('form');
  let jsonText = $state('');
  let jsonError = $state('');
  let message = $state('');
  let failure = $state('');
  // Initial values only: the editor keeps its own selection afterwards.
  let colorScheme = $state(untrack(() => (systemDark ? 'dark' : 'light')));
  let confirmDelete = $state(false);
  let importPath = $state('');
  let root = $state();

  const info = $derived(presets.find((p) => p.name === selectedName));
  const builtin = $derived(info?.builtin ?? false);
  const dirty = $derived(draft && JSON.stringify(draft) !== original);

  async function loadList() {
    presets = await invoke('preset_list');
  }

  async function select(name) {
    failure = '';
    message = '';
    confirmDelete = false;
    selectedName = name;
    const entry = presets.find((p) => p.name === name);
    if (entry?.error) {
      draft = null;
      failure = `Ce preset est invalide : ${entry.error}`;
      return;
    }
    try {
      draft = await invoke('preset_get', { name });
      original = JSON.stringify(draft);
      jsonText = JSON.stringify(draft, null, 2);
      jsonError = '';
    } catch (e) {
      draft = null;
      failure = String(e);
    }
  }

  $effect(() => {
    loadList().then(() => select(active));
    root?.focus();
  });

  async function run(fn, okMessage) {
    failure = '';
    message = '';
    try {
      const r = await fn();
      message = typeof okMessage === 'function' ? okMessage(r) : okMessage;
      return r;
    } catch (e) {
      failure = String(e);
    }
  }

  function uniqueName(base) {
    let name = `${base} (copie)`;
    for (let i = 2; presets.some((p) => p.name === name); i++) name = `${base} (copie ${i})`;
    return name;
  }

  async function save() {
    const previous = selectedName;
    const saved = await run(() => invoke('preset_save', { json: JSON.stringify(draft), previous }), 'Preset enregistré.');
    if (saved) {
      await loadList();
      await select(saved.name);
      message = 'Preset enregistré.';
    }
  }

  async function duplicate() {
    const p = await run(() => invoke('preset_duplicate', { name: selectedName, newName: uniqueName(selectedName) }), 'Copie créée.');
    if (p) {
      await loadList();
      await select(p.name);
    }
  }

  async function remove() {
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    const ok = await run(() => invoke('preset_delete', { name: selectedName }).then(() => true), 'Preset supprimé.');
    if (ok) {
      await loadList();
      await select('Défaut');
      message = 'Preset supprimé.';
    }
  }

  async function importFile() {
    const name = await run(() => invoke('preset_import', { path: importPath }), (n) => `Importé sous le nom « ${n} ».`);
    if (name) {
      importPath = '';
      await loadList();
      await select(name);
      message = `Importé sous le nom « ${name} ».`;
    }
  }

  async function onJsonInput() {
    try {
      const parsed = JSON.parse(jsonText);
      await invoke('preset_check', { json: jsonText });
      draft = parsed;
      jsonError = '';
    } catch (e) {
      jsonError = String(e);
    }
  }

  function switchMode(m) {
    if (m === 'json') jsonText = JSON.stringify(draft, null, 2);
    mode = m;
  }

  function moveModule(i, step) {
    const j = i + step;
    if (j < 0 || j >= draft.modules.length) return;
    const m = [...draft.modules];
    [m[i], m[j]] = [m[j], m[i]];
    draft.modules = m;
  }

  function onkeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<div class="settings" bind:this={root} tabindex="-1" {onkeydown} role="dialog" aria-modal="true" aria-label="Réglages Landry">
  <header>
    <h1>Réglages</h1>
    <div class="tabs main-tabs" role="tablist" aria-label="Sections">
      <button role="tab" aria-selected={tab === 'presets'} onclick={() => (tab = 'presets')}>Presets</button>
      <button role="tab" aria-selected={tab === 'general'} onclick={() => (tab = 'general')}>Général</button>
      <button role="tab" aria-selected={tab === 'accounts'} onclick={() => (tab = 'accounts')}>Comptes</button>
    </div>
    <button class="ghost close" onclick={onclose} aria-label="Fermer les réglages">Échap</button>
  </header>

  {#if tab === 'general'}
    <div class="single"><GeneralSettings /></div>
  {:else if tab === 'accounts'}
    <div class="single"><Accounts /></div>
  {:else}
  <div class="columns">
    <nav aria-label="Presets">
      <ul>
        {#each presets as p (p.name)}
          <li>
            <button class:current={p.name === selectedName} onclick={() => select(p.name)}>
              <span class="dot" class:on={p.name === active} title={p.name === active ? 'Preset actif' : ''}></span>
              <span class="pname">{p.name}</span>
              {#if p.builtin}<small>intégré</small>{/if}
              {#if p.error}<small class="bad">invalide</small>{/if}
            </button>
          </li>
        {/each}
      </ul>
      <div class="import">
        <label for="import-path">Importer un fichier</label>
        <input id="import-path" bind:value={importPath} placeholder="~/Téléchargements/preset.json" />
        <button onclick={importFile} disabled={!importPath.trim()}>Importer</button>
      </div>
    </nav>

    <div class="editor">
      {#if draft}
        <div class="preview-frame" style={styleString(draft, colorScheme === 'dark')} aria-hidden="true">
          <div class="mock">
            <div class="mock-field"><span class="mock-glyph">L</span>Que veux-tu faire ?</div>
            {#each ['Visual Studio Code', 'Terminal', 'Fichiers'].slice(0, Math.min(3, draft.max_results)) as t, i}
              <div class="mock-row" class:sel={i === 0}><span class="mock-letter">{t[0]}</span>{t}</div>
            {/each}
          </div>
        </div>

        <div class="toolbar">
          <div class="tabs" role="tablist">
            <button role="tab" aria-selected={mode === 'form'} onclick={() => switchMode('form')}>Formulaire</button>
            <button role="tab" aria-selected={mode === 'json'} onclick={() => switchMode('json')}>JSON</button>
          </div>
          <span class="spacer"></span>
          {#if dirty}<span class="dirty">Non enregistré</span>{/if}
          <button onclick={() => run(() => invoke('preset_use', { name: selectedName }), 'Preset activé.')} disabled={selectedName === active || dirty}>Utiliser</button>
          <button onclick={duplicate}>Dupliquer</button>
          {#if !builtin}
            <button onclick={() => run(() => invoke('preset_export', { name: selectedName }), (p) => `Exporté dans ${p}`)}>Exporter</button>
            <button class="danger" onclick={remove}>{confirmDelete ? 'Confirmer la suppression' : 'Supprimer'}</button>
            <button class="primary" onclick={save} disabled={!dirty || !!jsonError}>Enregistrer</button>
          {/if}
        </div>

        {#if builtin}<p class="hint">Le preset Défaut est intégré : duplique-le pour le modifier.</p>{/if}

        {#if mode === 'json'}
          <textarea bind:value={jsonText} oninput={onJsonInput} spellcheck="false" aria-label="Preset en JSON" readonly={builtin}></textarea>
          {#if jsonError}<p class="bad">{jsonError}</p>{/if}
        {:else}
          <fieldset class="form" disabled={builtin}>
            <div class="group">
              <h2>Général</h2>
              <label>Nom <input bind:value={draft.name} maxlength="40" /></label>
              <label>Thème
                <select bind:value={draft.theme}>
                  <option value="system">Suivre le système</option>
                  <option value="light">Clair</option>
                  <option value="dark">Sombre</option>
                </select>
              </label>
              <label>Police <input bind:value={draft.font.family} /></label>
              <label>Taille du texte <input type="number" min="8" max="48" bind:value={draft.font.size} /></label>
              <label>Densité
                <select bind:value={draft.density}>
                  <option value="compact">Compacte</option>
                  <option value="comfortable">Confortable</option>
                  <option value="spacious">Aérée</option>
                </select>
              </label>
              <label>Opacité <input type="range" min="0.2" max="1" step="0.01" bind:value={draft.opacity} /></label>
              <label>Animation (ms, 0 = aucune) <input type="number" min="0" max="2000" bind:value={draft.animation_ms} /></label>
            </div>

            <div class="group">
              <h2>Disposition</h2>
              <label>Couche
                <select bind:value={draft.layout.layer}>
                  <option value="screen">Écran (position libre)</option>
                  <option value="window">Fenêtre (plus rapide)</option>
                </select>
              </label>
              {#if draft.layout.layer === 'window'}
                <p class="hint">GNOME place la fenêtre lui-même : ancrage, marge et décalages sont ignorés. Ouverture plus rapide (environ 35 ms contre 80 ms).</p>
              {/if}
              <label>Ancrage
                <select bind:value={draft.layout.anchor} disabled={draft.layout.layer === 'window'}>
                  <option value="top">En haut</option>
                  <option value="center">Au centre</option>
                  <option value="bottom">En bas</option>
                </select>
              </label>
              <label>Largeur <input type="number" min="280" max="4000" bind:value={draft.layout.width} /></label>
              <label>Hauteur max <input type="number" min="60" max="3000" bind:value={draft.layout.max_height} /></label>
              <label>Décalage horizontal <input type="number" bind:value={draft.layout.offset_x} disabled={draft.layout.layer === 'window'} /></label>
              <label>Décalage vertical <input type="number" bind:value={draft.layout.offset_y} disabled={draft.layout.layer === 'window'} /></label>
              <label>Marge au bord <input type="number" min="0" max="2000" bind:value={draft.layout.margin} disabled={draft.layout.layer === 'window'} /></label>
              <label>Arrondi <input type="number" min="0" max="64" bind:value={draft.layout.radius} /></label>
            </div>

            <div class="group">
              <h2>Résultats</h2>
              <label>Nombre de résultats <input type="number" min="1" max="50" bind:value={draft.max_results} /></label>
              <label class="check"><input type="checkbox" bind:checked={draft.preview.visible} /> Panneau d'aperçu</label>
              <label>Côté de l'aperçu
                <select bind:value={draft.preview.side}>
                  <option value="right">À droite</option>
                  <option value="left">À gauche</option>
                </select>
              </label>
              <label>Largeur de l'aperçu (%) <input type="number" min="20" max="70" bind:value={draft.preview.width} /></label>
            </div>

            <div class="group">
              <h2>Couleurs</h2>
              <div class="tabs small" role="tablist">
                <button role="tab" aria-selected={colorScheme === 'light'} onclick={() => (colorScheme = 'light')}>Clair</button>
                <button role="tab" aria-selected={colorScheme === 'dark'} onclick={() => (colorScheme = 'dark')}>Sombre</button>
              </div>
              {#each COLOR_KEYS as [key, label]}
                <label class="color">{label}
                  <span>
                    <input type="color" bind:value={draft[colorScheme][key]} aria-label={`${label}, sélecteur`} />
                    <input bind:value={draft[colorScheme][key]} maxlength="9" aria-label={`${label}, valeur`} />
                  </span>
                </label>
              {/each}
            </div>

            <div class="group">
              <h2>Modules</h2>
              <ol class="modules">
                {#each draft.modules as m, i (m.id)}
                  <li>
                    <label class="check"><input type="checkbox" bind:checked={m.enabled} /> {MODULE_NAMES[m.id] ?? m.id}</label>
                    <button class="ghost" onclick={() => moveModule(i, -1)} disabled={i === 0} aria-label="Monter">↑</button>
                    <button class="ghost" onclick={() => moveModule(i, 1)} disabled={i === draft.modules.length - 1} aria-label="Descendre">↓</button>
                  </li>
                {/each}
              </ol>
            </div>
          </fieldset>
        {/if}
      {/if}
      {#if message}<p class="ok" role="status">{message}</p>{/if}
      {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
    </div>
  </div>
  {/if}
</div>

<style>
  .settings {
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
    display: grid;
    grid-template-columns: 1fr auto;
    padding: 20px 24px 14px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(120deg, color-mix(in srgb, var(--accent) 16%, transparent), transparent 60%);
  }
  h1 {
    margin: 0;
    font-size: 1.5em;
    letter-spacing: -0.02em;
  }
  .main-tabs {
    grid-column: 1;
    justify-self: start;
    margin-top: 10px;
  }
  .close {
    grid-row: 1 / 3;
    grid-column: 2;
    align-self: center;
  }
  .single {
    padding: 16px 20px;
    overflow-y: auto;
    flex: 1;
  }
  .columns {
    display: grid;
    grid-template-columns: 240px 1fr;
    min-height: 0;
    flex: 1;
  }
  nav {
    border-right: 1px solid var(--border);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
  }
  nav ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  nav li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    border: 0;
    background: transparent;
    padding: 8px 10px;
  }
  nav li button.current {
    background: var(--selection);
  }
  .pname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1px solid var(--muted);
    flex: none;
  }
  .dot.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  small {
    color: var(--muted);
    font-size: 0.72em;
  }
  .import {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 0.85em;
  }
  .editor {
    padding: 16px 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .preview-frame {
    border-radius: 14px;
    padding: 18px;
    background:
      radial-gradient(circle at 20% 20%, color-mix(in srgb, var(--accent) 30%, transparent), transparent 55%),
      repeating-linear-gradient(45deg, rgb(127 127 127 / 0.12) 0 10px, transparent 10px 20px);
    display: grid;
    place-items: center;
    font-family: var(--font);
    font-size: var(--font-size);
  }
  .mock {
    width: min(100%, calc(var(--width) * 0.6));
    color: var(--fg);
    background: color-mix(in srgb, var(--bg) var(--opacity), transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--pad);
    box-shadow: 0 14px 40px rgb(0 0 0 / 0.3);
  }
  .mock-field {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    padding: 6px 6px 10px;
    border-bottom: 1px solid var(--border);
    margin-bottom: var(--pad);
  }
  .mock-glyph,
  .mock-letter {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    font-weight: 700;
    font-size: 0.8em;
  }
  .mock-glyph {
    background: var(--accent);
    color: var(--bg);
  }
  .mock-letter {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .mock-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: calc(var(--row) * 0.8);
    padding: 0 8px;
    margin-bottom: var(--gap);
    border-radius: calc(var(--radius) * 0.6);
  }
  .mock-row.sel {
    background: var(--selection);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .spacer {
    flex: 1;
  }
  .dirty {
    font-size: 0.8em;
    color: var(--accent);
  }
  .tabs {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 2px;
  }
  .tabs button {
    border: 0;
    background: transparent;
    padding: 5px 12px;
  }
  .tabs button[aria-selected='true'] {
    background: var(--selection);
  }
  .tabs.small {
    margin-bottom: 4px;
  }

  button,
  input,
  select,
  textarea {
    font: inherit;
    color: inherit;
  }
  button {
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--fg) 4%, transparent);
    border-radius: 9px;
    padding: 6px 12px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
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
  button.ghost {
    background: transparent;
  }
  input,
  select,
  textarea {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 5px 8px;
    min-width: 0;
  }
  select option {
    background: var(--bg);
  }
  textarea {
    min-height: 320px;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.85em;
    resize: vertical;
  }

  .form {
    border: 0;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 14px;
  }
  .form:disabled {
    opacity: 0.7;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  .group h2 {
    margin: 0 0 4px;
    font-size: 0.8em;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
  }
  .group label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    font-size: 0.88em;
  }
  .group label > input:not([type='checkbox']),
  .group label > select {
    width: 52%;
    box-sizing: border-box;
  }
  .group label.check {
    justify-content: flex-start;
  }
  .color span {
    display: flex;
    gap: 6px;
    width: 52%;
  }
  .color input[type='color'] {
    width: 34px;
    padding: 0 2px;
    flex: none;
  }
  .color input:not([type='color']) {
    flex: 1;
    width: 0;
  }
  .modules {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .modules li {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .modules li label {
    flex: 1;
  }
  .hint,
  .ok,
  .bad {
    margin: 0;
    font-size: 0.85em;
  }
  .hint {
    color: var(--muted);
  }
  .ok {
    color: var(--accent);
  }
  .bad {
    color: #e5484d;
  }
</style>
