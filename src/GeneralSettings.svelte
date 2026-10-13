<script>
  import { invoke } from '@tauri-apps/api/core';
  import { parsePairs, formatPairs } from './lib/pairs.js';

  let settings = $state(null);
  let roots = $state('');
  let aliases = $state('');
  let prefixes = $state('');
  let message = $state('');
  let failure = $state('');

  $effect(() => {
    invoke('get_settings').then((s) => {
      settings = s;
      roots = s.file_roots.join('\n');
      aliases = formatPairs(s.aliases);
      prefixes = formatPairs(s.search_prefixes);
    });
  });

  async function save() {
    message = '';
    failure = '';
    const a = parsePairs(aliases);
    const p = parsePairs(prefixes);
    const bad = [...a.errors, ...p.errors];
    if (bad.length) {
      failure = `Ligne invalide (attendu « clé = valeur ») : ${bad.join(', ')}`;
      return;
    }
    const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null);
    const next = {
      ...settings,
      vat_rate: num(settings.vat_rate) ?? 20,
      micro_rate: num(settings.micro_rate),
      micro_tax_rate: num(settings.micro_tax_rate),
      file_roots: roots.split('\n').map((l) => l.trim()).filter(Boolean),
      aliases: a.map,
      search_prefixes: p.map,
    };
    try {
      await invoke('save_settings', { settings: next });
      settings = next;
      message = 'Réglages enregistrés. Réindexation des projets et des fichiers en cours.';
    } catch (e) {
      failure = String(e);
    }
  }
</script>

{#if settings}
  <div class="general">
    <div class="group">
      <h2>Recherche de fichiers</h2>
      <label for="roots">Dossiers indexés, un par ligne. Vide : Documents, Téléchargements, Bureau, Images, Musique, Vidéos.</label>
      <textarea id="roots" bind:value={roots} rows="4" placeholder="~/Documents" spellcheck="false"></textarea>
      <p class="hint">Les nouveaux dossiers sont surveillés en direct après un redémarrage de Landry.</p>
    </div>

    <div class="group">
      <h2>Développement</h2>
      <label>Dossier des projets <input bind:value={settings.projects_root} spellcheck="false" /></label>
      <label>Éditeur <input bind:value={settings.editor} spellcheck="false" /></label>
      <label>Terminal <input bind:value={settings.terminal} spellcheck="false" /></label>
      <label>Terminal avec commande <input bind:value={settings.terminal_run} spellcheck="false" /></label>
      <label>Équipe Vercel (identifiant, facultatif) <input bind:value={settings.vercel_team} spellcheck="false" /></label>
      <p class="hint">Variables : {'{path}'}, {'{dir}'}, {'{cmd}'}. Chaque variable devient un seul argument, sans passer par un shell.</p>
    </div>

    <div class="group">
      <h2>Montants</h2>
      <label>Taux de TVA (%) <input type="number" min="0" max="100" step="0.1" bind:value={settings.vat_rate} /></label>
      <label>Cotisations micro-entreprise (%) <input type="number" min="0" max="100" step="0.1" bind:value={settings.micro_rate} placeholder="à renseigner" /></label>
      <label>Versement libératoire (%, facultatif) <input type="number" min="0" max="100" step="0.1" bind:value={settings.micro_tax_rate} /></label>
      <p class="hint">Exemples : « 100 ht », « 120 ttc », « ca 3000 ». Les taux changent chaque année : vérifie-les sur le site de l'Urssaf.</p>
    </div>

    <div class="group">
      <h2>Alias</h2>
      <label for="aliases"><span>Un par ligne : <code>alias = cible</code>. La cible est un texte à chercher, ou l'identifiant d'un élément.</span></label>
      <textarea id="aliases" bind:value={aliases} rows="4" placeholder="nc = nacre" spellcheck="false"></textarea>
    </div>

    <div class="group">
      <h2>Traduction</h2>
      <label>Langue cible par défaut <input bind:value={settings.translate_target} maxlength="5" placeholder="EN" /></label>
      <label>Serveur LibreTranslate (facultatif) <input bind:value={settings.translate_url} placeholder="https://libretranslate.example/translate" spellcheck="false" /></label>
      <p class="hint">« tr bonjour » ou « tr de bonjour ». Sans serveur, Landry utilise DeepL avec la clé de l'onglet Comptes.</p>
    </div>

    <div class="group">
      <h2>Recherches web</h2>
      <label for="prefixes"><span>Un par ligne : <code>préfixe = URL</code>, avec {'{q}'} à la place du texte cherché.</span></label>
      <textarea id="prefixes" bind:value={prefixes} rows="6" spellcheck="false"></textarea>
    </div>

    <div class="bar-actions">
      {#if message}<p class="ok" role="status">{message}</p>{/if}
      {#if failure}<p class="bad" role="alert">{failure}</p>{/if}
      <button class="primary" onclick={save}>Enregistrer</button>
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
