// Development only: fakes the Tauri bridge so the UI can be checked in a normal browser.
// Open http://localhost:1420/dev/mock.html with `npm run dev`. Never shipped (not in index.html).
(() => {
  const presets = structuredClone(window.LANDRY_PRESETS);
  let active = 'Défaut';
  let notes = [{ id: 1, body: '# Idées pour Landry\n\n- [x] Presets\n- [ ] **Thèmes** saisonniers\n- Voir [Raycast](https://raycast.com)\n\n> Rester simple.\n\n```\nlandry toggle\n```', updated: Date.now() / 1000 - 3600 }, { id: 2, body: '# Courses\nPain, café', updated: Date.now() / 1000 - 86400 }];
  const callbacks = new Map();
  const listeners = {};
  let nextId = 1;

  const apps = [
    ['Visual Studio Code', 'Édition de code. Redéfinie.'],
    ['Terminal', 'Utiliser la ligne de commande'],
    ['Fichiers', 'Accéder aux fichiers et les organiser'],
    ['Firefox', 'Naviguer sur le Web'],
    ['Paramètres', 'Utilitaire de configuration de GNOME'],
    ['Calculatrice', 'Effectuer des calculs'],
    ['Vidéos', 'Lire des films'],
  ].map(([title, subtitle]) => ({ id: `app:${title}`, module: 'apps', title, subtitle, icon: '', actions: [{ id: 'launch', title: 'Lancer' }] }));
  const fileActions = [
    { id: 'open', title: 'Ouvrir' },
    { id: 'reveal', title: 'Afficher dans le dossier' },
    { id: 'copy-path', title: 'Copier le chemin' },
    { id: 'rename', title: 'Renommer', arg: 'Nouveau nom', value: 'facture-octobre.pdf' },
    { id: 'trash', title: 'Mettre à la corbeille', danger: true },
  ];
  const others = [
    { id: 'file:/home/user/Documents/facture-octobre.pdf', module: 'files', title: 'facture-octobre.pdf', subtitle: '~/Documents', icon: '', actions: fileActions },
    { id: 'proj:/home/user/dev/nacre', module: 'projects', title: 'nacre', subtitle: 'main · 2 modifiés · ~/dev/nacre', icon: '', actions: [{ id: 'editor', title: "Ouvrir dans l'éditeur" }, { id: 'terminal', title: 'Ouvrir un terminal' }, { id: 'npm:dev', title: 'npm run dev' }] },
    { id: 'snip:1', module: 'snippets', title: 'Relance devis', subtitle: 'relance · Bonjour {champ:Prénom},', icon: '', actions: [{ id: 'paste', title: 'Coller' }, { id: 'copy', title: 'Copier' }, { id: 'edit', title: "Modifier l'extrait" }] },
    { id: 'note:view', module: 'notes', title: 'Ouvrir les notes', subtitle: 'Toutes les notes, avec recherche', icon: '', actions: [{ id: 'notes-view', title: 'Ouvrir' }] },
    { id: 'tool:diff', module: 'tools', title: 'Comparer deux textes', subtitle: 'Diff ligne par ligne', icon: '', actions: [{ id: 'view', title: 'Ouvrir' }] },
  ];

  const fold = (s) => s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
  const preset = () => presets.find((p) => p.name === active);
  const emit = (event, payload) =>
    (listeners[event] ?? []).forEach((id) => callbacks.get(id)?.({ event, id: 0, payload }));

  const commands = {
    ui_state: () => ({ preset: preset(), warning: null, history: ['vsc', 'term'], locale: new URLSearchParams(location.search).get('lang') ?? 'fr', system_dark: matchMedia('(prefers-color-scheme: dark)').matches }),
    search: ({ query }) => {
      const cmds = [{ id: 'cmd:settings', module: 'commands', title: 'Réglages Landry', subtitle: 'Presets, apparence, alias', icon: '' }].concat(
        presets.map((p) => ({ id: `cmd:preset:${p.name}`, module: 'commands', title: `Preset : ${p.name}`, subtitle: p.name === active ? 'Preset actif' : 'Changer de preset', icon: '' })),
      );
      const q = fold(query.trim());
      if (/^b64 /.test(query)) return [{ id: 'tool:b64', module: 'tools', title: btoa(unescape(encodeURIComponent(query.slice(4)))), subtitle: 'Base64', icon: '', actions: [{ id: 'copy', title: 'Copier' }] }];
      const all = cmds.concat(apps, others);
      const hits = q ? all.filter((i) => fold(i.title).includes(q)) : apps.slice(0, 4);
      return hits.slice(0, preset().max_results);
    },
    activate: ({ id, action, arg }) => {
      if (id === 'cmd:settings') return { kind: 'settings' };
      if (id === 'tool:diff') return { kind: 'view', text: 'diff' };
      if (id === 'note:view') return { kind: 'view', text: 'notes:1' };
      if (id === 'snip:1' && action === 'edit') return { kind: 'view', text: 'settings:snippets' };
      if (id === 'snip:1' && !arg) return { kind: 'ask', text: ['Prénom', 'Projet'] };
      if (id === 'snip:1') return { kind: 'paste', text: { text: arg, back: 0 } };
      if (id.startsWith('cmd:preset:')) {
        active = id.slice('cmd:preset:'.length);
        emit('landry://preset', { preset: preset(), warning: null });
        return { kind: 'keep' };
      }
      if (action === 'rename') return { kind: 'message', text: `Renommé en ~/Documents/${arg}` };
      if (action === 'trash') return { kind: 'message', text: 'facture-octobre.pdf mis à la corbeille' };
      return { kind: 'hide' };
    },
    item_actions: ({ id }) => [...others, ...apps].find((i) => i.id === id)?.actions ?? [],
    preview: ({ id }) =>
      id.startsWith('proj:')
        ? { kind: 'props', rows: [['Branche', 'main'], ['État', '2 modifiés'], ['Scripts npm', 'dev, build']] }
        : id.startsWith('file:')
          ? { kind: 'props', rows: [['Taille', '182.4 Ko'], ['Modifié', '12/10/2026 18:02']] }
          : { kind: 'none' },
    diff_texts: ({ a, b }) => {
      const x = a.split('\n'), y = b.split('\n');
      return x.map((l, i) => (l === y[i] ? { tag: 'eq', text: l } : { tag: 'del', text: l })).concat(y.filter((l, i) => l !== x[i]).map((text) => ({ tag: 'ins', text })));
    },
    copy_text: () => null,
    get_settings: () => ({ version: 1, active_preset: active, locale: 'fr', aliases: { nc: 'nacre' }, search_prefixes: { g: 'https://www.google.com/search?q={q}', gh: 'https://github.com/search?q={q}' }, file_roots: [], projects_root: '~/dev', editor: 'code {path}', terminal: 'gnome-terminal --working-directory={dir}', terminal_run: 'gnome-terminal --working-directory={dir} -- bash -lc {cmd}', vercel_team: '' }),
    save_settings: () => null,
    accounts: () => [{ id: 'github', label: 'GitHub', configured: true }, { id: 'vercel', label: 'Vercel', configured: false }],
    set_token: () => null,
    snippets_list: () => [{ id: 1, name: 'Relance devis', keyword: 'relance', body: 'Bonjour {champ:Prénom},\n\nJe reviens vers vous au sujet du devis {champ:Projet} envoyé le {date}.{curseur}' }],
    snippet_save: ({ snippet }) => snippet.id || 2,
    snippet_delete: () => null,
    notes_list: () => notes,
    note_save: ({ id, body }) => {
      const n = notes.find((x) => x.id === id);
      if (n) { n.body = body; n.updated = Date.now() / 1000; return id; }
      const nid = notes.length + 1;
      notes.unshift({ id: nid, body, updated: Date.now() / 1000 });
      return nid;
    },
    note_delete: ({ id }) => { notes = notes.filter((n) => n.id !== id); return null; },
    painted: () => null,
    hide: () => console.log('[mock] hide'),
    settings_layer: () => null,
    preset_list: () => presets.map((p) => ({ name: p.name, builtin: p.name === 'Défaut', error: null })),
    preset_get: ({ name }) => structuredClone(presets.find((p) => p.name === name)),
    preset_check: () => null,
    preset_save: ({ json, previous }) => {
      const p = JSON.parse(json);
      presets[presets.findIndex((x) => x.name === previous)] = p;
      if (previous === active) {
        active = p.name;
        emit('landry://preset', { preset: p, warning: null });
      }
      return p;
    },
    preset_duplicate: ({ name, newName }) => {
      const p = { ...structuredClone(presets.find((x) => x.name === name)), name: newName };
      presets.push(p);
      return p;
    },
    preset_delete: ({ name }) => presets.splice(presets.findIndex((p) => p.name === name), 1) && null,
    preset_use: ({ name }) => {
      active = name;
      emit('landry://preset', { preset: preset(), warning: null });
    },
    preset_export: ({ name }) => `/home/user/Téléchargements/landry-preset-${name}.json`,
    preset_import: () => {
      throw 'import indisponible dans la maquette';
    },
    'plugin:event|listen': ({ event, handler }) => {
      (listeners[event] ??= []).push(handler);
      return handler;
    },
    'plugin:event|unlisten': () => null,
  };

  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'bar' }, currentWebview: { label: 'bar', windowLabel: 'bar' } },
    transformCallback(cb) {
      const id = nextId++;
      callbacks.set(id, cb);
      return id;
    },
    unregisterCallback: (id) => callbacks.delete(id),
    async invoke(cmd, args) {
      const fn = commands[cmd];
      if (!fn) throw `mock: commande inconnue ${cmd}`;
      return fn(args ?? {});
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.landryEmit = emit;
  addEventListener('load', () => setTimeout(() => emit('landry://shown', null), 300));
})();
