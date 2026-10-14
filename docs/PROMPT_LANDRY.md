# Prompt : Landry, compagnon du quotidien pour Ubuntu GNOME

Tu es l'agent de développement de ce projet. Lis ce document en entier avant de toucher au code.

Le projet vit à la racine du dépôt `/home/gemini/dev/perso/landry/`. Le fichier `docs/PROMPT_LANDRY.md` est ce prompt : ne le modifie pas.

## 0. Règles non négociables

- **Plateforme cible** : Ubuntu 24.04, GNOME 46, session Wayland. Windows n'est PAS dans le périmètre : n'écris aucun code Windows et pas de couche d'abstraction plateforme pour l'instant.
- **Git** : travaille uniquement sur la branche `dev`, créée depuis `main`. Ne touche jamais à `main`. Ne pousse pas sur `origin` sans demande explicite de l'utilisateur.
- **Auteur des commits** : Landry-16. Vérifie `git config user.name`. S'il n'est pas `Landry-16`, configure-le localement dans le dépôt (`git config user.name Landry-16`). Email validé par l'utilisateur : `sandes.savarimuthu@epitech.eu`. Si `.git/config` n'est pas modifiable, passe l'identité à chaque commit : `git -c user.name=Landry-16 -c user.email=sandes.savarimuthu@epitech.eu commit ...`. Ne mets jamais Claude comme auteur, et aucune ligne `Co-Authored-By` mentionnant Claude.
- **Style des commits et documents** : pas d'emoji, pas de tiret cadratin. Ne committe aucun fichier PDF.
- **Commits fréquents** : un commit par étape terminée. Le build et les tests doivent passer à chaque commit.
- **Secrets** : aucun jeton (GitHub, Vercel, clé API) dans le dépôt. Ils passent par le trousseau système (crate `keyring`).
- **Sudo** : ne lance jamais `sudo` toi-même. Les dépendances système sont listées dans `scripts/install-deps.sh`, que l'utilisateur exécutera lui-même.
- **Pas de questions en cours de route** : si un choix n'est pas tranché ici, prends le plus simple, note-le dans `STATUS.md` sous « Décisions », et continue.
- **Fichier `.mcp.json`** à la racine du dépôt : il n'appartient pas au projet. Ne le lis pas et ne le committe pas.

## 1. Le produit

Landry est un compagnon du quotidien sur le bureau. Un raccourci clavier ouvre en moins de 50 ms une barre centrée. On tape quelques lettres pour lancer une application, ouvrir un projet, calculer, convertir, chercher un fichier ou lancer une action. Tout reste local.

Nom du projet : **Landry**. Binaire : `landry`. Le nom affiché est configurable.

## 2. Stack imposée

- **Tauri 2** : cœur Rust, interface web en **Svelte 5**. Sous Linux, le moteur est WebKitGTK 4.1.
- **Crates Rust** :
  - `ashpd` pour les portails (RemoteDesktop, Screenshot)
  - `nucleo` pour la recherche floue
  - `fend-core` pour la calculatrice et les unités
  - `rusqlite` pour la base
  - `notify` pour surveiller les dossiers
  - `keyring` pour les secrets
  - `notify-rust` pour les notifications
  - `chrono` et `chrono-tz` pour les dates
  - `serde` et `serde_json` pour la configuration
  - `reqwest` pour GitHub, Vercel, la BCE et la traduction
- Vérifie les versions actuelles de chaque crate et paquet npm avant de les ajouter. Aucune version inventée. Préfère les versions stables.
- **Tests** : `cargo test` pour le cœur, Vitest pour l'interface. Pas d'autre framework sans raison.

## 3. Architecture

- **Processus résident** : démarré avec la session (fichier `.desktop` dans `~/.config/autostart`). Fenêtre cachée au départ.
- **CLI `landry`** : `landry toggle`, `landry run <commande>`, `landry preset use <nom>`. Elle parle au processus résident par socket Unix dans `$XDG_RUNTIME_DIR`. Si le processus ne tourne pas, la CLI le démarre.
- **Raccourci global** : raccourci personnalisé GNOME (`org.gnome.settings-daemon.plugins.media-keys custom-keybindings`, via gsettings) qui exécute `landry toggle`. Le portail GlobalShortcuts n'existe pas sur GNOME 46 : ne l'utilise pas.
- **Indexeurs en arrière-plan** :
  - applications : `.desktop` XDG, Flatpak, Snap
  - fichiers des dossiers choisis, via `notify` (inotify)
  - dépôts git sous `~/dev`
  - favoris Firefox et Chrome : copie temporaire de `places.sqlite`, lecture seule. Ne jamais toucher la base d'origine.
  - hôtes SSH depuis `~/.ssh/config`
