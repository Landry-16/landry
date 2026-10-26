# Fonctionnalités de Landry

Catalogue de tout ce que fait Landry, avec l'état de chaque point. Le mode d'emploi est dans le [README](README.md), l'historique et les décisions dans [STATUS.md](STATUS.md).

État : **fait** (codé et testé), **à vérifier** (codé et testé, pas encore essayé sur un vrai bureau ou avec un vrai compte), **à faire**.

## La barre

| Fonctionnalité | État |
| --- | --- |
| Ouverture au raccourci GNOME (`landry toggle`), focus immédiat | fait ; à vérifier avec le vrai raccourci |
| Recherche floue sans accents, par initiales, fautes tolérées | fait |
| Classement par usage (fréquence, récence, heure) | fait |
| Entrée : action principale ; Ctrl+K : toutes les actions ; Tab : compléter | fait |
| Historique des recherches (flèche du haut sur barre vide) | fait |
| Panneau d'aperçu (texte, image, propriétés) | fait |
| Alias vers un élément, une recherche ou une commande complète (`pp = /spotify play`) | fait |
| Alias suivi d'arguments (`sp = /spotify`, puis `sp daft punk`) | fait |
| Ouverture jamais bloquée par une action en cours ou une application qui ne répond pas | fait |
| Actions dangereuses confirmées par un deuxième Entrée | fait |
| Ligne de raccourcis en bas, indications sur barre vide ou sans résultat | fait |
| Aide F1 (raccourcis et exemples) | fait |
| La liste défile pour suivre la sélection aux flèches | fait |
| Accueil personnalisable : Réglages > Général > Accueil liste les commandes, applications ou éléments montrés sur la barre vide, et seulement eux jusqu'à la première lettre | fait |

## Trouver

| Fonctionnalité | Exemple | État |
| --- | --- | --- |
| Applications (XDG, Flatpak, Snap) avec icônes | `vsc`, `paramètres` | fait |
| Basculer vers la fenêtre d'une application déjà ouverte (Ctrl+K, « Nouvelle fenêtre », en ouvre une autre) | `firefox` | fait ; testé dans un GNOME Shell imbriqué |
| Fenêtres ouvertes dans la recherche, et `/fenetres` | `/fenetres` | fait ; testé dans un GNOME Shell imbriqué |
| Recherche dans le contenu des fichiers (ripgrep ou grep) | `/contenu devis 2024` | fait |
| Index des applications mis à jour à l'installation et à la suppression | | fait |
| Désinstaller une application Flatpak ou Snap | Ctrl+K, « Désinstaller » | fait ; à vérifier |
| Fichiers (dossiers XDG, récents, favoris), filtres de type et de date | `pdf: facture`, `img: <7j` | fait |
| Convertisseur dans Ctrl+K : images, PDF, documents, vidéo, audio | Ctrl+K sur un fichier | fait ; PDF et documents vérifiés en vrai, images, vidéo et audio à vérifier (FFmpeg et ImageMagick absents ici) |
| Actions fichiers : ouvrir, dossier, terminal, renommer, déplacer, compresser, corbeille, ouvrir avec | | fait |
| Projets git : branche, état, éditeur, terminal, pull, PR, scripts npm | `nacre` | fait |
| Ports occupés, arrêt du processus | `port 5173` | fait |
| Hôtes SSH, conteneurs Docker | `ssh prod`, `docker` | fait |
| GitHub (PR, CI, issues) et Vercel (déploiements, redéploiement) | `pr`, `vercel` | fait ; à vérifier avec de vrais jetons |
| Favoris Firefox, Chrome, Chromium, Brave | | fait |
| Recherches web par préfixe, URL tapée | `gh texte`, `wiki texte` | fait |

## Calculer et convertir

| Fonctionnalité | Exemple | État |
| --- | --- | --- |
| Calculatrice, pourcentages, unités | `12*7%`, `5 km en m` | fait |
| Devises (taux BCE, hors ligne) | `10 EUR en USD` | fait |
| Fuseaux, dates relatives, jours ouvrés (fériés français) | `15h Paris en New York` | fait |
| TVA et cotisations micro-entreprise | `100 ht`, `ca 3000` | fait |

## Texte

