# Guide de développement

Ce guide explique comment installer l'environnement, lancer Landry en développement, comprendre son architecture et le modifier. Pour l'utilisation au quotidien, voir le [README](../README.md).

## Sommaire

1. [Prérequis](#1-prérequis)
2. [Premier lancement](#2-premier-lancement)
3. [Boucle de développement](#3-boucle-de-développement)
4. [Tests et vérifications](#4-tests-et-vérifications)
5. [Architecture](#5-architecture)
6. [Arborescence](#6-arborescence)
7. [Recettes : modifier le projet](#7-recettes--modifier-le-projet)
8. [Paquets et version](#8-paquets-et-version)
9. [Dépannage](#9-dépannage)
10. [Conventions](#10-conventions)

## 1. Prérequis

Machine visée : Ubuntu 24.04, GNOME 46, session Wayland.

| Outil | Version testée | Installation |
| --- | --- | --- |
| Rust (rustup) | 1.98 | `curl https://sh.rustup.rs -sSf \| sh` |
| Node.js et npm | Node 22, npm 10 | paquet `nodejs` ou nvm |
| Bibliothèques système | voir le script | `./install.sh` les installe (demande sudo) |

`./install.sh` installe la chaîne de compilation, WebKitGTK 4.1, les en-têtes nécessaires à Tauri (dont `libayatana-appindicator3-dev` pour l'icône de barre système) et Tesseract pour l'OCR. Si `apt-get update` échoue à cause d'un dépôt tiers cassé, le script continue quand même et l'installation se fait avec les listes déjà connues.

Outils utilisés à l'exécution (vérifiés par `landry doctor`) : `gio`, `gsettings`, `git`, `zip`, `gnome-terminal`, l'éditeur configuré (`code` par défaut), `tesseract`, et en option `docker`.

## 2. Premier lancement

```bash
git clone git@github.com:Landry-16/landry.git
cd landry
git checkout dev
./install.sh
npm install
```

Pour lancer Landry en développement, avec l'interface servie par Vite et rechargée à chaud :

```bash
npx tauri dev -- -- daemon
```

La première compilation prend environ 90 secondes. Ensuite, dans un autre terminal :

```bash
src-tauri/target/debug/landry toggle
```

Le premier `--` sépare les options de Tauri de celles de cargo, le second celles de cargo de celles de Landry : le binaire est lancé en mode `daemon` (le processus résident). Sans argument, le binaire se comporte comme la CLI.

Un seul processus résident peut tourner à la fois, car ils partagent le socket `$XDG_RUNTIME_DIR/landry.sock`. Si la version installée (`/usr/bin/landry`) tourne déjà, arrête-la d'abord avec `landry quit`.

### Isoler les données de développement

Par défaut, la version de développement lit et écrit les mêmes fichiers que la version installée. Pour tester sans toucher à tes vrais réglages ni à ta base :

```bash
export XDG_CONFIG_HOME=/tmp/landry-dev/config
export XDG_DATA_HOME=/tmp/landry-dev/data
npx tauri dev -- -- daemon
```

La CLI trouve le résident par son socket, quels que soient ces dossiers. Garde quand même les mêmes variables dans le terminal de la CLI : si le résident n'est pas lancé, c'est elle qui le démarre, avec son propre environnement. Attention : avec `XDG_DATA_HOME` déplacé, les applications installées dans `~/.local/share/applications` et les fichiers récents de GTK ne sont plus vus.

## 3. Boucle de développement

| Je modifie | Ce qui se passe |
| --- | --- |
| un fichier de `src/` (Svelte, JS, CSS) | Vite recharge l'interface tout seul |
| un fichier de `src-tauri/src/` (Rust) | `tauri dev` recompile et relance le résident |
| `src/i18n/en.json` | lu par l'interface (rechargement) et par Rust (recompilation) |
| un preset dans `~/.config/landry/presets/` | `landry reload`, ou enregistrer depuis les Réglages |

Autres façons de lancer :

```bash
# Interface seule dans un navigateur, sans Rust (API Tauri imitée)
npx vite --port 1420
# puis ouvrir http://localhost:1420/dev/mock.html (ajouter ?lang=en pour l'anglais)

# Binaire de débogage autonome, interface embarquée (comme en production)
npm run build
(cd src-tauri && cargo build --features custom-protocol)
src-tauri/target/debug/landry daemon        # dans un terminal : les journaux s'affichent
```

Sans `--features custom-protocol`, un binaire construit avec `cargo build` cherche l'interface sur le serveur Vite (`localhost:1420`) et affiche une fenêtre vide s'il ne tourne pas.

Variables utiles :

| Variable | Effet |
| --- | --- |
| `LANDRY_KEEP_OPEN=1` | la barre ne se cache plus quand elle perd le focus (pratique avec un débogueur ou des outils ouverts à côté) |
| `LANDRY_TIMING=1` | la CLI affiche le temps d'aller-retour ; `landry show` affiche aussi le délai de rendu et de focus |

La maquette `dev/mock.html` simule les commandes Tauri dans `dev/tauri-mock.js` avec des données factices. Elle sert à vérifier un écran dans un navigateur, avec Playwright par exemple. Quand tu ajoutes une commande Tauri utilisée par l'interface, ajoute-la aussi à la maquette.

## 4. Tests et vérifications

```bash
(cd src-tauri && cargo test)   # cœur : environ 95 tests
npm test                       # interface : Vitest
npm run build                  # l'interface doit compiler sans avertissement
landry doctor                  # environnement de la machine
```

Ce que couvrent les tests :

- **Rust** : chaque module a ses tests à la fin du fichier (`#[cfg(test)] mod tests`). Les tests qui touchent au disque travaillent dans un dossier temporaire et ne lisent jamais `~/.config/landry`.
- **Traductions** : `i18n::tests` (Rust) et `src/lib/i18n.test.js` (interface) parcourent les sources et échouent si un texte passé à `t()`, `tf()`, `Action::new()` ou `.ask()` n'a pas d'entrée dans `src/i18n/en.json`.
- **Interface** : thème, contraste, Markdown, paires clé = valeur.

Ce qu'aucun test automatique ne couvre, à vérifier à la main : le raccourci GNOME, le focus de la fenêtre, le collage automatique (portail RemoteDesktop), la pipette, la capture et l'OCR, les appels GitHub, Vercel et traduction avec de vrais jetons.

## 5. Architecture

### Un binaire, deux rôles

```
raccourci GNOME ──> landry toggle ──(socket Unix)──> landry daemon
                     (CLI, meurt aussitôt)            (résident : fenêtre Tauri + moteur)
```

- `landry daemon` est le processus résident : il garde la fenêtre cachée, les index en mémoire et la base ouverte. C'est ce qui permet d'ouvrir la barre en quelques dizaines de millisecondes.
- Toute autre invocation (`landry toggle`, `landry search …`) est un client qui envoie une ligne sur le socket et affiche la réponse. Si le résident ne répond pas, le client le démarre (`ipc.rs`).

### Le cœur (Rust, `src-tauri/src/`)

- **`main.rs`** : démarrage, commandes Tauri appelées par l'interface, commandes du socket (`handle`), fils d'arrière-plan, effets système (presse-papiers GTK, notifications, portails, scripts).
- **`engine.rs`** : le moteur, derrière un `Mutex`. Il réunit les index de chaque module, classe les résultats (`search`) et exécute les actions (`activate`).
- **`search.rs`** : le type `Item` (un résultat), le type `Action` (Entrée, ou une ligne du panneau Ctrl+K), et le classement flou (nucleo, sans accents, bonus d'initiales et d'usage).
- **`modules/`** : un fichier par famille de résultats (voir l'arborescence).
- **`config.rs`** : réglages globaux (`settings.json`) et presets (schéma, validation, stockage).
- **`usage.rs`** : la base SQLite (historique, extraits, notes, tâches, minuteurs, suivi du temps).
- **`i18n.rs`** : traduction des textes produits par le cœur.

### Deux sortes de résultats

1. **Indexés** : calculés en arrière-plan puis gardés en mémoire (`Engine::set_index`) : applications, fichiers, projets, hôtes SSH, favoris, scripts, extraits, notes, tâches, catalogue d'outils. Ils sont classés par le moteur flou.
2. **Calculés à la volée** : produits à chaque frappe à partir du texte tapé (`modules::query`, `Engine::organizer_query`) : calculs, conversions, dates, outils (`b64 …`), ports, Docker, GitHub, minuteurs. Ils passent avant les résultats indexés.

Les appels réseau (GitHub, Vercel, traduction) ne bloquent jamais la frappe : `cloud::cached` renvoie le cache ou une ligne « Chargement… », lance la requête dans un fil, puis l'interface est prévenue par l'événement `landry://refresh`.

### Ce qui se passe quand on valide un résultat

```
Entrée ──> commande activate(id, action, arg, query)
       ──> Engine::activate ──> modules::run (ou run_snippet, organizer_run)
       ──> Outcome
       ──> main.rs::apply_outcome (effets système)  +  App.svelte (affichage)
```

`Outcome` décrit la suite :

| Outcome | Effet |
| --- | --- |
| `Hide` | cacher la barre |
| `Keep` | garder la barre, relancer la recherche |
| `Message(texte)` | afficher un message dans la barre |
| `Copy(texte)` | copier puis cacher |
| `Paste { text, back }` | copier, cacher, coller dans l'application précédente (portail RemoteDesktop) |
| `Fill(texte)` | remplacer la saisie (catalogue d'outils) |
| `View(nom)` | ouvrir une vue de l'interface : `settings`, `settings:snippets`, `notes:12`, `diff` |
| `Ask(champs)` | demander des champs, puis rappeler `activate` avec leurs valeurs en JSON |
| `Settings` | ouvrir les réglages |
| `Pick` | pipette (portail Screenshot) |
| `Capture("save" ou "ocr")` | capture d'une zone, ou OCR |
| `Run { … }` | lancer un script hors du verrou du moteur |

### L'interface (Svelte 5, `src/`)

- **`App.svelte`** : la barre (saisie, résultats, aperçu, panneau d'actions, saisie d'argument) et le routage entre les vues.
- **`Settings.svelte`** et ses onglets : `GeneralSettings`, `Snippets`, `Accounts`. Vues `Notes.svelte` et `Diff.svelte`.
- **`lib/theme.js`** : la seule passerelle entre un preset et le CSS (variables CSS appliquées sur la racine).
- **`lib/i18n.svelte.js`** : `t('texte français', { variables })`.

Événements envoyés par le cœur à l'interface :

| Événement | Quand |
| --- | --- |
| `landry://shown` | la barre vient de s'afficher (remise à zéro, focus) |
| `landry://preset` | le preset actif a changé |
| `landry://theme` | GNOME est passé en clair ou en sombre |
| `landry://refresh` | des données sont arrivées (réseau, minuteur qui tourne) |
| `landry://notice` | message à afficher (sortie d'un script, OCR) |
| `landry://fill`, `landry://view`, `landry://open-settings` | piloter l'interface depuis le cœur ou la CLI |

### Fils d'arrière-plan du résident

- indexation des applications (icônes résolues sur le fil principal GTK) ;
- indexation des fichiers, projets, favoris, scripts, puis surveillance des dossiers (`notify`) ;
- `clock_loop` : minuteurs, rappels de tâches, rafraîchissement de la barre une fois par seconde ;
- `watch_theme` : suit `gsettings monitor` pour le mode clair ou sombre ;
- le serveur du socket Unix.

### Pourquoi la fenêtre a deux « couches »

Sous Wayland, une application ne peut pas placer sa propre fenêtre. Couche `screen` : une fenêtre transparente de la taille de l'écran, dans laquelle la barre est placée en CSS (position libre, environ 80 à 110 ms à l'ouverture). Couche `window` : une fenêtre à la taille de la barre, placée par GNOME (environ 45 ms). `fit_layer` dans `main.rs` applique ce choix. Les essais et mesures sont détaillés dans `STATUS.md`.

## 6. Arborescence

```
landry/
├── src/                      interface Svelte
│   ├── App.svelte            barre de recherche et routage des vues
│   ├── Settings.svelte       réglages : presets (formulaire, JSON, aperçu)
│   ├── GeneralSettings.svelte, Snippets.svelte, Accounts.svelte
│   ├── Notes.svelte, Diff.svelte
│   ├── i18n/en.json          dictionnaire anglais (interface et cœur)
│   └── lib/                  theme, i18n, markdown, contrast, pairs (+ tests)
├── src-tauri/
│   ├── tauri.conf.json       fenêtre, version, paquets
│   ├── Cargo.toml            dépendances Rust
│   └── src/
│       ├── main.rs           démarrage, commandes Tauri et socket, effets système
│       ├── engine.rs         moteur : index, recherche, activation
│       ├── search.rs         Item, Action, classement flou
│       ├── config.rs         settings.json et presets
│       ├── usage.rs          base SQLite
│       ├── organizer.rs      notes, tâches, minuteurs, suivi (côté moteur)
│       ├── i18n.rs           traduction du cœur
│       ├── ipc.rs            socket Unix et client CLI
│       ├── paste.rs          collage automatique (portail RemoteDesktop)
│       ├── backup.rs         sauvegarde et import
│       ├── doctor.rs         landry doctor
│       ├── apps.rs, paths.rs, util.rs
│       └── modules/
│           ├── mod.rs        aiguillage : query, run, actions, preview
│           ├── files.rs      fichiers, récents, favoris GTK, filtres
│           ├── projects.rs   dépôts git, scripts npm
│           ├── system.rs     ports, SSH, Docker
│           ├── cloud.rs      GitHub, Vercel, cache réseau, trousseau
│           ├── web.rs        préfixes, favoris navigateurs, traduction
│           ├── tools.rs      boîte à outils, QR codes
│           ├── color.rs      couleurs et contraste
│           ├── calc.rs       calculatrice, unités, devises BCE
│           ├── dates.rs      fuseaux, dates relatives, jours ouvrés
│           ├── money.rs      TVA, cotisations
│           ├── organize.rs   analyse des échéances, durées, CSV
│           ├── snippets.rs   variables des extraits
│           ├── text.rs       transformations de texte
│           └── scripts.rs    scripts utilisateur
├── dev/                      maquette de l'interface sans Tauri
├── install.sh                installation, mise à jour, désinstallation en une commande
├── scripts/                  install-shortcut, package
├── docs/                     ce guide, prompt d'origine
└── STATUS.md                 avancement, mesures, décisions
```

Données de l'utilisateur (hors dépôt) :

| Chemin | Contenu |
| --- | --- |
| `~/.config/landry/settings.json` | réglages globaux |
| `~/.config/landry/presets/*.json` | presets |
| `~/.config/landry/scripts/` | scripts utilisateur |
| `~/.local/share/landry/landry.db` | base SQLite |
| `~/.local/share/landry/ecb.xml` | cache des taux BCE |
| `~/.local/share/landry/remote-desktop.token` | jeton de restauration du collage automatique |
| trousseau du système, service `landry` | jetons GitHub, Vercel, traduction |

## 7. Recettes : modifier le projet

### Ajouter un mot-clé calculé (par exemple `dé 6`)

1. Dans `src-tauri/src/modules/tools.rs`, ajoute un cas à `query` : `("de" | "dé", false) => vec![copy(…)]`.
2. Ajoute une entrée au `CATALOG` pour qu'on le trouve en tapant « dé ».
3. Ajoute un test dans `mod tests` du même fichier.
4. Ajoute les textes français à `src/i18n/en.json` (le test `i18n` te dira lesquels).

### Ajouter un module complet

1. Crée `src-tauri/src/modules/mon_module.rs` avec au minimum `query(…) -> Vec<Item>` (résultats calculés) ou `scan(…) -> Vec<Item>` (résultats indexés), et `run(item, action, arg) -> Result<Outcome, String>`.
2. Déclare-le dans `modules/mod.rs` (`pub mod mon_module;`), puis branche-le dans `query`, `run` et éventuellement `preview`.
3. Ajoute son identifiant à `MODULES` dans `config.rs` : il apparaît alors dans la liste des modules de chaque preset, et l'utilisateur peut le désactiver ou le déplacer.
4. Pour un module indexé, appelle `engine(app).set_index("mon_module", …)` dans `index_others` (`main.rs`), et aussi dans `save_settings` s'il dépend des réglages ou contient du texte fixe à traduire.
5. Ajoute son libellé dans `MODULE_LABELS` (`App.svelte`) et `MODULE_NAMES` (`Settings.svelte`).
6. Écris ses tests, ajoute les traductions, et mets à jour le README (tableau « Dans la barre »).

Règles pour les items : `id` unique et stable (`monmodule:…`), `module` égal à l'identifiant du module, `data` pour les informations nécessaires à l'action (jamais envoyé à l'interface). La première action est celle de la touche Entrée. Une action destructrice prend `.danger()`, une action qui demande un texte prend `.ask("Invite", "valeur par défaut")`.

### Ajouter un réglage de preset

1. `config.rs` : ajoute le champ à la structure concernée (`Layout`, `Preview`, `Preset`…), sa valeur dans le `Default`, et ses limites dans `Preset::validate` si c'est un nombre ou une couleur.
2. `src/lib/theme.js` : expose-le en variable CSS dans `cssVars`, puis utilise cette variable dans le CSS de `App.svelte`.
3. `src/Settings.svelte` : ajoute le contrôle au formulaire.
4. Grâce à `#[serde(default)]`, les presets existants restent valides. Si tu changes le sens d'un champ existant, augmente `SCHEMA_VERSION` et ajoute la migration dans `Preset::parse`.
5. Tests : `config.rs` (validation) et `src/lib/theme.test.js`.

### Ajouter un réglage global

Ajoute le champ à `Settings` dans `config.rs` (avec sa valeur par défaut), puis le contrôle dans `GeneralSettings.svelte`. L'enregistrement passe par la commande `save_settings`.

### Ajouter une commande Tauri (appelée par l'interface)

1. Dans `main.rs` : écris `#[tauri::command] fn ma_commande(…)` puis ajoute-la à la liste `generate_handler![…]`. Si elle fait un appel réseau ou attend quelque chose, déclare-la `async` : les commandes synchrones tournent sur le fil principal et figeraient l'interface.
2. Dans l'interface : `await invoke('ma_commande', { monArgument })`. Les noms d'arguments passent en camelCase côté JS.
3. Ajoute-la à `dev/tauri-mock.js` pour que la maquette continue de marcher.

### Ajouter une commande à la CLI

Ajoute un cas au `match` de `handle` dans `main.rs`. La réponse est une seule ligne : `ok …` en cas de succès, `err …` en cas d'échec. Pense à compléter le message « Commandes : … » et le tableau du README.

### Traduire un texte

- Interface : `{t('Texte en français')}`, ou `t('Bonjour {nom}', { nom })` avec une variable.
- Cœur : `t("Texte")` ou `tf("Bonjour {nom}", &[("nom", &nom)])`.
- Ajoute la paire dans `src/i18n/en.json` : la clé est le texte français exact, la valeur l'anglais.
- Les titres d'actions (`Action::new("id", "Titre")`) sont traduits automatiquement à la sortie du moteur : seule l'entrée du dictionnaire est nécessaire.
- Les tests `i18n` signalent toute clé manquante.

### Changer l'apparence par défaut

Les presets livrés sont dans `shipped_presets()` (`config.rs`), le preset intégré dans `impl Default for Preset`. Ils ne sont copiés dans `~/.config/landry/presets/` qu'au tout premier démarrage : pour voir un changement, supprime ce dossier ou utilise un `XDG_CONFIG_HOME` de test. Le contrôle des contrastes de l'éditeur de presets doit rester au-dessus de 4,5:1.

### Ajouter un script personnel

Pas besoin de toucher au code : dépose un fichier dans `~/.config/landry/scripts/` (voir le README). Il apparaît tout de suite dans la barre.

## 8. Paquets et version

1. Change la version aux trois endroits : `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Construis :
   ```bash
   scripts/package.sh
   ```
   - Avec `libayatana-appindicator3-dev` installé : `.deb` et AppImage par le bundler Tauri.
   - Sinon : `.deb` construit directement avec `dpkg-deb`, sans AppImage.
3. Installe ou mets à jour :
   ```bash
   sudo apt install ./dist-packages/landry_<version>_amd64.deb
   landry quit; landry toggle      # relance le résident avec la nouvelle version
   ```

Le build release (LTO) prend environ 3 minutes. Les paquets sont écrits dans `dist-packages/`, ignoré par git.

## 9. Dépannage

| Symptôme | Cause probable et solution |
| --- | --- |
| `landry daemon` : « un processus résident Landry tourne déjà » | un autre résident tient le socket : `landry quit` (ou celui de la version installée) |
| fenêtre vide en développement | binaire construit sans `--features custom-protocol` et Vite arrêté : utilise `npx tauri dev -- -- daemon` |
| `npx tauri build` : « Can't detect any appindicator library » | `libayatana-appindicator3-dev` manque : `./install.sh`, ou `scripts/package.sh` qui sait s'en passer pour le `.deb` |
| `install.sh` : erreur de dépôt pendant `apt-get update` | dépôt tiers cassé, sans gravité ; pour le retirer : `sudo add-apt-repository --remove ppa:<nom>` |
| le raccourci n'ouvre rien | `landry doctor` ; vérifie le chemin passé à `scripts/install-shortcut.sh` |
| la barre ne prend pas le focus | lance `LANDRY_TIMING=1 landry show` et regarde `focused=` ; note le résultat dans `STATUS.md` |
| le collage automatique ne colle pas | refus du portail ou jeton non réutilisé par GNOME : supprime `~/.local/share/landry/remote-desktop.token` et réessaie ; l'extrait reste dans le presse-papiers |
| « Chargement… » qui ne disparaît pas | erreur réseau ou jeton invalide : relance la recherche après quelques secondes, vérifie l'onglet Comptes |
| un preset modifié à la main n'est pas pris | JSON invalide : la barre affiche le message de validation et revient au preset Défaut |

Pour voir les journaux du résident, lance-le dans un terminal : `landry quit && landry daemon`.

## 10. Conventions

- **Branches** : le travail se fait sur `dev`, jamais directement sur `main`.
- **Commits** : auteur `Landry-16`, messages en anglais à l'impératif, sans emoji ni tiret cadratin, et un commit par étape qui compile et passe les tests.
- **Code** : simple, sans abstraction pour un usage futur. Commentaires seulement pour expliquer un pourquoi, en une ligne. Pas de `unwrap()` sur une entrée utilisateur ou un fichier externe.
- **Tests** : toute logique non triviale (analyse, calcul, conversion, validation) a au moins un test.
- **Textes** : en français dans le code, avec leur traduction dans `src/i18n/en.json`.
- **Valeurs visuelles** : jamais codées en dur ; elles viennent du preset, via `theme.js`.
- **Secrets** : uniquement dans le trousseau, jamais dans un fichier, un log ou une sauvegarde.
- **`STATUS.md`** : à mettre à jour avec les décisions et les mesures quand elles changent.
