<script>
  import { i18n, t } from './lib/i18n.svelte.js';
  import { invoke } from '@tauri-apps/api/core';
  import { parsePairs, formatPairs } from './lib/pairs.js';

  let settings = $state(null);
  let roots = $state('');
  let home = $state('');
  let aliases = $state('');
  let prefixes = $state('');
  let message = $state('');
  let autostart = $state(false);
  let importPath = $state('');
  let failure = $state('');

  $effect(() => {
    invoke('autostart_get').then((v) => (autostart = v));
    invoke('get_settings').then((s) => {
      settings = s;
      roots = s.file_roots.join('\n');
      home = (s.home ?? []).join('\n');
      aliases = formatPairs(s.aliases);
      prefixes = formatPairs(s.search_prefixes);
    });
  });

  async function run(fn) {
    message = '';
    failure = '';
    try {
      message = await fn();
    } catch (e) {
      failure = String(e);
    }
  }

  const toggleAutostart = () =>
    run(async () => {
      await invoke('autostart_set', { on: autostart });
      return autostart ? t('Landry démarrera avec la session.') : t('Démarrage automatique désactivé.');
    });

  async function save() {
    message = '';
    failure = '';
    const a = parsePairs(aliases);
    const p = parsePairs(prefixes);
    const bad = [...a.errors, ...p.errors];
    if (bad.length) {
      failure = t('Ligne invalide (attendu « clé = valeur ») : {lines}', { lines: bad.join(', ') });
      return;
    }
    const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null);
    const next = {
      ...settings,
      vat_rate: num(settings.vat_rate) ?? 20,
      micro_rate: num(settings.micro_rate),
      micro_tax_rate: num(settings.micro_tax_rate),
      file_roots: roots.split('\n').map((l) => l.trim()).filter(Boolean),
      // Keep a trailing space: it means "write in the bar" rather than "run".
      home: home.split('\n').map((l) => l.trimStart()).filter((l) => l.trim()),
      aliases: a.map,
      search_prefixes: p.map,
    };
    try {
      await invoke('save_settings', { settings: next });
      settings = next;
      message = t('Réglages enregistrés. Réindexation des projets et des fichiers en cours.');
    } catch (e) {
      failure = String(e);
    }
  }
</script>

