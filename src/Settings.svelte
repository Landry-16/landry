<script>
  import { t } from './lib/i18n.svelte.js';
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { styleString } from './lib/theme.js';
  import { contrast } from './lib/contrast.js';
  import GeneralSettings from './GeneralSettings.svelte';
  import Accounts from './Accounts.svelte';
  import Snippets from './Snippets.svelte';

  let { active, systemDark, onclose, initialTab = 'presets' } = $props();

  const MODULE_NAMES = {
    commands: 'Commandes Landry',
    apps: 'Applications',
    calc: 'Calcul, dates, montants',
    snippets: 'Extraits',
    text: 'Transformations de texte',
    notes: 'Notes',
    tasks: 'Tâches et rappels',
    timers: 'Minuteurs, Pomodoro',
    tracking: 'Suivi du temps',
    web: 'Web, favoris, traduction',
    scripts: 'Scripts',
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

  let tab = $state(untrack(() => initialTab));
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
  // Text pairs that must stay readable (WCAG AA: 4.5:1).
  const checks = $derived(
    draft
      ? [
          ['Texte', 'fg', 'bg'],
          ['Texte discret', 'muted', 'bg'],
          ['Texte discret sur la sélection', 'muted', 'selection'],
          ['Accent', 'accent', 'bg'],
        ].map(([label, a, b]) => ({ label, ratio: contrast(draft[colorScheme][a], draft[colorScheme][b]) }))
      : [],
  );

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
      failure = t('Ce preset est invalide : {e}', { e: entry.error });
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
    let name = t('{base} (copie)', { base });
    for (let i = 2; presets.some((p) => p.name === name); i++) name = t('{base} (copie {i})', { base, i });
    return name;
  }

  async function save() {
    const previous = selectedName;
    const saved = await run(() => invoke('preset_save', { json: JSON.stringify(draft), previous }), t('Preset enregistré.'));
    if (saved) {
      await loadList();
      await select(saved.name);
      message = t('Preset enregistré.');
    }
  }

  async function duplicate() {
    const p = await run(() => invoke('preset_duplicate', { name: selectedName, newName: uniqueName(selectedName) }), t('Copie créée.'));
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
    const ok = await run(() => invoke('preset_delete', { name: selectedName }).then(() => true), t('Preset supprimé.'));
    if (ok) {
      await loadList();
      await select('Défaut');
      message = t('Preset supprimé.');
    }
  }

  async function importFile() {
    const name = await run(() => invoke('preset_import', { path: importPath }), (n) => t('Importé sous le nom « {n} ».', { n }));
    if (name) {
      importPath = '';
      await loadList();
      await select(name);
      message = t('Importé sous le nom « {n} ».', { n: name });
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

<div class="settings" bind:this={root} tabindex="-1" {onkeydown} role="dialog" aria-modal="true" aria-label={t('Réglages Landry')}>
  <header>
    <h1>{t('Réglages')}</h1>
    <div class="tabs main-tabs" role="tablist" aria-label={t('Sections')}>
      <button role="tab" aria-selected={tab === 'presets'} onclick={() => (tab = 'presets')}>{t('Presets')}</button>
      <button role="tab" aria-selected={tab === 'snippets'} onclick={() => (tab = 'snippets')}>{t('Extraits')}</button>
      <button role="tab" aria-selected={tab === 'general'} onclick={() => (tab = 'general')}>{t('Général')}</button>
      <button role="tab" aria-selected={tab === 'accounts'} onclick={() => (tab = 'accounts')}>{t('Comptes')}</button>
    </div>
    <button class="ghost close" onclick={onclose} aria-label={t('Fermer les réglages')}>{t('Échap')}</button>
  </header>

  {#if tab === 'snippets'}
    <div class="single"><Snippets /></div>
  {:else if tab === 'general'}
    <div class="single"><GeneralSettings /></div>
  {:else if tab === 'accounts'}
    <div class="single"><Accounts /></div>
  {:else}
  <div class="columns">
    <nav aria-label={t('Presets')}>
      <ul>
        {#each presets as p (p.name)}
          <li>
            <button class:current={p.name === selectedName} onclick={() => select(p.name)}>
              <span class="dot" class:on={p.name === active} title={p.name === active ? t('Preset actif') : ''}></span>
              <span class="pname">{p.name}</span>
              {#if p.builtin}<small>{t('intégré')}</small>{/if}
              {#if p.error}<small class="bad">{t('invalide')}</small>{/if}
            </button>
          </li>
        {/each}
      </ul>
      <div class="import">
        <label for="import-path">{t('Importer un fichier')}</label>
        <input id="import-path" bind:value={importPath} placeholder="~/Téléchargements/preset.json" />
        <button onclick={importFile} disabled={!importPath.trim()}>{t('Importer')}</button>
      </div>
    </nav>

    <div class="editor">
      {#if draft}
        <div class="preview-frame" style={styleString(draft, colorScheme === 'dark')} aria-hidden="true">
          <div class="mock">
            <div class="mock-field"><span class="mock-glyph">L</span>{t('Que veux-tu faire ?')}</div>
            {#each ['Visual Studio Code', 'Terminal', 'Fichiers'].slice(0, Math.min(3, draft.max_results)) as t, i}
              <div class="mock-row" class:sel={i === 0}><span class="mock-letter">{t[0]}</span>{t}</div>
            {/each}
          </div>
        </div>

        <div class="toolbar">
          <div class="tabs" role="tablist">
            <button role="tab" aria-selected={mode === 'form'} onclick={() => switchMode('form')}>{t('Formulaire')}</button>
            <button role="tab" aria-selected={mode === 'json'} onclick={() => switchMode('json')}>{t('JSON')}</button>
          </div>
          <span class="spacer"></span>
          {#if dirty}<span class="dirty">{t('Non enregistré')}</span>{/if}
          <button onclick={() => run(() => invoke('preset_use', { name: selectedName }), t('Preset activé.'))} disabled={selectedName === active || dirty}>{t('Utiliser')}</button>
          <button onclick={duplicate}>{t('Dupliquer')}</button>
          {#if !builtin}
            <button onclick={() => run(() => invoke('preset_export', { name: selectedName }), (p) => t('Exporté dans {p}', { p }))}>{t('Exporter')}</button>
            <button class="danger" onclick={remove}>{confirmDelete ? t('Confirmer la suppression') : t('Supprimer')}</button>
            <button class="primary" onclick={save} disabled={!dirty || !!jsonError}>{t('Enregistrer')}</button>
          {/if}
        </div>

        {#if builtin}<p class="hint">{t('Le preset Défaut est intégré : duplique-le pour le modifier.')}</p>{/if}

        {#if mode === 'json'}
          <textarea bind:value={jsonText} oninput={onJsonInput} spellcheck="false" aria-label={t('Preset en JSON')} readonly={builtin}></textarea>
          {#if jsonError}<p class="bad">{jsonError}</p>{/if}
        {:else}
          <fieldset class="form" disabled={builtin}>
            <div class="group">
              <h2>{t('Général')}</h2>
              <label>{t('Nom')} <input bind:value={draft.name} maxlength="40" /></label>
              <label>{t('Thème')}
                <select bind:value={draft.theme}>
                  <option value="system">{t('Suivre le système')}</option>
                  <option value="light">{t('Clair')}</option>
                  <option value="dark">{t('Sombre')}</option>
                </select>
              </label>
              <label>{t('Police')} <input bind:value={draft.font.family} /></label>
              <label>{t('Taille du texte')} <input type="number" min="8" max="48" bind:value={draft.font.size} /></label>
              <label>{t('Densité')}
                <select bind:value={draft.density}>
                  <option value="compact">{t('Compacte')}</option>
                  <option value="comfortable">{t('Confortable')}</option>
                  <option value="spacious">{t('Aérée')}</option>
                </select>
              </label>
              <label>{t('Opacité')} <input type="range" min="0.2" max="1" step="0.01" bind:value={draft.opacity} /></label>
              <label>{t('Animation (ms, 0 = aucune)')} <input type="number" min="0" max="2000" bind:value={draft.animation_ms} /></label>
            </div>

            <div class="group">
              <h2>{t('Disposition')}</h2>
              <label>{t('Couche')}
                <select bind:value={draft.layout.layer}>
                  <option value="screen">{t('Écran (position libre)')}</option>
                  <option value="window">{t('Fenêtre (plus rapide)')}</option>
                </select>
              </label>
              {#if draft.layout.layer === 'window'}
                <p class="hint">{t('GNOME place la fenêtre lui-même : ancrage, marge et décalages sont ignorés. Ouverture plus rapide (environ 35 ms contre 80 ms).')}</p>
              {/if}
              <label>{t('Ancrage')}
                <select bind:value={draft.layout.anchor} disabled={draft.layout.layer === 'window'}>
                  <option value="top">{t('En haut')}</option>
                  <option value="center">{t('Au centre')}</option>
                  <option value="bottom">{t('En bas')}</option>
                </select>
              </label>
              <label>{t('Largeur')} <input type="number" min="280" max="4000" bind:value={draft.layout.width} /></label>
              <label>{t('Hauteur max')} <input type="number" min="60" max="3000" bind:value={draft.layout.max_height} /></label>
              <label>{t('Décalage horizontal')} <input type="number" bind:value={draft.layout.offset_x} disabled={draft.layout.layer === 'window'} /></label>
              <label>{t('Décalage vertical')} <input type="number" bind:value={draft.layout.offset_y} disabled={draft.layout.layer === 'window'} /></label>
              <label>{t('Marge au bord')} <input type="number" min="0" max="2000" bind:value={draft.layout.margin} disabled={draft.layout.layer === 'window'} /></label>
              <label>{t('Arrondi')} <input type="number" min="0" max="64" bind:value={draft.layout.radius} /></label>
            </div>

            <div class="group">
              <h2>{t('Résultats')}</h2>
              <label>{t('Nombre de résultats')} <input type="number" min="1" max="50" bind:value={draft.max_results} /></label>
              <label class="check"><input type="checkbox" bind:checked={draft.preview.visible} /> {t("Panneau d'aperçu")}</label>
              <label>{t("Côté de l'aperçu")}
                <select bind:value={draft.preview.side}>
                  <option value="right">{t('À droite')}</option>
                  <option value="left">{t('À gauche')}</option>
                </select>
              </label>
              <label>{t("Largeur de l'aperçu (%)")} <input type="number" min="20" max="70" bind:value={draft.preview.width} /></label>
            </div>

            <div class="group">
              <h2>{t('Couleurs')}</h2>
              <div class="tabs small" role="tablist">
                <button role="tab" aria-selected={colorScheme === 'light'} onclick={() => (colorScheme = 'light')}>{t('Clair')}</button>
                <button role="tab" aria-selected={colorScheme === 'dark'} onclick={() => (colorScheme = 'dark')}>{t('Sombre')}</button>
              </div>
              <ul class="contrast" aria-label={t('Contrastes')}>
                {#each checks as c}
                  <li class:low={c.ratio !== null && c.ratio < 4.5}>
                    {t(c.label)} : {c.ratio === null ? '?' : `${c.ratio.toFixed(2)}:1`}{c.ratio !== null && c.ratio < 4.5 ? ` · ${t('peu lisible')}` : ''}
                  </li>
                {/each}
              </ul>
              {#each COLOR_KEYS as [key, label]}
                <label class="color">{t(label)}
                  <span>
                    <input type="color" bind:value={draft[colorScheme][key]} aria-label={t('{label}, sélecteur', { label: t(label) })} />
                    <input bind:value={draft[colorScheme][key]} maxlength="9" aria-label={t('{label}, valeur', { label: t(label) })} />
                  </span>
                </label>
              {/each}
            </div>

            <div class="group">
              <h2>{t('Modules')}</h2>
              <ol class="modules">
                {#each draft.modules as m, i (m.id)}
                  <li>
                    <label class="check"><input type="checkbox" bind:checked={m.enabled} /> {MODULE_NAMES[m.id] ? t(MODULE_NAMES[m.id]) : m.id}</label>
                    <button class="ghost" onclick={() => moveModule(i, -1)} disabled={i === 0} aria-label={t('Monter')}>↑</button>
                    <button class="ghost" onclick={() => moveModule(i, 1)} disabled={i === draft.modules.length - 1} aria-label={t('Descendre')}>↓</button>
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
  .contrast {
    margin: 0 0 4px;
    padding: 0;
    list-style: none;
    font-size: 0.78em;
    color: var(--muted);
  }
  .contrast .low {
    color: #e5484d;
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
