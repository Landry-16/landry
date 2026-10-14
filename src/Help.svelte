<script>
  // F1 or /aide: every shortcut and a few examples, readable without leaving the bar.
  import { t } from './lib/i18n.svelte.js';
  let { onclose } = $props();

  const KEYS = [
    ['Entrée', 'Action principale'],
    ['Ctrl+K', "Toutes les actions de l'élément"],
    ['Tab', 'Compléter avec le titre'],
    ['↑ sur une barre vide', 'Historique des recherches'],
    ['Échap', 'Effacer, puis fermer'],
    ['Ctrl+,', 'Réglages'],
    ['Ctrl+plus / Ctrl+moins', 'Agrandir ou réduire le texte (Ctrl+0 : taille normale)'],
    ['F1', 'Afficher ou cacher cette aide'],
  ];
  const EXAMPLES = [
    ['/', 'Toutes les commandes'],
    ['/spotify daft punk', 'Chercher et écouter dans Spotify'],
    ['/volume 40', 'Régler le volume'],
    ['/presse-papiers', 'Historique du presse-papiers'],
    ['/emoji coeur', 'Chercher un émoji, en français ou en anglais'],
    ['/def maison', 'Définitions du Wiktionnaire'],
    ['12*7, 100 ht, 15h Paris en Tokyo', 'Calculs, montants, fuseaux'],
    ['minuteur 10 min pâtes', 'Lancer un minuteur'],
    ['pdf: facture', 'Fichiers filtrés par type'],
  ];
</script>

<section class="help" aria-labelledby="help-title">
  <header>
    <h2 id="help-title">{t('Aide')}</h2>
    <button onclick={onclose}>{t('Fermer')} <kbd>Échap</kbd></button>
  </header>
  <h3>{t('Clavier')}</h3>
  <dl>
    {#each KEYS as [k, d]}<dt><kbd>{t(k)}</kbd></dt><dd>{t(d)}</dd>{/each}
  </dl>
  <h3>{t('Exemples')}</h3>
  <dl>
    {#each EXAMPLES as [k, d]}<dt><code>{k}</code></dt><dd>{t(d)}</dd>{/each}
  </dl>
</section>

<style>
  .help {
    padding: calc(var(--pad) + 8px) calc(var(--pad) + 12px);
    overflow-y: auto;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 1.05em;
  }
  button {
    font: inherit;
    font-size: 0.8em;
    color: var(--muted);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 3px 10px;
    cursor: pointer;
  }
  h3 {
    margin: 12px 0 6px;
    font-size: 0.72em;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  dl {
    display: grid;
    grid-template-columns: minmax(0, 15em) 1fr;
    gap: 6px 12px;
    margin: 0;
    font-size: 0.85em;
    align-items: baseline;
  }
  dd {
    margin: 0;
    color: var(--muted);
  }
  kbd,
  code {
    font-family: inherit;
    font-size: 0.92em;
    white-space: nowrap;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 5px;
    color: var(--fg);
  }
  code {
    border-bottom-width: 1px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
</style>