{#if settings}
  <div class="general">
    <div class="group">
      <h2>{t('Démarrage et sauvegarde')}</h2>
      <label>{t("Langue de l'interface")}
        <select bind:value={settings.locale} onchange={() => (i18n.locale = settings.locale)}>
          <option value="fr">Français</option>
          <option value="en">English</option>
        </select>
      </label>
      <label class="check"><input type="checkbox" bind:checked={autostart} onchange={toggleAutostart} /> {t('Démarrer Landry avec la session')}</label>
      <label class="check"><input type="checkbox" bind:checked={settings.switch_to_window} /> {t('Entrée sur une application déjà ouverte : passer à sa fenêtre (Ctrl+K pour en ouvrir une nouvelle)')}</label>
      <label class="check"><input type="checkbox" bind:checked={settings.read_selection} /> {t('Lire le texte sélectionné à l’ouverture (pour maj, titre, typo…)')}</label>
      <button onclick={() => run(async () => t('Sauvegarde : {p}', { p: await invoke('backup_export') }))}>{t('Exporter une sauvegarde complète')}</button>
      <label for="backup-path">{t("Importer une sauvegarde (rien n'est supprimé, les doublons sont ignorés)")}</label>
      <div class="inline">
        <input id="backup-path" bind:value={importPath} placeholder="~/Téléchargements/landry-sauvegarde-….json" spellcheck="false" />
        <button disabled={!importPath.trim()} onclick={() => run(() => invoke('backup_import', { path: importPath }))}>{t('Importer')}</button>
      </div>
      <p class="hint">{t('Réglages, presets, extraits et notes. Les jetons restent dans le trousseau et ne sont jamais exportés.')}</p>
    </div>

    <div class="group">
      <h2>{t('Recherche de fichiers')}</h2>
      <label for="roots">{t('Dossiers indexés, un par ligne. Vide : Documents, Téléchargements, Bureau, Images, Musique, Vidéos.')}</label>
      <textarea id="roots" bind:value={roots} rows="4" placeholder="~/Documents" spellcheck="false"></textarea>
      <p class="hint">{t('Les nouveaux dossiers sont surveillés en direct après un redémarrage de Landry.')}</p>
    </div>

    <div class="group">
      <h2>{t('Développement')}</h2>
      <label>{t('Dossier des projets')} <input bind:value={settings.projects_root} spellcheck="false" /></label>
      <label>{t('Éditeur')} <input bind:value={settings.editor} spellcheck="false" /></label>
      <label>{t('Terminal')} <input bind:value={settings.terminal} spellcheck="false" /></label>
      <label>{t('Terminal avec commande')} <input bind:value={settings.terminal_run} spellcheck="false" /></label>
      <label>{t('Équipe Vercel (identifiant, facultatif)')} <input bind:value={settings.vercel_team} spellcheck="false" /></label>
      <p class="hint">{t('Variables : {a}, {b}, {c}. Chaque variable devient un seul argument, sans passer par un shell.', { a: '{path}', b: '{dir}', c: '{cmd}' })}</p>
    </div>

    <div class="group">
      <h2>{t('Montants')}</h2>
      <label>{t('Taux de TVA (%)')} <input type="number" min="0" max="100" step="0.1" bind:value={settings.vat_rate} /></label>
      <label>{t('Cotisations micro-entreprise (%)')} <input type="number" min="0" max="100" step="0.1" bind:value={settings.micro_rate} placeholder={t('à renseigner')} /></label>
      <label>{t('Versement libératoire (%, facultatif)')} <input type="number" min="0" max="100" step="0.1" bind:value={settings.micro_tax_rate} /></label>
      <p class="hint">{t("Exemples : « 100 ht », « 120 ttc », « ca 3000 ». Les taux changent chaque année : vérifie-les sur le site de l'Urssaf.")}</p>
    </div>

    <div class="group">
      <h2>{t('Accueil')}</h2>
      <label for="home">{t("Ce que la barre montre avant que tu tapes, une ligne par élément : une commande (/spotify play, pomodoro), le nom d'une application, ou l'identifiant d'un élément. Vide : l'accueil habituel (minuteurs en cours, éléments fréquents).")}</label>
      <textarea id="home" bind:value={home} rows="5" placeholder={'Firefox\n/spotify play\npomodoro\n/meteo Paris'} spellcheck="false" aria-describedby="home-hint"></textarea>
      <p class="hint" id="home-hint">{t("Entrée lance la commande, Ctrl+K la recopie dans la barre. Une commande finie par une espace, comme « /spotify  », s'écrit dans la barre pour que tu la complètes.")}</p>
    </div>

    <div class="group">
      <h2>{t('Alias')}</h2>
      <label for="aliases"><span>{t('Un par ligne :')} <code>{t('alias = cible')}</code>{t(". La cible est une commande complète, un texte à chercher, ou l'identifiant d'un élément. Des mots tapés après l'alias s'ajoutent à la cible.")}</span></label>
      <textarea id="aliases" bind:value={aliases} rows="5" placeholder={'pp = /spotify play\nsp = /spotify\nnc = nacre'} spellcheck="false" aria-describedby="aliases-examples"></textarea>
      <p class="hint" id="aliases-examples">{t('Exemples : « pp = /spotify play » met la musique en pause d’un seul mot ; avec « sp = /spotify », taper « sp daft punk » cherche Daft Punk dans Spotify.')}</p>
    </div>

    <div class="group">
      <h2>{t('Traduction')}</h2>
      <label>{t('Langue cible par défaut')} <input bind:value={settings.translate_target} maxlength="5" placeholder="EN" /></label>
      <label>{t('Serveur LibreTranslate (facultatif)')} <input bind:value={settings.translate_url} placeholder="https://libretranslate.example/translate" spellcheck="false" /></label>
      <p class="hint">{t("« tr bonjour » ou « tr de bonjour ». Sans serveur, Landry utilise DeepL avec la clé de l'onglet Comptes.")}</p>
    </div>

    <div class="group">
      <h2>{t('Recherches web')}</h2>
      <label for="prefixes"><span>{t('Un par ligne :')} <code>{t('préfixe = URL')}</code>{t(', avec {q} à la place du texte cherché.', { q: '{q}' })}</span></label>
      <textarea id="prefixes" bind:value={prefixes} rows="6" spellcheck="false"></textarea>
    </div>

    <div class="bar-actions">
      {#if message}<p class="ok" role="status">{message}</p>{/if}
      {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
      <button class="primary" onclick={save}>{t('Enregistrer')}</button>
    </div>
  </div>
{/if}

<style>
  .general {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 14px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 0.8em;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
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
  select,
  textarea {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 8px;
  }
  textarea {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.85em;
    resize: vertical;
  }
  code {
    color: var(--fg);
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--fg);
  }
  .inline {
    display: flex;
    gap: 6px;
  }
  .inline input {
    flex: 1;
    min-width: 0;
  }
  .group > button,
  .inline button {
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 9px;
    padding: 6px 12px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
  }
  .hint {
    margin: 0;
    font-size: 0.8em;
    color: var(--muted);
  }
  .bar-actions {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
  }
  .bar-actions p {
    margin: 0;
    flex: 1;
    font-size: 0.85em;
  }
  .ok {
    color: var(--accent);
  }
  .bad {
    color: #e5484d;
  }
  button.primary {
    background: var(--accent);
    color: var(--bg);
    border: 1px solid var(--accent);
    border-radius: 9px;
    padding: 6px 14px;
    font-weight: 600;
    cursor: pointer;
  }
</style>
