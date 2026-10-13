<script>
  import { t } from './lib/i18n.svelte.js';
  import { invoke } from '@tauri-apps/api/core';

  let accounts = $state([]);
  let drafts = $state({});
  let message = $state('');
  let failure = $state('');

  async function load() {
    accounts = await invoke('accounts');
  }

  $effect(() => {
    load().catch((e) => (failure = String(e)));
  });

  async function save(id, token) {
    message = '';
    failure = '';
    try {
      await invoke('set_token', { account: id, token });
      drafts[id] = '';
      await load();
      message = token ? t('Jeton enregistré dans le trousseau.') : t('Jeton supprimé.');
    } catch (e) {
      failure = String(e);
    }
  }
</script>

<div class="accounts">
  <p class="intro">
    {t('Les jetons sont rangés dans le trousseau du système (Secret Service), jamais dans un fichier. Landry ne les affiche pas une fois enregistrés.')}
  </p>
  {#each accounts as a (a.id)}
    <div class="account">
      <div class="head">
        <h2>{t(a.label)}</h2>
        <span class="state" class:on={a.configured}>{a.configured ? t('Jeton enregistré') : t('Aucun jeton')}</span>
      </div>
      <label>
        {a.configured ? t('Remplacer le jeton') : t('Jeton d’accès')}
        <input type="password" bind:value={drafts[a.id]} autocomplete="off" spellcheck="false" placeholder={t({ github: 'ghp_… (droits : repo, read:org)', vercel: 'Jeton Vercel', translate: 'Clé DeepL (…:fx) ou clé LibreTranslate' }[a.id])} />
      </label>
      <div class="buttons">
        {#if a.configured}<button class="danger" onclick={() => save(a.id, '')}>{t('Supprimer')}</button>{/if}
        <button class="primary" disabled={!drafts[a.id]?.trim()} onclick={() => save(a.id, drafts[a.id])}>{t('Enregistrer')}</button>
      </div>
      <p class="hint">
        {t({
          github: 'Utilisé par « pr » (tes pull requests et leur CI) et « issues » (issues assignées).',
          vercel: 'Utilisé par « vercel » (déploiements, logs, redéploiement).',
          translate: 'Utilisé par « tr <texte> ». Le texte à traduire est envoyé au service choisi.',
        }[a.id])}
      </p>
    </div>
  {/each}
  {#if message}<p class="ok" role="status">{message}</p>{/if}
  {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
</div>

<style>
  .accounts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 14px;
  }
  .intro {
    grid-column: 1 / -1;
    margin: 0;
    color: var(--muted);
    font-size: 0.88em;
  }
  .account {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 1.05em;
  }
  .state {
    font-size: 0.75em;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
  }
  .state.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85em;
    color: var(--muted);
  }
  input,
  button {
    font: inherit;
    color: var(--fg);
  }
  input {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 8px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
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
  button.primary {
    background: var(--accent);
    color: var(--bg);
    border-color: var(--accent);
    font-weight: 600;
  }
  button.danger {
    color: #e5484d;
  }
  .hint,
  .ok,
  .bad {
    margin: 0;
    font-size: 0.82em;
  }
  .hint {
    color: var(--muted);
  }
  .ok {
    color: var(--accent);
    grid-column: 1 / -1;
  }
  .bad {
    color: #e5484d;
    grid-column: 1 / -1;
  }
</style>
