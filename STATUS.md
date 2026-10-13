# STATUS

Phase courante : **toutes les phases livrées ; restent les vérifications manuelles ci-dessous**

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

## Phase 2 : fichiers, outils développeur, boîte à outils
- [x] Actions secondaires (Ctrl+K), saisie d'argument, confirmation en deux temps des actions destructrices, messages, copie (presse-papiers GTK), aperçu à la demande (texte, image, propriétés)
- [x] Fichiers : index des dossiers XDG (ou `file_roots`), récents GTK, favoris GTK, surveillance `notify` (réindexation après 2 s de calme), filtres `img:` `pdf:` `doc:` `vid:` `audio:` `code:` `zip:` `dir:` et `<7j` `<2s` `<3m` `<1a`
- [x] Actions fichiers : ouvrir, afficher dans le dossier, terminal ici, copier le chemin, renommer, déplacer, compresser (.zip), corbeille, ouvrir avec (applications recommandées)
- [x] Projets git sous `projects_root` : branche, état (modifiés, retard, avance), éditeur, terminal, git pull, dépôt en ligne, ouvrir une PR, scripts npm
- [x] Ports occupés (`port`, `port 5173`), arrêt du processus ; hôtes SSH ; Docker (`docker`) : démarrer, arrêter, redémarrer, logs, shell
- [x] GitHub (`pr`, `ci`, `issues`) et Vercel (`vercel`) : jetons dans le trousseau, chargement en arrière-plan, cache 60 s, redéploiement confirmé
- [x] Boîte à outils : uuid, b64, b64d, url, urld, json, jsonmin, sha256, sha1, md5, jwt, ts, regex, couleurs (#hex, rgb, hsl, oklch), contraste, pipette (portail), comparer deux textes
- [x] Réglages : onglets Général (dossiers, projets, éditeur, terminal, alias, préfixes, équipe Vercel) et Comptes (jetons)
- [x] `landry search <texte>` dans la CLI
- [ ] Non testé avec de vrais jetons GitHub et Vercel (aucun jeton sur la machine) ; pipette et presse-papiers non testés à la main

## Phase 3 : texte, presse-papiers, calcul
- [x] Calculatrice `fend-core` : virgule décimale française, pourcentages, unités (en, vers, Mo, Gio...), limite de 150 ms
- [x] Devises : taux BCE quotidiens, cache `~/.local/share/landry/ecb.xml` (rafraîchi toutes les 12 h, utilisé hors ligne)
- [x] Fuseaux (« 15h Paris en New York », « heure à Tokyo »), dates relatives (« dans 3 semaines »), jours et jours ouvrés (fériés français, Pâques calculé)
- [x] Montants : « 100 ht », « 120 ttc » (TVA des réglages), « ca 3000 » (taux micro-entreprise à saisir dans Général)
- [x] Extraits (SQLite) avec {date}, {heure}, {date:format}, {presse-papiers}, {curseur}, {champ:Nom} demandés à l'insertion ; onglet Extraits dans les réglages
- [x] Collage automatique : session RemoteDesktop courte par collage, jeton de restauration dans `~/.local/share/landry/remote-desktop.token`, repli sur copie + notification
- [x] Transformations de la sélection (sélection primaire lue à l'ouverture) : maj, min, titre, brut, typo, mots
- [ ] À vérifier par l'utilisateur : la boîte de dialogue RemoteDesktop au premier collage, le collage réel dans une autre application, et si GNOME 46 réutilise bien le jeton (sinon la demande revient à chaque collage)

## Phase 4 : organisation
- [x] Notes Markdown (SQLite) : vue avec liste, recherche, édition, aperçu (rendu qui échappe le HTML), enregistrement automatique ; `note <texte>` crée une note ; notes indexées dans la recherche
- [x] Tâches : `todo <texte> [échéance]` (demain 10h, vendredi, le 24/10 14h, dans 30 min, 18h), liste, faite, suppression ; rappel par notification
- [x] Minuteur (`minuteur 10 min pâtes`), chronomètre (`chrono`), Pomodoro 25/5 en boucle ; affichés en tête de la barre à l'ouverture
- [x] Suivi du temps : `suivi <client>`, `suivi stop`, totaux du jour, export CSV dans Téléchargements
- [x] Boucle d'une seconde dans le résident (minuteurs, rappels, rafraîchissement de la barre si un minuteur tourne)

## Phase 5 : web et captures
- [x] Recherches par préfixe (`gh texte`, `wiki`, ajoutables dans Général), URL tapée ouverte directement
- [x] Favoris Firefox (dont Firefox Snap, lus sur une copie de places.sqlite) et Chrome, Chromium, Brave
- [x] Traduction `tr [code] texte` : DeepL (clé dans le trousseau) ou serveur LibreTranslate, en arrière-plan avec cache
- [x] Capture d'une zone (portail Screenshot interactif), image copiée ; OCR Tesseract (fra+eng si présent) avec texte affiché et copié
- [x] QR codes (`qr texte`) avec aperçu et enregistrement SVG
- [ ] Non testé à la main : capture et OCR (Tesseract absent de la machine), traduction sans clé

## Phase 6 : finitions
- [x] Scripts utilisateur (`~/.config/landry/scripts`), en-têtes `@landry`, rechargement à chaud, sortie dans la barre, limite de 60 s
- [x] `activate` et `preview` asynchrones : plus de gel de l'interface pendant un appel réseau
- [x] Sauvegarde complète (réglages, presets, extraits, notes ; jamais les jetons) et import fusionnant sans suppression
- [x] Démarrage avec la session (entrée XDG autostart) et icône de barre système (Ouvrir, Réglages, Quitter)
- [x] Interface FR/EN (dictionnaire `src/i18n/en.json`, test de couverture), langue choisie dans Général
- [x] Accessibilité : rôles ARIA, focus visible, taille de police par preset, mouvement réduit, contrôle des contrastes dans l'éditeur, accent clair corrigé (4,31 vers 5,27:1)
- [x] Paquet .deb (`scripts/package.sh`, vérifié) ; AppImage via le bundler Tauri une fois `libayatana-appindicator3-dev` installé
- [x] README

## Phase 7 : direction artistique, commandes « / », confort
- [x] Direction artistique dans Figma (« Landry - Direction artistique », équipe Sandes Educationnal Team) : manifeste, vouivre et ses 9 réactions animées, fiche de comportement, presets
- [x] Bloc `style` des presets (compagnon, ornement, particules, couleur secondaire), presets Rose noire, Soie, Fresque, Pixel ; les nouveaux presets livrés s'ajoutent à une installation existante (`presets/.shipped`)
- [x] Vouivre dans la barre (arrivée, repos, écoute, désigne, perdue, joie, erreur, sommeil), ornements et particules, groupe Style dans les réglages
- [x] Commandes « / » : palette, Spotify, média, volume, luminosité, thème, ne pas déranger, wifi, session, panneaux GNOME, processus, météo, aide ; les mots-clés existants marchent avec « / »
- [x] Accessibilité et confort : nombre de résultats annoncé aux lecteurs d'écran, aide F1, zoom Ctrl+plus/moins, pied de raccourcis, indications sur barre vide, couleurs forcées, Entrée deux fois pour une action dangereuse
- [ ] Non testé sur le vrai bureau : la vouivre et les ornements dans la vraie fenêtre (vérifiés dans la maquette `dev/mock.html`), `/spotify` avec l'application installée, `/luminosite` sur un portable, `/media` (playerctl absent de la machine)

## Phase 8 : idées reprises de Weft (PhilibertG/Weft)
- [x] Assistant de premier lancement : raccourci saisi au clavier, démarrage avec la session, preset en un clic, outils facultatifs manquants avec la commande à copier ; idempotent ; `/bienvenue`, `landry setup`
- [x] Service systemd utilisateur (`Restart=on-failure`), repli sur l'entrée XDG autostart, migration de l'ancienne entrée ; un second résident sort avec le code 0 (pas de boucle de redémarrage) ; la CLI démarre le résident par systemd quand le service existe
- [x] Index des applications mis à jour à l'installation et à la désinstallation (surveillance des dossiers `applications`, 2 s de calme)
- [x] Désinstaller une application Flatpak (`--user` ou `--system` selon l'emplacement) ou Snap (`pkexec snap remove`) depuis Ctrl+K, avec confirmation ; limite de 15 min pour laisser le temps du mot de passe
- [ ] Non testé sur le vrai bureau : enregistrement du raccourci et activation du service depuis l'assistant (pas de modification de la configuration GNOME de l'utilisateur pendant le développement), désinstallation réelle
- Écartés pour l'instant : jeux Steam (demandé plus tard), moteur Windows de Weft (projet à part entière), extension GNOME pour le presse-papiers et les fenêtres (proposée, pas encore retenue)

## Phase 9 : robustesse, alias, installation en une commande (0.3.0)
- [x] La barre ne se rouvrait pas tant que Spotify tournait : `/spotify` lançait `gio open` par `util::output`, qui attendait la fermeture des tubes ; Spotify, démarré par gio, les héritait. `util::output` n'attend plus que le programme, avec une durée maximale (2 min, 10 s pour les actions « / », 2 s pour les lectures d'état)
- [x] Ouverture de la barre sans attendre le verrou du moteur ; commandes appelées par la barre passées en async (Tauri exécute les commandes synchrones sur le fil principal)
- [x] Lecture de la sélection asynchrone (un propriétaire de sélection qui ne répond pas bloquait la boucle GTK)
- [x] Assistant bloqué sur « Vérification… » : le contrôle du trousseau paniquait dans le runtime async de Tauri ; contrôles déplacés sur un fil ordinaire
- [x] Alias de commandes complètes et alias suivis d'arguments
- [x] `./install.sh` : paquets système manquants, Rust, Node 22 vérifié par somme de contrôle si besoin, construction, installation (`apt --reinstall`, paquets recommandés compris), redémarrage, `landry doctor` ; `--uninstall` ; `LANDRY_DRY_RUN=1`
- [x] Module « / » désactivable par preset ; lecture de la sélection désactivable (Général)
- [ ] À vérifier : `./install.sh` sur une machine neuve (testé ici en mode `LANDRY_DRY_RUN`, construction réelle)

## À vérifier par l'utilisateur
1. Le vrai raccourci GNOME (`scripts/install-shortcut.sh`) : la barre doit prendre le focus.
2. Le collage automatique d'un extrait (boîte de dialogue RemoteDesktop au premier essai, puis plus de question si GNOME accepte le jeton).
3. Pipette, capture et OCR (installer `tesseract-ocr tesseract-ocr-fra`).
4. GitHub, Vercel et traduction avec de vrais jetons.
5. L'AppImage avec `scripts/package.sh` (dépendances installées par `./install.sh`).

## Prochaine étape
Aucune phase restante. Pistes : extensions TypeScript, intégration IA prévue plus tard. Écartés pour Wayland : historique du presse-papiers (GNOME ne laisse pas lire le presse-papiers en arrière-plan) et liste des fenêtres (pas d'API sans extension GNOME Shell).

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
Première ouverture après démarrage : 200 à 1000 ms (vue web froide) ; la fenêtre part en 1920x1080 puis s'ajuste à l'écran (GTK n'applique la taille qu'à l'affichage).
Build release, ouvertures suivantes : couche `screen` 82 à 110 ms, couche `window` 43 à 62 ms.

## Décisions
- Un seul binaire pour la CLI et le résident (plus simple à installer et à mettre à jour).
- Interface en JavaScript + Svelte 5, sans TypeScript (moins d'outillage).
- Tauri 2.12 stable (crates.io propose une 3.0 alpha, écartée).
- La barre se cache quand elle perd le focus. `LANDRY_KEEP_OPEN=1` désactive ce comportement pour le développement.
- Raccourci par défaut proposé : `<Control><Alt>space` (Super+Espace sert déjà à changer de disposition clavier dans GNOME).
- Alias, préfixes de recherche et langue dans `settings.json` (global) ; apparence, disposition et modules dans les presets.
- Couleurs validées en hexadécimal uniquement (pas d'injection CSS possible depuis un preset).
- Import et export sans boîte de dialogue : export dans le dossier Téléchargements, import par chemin saisi.
- Alias : la cible est soit l'identifiant d'un élément (mis en tête), soit un texte qui remplace la requête.
- Les résultats dont le score est inférieur au tiers du meilleur sont écartés (moins de bruit).
- Mots-clés `pr`, `ci`, `issues`, `vercel` pour les services en ligne ; `gh` reste réservé à la recherche web de la phase 5.
- Actions système (verrouiller, volume...) et ouverture des panneaux de réglages GNOME : absentes de la liste du prompt, non faites.
- Calculatrice appelée seulement si dates et montants n'ont rien compris (sinon « 15h » devient 15 heures).
- Taux de cotisations jamais par défaut : l'utilisateur les saisit (ils changent chaque année).
- Collage : nouvelle session RemoteDesktop à chaque fois, pour ne pas laisser l'indicateur de contrôle à distance de GNOME allumé en permanence.
- Minuteurs enregistrés en base : ils survivent à un redémarrage ; un minuteur échu pendant l'arrêt est notifié au redémarrage.
- Sous un résultat calculé (calcul, outil), seules les correspondances textuelles nettes restent affichées.
- Interface et cœur traduits : un seul dictionnaire `src/i18n/en.json` (clé = texte français), lu par Svelte (`t()`) et par Rust (`i18n::t`, `i18n::tf`) ; titres d'actions traduits à la sortie du moteur ; index reconstruits à l'enregistrement des réglages. Restent en français : la CLI (`landry doctor`, réponses de `landry …`) et les journaux.
- L'assistant de premier lancement s'ouvre tant qu'il n'a pas été fermé une fois (Échap ou « C'est parti »), marqueur `~/.config/landry/.setup-done`.
- Démarrage avec la session : le service est seulement activé, pas démarré, quand un résident tourne déjà ; systemd prend le relais à la connexion suivante.
- Commandes « / » : une commande inconnue redevient la recherche sans la barre oblique, pour ne rien déclarer deux fois. Les actions instantanées et risquées (veille, arrêt de processus) demandent deux fois Entrée ; extinction, redémarrage et déconnexion passent par la confirmation de GNOME.
- La vouivre vit dans le champ de saisie ; les ornements débordent du cadre mais ne couvrent jamais les résultats.
- Email de commit validé par l'utilisateur : `sandes.savarimuthu@epitech.eu`, auteur `Landry-16`.

## Problèmes ouverts
- Dans le bac à sable de l'agent : `~/.cargo` et `~/.npm` sont en lecture seule, `.git/config` est verrouillé, et le socket dans `$XDG_RUNTIME_DIR` est interdit. Contournements : `CARGO_HOME=$TMPDIR/cargo-home`, `npm_config_cache=$TMPDIR/npm-cache`, `git -c user.name=... -c user.email=...`, lancement du résident hors bac à sable. Rien de tout cela ne concerne une utilisation normale.
- `libayatana-appindicator3-dev` et `librsvg2-dev` absents (icône de barre système, paquets) ; `tesseract-ocr` absent (OCR). `./install.sh` les installe.

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