| Fonctionnalité | Exemple | État |
| --- | --- | --- |
| Extraits avec variables (date, presse-papiers, curseur, champs) | nom de l'extrait | fait |
| Historique du presse-papiers : 100 textes en mémoire, mots de passe ignorés, pause et vidage | `/presse-papiers`, `/clip` | fait ; testé dans un GNOME Shell imbriqué |
| Émojis (1914, noms français et anglais) et caractères spéciaux | `/emoji coeur`, `/car tiret` | fait |
| Dictionnaire et synonymes (Wiktionnaire, Wiktionary, Datamuse) | `/def maison`, `/syn rapide`, `/def en house` | fait |
| Collage automatique dans l'application précédente | | fait ; à vérifier (dialogue RemoteDesktop) |
| Transformations de la sélection | `maj`, `titre`, `typo`, `mots` | fait |
| Traduction (DeepL ou LibreTranslate) | `tr en bonjour` | fait ; à vérifier avec une clé |
| Boîte à outils : uuid, base64, URL, JSON, hachages, JWT, timestamps, regex, couleurs, contraste | `b64 texte`, `#ff8800` | fait |
| Comparer deux textes | `comparer` | fait |

## S'organiser

| Fonctionnalité | Exemple | État |
| --- | --- | --- |
| Notes Markdown avec recherche et aperçu | `note idée`, `notes` | fait |
| Tâches avec échéance et rappel | `todo appeler demain 10h` | fait |
| Minuteur, chronomètre, Pomodoro (gardés après un redémarrage) | `minuteur 10 min pâtes` | fait |
| Suivi du temps par client, export CSV | `suivi Nacre` | fait |
| Obsidian : note ou tâche dans l'inbox du coffre, notes de Landry recopiées dans le coffre (dans les deux sens), ouvrir une note dans Obsidian | `obs payer le loyer demain` | fait |

## Captures

| Fonctionnalité | Exemple | État |
| --- | --- | --- |
| Capture d'une zone | `capture` | fait ; à vérifier |
| OCR (Tesseract) | `ocr` | fait ; à vérifier |
| Pipette | `pipette` | fait ; à vérifier |
| QR codes, enregistrement SVG | `qr texte` | fait |

## Commandes « / »

