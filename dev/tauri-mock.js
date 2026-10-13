// Development only: fakes the Tauri bridge so the UI can be checked in a normal browser.
// Open http://localhost:1420/dev/mock.html with `npm run dev`. Never shipped (not in index.html).
(() => {
  const presets = structuredClone(window.LANDRY_PRESETS);
  let active = 'Défaut';
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
  ].map(([title, subtitle]) => ({ id: `app:${title}`, module: 'apps', title, subtitle, icon: '' }));

  const fold = (s) => s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
  const preset = () => presets.find((p) => p.name === active);
  const emit = (event, payload) =>
    (listeners[event] ?? []).forEach((id) => callbacks.get(id)?.({ event, id: 0, payload }));

  const commands = {
    ui_state: () => ({ preset: preset(), warning: null, history: ['vsc', 'term'], system_dark: matchMedia('(prefers-color-scheme: dark)').matches }),
    search: ({ query }) => {
      const cmds = [{ id: 'cmd:settings', module: 'commands', title: 'Réglages Landry', subtitle: 'Presets, apparence, alias', icon: '' }].concat(
        presets.map((p) => ({ id: `cmd:preset:${p.name}`, module: 'commands', title: `Preset : ${p.name}`, subtitle: p.name === active ? 'Preset actif' : 'Changer de preset', icon: '' })),
      );
      const q = fold(query.trim());
      const all = cmds.concat(apps);
      const hits = q ? all.filter((i) => fold(i.title).includes(q)) : apps.slice(0, 4);
      return hits.slice(0, preset().max_results);
    },
    activate: ({ id }) => {
      if (id === 'cmd:settings') return 'settings';
      if (id.startsWith('cmd:preset:')) {
        active = id.slice('cmd:preset:'.length);
        emit('landry://preset', { preset: preset(), warning: null });
        return 'keep';
      }
      return 'hide';
    },
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
