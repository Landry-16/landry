<script>
  import { tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { applyToRoot, styleOf } from './lib/theme.js';
  import Companion from './Companion.svelte';
  import Ornaments from './Ornaments.svelte';
  import Particles from './Particles.svelte';
  import Help from './Help.svelte';
  import Setup from './Setup.svelte';
  import Settings from './Settings.svelte';
  import { i18n, t } from './lib/i18n.svelte.js';
  import Diff from './Diff.svelte';
  import Notes from './Notes.svelte';

  const MODULE_LABELS = {
    apps: 'Application',
    commands: 'Commande',
    tools: 'Outil',
    calc: 'Calcul',
    snippets: 'Extrait',
    notes: 'Note',
    tasks: 'Tâche',
    timers: 'Minuteur',
    tracking: 'Suivi',
    web: 'Web',
    scripts: 'Script',
    text: 'Texte',
    projects: 'Projet',
    files: 'Fichier',
    system: 'Système',
    cloud: 'En ligne',
    slash: 'Commande',
    windows: 'Fenêtre',
    clipboard: 'Presse-papiers',
    emoji: 'Caractère',
    dict: 'Dictionnaire',
  };
  const moduleLabel = (m) => (MODULE_LABELS[m] ? t(MODULE_LABELS[m]) : m);

  let preset = $state(null);
  let systemDark = $state(false);
  let warning = $state('');
  let error = $state('');
  let message = $state('');
  let history = $state([]);
  let query = $state('');
  let results = $state([]);
  let selected = $state(0);
  let view = $state('bar');
  let settingsTab = $state('presets');
  let noteId = $state(0);
  let shownCount = $state(0);
  let input = $state();
  let list = $state();
  let promptInput = $state();
  // Ctrl+K panel: { item, actions, sel, confirm }
  let panel = $state(null);
  // Argument prompt: { item, action, value } or, for snippet fields, { item, action, fields, values }
  let prompt = $state(null);
  let previewData = $state(null);
  let histPos = -1;
  let searchSeq = 0;
  let lastQuery = null;
  let previewSeq = 0;

  const current = $derived(results[selected]);
  const style = $derived(preset ? styleOf(preset) : null);
  let help = $state(false);
  // Id of the item whose dangerous Enter action waits for a second Enter.
  let armed = $state('');

  // Companion mood. Short reactions (arrive, joy, error) win for a moment, then the bar's state decides.
  let clock = $state(0);
  let flash = $state(null);
  let typingAt = 0;
  let pointAt = 0;
  let asleep = $state(false);
  let sleepTimer;
  const tick_ = (ms) => {
    clock = performance.now();
    setTimeout(() => (clock = performance.now()), ms + 20);
  };
  function react(mood, ms) {
    flash = { mood, until: performance.now() + ms };
    tick_(ms);
  }
  function wake() {
    asleep = false;
    clearTimeout(sleepTimer);
    sleepTimer = setTimeout(() => (asleep = true), 30000);
  }
  const mood = $derived.by(() => {
    if (flash && flash.until > clock) return flash.mood;
    if (asleep) return 'sleep';
    if (query && !results.length && !prompt) return 'lost';
    if (clock - pointAt < 1400) return 'point';
    if (clock - typingAt < 1400) return 'listen';
    return 'idle';
  });

  // Read by screen readers after each search; the selected row is read through aria-activedescendant.
  const announce = $derived(
    !query ? '' : results.length === 0 ? t('Aucun résultat') : results.length === 1 ? t('1 résultat') : t('{n} résultats', { n: results.length }),
  );

  // Ctrl+plus / Ctrl+minus: text size for this viewer, kept across sessions.
  let zoom = $state(1);
  try {
    zoom = Number(localStorage.getItem('landry.zoom')) || 1;
  } catch {}
  $effect(() => {
    document.documentElement.style.setProperty('--zoom', String(zoom));
    try {
      localStorage.setItem('landry.zoom', String(zoom));
    } catch {}
  });

  async function refresh() {
    const s = await invoke('ui_state');
    preset = s.preset;
    systemDark = s.system_dark;
    warning = s.warning ?? '';
    history = s.history;
    i18n.locale = s.locale;
    // First launch: the assistant opens by itself until it is closed once.
    firstRun = s.first_run;
    if (firstRun && view === 'bar') view = 'setup';
  }
  let firstRun = false;

  async function runSearch(q) {
    const seq = ++searchSeq;
    const r = await invoke('search', { query: q });
    if (seq === searchSeq) {
      // Keep the selected item when the same list refreshes (live timers).
      const keep = results[selected]?.id;
      const i = keep ? r.findIndex((x) => x.id === keep) : -1;
      results = r;
      selected = q === lastQuery && i >= 0 ? i : 0;
      lastQuery = q;
    }
  }

  $effect(() => {
    if (preset) applyToRoot(preset, systemDark);
  });

  $effect(() => {
    // Re-run on every query change, and when the preset changes (module list, max results).
    preset;
    runSearch(query);
  });

  $effect(() => {
    invoke('settings_layer', { on: view !== 'bar' });
  });

  $effect(() => {
    // querySelector is not reactive: read `selected` so the effect follows the arrow keys.
    selected;
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  });

  $effect(() => {
    // Rich preview (file content, project state...) for the selected item.
    const id = current?.id;
    const seq = ++previewSeq;
    previewData = null;
    if (!id || !preset?.preview.visible) return;
    invoke('preview', { id }).then((p) => {
      if (seq === previewSeq) previewData = p;
    });
  });

  function resetBar() {
    view = firstRun ? 'setup' : 'bar';
    help = false;
    query = '';
    error = '';
    message = '';
    panel = null;
    prompt = null;
    histPos = -1;
  }

  function focusInput() {
    tick().then(() => input?.focus());
  }

  $effect(() => {
    refresh();
    const subs = [
      listen('landry://shown', () => {
        resetBar();
        shownCount++;
        react('arrive', 360);
        wake();
        // The keyed block re-creates the input: focus the new one once rendered.
        focusInput();
        // Two frames: the first is queued with the show, the second proves it was painted.
        requestAnimationFrame(() => requestAnimationFrame(() => invoke('painted')));
        refresh();
        runSearch('');
      }),
      listen('landry://preset', (e) => {
        preset = e.payload.preset;
        warning = e.payload.warning ?? '';
      }),
      listen('landry://theme', (e) => (systemDark = e.payload)),
      listen('landry://open-settings', () => openView('settings')),
      listen('landry://fill', (e) => {
        query = e.payload;
        focusInput();
      }),
      listen('landry://view', (e) => openView(e.payload)),
      listen('landry://notice', (e) => (message = e.payload)),
      // Background data (GitHub, Vercel) arrived: search again.
      listen('landry://refresh', () => view === 'bar' && runSearch(query)),
    ];
    return () => subs.forEach((p) => p.then((f) => f()));
  });

  async function activate(item, action = '', arg = '') {
    if (!item) return;
    error = '';
    message = '';
    panel = null;
    prompt = null;
    try {
      const out = await invoke('activate', { id: item.id, action, arg, query });
      if (out.kind === 'message' || out.kind === 'keep') react('joy', 900);
      switch (out.kind) {
        case 'hide':
        case 'copy':
        case 'paste':
          query = '';
          break;
        case 'ask':
          prompt = { item, action: { id: action, title: t('Compléter') }, fields: out.text, values: Object.fromEntries(out.text.map((f) => [f, ''])) };
          tick().then(() => promptInput?.focus());
          return;
        case 'settings':
          openView('settings');
          break;
        case 'fill':
          query = out.text;
          break;
        case 'view':
          if (out.text === 'help') help = true;
          else openView(out.text);
          break;
        case 'message':
          message = out.text;
          runSearch(query);
          break;
        case 'keep':
          runSearch(query);
          break;
        case 'run':
          if (out.text.silent) query = '';
          else message = t('Exécution : {label}…', { label: out.text.label });
          break;
      }
    } catch (e) {
      error = String(e);
      react('error', 1400);
    }
    focusInput();
  }

  async function openPanel() {
    if (!current) return;
    const actions = await invoke('item_actions', { id: current.id });
    if (actions.length) panel = { item: current, actions, sel: 0, confirm: false };
  }

  function choose(action) {
    const item = panel.item;
    if (action.arg) {
      prompt = { item, action, value: action.value ?? '' };
      panel = null;
      tick().then(() => {
        promptInput?.focus();
        promptInput?.select();
      });
    } else if (action.danger && !panel.confirm) {
      // Destructive actions need a second Enter.
      panel.confirm = true;
    } else {
      activate(item, action.id);
    }
  }

  function recall(step) {
    histPos = Math.max(-1, Math.min(histPos + step, history.length - 1));
    query = histPos < 0 ? '' : history[histPos];
  }

  function panelKeys(e) {
    const n = panel.actions.length;
    if (e.key === 'ArrowDown') panel.sel = (panel.sel + 1) % n;
    else if (e.key === 'ArrowUp') panel.sel = (panel.sel - 1 + n) % n;
    else if (e.key === 'Enter') choose(panel.actions[panel.sel]);
    else if (e.key === 'Escape' || (e.key === 'k' && e.ctrlKey)) panel = null;
    else return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') panel.confirm = false;
    e.preventDefault();
  }

  function onkeydown(e) {
    wake();
    if (e.key === 'F1') {
      e.preventDefault();
      help = !help;
      return;
    }
    if (e.ctrlKey && ['+', '=', '-', '0'].includes(e.key)) {
      e.preventDefault();
      zoom = e.key === '0' ? 1 : Math.min(1.6, Math.max(0.8, Math.round((zoom + (e.key === '-' ? -0.1 : 0.1)) * 10) / 10));
      return;
    }
    if (help && e.key === 'Escape') {
      e.preventDefault();
      help = false;
      return;
    }
    if (panel) return panelKeys(e);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      pointAt = performance.now();
      tick_(1400);
    }
    const browsing = histPos >= 0;
    if (e.key === 'k' && e.ctrlKey) {
      e.preventDefault();
      openPanel();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (browsing) recall(-1);
      else selected = Math.min(selected + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if ((query === '' || browsing) && history.length) recall(1);
      else selected = Math.max(selected - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (current?.actions?.[0]?.danger && armed !== current.id) {
        armed = current.id;
        message = t('Entrée pour confirmer : {a}', { a: current.actions[0].title });
      } else {
        armed = '';
        activate(current);
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      // A command of the / palette completes itself; anything else completes with its title.
      if (current?.id.startsWith('slash:cmd:')) activate(current);
      else if (current) query = current.title;
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (query) query = '';
      else invoke('hide');
    } else if (e.key === ',' && e.ctrlKey) {
      e.preventDefault();
      openView('settings');
    }
  }

  function promptKeys(e) {
    if (e.key === 'Enter' && prompt.fields) {
      // Enter moves to the next field, and submits on the last one.
      e.preventDefault();
      const inputs = [...e.currentTarget.closest('.prompt').querySelectorAll('input')];
      const i = inputs.indexOf(e.target);
      if (i < inputs.length - 1) inputs[i + 1].focus();
      else activate(prompt.item, prompt.action.id, JSON.stringify(prompt.values));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      activate(prompt.item, prompt.action.id, prompt.value);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      prompt = null;
      focusInput();
    }
  }

  /** "settings:snippets" opens the settings on that tab, "notes:12" opens note 12. */
  function openView(name) {
    const [v, arg] = name.split(':');
    if (v === 'settings') settingsTab = arg ?? 'presets';
    if (v === 'notes') noteId = Number(arg) || 0;
    view = v;
  }

  function backToBar() {
    view = 'bar';
    query = '';
    focusInput();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" class:centered={view !== 'bar'} onmousedown={(e) => e.target === e.currentTarget && invoke('hide')}>
  {#if preset && view === 'settings'}
    <Settings active={preset.name} {systemDark} onclose={backToBar} initialTab={settingsTab} />
  {:else if preset && view === 'setup'}
    <Setup
      active={preset.name}
      onclose={() => {
        firstRun = false;
        backToBar();
      }}
    />
  {:else if preset && view === 'diff'}
    <Diff onclose={backToBar} />
  {:else if preset && view === 'notes'}
    <Notes onclose={backToBar} {noteId} />
  {:else if preset}
    {#key shownCount}
      <div class="frame">
      <Ornaments ornament={style.ornament} />
      <section class="bar" aria-label="Landry">
        <Particles count={style.particles} ornament={style.ornament} />
        <div class="field">
          {#if style.companion}
            <Companion {mood} />
          {:else}
            <span class="glyph" aria-hidden="true">L</span>
          {/if}
          <input
            bind:this={input}
            bind:value={query}
            oninput={() => {
              histPos = -1;
              armed = '';
              help = false;
              typingAt = performance.now();
              tick_(1400);
            }}
            {onkeydown}
            placeholder={t('Que veux-tu faire ?')}
            aria-label={t('Recherche')}
            aria-controls="results"
            aria-activedescendant={current ? `r-${selected}` : undefined}
            spellcheck="false"
            autocomplete="off"
          />
          <span class="preset-chip" title={t('Preset actif')}>{preset.name}</span>
        </div>

        {#if warning}<p class="notice" role="status">{warning}</p>{/if}
        {#if message}<p class="notice" role="status">{message}</p>{/if}
        {#if error}<p class="notice error" role="alert">{error}</p>{/if}

        <p class="sr-only" aria-live="polite">{announce}</p>

        {#if help}
          <Help onclose={() => {
            help = false;
            focusInput();
          }} />
        {:else if prompt?.fields}
          <div class="prompt">
            <p class="prompt-title">{prompt.item.title}</p>
            {#each prompt.fields as f, i (f)}
              <label class="field-row">
                {f}
                {#if i === 0}
                  <input bind:this={promptInput} bind:value={prompt.values[f]} onkeydown={promptKeys} autocomplete="off" />
                {:else}
                  <input bind:value={prompt.values[f]} onkeydown={promptKeys} autocomplete="off" />
                {/if}
              </label>
            {/each}
            <small>{t('Entrée pour passer au champ suivant puis insérer, Échap pour annuler')}</small>
          </div>
        {:else if prompt}
          <div class="prompt">
            <label for="prompt-input">{prompt.action.title} : <strong>{prompt.item.title}</strong></label>
            <input
              id="prompt-input"
              bind:this={promptInput}
              bind:value={prompt.value}
              onkeydown={promptKeys}
              placeholder={prompt.action.arg}
              spellcheck="false"
              autocomplete="off"
            />
            <small>{t('Entrée pour valider, Échap pour annuler')}</small>
          </div>
        {:else if results.length}
          <div class="body" class:preview-left={preset.preview.side === 'left'}>
            <ul id="results" role="listbox" bind:this={list}>
              {#each results as item, i (item.id)}
                <li
                  id={`r-${i}`}
                  role="option"
                  aria-selected={i === selected}
                  onmousemove={() => (selected = i)}
                  onclick={() => activate(item)}
                >
                  {#if item.icon}
                    <img src={item.icon} alt="" />
                  {:else}
                    <span class="letter" aria-hidden="true">{item.title.slice(0, 1)}</span>
                  {/if}
                  <span class="text">
                    <span class="title">{item.title}</span>
                    {#if item.subtitle}<span class="subtitle">{item.subtitle}</span>{/if}
                  </span>
                  <span class="module">{moduleLabel(item.module)}</span>
                </li>
              {/each}
            </ul>

            {#if panel}
              <div class="actions" role="listbox" aria-label={t('Actions pour {title}', { title: panel.item.title })}>
                <p class="actions-title">{panel.item.title}</p>
                {#each panel.actions as a, i (a.id)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <div
                    role="option"
                    tabindex="-1"
                    aria-selected={i === panel.sel}
                    class:danger={a.danger}
                    onmousemove={() => (panel.sel = i)}
                    onclick={() => choose(a)}
                  >
                    {i === panel.sel && panel.confirm ? t('Entrée pour confirmer : ') : ''}{a.title}{a.arg ? '…' : ''}
                  </div>
                {/each}
              </div>
            {:else if preset.preview.visible && current}
              <aside class="preview" aria-label={t('Aperçu')}>
                {#if previewData?.kind === 'image'}
                  <img class="shot" src={previewData.src} alt={current.title} />
                {:else}
                  {#if current.icon}<img src={current.icon} alt="" />{:else}<span class="letter big">{current.title.slice(0, 1)}</span>{/if}
                {/if}
                <h2>{current.title}</h2>
                {#if current.subtitle}<p>{current.subtitle}</p>{/if}
                {#if previewData?.kind === 'text'}
                  <pre>{previewData.text}</pre>
                {:else if previewData?.kind === 'props'}
                  <dl>
                    {#each previewData.rows as [k, v]}<dt>{k}</dt><dd>{v}</dd>{/each}
                  </dl>
                {/if}
                <p class="meta">{moduleLabel(current.module)} · {t('Entrée')}{current.actions?.[0] ? ` : ${current.actions[0].title.toLowerCase()}` : ''}{current.actions?.length > 1 ? ` · ${t('Ctrl+K : actions')}` : ''}</p>
              </aside>
            {/if}
          </div>
        {:else if query}
          <p class="empty">{t('Aucun résultat pour « {q} »', { q: query })}</p>
          <p class="empty hint">{t("Essaie / pour la liste des commandes, ou F1 pour l'aide.")}</p>
        {:else}
          <p class="empty hint">{t('Une application, un fichier, un calcul, « / » pour les commandes. F1 : aide.')}</p>
        {/if}
        {#if results.length && !help && !prompt}
          <footer class="keys" aria-hidden="true">
            <span><kbd>↵</kbd> {current?.actions?.[0]?.title ?? t('Ouvrir')}</span>
            <span><kbd>Ctrl K</kbd> {t('actions')}</span>
            <span><kbd>/</kbd> {t('commandes')}</span>
            <span><kbd>F1</kbd> {t('aide')}</span>
          </footer>
        {/if}
      </section>
      </div>
    {/key}
  {/if}
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    /* Center anchor keeps the input still while results grow below it. */
    padding-top: max(16px, calc(50vh - var(--max-height) / 2));
    box-sizing: border-box;
  }
  :global(:root[data-anchor='top']) .overlay {
    padding-top: var(--margin);
  }
  :global(:root[data-anchor='bottom']) .overlay {
    align-items: flex-end;
    padding-top: 0;
    padding-bottom: var(--margin);
  }
  /* Window layer: the window is the size of the bar and GNOME places it. */
  :global(:root[data-layer='window']) .overlay {
    align-items: flex-start;
    padding: 32px 0 0;
  }
  :global(:root[data-layer='window']) .frame {
    transform: none;
  }

  .overlay.centered {
    align-items: center;
    padding: 0;
  }

  .frame {
    position: relative;
    width: min(var(--width), calc(100vw - 32px));
    max-height: var(--max-height);
    transform: translate(var(--offset-x), var(--offset-y));
    animation: rise var(--anim) cubic-bezier(0.2, 0.9, 0.3, 1);
    display: flex;
  }
  .bar {
    position: relative;
    z-index: 1;
    flex: 1;
    min-width: 0;
    max-height: var(--max-height);
    display: flex;
    flex-direction: column;
    color: var(--fg);
    background: color-mix(in srgb, var(--bg) var(--opacity), transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow:
      0 1px 0 color-mix(in srgb, var(--fg) 8%, transparent) inset,
      0 28px 80px rgb(0 0 0 / 0.38);
    overflow: hidden;
  }
  /* Results and fields above the particles. */
  .bar > :global(:not(.particles)) {
    position: relative;
    z-index: 1;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(var(--offset-x), calc(var(--offset-y) + 8px)) scale(0.985);
    }
  }

  .field {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: calc(var(--pad) + 6px) calc(var(--pad) + 8px);
    border-bottom: 1px solid var(--border);
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    font-weight: 800;
    color: var(--bg);
    background: var(--accent);
    flex: none;
  }
  input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    outline: none;
    color: inherit;
    font: inherit;
    font-size: 1.3em;
    caret-color: var(--accent);
  }
  input::placeholder {
    color: var(--muted);
  }
  .preset-chip {
    font-size: 0.75em;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
    white-space: nowrap;
  }

  .notice {
    margin: 0;
    white-space: pre-wrap;
    max-height: 40%;
    overflow-y: auto;
    padding: 8px calc(var(--pad) + 8px);
    font-size: 0.85em;
    color: var(--fg);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .notice.error {
    background: color-mix(in srgb, #e5484d 28%, transparent);
  }

  .body {
    display: flex;
    min-height: 0;
    flex: 1;
  }
  .body.preview-left {
    flex-direction: row-reverse;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: var(--pad);
    overflow-y: auto;
    flex: 1;
    min-width: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: var(--row);
    padding: 0 10px;
    margin-bottom: var(--gap);
    border-radius: calc(var(--radius) * 0.6);
    cursor: default;
    position: relative;
  }
  li[aria-selected='true'] {
    background: var(--selection);
  }
  li[aria-selected='true']::before {
    content: '';
    position: absolute;
    left: 0;
    top: 22%;
    bottom: 22%;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }
  li img,
  .letter {
    width: 28px;
    height: 28px;
    flex: none;
  }
  .letter {
    display: grid;
    place-items: center;
    border-radius: 8px;
    font-weight: 700;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .title,
  .subtitle {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtitle {
    font-size: 0.8em;
    color: var(--muted);
  }
  .module {
    font-size: 0.72em;
    color: var(--muted);
    letter-spacing: 0.02em;
  }

  .preview {
    width: var(--preview-width);
    flex: none;
    box-sizing: border-box;
    padding: calc(var(--pad) + 10px);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }
  .preview-left .preview {
    border-left: 0;
    border-right: 1px solid var(--border);
  }
  .preview img,
  .letter.big {
    width: 56px;
    height: 56px;
    font-size: 1.6em;
    border-radius: 14px;
  }
  .preview h2 {
    margin: 8px 0 0;
    font-size: 1.15em;
  }
  .preview p {
    margin: 0;
    color: var(--muted);
    font-size: 0.88em;
    line-height: 1.45;
  }
  .preview .meta {
    margin-top: auto;
    font-size: 0.75em;
  }
  .empty {
    margin: 0;
    padding: calc(var(--pad) + 10px);
    color: var(--muted);
  }

  @media (prefers-reduced-motion: reduce) {
    .frame {
      animation: none;
    }
  }
  :global(:root[data-still]) .frame {
    animation: none;
  }
  .sr-only {
    position: absolute !important;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .empty.hint {
    padding-top: 0;
    font-size: 0.85em;
  }
  .sr-only + .empty.hint {
    padding-top: calc(var(--pad) + 10px);
  }
  .keys {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    padding: 6px calc(var(--pad) + 8px);
    border-top: 1px solid var(--border);
    font-size: 0.72em;
    color: var(--muted);
  }
  kbd {
    font: inherit;
    font-size: 0.95em;
    padding: 0 5px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    color: var(--fg);
  }
  @media (forced-colors: active) {
    .bar {
      border: 2px solid CanvasText;
    }
    li[aria-selected='true'] {
      outline: 2px solid Highlight;
      outline-offset: -2px;
    }
  }

  .prompt {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: calc(var(--pad) + 10px) calc(var(--pad) + 8px);
  }
  .prompt input {
    color: var(--fg);
    font: inherit;
    font-size: 1.05em;
    padding: 8px 10px;
    border-radius: 10px;
    border: 1px solid var(--accent);
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    caret-color: var(--accent);
  }
  .prompt small {
    color: var(--muted);
  }
  .prompt-title {
    margin: 0;
    font-weight: 600;
  }
  .field-row {
    display: grid;
    grid-template-columns: 140px 1fr;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 0.9em;
  }

  .actions {
    width: var(--preview-width);
    min-width: 220px;
    flex: none;
    box-sizing: border-box;
    padding: var(--pad);
    border-left: 1px solid var(--border);
    overflow-y: auto;
    background: color-mix(in srgb, var(--selection) 40%, transparent);
  }
  .actions-title {
    margin: 4px 10px 8px;
    font-size: 0.75em;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .actions [role='option'] {
    padding: 7px 10px;
    border-radius: calc(var(--radius) * 0.5);
    font-size: 0.92em;
    cursor: default;
  }
  .actions [aria-selected='true'] {
    background: var(--selection);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .actions .danger {
    color: #e5484d;
  }

  .preview pre {
    margin: 4px 0 0;
    padding: 8px;
    max-height: 100%;
    overflow: hidden;
    font-size: 0.75em;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
    border-radius: 8px;
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    color: var(--fg);
  }
  .preview img.shot {
    width: 100%;
    height: auto;
    max-height: 45%;
    object-fit: contain;
    border-radius: 8px;
  }
  .preview dl {
    margin: 4px 0 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 10px;
    font-size: 0.82em;
  }
  .preview dt {
    color: var(--muted);
  }
  .preview dd {
    margin: 0;
    word-break: break-word;
  }
</style>
