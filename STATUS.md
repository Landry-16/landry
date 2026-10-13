# STATUS

Phase courante : **Phase 1, cœur**

## Phase 0 : focus et raccourci
- [x] Binaire unique `landry` : `landry daemon` (processus résident), `landry [toggle|show|hide|ping|quit]` (CLI)
- [x] Socket Unix `$XDG_RUNTIME_DIR/landry.sock`, la CLI démarre le résident s'il ne tourne pas
- [x] Fenêtre sans décoration, transparente, au premier plan, cachée au départ
- [x] Jeton d'activation (`XDG_ACTIVATION_TOKEN` / `DESKTOP_STARTUP_ID`) transmis au résident et passé à GTK (`set_startup_id`) s'il existe
- [x] Mesure du délai et du focus renvoyée par le résident (`LANDRY_TIMING=1 landry show`)
- [ ] Vérification par l'utilisateur via le vrai raccourci GNOME (`scripts/install-shortcut.sh`)

## Phase 1 : cœur
- [x] Configuration et presets (schéma versionné, Défaut intégré, Minimal, Dense, Développeur, repli sur Défaut si invalide)
- [x] Thème piloté par la config (variables CSS), suivi du mode clair ou sombre du système (`gsettings monitor`)
- [x] Indexeur d'applications (.desktop XDG, Flatpak, Snap) avec icônes du thème GTK
- [x] Recherche floue (nucleo), sans accents, par initiales
- [x] Historique d'usage SQLite (fréquence, récence, heure), classement
- [x] Interface : liste de résultats, aperçu, navigation clavier, Échap, Tab, historique des requêtes (flèches)
- [x] Lancement d'application (`gio launch`)
- [x] Commandes `landry run <texte>`, `landry preset list|show|use <nom>`, `landry reload`
- [x] Écran de réglages avec aperçu en direct, formulaire, édition JSON validée, dupliquer, renommer, supprimer, importer, exporter
- [x] Alias (settings.json, `aliases`) : l'élément ciblé passe en tête
- [ ] Vérification visuelle sur le vrai bureau (seulement vérifié dans un navigateur avec une maquette de l'API Tauri, `dev/mock.html`)

## Prochaine étape
Phase 2, étape 1 : module `src-tauri/src/files.rs` (index des dossiers choisis dans settings.json `file_roots`, surveillance `notify`, filtres `img:` `pdf:` et date), branché comme module `files` dans `engine.rs::candidates`. Puis panneau d'actions secondaires (Ctrl+K) dans `App.svelte`, nécessaire aux actions sur fichiers.

## Résultats mesurés (build debug, 2026-10-13)
- Lancement depuis un processus non focalisé (shell de l'agent) : la fenêtre obtient le focus.
- Première ouverture (webview froide) : 143 ms jusqu'au rendu.
- Ouvertures suivantes : rendu en 16 à 26 ms, focus confirmé en 36 à 54 ms.
- Pas encore testé : déclenchement par le raccourci GNOME réel, et build release.

## Couche d'affichage (mesuré, build debug, écran 2880x1620 à 125 %)
Sous Wayland une application ne peut pas placer sa fenêtre. Deux modes, choisis par preset (`layout.layer`) :
- `screen` (défaut) : fenêtre transparente de la taille de l'écran, barre placée en CSS (ancre, marge, décalages). Ouverture 68 à 116 ms.
- `window` (preset Minimal) : fenêtre de la taille de la barre, placée par GNOME (`center-new-windows` vaut false ici, donc position non garantie). Ouverture 32 à 43 ms.
Essayé sans gain : état plein écran (plus lent), vue web réduite dans une grande fenêtre GTK (même coût : c'est la surface Wayland qui coûte), modes de rendu WebKit (plus lents).
Première ouverture après démarrage : 270 à 780 ms (vue web froide).

## Décisions
- Un seul binaire pour la CLI et le résident (plus simple à installer et à mettre à jour).
- Interface en JavaScript + Svelte 5, sans TypeScript (moins d'outillage).
- Tauri 2.12 stable (crates.io propose une 3.0 alpha, écartée).
- La barre se cache quand elle perd le focus. `LANDRY_KEEP_OPEN=1` désactive ce comportement pour le développement.
- Raccourci par défaut proposé : `<Control><Alt>space` (Super+Espace sert déjà à changer de disposition clavier dans GNOME).
- Alias, préfixes de recherche et langue dans `settings.json` (global) ; apparence, disposition et modules dans les presets.
- Couleurs validées en hexadécimal uniquement (pas d'injection CSS possible depuis un preset).
- Import et export sans boîte de dialogue : export dans le dossier Téléchargements, import par chemin saisi.
- Textes de l'interface en dur en français pour l'instant ; extraction en dictionnaires FR/EN prévue en phase 6.
- Email de commit validé par l'utilisateur : `sandes.savarimuthu@epitech.eu`, auteur `Landry-16`.

## Problèmes ouverts
- Dans le bac à sable de l'agent : `~/.cargo` et `~/.npm` sont en lecture seule, `.git/config` est verrouillé, et le socket dans `$XDG_RUNTIME_DIR` est interdit. Contournements : `CARGO_HOME=$TMPDIR/cargo-home`, `npm_config_cache=$TMPDIR/npm-cache`, `git -c user.name=... -c user.email=...`, lancement du résident hors bac à sable. Rien de tout cela ne concerne une utilisation normale.
- `libayatana-appindicator3-dev` et `librsvg2-dev` absents (icône de barre système, paquets) ; `tesseract-ocr` absent (OCR). Voir `scripts/install-deps.sh`.

## Commandes
```
npm install
npm run build                                        # interface dans dist/
cd src-tauri && cargo build --features custom-protocol
cargo test                                           # depuis src-tauri/
npm test                                             # tests de l'interface
npx vite --port 1420                                 # puis http://localhost:1420/dev/mock.html (interface sans Tauri)
LANDRY_TIMING=1 src-tauri/target/debug/landry show   # mesure
```