- **Base SQLite** dans `~/.local/share/landry/` : historique d'usage (fréquence, récence, heure), extraits, notes, tâches, suivi du temps, cache des devises.
- **Configuration** dans `~/.config/landry/` (section 5).

## 4. Phase 0 : le test qui décide de tout (à faire en premier)

Prototype minimal : processus résident, et `landry toggle` ouvre la fenêtre au premier plan avec le focus, déclenché par le raccourci GNOME personnalisé, sans fenêtre déjà active.

Mesure le délai entre la commande et l'affichage. Objectif : moins de 50 ms. Note le chiffre réel dans `STATUS.md`.

Si la fenêtre n'obtient pas le focus, essaie dans cet ordre et note chaque résultat dans `STATUS.md` :
1. `present` / `set_focus` de Tauri, avec `always_on_top` temporaire
2. lancer le cœur sous XWayland (`GDK_BACKEND=x11`)
3. toute autre piste documentée

Si aucune ne marche proprement : arrête le projet à cette phase. Dans `STATUS.md`, décris ce qui a été testé et les options restantes, puis termine. Ne construis pas le reste sur un focus cassé.

## 5. Personnalisation et presets (exigence centrale)

**La barre est entièrement personnalisable.** Toute l'apparence et le comportement sont pilotés par la configuration. Aucune valeur n'est codée en dur :
- position : ancre, marges, décalage
- dimensions : largeur, hauteur, rayon
- couleurs (tokens), police, taille, densité, opacité
- animation : durée, ou désactivée
- nombre de résultats affichés
- panneau d'aperçu : visible ou non, côté
- ordre et visibilité des modules dans la barre
- raccourci par commande, alias, préfixes de recherche
- modules actifs

Les couleurs passent par des variables CSS générées depuis la configuration. Modes clair, sombre et « suivre le système » (`org.gnome.desktop.interface color-scheme`).

**Presets** :
- fichiers JSON dans `~/.config/landry/presets/`
- un preset « Défaut » intégré, impossible à supprimer
- au moins trois presets livrés (par exemple Minimal, Dense, Développeur)
- changer de preset depuis la barre (commande « preset ») et via `landry preset use <nom>`, sans redémarrage
- créer, dupliquer, renommer, supprimer, importer, exporter (un fichier par preset)

**Édition** :
- un écran de réglages avec aperçu en direct
- édition du JSON brut, avec validation du schéma
- une configuration invalide n'empêche jamais la barre de s'ouvrir : retour au preset Défaut et message clair
- schéma versionné (champ `version`) pour que les futures versions migrent les presets

## 6. Fonctionnalités à livrer (périmètre strict)

**Hors périmètre pour l'instant** : toute IA (intégration future), Windows, extensions TypeScript, catalogue, expansion de texte au clavier, liste et placement des fenêtres, historique du presse-papiers, portail GlobalShortcuts.

### Lancement
- Recherche floue sans accents, par initiales (« vsc » trouve VS Code)
- Classement par fréquence, récence et heure de la journée
- Alias, raccourci dédié par commande, commandes avec arguments (« deploy nacre »)
- Historique des requêtes (flèche du haut), navigation clavier complète, Échap pour revenir, Tab pour compléter
- Panneau d'actions secondaires (Ctrl+K) et aperçu à droite

### Fichiers
- Recherche dans les dossiers choisis, index à jour en direct, filtres `img:` `pdf:` et par date
- Dossiers récents et favoris
- Actions : ouvrir, afficher dans le dossier, copier le chemin, renommer, déplacer, compresser, ouvrir avec une autre application

### Développeur
- Projets git sous `~/dev` : branche courante, état (modifié, en retard sur origin). Actions : ouvrir dans VS Code, ouvrir un terminal, `git pull`, ouvrir sur GitHub
- Scripts npm (`dev`, `build`)
- Ports occupés : voir le processus, le tuer
- Hôtes SSH : connexion en un geste
- Docker : liste, démarrer, arrêter, logs (CLI `docker`, sortie JSON)
- GitHub (jeton dans le trousseau) : mes PR, issues assignées, état de la CI, ouvrir une PR
- Vercel (jeton dans le trousseau) : derniers déploiements par projet, état, redéployer, ouvrir les logs
- Boîte à outils : formater JSON, Base64 et URL, UUID, hachage, décodage JWT, conversion de timestamp, test de regex, diff de deux textes, couleurs (HEX, RGB, HSL, OKLCH, contraste WCAG)
- Pipette couleur via le portail Screenshot

### Texte et presse-papiers
- Extraits avec variables : date du jour, presse-papiers, curseur, champ à remplir à l'insertion
- Collage automatique via le portail RemoteDesktop (autorisation demandée une fois). Sans autorisation, l'extrait est seulement copié
- Transformations du texte sélectionné : majuscules, casse de titre, retirer la mise en forme, compter les mots, espaces insécables avant « : ; ? ! » en français

