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
- [ ] Configuration et presets (schéma versionné, Défaut intégré, Minimal, Dense, Développeur, repli sur Défaut si invalide)
- [ ] Thème piloté par la config (variables CSS), suivi du mode clair ou sombre du système
- [ ] Indexeur d'applications (.desktop XDG, Flatpak, Snap)
- [ ] Recherche floue (nucleo), sans accents, par initiales
- [ ] Historique d'usage SQLite (fréquence, récence, heure), classement
- [ ] Interface : liste de résultats, navigation clavier, Échap, Tab, historique des requêtes
- [ ] Lancement d'application
- [ ] Commandes `landry run <cmd>` et `landry preset use <nom>`
- [ ] Écran de réglages avec aperçu, édition JSON validée

## Prochaine étape
Phase 1, étape 1 : module `src-tauri/src/config.rs` (struct `Preset`, chargement depuis `~/.config/landry/presets/`, presets intégrés, validation, repli sur Défaut), avec tests.

## Résultats mesurés (build debug, 2026-10-13)
- Lancement depuis un processus non focalisé (shell de l'agent) : la fenêtre obtient le focus.
- Première ouverture (webview froide) : 143 ms jusqu'au rendu.
- Ouvertures suivantes : rendu en 16 à 26 ms, focus confirmé en 36 à 54 ms.
- Pas encore testé : déclenchement par le raccourci GNOME réel, et build release.

## Décisions
- Un seul binaire pour la CLI et le résident (plus simple à installer et à mettre à jour).
- Interface en JavaScript + Svelte 5, sans TypeScript (moins d'outillage).
- Tauri 2.12 stable (crates.io propose une 3.0 alpha, écartée).
- La barre se cache quand elle perd le focus. `LANDRY_KEEP_OPEN=1` désactive ce comportement pour le développement.
- Raccourci par défaut proposé : `<Control><Alt>space` (Super+Espace sert déjà à changer de disposition clavier dans GNOME).
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
LANDRY_TIMING=1 src-tauri/target/debug/landry show   # mesure
```
