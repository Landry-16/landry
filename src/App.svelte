<script>
  import { tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { applyToRoot } from './lib/theme.js';
  import Settings from './Settings.svelte';

  const MODULE_LABELS = { apps: 'Application', commands: 'Commande' };

  let preset = $state(null);
  let systemDark = $state(false);
  let warning = $state('');
  let error = $state('');
  let history = $state([]);
  let query = $state('');
  let results = $state([]);
  let selected = $state(0);
  let view = $state('bar');
  let shownCount = $state(0);
  let input = $state();
  let list = $state();
  let histPos = -1;
  let searchSeq = 0;

  const current = $derived(results[selected]);

  async function refresh() {
    const s = await invoke('ui_state');
    preset = s.preset;
    systemDark = s.system_dark;
    warning = s.warning ?? '';
    history = s.history;
  }

  async function runSearch(q) {
    const seq = ++searchSeq;
    const r = await invoke('search', { query: q });
    if (seq === searchSeq) {
      results = r;
      selected = 0;
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
    invoke('settings_layer', { on: view === 'settings' });
  });

  $effect(() => {
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  });

  $effect(() => {
    refresh();
    const subs = [
      listen('landry://shown', () => {
        view = 'bar';
        query = '';
        error = '';
        histPos = -1;
        shownCount++;
        // The keyed block re-creates the input: focus the new one once rendered.
        tick().then(() => input?.focus());
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
      listen('landry://open-settings', () => (view = 'settings')),
    ];
    return () => subs.forEach((p) => p.then((f) => f()));
  });

  async function activate(item) {
    if (!item) return;
    error = '';
    try {
      const outcome = await invoke('activate', { id: item.id, query });
      if (outcome === 'hide') query = '';
      else if (outcome === 'settings') view = 'settings';
      else runSearch(query);
    } catch (e) {
      error = String(e);
    }
  }

  function recall(step) {
    histPos = Math.max(-1, Math.min(histPos + step, history.length - 1));
    query = histPos < 0 ? '' : history[histPos];
  }

  function onkeydown(e) {
    const browsing = histPos >= 0;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (browsing) recall(-1);
      else selected = Math.min(selected + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if ((query === '' || browsing) && history.length) recall(1);
      else selected = Math.max(selected - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      activate(current);
    } else if (e.key === 'Tab') {
      e.preventDefault();
      if (current) query = current.title;
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (query) query = '';
      else invoke('hide');
    } else if (e.key === ',' && e.ctrlKey) {
      e.preventDefault();
      view = 'settings';
    }
  }

  function closeSettings() {
    view = 'bar';
    query = '';
    requestAnimationFrame(() => input?.focus());
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" class:centered={view === 'settings'} onmousedown={(e) => e.target === e.currentTarget && invoke('hide')}>
  {#if preset && view === 'settings'}
    <Settings active={preset.name} {systemDark} onclose={closeSettings} />
  {:else if preset}
    {#key shownCount}
      <section class="bar" class:with-preview={preset.preview.visible && current} aria-label="Landry">
        <div class="field">
          <span class="glyph" aria-hidden="true">L</span>
          <input
            bind:this={input}
            bind:value={query}
            oninput={() => (histPos = -1)}
            {onkeydown}
            placeholder="Que veux-tu faire ?"
            aria-label="Recherche"
            aria-controls="results"
            aria-activedescendant={current ? `r-${selected}` : undefined}
            spellcheck="false"
            autocomplete="off"
          />
          <span class="preset-chip" title="Preset actif">{preset.name}</span>
        </div>

        {#if warning}<p class="notice" role="status">{warning}</p>{/if}
        {#if error}<p class="notice error" role="alert">{error}</p>{/if}

        {#if results.length}
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
                  <span class="module">{MODULE_LABELS[item.module] ?? item.module}</span>
                </li>
              {/each}
            </ul>
            {#if preset.preview.visible && current}
              <aside class="preview" aria-label="Aperçu">
                {#if current.icon}<img src={current.icon} alt="" />{:else}<span class="letter big">{current.title.slice(0, 1)}</span>{/if}
                <h2>{current.title}</h2>
                {#if current.subtitle}<p>{current.subtitle}</p>{/if}
                <p class="meta">{MODULE_LABELS[current.module] ?? current.module} · Entrée pour lancer</p>
              </aside>
            {/if}
          </div>
        {:else if query}
          <p class="empty">Aucun résultat pour « {query} »</p>
        {/if}
      </section>
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
  :global(:root[data-layer='window']) .bar {
    transform: none;
  }

  .overlay.centered {
    align-items: center;
    padding: 0;
  }

  .bar {
    width: min(var(--width), calc(100vw - 32px));
    max-height: var(--max-height);
    transform: translate(var(--offset-x), var(--offset-y));
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
    animation: rise var(--anim) cubic-bezier(0.2, 0.9, 0.3, 1);
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
    .bar {
      animation: none;
    }
  }
</style>
