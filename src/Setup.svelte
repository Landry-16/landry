<script>
  // First-run assistant: shortcut, start with the session, look, optional tools.
  // Every step is optional and can be done again; reopen with /bienvenue or `landry setup`.
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from './lib/i18n.svelte.js';
  import { fromEvent, label } from './lib/accel.js';
  import Companion from './Companion.svelte';

  let { active, onclose } = $props();

  const DEFAULT = '<Control><Alt>space';
  let info = $state(null);
  let captured = $state('');
  let listening = $state(false);
  let presets = $state([]);
  // Initial value only: the assistant tracks the preset it switches to.
  let current = $state(untrack(() => active));
  let status = $state('');
  let failure = $state('');
  let root = $state();

  $effect(() => {
    invoke('setup_state').then((s) => (info = s));
    invoke('preset_list').then(async (list) => {
      const ok = list.filter((p) => !p.error);
      presets = await Promise.all(ok.map((p) => invoke('preset_get', { name: p.name })));
    });
    root?.focus();
  });

  async function run(fn, ok) {
    failure = '';
    status = '';
    try {
      await fn();
      status = ok;
    } catch (e) {
      failure = String(e);
    }
  }

  function capture(e) {
    if (e.key === 'Escape' || e.key === 'Tab') return;
    e.preventDefault();
    e.stopPropagation();
    const a = fromEvent(e);
    if (a) {
      captured = a;
      listening = false;
    }
  }

  const saveShortcut = (accel) =>
    run(async () => {
      await invoke('shortcut_set', { accel });
      info.shortcut = accel;
      captured = '';
    }, t('Raccourci enregistré : {k}', { k: label(accel) }));

  const toggleStart = () =>
    run(() => invoke('autostart_set', { on: info.autostart }), info.autostart ? t('Landry démarrera avec la session.') : t('Démarrage automatique désactivé.'));

  const usePreset = (name) =>
    run(async () => {
      await invoke('preset_use', { name });
      current = name;
    }, t('Preset activé.'));

  async function finish() {
    try {
      await invoke('setup_done');
    } catch {}
    onclose();
  }

  function onkeydown(e) {
    if (e.key === 'Escape' && !listening) {
      e.preventDefault();
      finish();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="setup" bind:this={root} tabindex="-1" {onkeydown} role="dialog" aria-modal="true" aria-labelledby="setup-title">
  <header>
    <Companion mood="joy" />
    <div>
      <h1 id="setup-title">{t('Bienvenue dans Landry')}</h1>
      <p>{t('Quatre réglages, tous facultatifs. Tu peux revenir ici avec /bienvenue.')}</p>
    </div>
  </header>

  <ol>
    <li>
      <h2>{t('Raccourci pour ouvrir la barre')}</h2>
      {#if info}
        <p class="state" class:done={info.shortcut}>
          {info.shortcut ? t('Installé : {k}', { k: label(info.shortcut) }) : t('Pas encore de raccourci.')}
        </p>
        <div class="row">
          <button
            class="capture"
            class:listening
            onclick={() => (listening = true)}
            onkeydown={(e) => listening && capture(e)}
            onblur={() => (listening = false)}
            aria-live="polite"
          >
            {listening ? t('Appuie sur la combinaison…') : captured ? label(captured) : t('Choisir une combinaison')}
          </button>
          {#if captured}
            <button class="primary" onclick={() => saveShortcut(captured)}>{t('Enregistrer')}</button>
          {:else if info.shortcut !== DEFAULT}
            <button onclick={() => saveShortcut(DEFAULT)}>{t('Utiliser {k}', { k: label(DEFAULT) })}</button>
          {/if}
        </div>
        {#if captured === '<Super>space'}<p class="warn">{t('Super + Espace change déjà la disposition du clavier dans GNOME.')}</p>{/if}
      {:else}
        <p class="state">{t('Vérification…')}</p>
      {/if}
    </li>

    <li>
      <h2>{t('Démarrer avec la session')}</h2>
      {#if info}
        <label class="check"><input type="checkbox" bind:checked={info.autostart} onchange={toggleStart} /> {t('Landry est prêt dès la connexion, et redémarre tout seul s’il s’arrête.')}</label>
      {:else}
        <p class="state">{t('Vérification…')}</p>
      {/if}
    </li>

    <li>
      <h2>{t('Apparence')}</h2>
      <div class="presets" role="group" aria-label={t('Presets')}>
        {#each presets as p (p.name)}
          {@const c = p.theme === 'light' ? p.light : p.dark}
          <button class="preset" aria-pressed={p.name === current} onclick={() => usePreset(p.name)} style="--pbg: {c.bg}; --pfg: {c.fg}; --pac: {c.accent}; --pti: {p.style?.tint ?? c.accent}">
            <span class="swatch" aria-hidden="true"><i></i><i></i></span>
            {p.name}
          </button>
        {/each}
      </div>
    </li>

    <li>
      <h2>{t('Outils facultatifs')}</h2>
      {#if !info}
        <p class="state">{t('Vérification…')}</p>
      {:else if !info.missing.length}
        <p class="info done">{t('Tout est installé.')}</p>
      {:else}
        <ul class="missing">
          {#each info.missing as [what, cmd]}
            <li>
              <span>{what}</span>
              <code>{cmd}</code>
              <button onclick={() => run(() => invoke('copy_text', { text: cmd }), t('Commande copiée.'))} aria-label={t('Copier : {c}', { c: cmd })}>{t('Copier')}</button>
            </li>
          {/each}
        </ul>
      {/if}
    </li>
  </ol>

  <footer>
    <p role="status">{status}</p>
    {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
    <button class="primary" onclick={finish}>{t("C'est parti")}</button>
  </footer>
</div>

<style>
  .setup {
    width: min(720px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 18px;
    box-shadow: 0 32px 90px rgb(0 0 0 / 0.42);
    outline: none;
  }
  header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 22px 26px 16px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(120deg, color-mix(in srgb, var(--accent) 16%, transparent), transparent 60%);
  }
  header :global(.vouivre) {
    width: 64px;
    height: 58px;
  }
  h1 {
    margin: 0;
    font-size: 1.45em;
  }
  header p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 0.9em;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 8px 26px;
    counter-reset: step;
  }
  ol > li {
    counter-increment: step;
    padding: 14px 0 16px 40px;
    position: relative;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  ol > li:last-child {
    border-bottom: 0;
  }
  ol > li::before {
    content: counter(step);
    position: absolute;
    left: 0;
    top: 14px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-weight: 700;
    font-size: 0.85em;
    color: var(--bg);
    background: var(--accent);
  }
  h2 {
    margin: 2px 0 8px;
    font-size: 1.02em;
  }
  .info {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 0.9em;
  }
  .info.done::before {
    content: '✓ ';
    color: var(--accent);
    font-weight: 700;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    font: inherit;
    font-size: 0.9em;
    color: var(--fg);
    background: color-mix(in srgb, var(--fg) 6%, transparent);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 7px 14px;
    cursor: pointer;
  }
  button:hover {
    border-color: var(--accent);
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--bg);
    font-weight: 600;
  }
  .capture {
    min-width: 220px;
    text-align: left;
    font-variant-numeric: tabular-nums;
  }
  .capture.listening {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .warn,
  .bad {
    color: #e5484d;
    font-size: 0.85em;
    margin: 8px 0 0;
  }
  .check {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    font-size: 0.92em;
  }
  .check input {
    accent-color: var(--accent);
    margin-top: 3px;
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 8px;
  }
  .preset {
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
  }
  .preset[aria-pressed='true'] {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .swatch {
    width: 30px;
    height: 22px;
    flex: none;
    border-radius: 6px;
    background: var(--pbg);
    border: 1px solid color-mix(in srgb, var(--pfg) 25%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
  }
  .swatch i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--pac);
  }
  .swatch i + i {
    background: var(--pti);
  }
  .missing {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .missing li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 4px 10px;
    align-items: center;
    font-size: 0.9em;
  }
  .missing code {
    grid-column: 1;
    font-size: 0.85em;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .missing button {
    grid-column: 2;
    grid-row: 1 / 3;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 26px 20px;
    border-top: 1px solid var(--border);
  }
  footer p {
    flex: 1;
    margin: 0;
    color: var(--muted);
    font-size: 0.88em;
  }
  footer .bad {
    flex: 1;
    margin: 0;
  }
</style>