### Calcul et conversion
- Calculatrice (`fend-core`)
- Unités : longueurs, poids, températures, données (Mo et Gio), vitesses
- Devises avec les taux BCE quotidiens, mis en cache pour le hors ligne
- Dates et fuseaux : « 15h Paris en New York », « dans 3 semaines », jours ouvrés entre deux dates
- Montants : HT vers TTC, chiffre d'affaires vers cotisations micro-entreprise. Les taux sont saisis dans la configuration, jamais codés en dur

### Organisation
- Notes Markdown avec recherche
- Tâches : capture rapide (« todo appeler la boulangerie demain 10h »), rappel par notification système
- Minuteur, chronomètre, Pomodoro
- Suivi du temps par client ou projet : démarrer et arrêter depuis la barre, export CSV

### Web
- Recherches par préfixe (`g`, `gh`, `mdn`, `npm`, `yt`, `wiki`), préfixes ajoutables dans la configuration
- Favoris Firefox et Chrome
- Traduction via une API configurable (clé dans le trousseau)

### Captures
- Capture de zone via le portail Screenshot, puis copier ou enregistrer
- OCR local (Tesseract, appelé en processus) : texte d'une zone copié dans le presse-papiers
- QR codes à partir d'un texte ou d'une URL

### Extensions et confort
- Scripts simples : un fichier shell, Python ou Node dont un en-tête déclare titre, icône et arguments, devient une commande. Sa sortie s'affiche dans la barre
- Dossier d'extensions locales, rechargé à chaud
- Démarrage avec la session, icône dans la barre système
- Secrets dans le trousseau, aucune télémétrie, tout en local
- Interface en français et en anglais (dictionnaires JSON)
- Export et import des réglages, presets, extraits et notes dans un seul fichier JSON
- Accessibilité : focus visible, rôles ARIA, taille de police réglable, contraste suffisant

### Livraison
- Paquets `.deb` et AppImage, générés par un script
- `README.md` (à la racine) : installation, raccourci à créer, commandes CLI, présentation des presets

## 7. Phases et critères de fin

Une phase est finie seulement si le build passe, les tests passent, les fonctions de la phase marchent sur la machine, et le travail est commité. Si une vérification visuelle est nécessaire, indique-la dans `STATUS.md` au lieu de la prétendre faite.

1. **Phase 0** : focus et raccourci (section 4)
2. **Phase 1** : cœur. Processus résident, CLI, fenêtre, recherche, indexeur d'applications, historique d'usage, configuration et presets, thèmes
3. **Phase 2** : fichiers, outils développeur, boîte à outils
4. **Phase 3** : texte, presse-papiers, calcul, conversion, dates, montants
5. **Phase 4** : notes, tâches, minuteur, suivi du temps
6. **Phase 5** : web, captures, OCR, QR codes
7. **Phase 6** : scripts, export et import, i18n, accessibilité, démarrage automatique, paquets

## 8. Gestion du budget (à lire avec attention)

Tu peux ne pas aller jusqu'au bout dans une seule session. Ce n'est pas un échec, à condition de t'arrêter proprement.

- Découpe chaque phase en petites étapes committables.
- Au début de chaque session, lis `STATUS.md` et reprends à « Prochaine étape ». Ne refais pas ce qui est coché.
- Après chaque étape : commit, puis mise à jour de `STATUS.md`.
- Si le contexte s'allonge ou que la limite approche, ne commence pas une nouvelle étape. Termine l'étape en cours (ou annule-la avec `git stash` et note-le), committe, mets `STATUS.md` à jour, puis écris une ligne `ARRÊT PROPRE : reprendre à <étape>` et arrête-toi.
- Ne laisse jamais le dépôt cassé : pas de commit qui ne compile pas, pas de code à moitié écrit hors de la branche de travail.
- Termine la phase en cours plutôt que d'en commencer une qui ne pourra pas être finie.

**`STATUS.md`** à créer à la racine du dépôt dès le départ, avec :
- phase courante, étapes cochées
- prochaine étape, précise (fichier et fonction concernés)
- décisions prises
- résultats des tests (focus, délai d'ouverture mesuré)
- problèmes ouverts et ce qui a été essayé
- commande pour lancer les tests

## 9. Style de code

- Code simple. Pas d'abstraction pour un usage futur. Les valeurs de configuration viennent du preset, jamais du code.
- Commentaires seulement pour le pourquoi, en une ligne.
- Chaque logique non triviale (parseur, recherche, calcul, conversion, configuration) a au moins un test.
- Dans le cœur Rust, pas de `unwrap()` sur une entrée utilisateur ou un fichier externe.

## 10. Ce que tu rends à la fin de ta session

Un résumé court : phase atteinte, ce qui marche (et comment tu l'as vérifié), ce qui ne marche pas, la prochaine étape, les commandes pour lancer et tester. Rien de plus.