| Commande | État |
| --- | --- |
| `/` : palette de toutes les commandes, Entrée ou Tab pour compléter ; module désactivable par preset | fait |
| Tout mot-clé avec une barre oblique (`/note`, `/minuteur`, `/g`) | fait |
| `/spotify <texte>` (recherche dans l'application, repli sur le web), `/spotify pause, suivant, precedent` | fait ; à vérifier avec Spotify et playerctl |
| `/media` : morceau en cours et contrôles, tout lecteur | fait ; à vérifier (playerctl) |
| `/volume`, `/muet`, `/luminosite` | fait ; luminosité à vérifier sur un portable |
| `/sombre`, `/clair`, `/dnd`, `/wifi` | fait |
| `/verrouiller`, `/veille`, `/eteindre`, `/redemarrer`, `/deconnexion` | fait |
| `/reglages <panneau>` : panneaux des paramètres GNOME | fait |
| `/tuer <processus>` | fait |
| `/meteo [ville]` | fait |
| `/aide`, `/bienvenue` | fait |

## Apparence

| Fonctionnalité | État |
| --- | --- |
| Presets JSON : couleurs claires et sombres, police, densité, opacité, animation, taille, position, aperçu, modules | fait |
| Éditeur de presets avec aperçu en direct, contrôle des contrastes, JSON validé, import et export | fait |
| Presets livrés : Défaut, Minimal, Dense, Développeur, Rose noire, Soie, Fresque, Pixel | fait |
| Nouveaux presets livrés ajoutés par une mise à jour, sans faire revenir ceux supprimés | fait |
| La vouivre, compagnon en pixels, 8 réactions : arrivée, repos, écoute, désigne, perdue, joie, erreur, sommeil | fait ; à vérifier dans la vraie fenêtre |
| Ornements : baroque, soie, fresque, pixel | fait ; à vérifier dans la vraie fenêtre |
| Particules (lucioles, pétales, feuilles d'or, pixels) | fait |
| Couleur secondaire par preset | fait |
| Suivi du thème clair ou sombre du système | fait |
| Direction artistique dans Figma (« Landry - Direction artistique ») | fait ; section 03 à mettre à jour |

## Intégration au bureau

| Fonctionnalité | État |
| --- | --- |
| Assistant de premier lancement (raccourci, démarrage, apparence, outils manquants) | fait ; à vérifier sur le vrai bureau |
| Démarrage avec la session par un service systemd utilisateur, redémarrage après un plantage | fait ; à vérifier |
| Icône de barre système (ouvrir, réglages, quitter) | fait |
| Scripts utilisateur (`~/.config/landry/scripts`), rechargés à chaud | fait |
| Ligne de commande : `toggle`, `setup`, `search`, `run`, `preset`, `reload`, `doctor`, `quit` | fait |
| `landry doctor` : ce qui manque et comment l'installer | fait |
| Sauvegarde complète et import sans suppression | fait |
| Interface en français et en anglais | fait |
| `./install.sh` : installation, mise à jour, réinstallation et désinstallation en une commande, dépendances comprises | fait ; à vérifier sur une machine neuve |
| Paquet .deb avec outils recommandés installés d'office (playerctl, FFmpeg, ImageMagick, Tesseract, ripgrep…) et l'extension GNOME ; AppImage avec `scripts/package.sh` | fait |
| Versions GitHub avec le `.deb` attaché à chaque étiquette `v*` ; `./install.sh --release` l'installe sans compiler | fait ; jamais lancé (aucune étiquette poussée) |
| Tests automatiques sur GitHub à chaque push (`ci.yml`) | fait ; jamais lancé sur GitHub |
| README en anglais, site en anglais (`site/`, GitHub Pages) | fait ; Pages à activer |
| Lecture du texte sélectionné à l'ouverture, désactivable | fait |

## Accessibilité et confort

| Fonctionnalité | État |
| --- | --- |
| Rôles ARIA, focus visible, navigation entière au clavier | fait |
| Nombre de résultats annoncé aux lecteurs d'écran | fait |
| Taille du texte par preset, et Ctrl+plus / Ctrl+moins gardés d'une ouverture à l'autre | fait |
| Mouvement réduit respecté ; `animation_ms = 0` arrête toute animation | fait |
| Mode couleurs forcées (contraste élevé) | fait |
| Contrôle des contrastes (WCAG AA) dans l'éditeur de presets | fait |

## Licences

| Fonctionnalité | État |
| --- | --- |
| Licence MIT (`LICENSE`), déclarée dans Cargo.toml, package.json et le paquet | fait |
| Licences des composants tiers (`THIRD_PARTY_LICENSES.md`, généré par `scripts/third-party-licenses.py`), installées dans `/usr/share/doc/landry/` | fait |
| Source citée à côté des définitions et synonymes (Wiktionnaire en CC BY-SA, Datamuse) | fait |

## Confidentialité

- Tout reste en local. Réseau seulement pour les taux BCE, GitHub, Vercel, la traduction et `/meteo`, quand tu les demandes.
- Jetons dans le trousseau du système, jamais dans un fichier ni dans une sauvegarde.
- Aucune télémétrie.

## À faire

| Sujet | Détail |
| --- | --- |
| Essais sur le vrai bureau | Raccourci et service depuis l'assistant, vouivre et ornements dans la vraie fenêtre, désinstallation réelle, `/spotify`, `/media`, `/luminosite`, collage automatique, capture, OCR, pipette, jetons GitHub, Vercel et traduction |
| Jeux Steam | Lire les `appmanifest_*.acf` de toutes les bibliothèques, lancer par `steam://rungameid/<id>` (idée reprise de Weft) |
| Applications de Weft | Trouver les programmes Windows installés par Weft, sans refaire son moteur |
| Figma | Section 03 « Barre vivante » à aligner sur les 9 réactions de la vouivre |
| Guide de développement | Ajouter les modules `slash` et `session`, l'assistant et le bloc `style` des presets |
| Extension GNOME sur le vrai bureau | Testée dans un GNOME Shell imbriqué ; à vérifier après une reconnexion |
| Première version publiée | Pousser l'étiquette `v0.4.0` pour lancer la construction du paquet sur GitHub |
| Livraison | Fusionner `dev` dans `main` une fois la 0.4.0 essayée sur le bureau |
| Diffusion sans rendre le dépôt public | Dépôt public `landry-releases` qui ne contient que le `.deb` et `install.sh`, alimenté par le workflow de publication ; en attente de ton accord |
| Avant une diffusion publique | Mentions légales du site (hébergeur), vérification du nom « Landry » auprès de l'INPI |
| Centrage exact | La fenêtre de la taille de l'écran est décalée par le dock et la barre du haut ; le plein écran a été essayé puis retiré (fond noir). Piste : laisser l'extension GNOME placer la fenêtre |
| Épingler depuis la barre | Action Ctrl+K « Épingler à l'accueil » sur n'importe quel élément |
| Pistes à plus long terme | Intégration IA |

## Plus tard

Idées retenues pour une prochaine étape.

| Idée | Détail |
| --- | --- |
| Vouivre qui grandit | Elle évolue avec l'usage (nouvelles écailles, couleur de flamme), sans aucun effet sur le fonctionnement |
| Éditeur de thèmes plus visuel | Une pipette sur un fond d'écran pour générer un preset à partir de ses couleurs |
| Partage de presets | Export et import par lien ou QR code entre amis |
| Extensions | Un dossier où chacun dépose une commande « / » en JavaScript ou en script, avec une petite documentation, pour rendre Landry adaptable par d'autres |
