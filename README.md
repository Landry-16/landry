# Landry

Landry est un compagnon du quotidien pour Ubuntu GNOME. Un raccourci clavier ouvre une barre : on tape quelques lettres pour lancer une application, ouvrir un fichier ou un projet, calculer, convertir, coller un extrait, noter une idée, lancer un minuteur ou un script. Tout reste en local, sauf les services que tu choisis de brancher (GitHub, Vercel, traduction).

Cible : Ubuntu 24.04, GNOME 46, session Wayland. Pile : Tauri 2 (Rust) et Svelte 5.

La liste complète des fonctionnalités, avec leur état et ce qui reste à faire, est dans [FEATURES.md](FEATURES.md).

## Installation

```bash
scripts/install-deps.sh        # dépendances système (demande sudo)
npm install
scripts/package.sh             # construit dist-packages/landry_<version>_amd64.deb (et l'AppImage si les dépendances sont là)
sudo apt install ./dist-packages/landry_0.2.0_amd64.deb
```

`landry doctor` vérifie ensuite que tout est en place.

Si `apt-get update` échoue à cause d'un dépôt tiers cassé, le script continue quand même. Pour faire disparaître l'erreur, retire le dépôt concerné, par exemple `sudo add-apt-repository --remove ppa:alexlarsson/flatpak`.

Sans les dépendances de développement, `scripts/package.sh` construit quand même le `.deb` avec `dpkg-deb`. L'AppImage passe par le bundler Tauri, qui a besoin de `libayatana-appindicator3-dev`.

## Premier lancement

Lance `landry` (ou Landry depuis la liste des applications). Un assistant s'ouvre, une seule fois, avec quatre réglages facultatifs :

1. le raccourci clavier : appuie sur la combinaison voulue, ou garde Ctrl+Alt+Espace ;
2. le démarrage avec la session ;
3. l'apparence, en un clic sur un preset ;
4. les outils facultatifs qui manquent, avec la commande à copier.

On le rouvre avec `/bienvenue` dans la barre ou `landry setup`.

GNOME 46 n'a pas encore le portail des raccourcis globaux : le raccourci est un raccourci personnalisé de GNOME qui exécute `landry toggle`. `scripts/install-shortcut.sh` fait la même chose en ligne de commande. Super+Espace est déjà pris par GNOME pour changer la disposition du clavier.

## Démarrage

`landry` (ou `landry toggle`) démarre le processus résident s'il ne tourne pas, puis ouvre la barre. « Démarrer avec la session » (assistant, ou Réglages, onglet Général) installe un service systemd utilisateur (`~/.config/systemd/user/landry.service`) : Landry est prêt dès la connexion et redémarre tout seul après un plantage, mais pas après « Quitter ». Sans systemd utilisateur, Landry revient à une entrée de démarrage automatique classique. Une icône dans la barre du haut donne accès à la barre, aux réglages et à l'arrêt.

Les applications installées ou supprimées apparaissent et disparaissent de la barre d'elles-mêmes, sans redémarrer Landry. Une application Flatpak ou Snap se désinstalle depuis la barre : Ctrl+K, « Désinstaller », puis Entrée pour confirmer (le mot de passe est demandé si besoin). Les paquets apt restent du ressort de Logiciels Ubuntu.

## Ligne de commande

| Commande | Effet |
| --- | --- |
| `landry toggle` | ouvre ou ferme la barre |
| `landry setup` | ouvre l'assistant de premier lancement |
| `landry doctor` | vérifie l'installation (raccourci, portails, outils) et dit quoi corriger |
| `landry search <texte>` | affiche les résultats sans ouvrir la barre |
| `landry run <texte>` | exécute le premier résultat |
| `landry preset list` | liste les presets |
| `landry preset show <nom>` | affiche un preset en JSON |
| `landry preset use <nom>` | change de preset, sans redémarrer |
| `landry reload` | relit la configuration |
| `landry quit` | arrête le processus résident |

`LANDRY_TIMING=1 landry show` affiche le délai d'ouverture mesuré.

## Dans la barre

- F1 : aide (raccourcis et exemples). Ctrl+plus et Ctrl+moins : taille du texte, gardée d'une ouverture à l'autre.
- Entrée : action principale. Ctrl+K : toutes les actions de l'élément. Tab : compléter. Flèche du haut sur une barre vide : historique. Échap : effacer, puis fermer. Ctrl+, : réglages.
- Les actions destructrices (corbeille, arrêt d'un processus, redéploiement) demandent un deuxième Entrée.

| Tape | Pour |
| --- | --- |
| `vsc`, `paramètres` | applications (initiales, sans accents, fautes tolérées) |
| `facture`, `pdf: facture`, `img: <7j`, `dir:` | fichiers, avec filtres de type et de date |
| `nacre` | projets git : éditeur, terminal, git pull, PR, scripts npm |
| `port 5173`, `docker`, `ssh prod` | ports occupés, conteneurs, hôtes SSH |
| `pr`, `issues`, `vercel` | GitHub et Vercel (jetons dans le trousseau) |
| `12*7%`, `5 km en m`, `10 EUR en USD` | calculs, unités, devises (taux BCE) |
| `15h Paris en New York`, `dans 3 semaines`, `jours ouvrés entre 01/11 et 30/11` | heures et dates |
| `100 ht`, `120 ttc`, `ca 3000` | TVA et cotisations (taux dans les réglages) |
| `uuid`, `b64 texte`, `json {...}`, `jwt ...`, `ts`, `regex /a+/ texte`, `#ff8800`, `contraste #fff #000` | boîte à outils |
| `pipette`, `capture`, `ocr`, `qr texte`, `comparer` | captures, QR codes, diff |
| `maj`, `titre`, `typo`, `mots` | transformations du texte sélectionné |
| nom d'un extrait | collage dans l'application d'où tu viens |
| `todo appeler demain 10h`, `minuteur 10 min`, `chrono`, `pomodoro`, `suivi Nacre` | tâches et rappels, minuteurs, suivi du temps |
| `note idée`, `notes` | notes Markdown |
| `gh texte`, `wiki texte`, `tr en bonjour` | recherches web, traduction |

## Commandes « / »

Tape `/` pour voir toutes les commandes ; Entrée ou Tab complète celle qui est sélectionnée. Tout mot-clé ci-dessus marche aussi avec une barre oblique (`/note idée`, `/minuteur 10 min`, `/g texte`).

| Commande | Effet |
| --- | --- |
| `/spotify daft punk` | ouvre la recherche dans l'application Spotify (le lecteur web si l'application manque) ; `/spotify pause`, `suivant`, `precedent` |
| `/media` | morceau en cours et contrôles, pour tout lecteur (`playerctl`) |
| `/volume 40`, `/volume +10`, `/muet` | volume du son (`wpctl`) |
| `/luminosite 60` | luminosité de l'écran (portables) |
| `/sombre`, `/clair`, `/dnd`, `/wifi off` | thème du système, ne pas déranger, wifi |
| `/verrouiller`, `/veille`, `/eteindre`, `/redemarrer`, `/deconnexion` | session ; la veille demande deux fois Entrée, GNOME confirme les trois autres |
| `/reglages son` | ouvre un panneau des paramètres GNOME |
| `/tuer firefox` | processus correspondants, les plus gourmands d'abord, arrêt en deux temps |
| `/meteo Lyon` | météo du jour et des deux suivants (wttr.in, seulement quand tu la demandes) |
| `/aide` | l'aide de la barre |
| `/bienvenue` | l'assistant de premier lancement |

## Presets

Tout l'aspect de la barre vient du preset actif : position, taille, couleurs claires et sombres, police, densité, opacité, animation, nombre de résultats, panneau d'aperçu, ordre et visibilité des modules. Le preset « Défaut » est intégré ; Minimal, Dense, Développeur, Rose noire, Soie, Fresque et Pixel sont livrés et modifiables. Une mise à jour ajoute les nouveaux presets livrés sans faire revenir ceux que tu as supprimés.

Le bloc `style` donne vie à la barre :

- `companion` : la vouivre, un petit serpent ailé en pixels à la place du glyphe. Elle arrive à l'ouverture, écoute quand tu tapes, regarde les résultats quand tu changes de ligne, se perd quand rien ne correspond, saute et crache une flamme quand une action réussit, rougit et fume sur une erreur, et s'endort après 30 s.
- `ornament` : `none`, `baroque` (cimier, roses, volutes), `silk` (perle, nuages), `fresco` (soleil vermillon, encre) ou `pixel` (coins). Les ornements débordent du cadre : ils se voient mieux en couche `screen`.
- `particles` : de 0 à 40 particules lentes derrière les résultats (lucioles, pétales, feuilles d'or ou pixels selon l'ornement).
- `tint` : la couleur secondaire (ailes, ornements, particules).

Avec `animation_ms` à 0, ou si le système demande moins de mouvement, rien ne bouge : la vouivre change seulement de pose. La direction artistique (compagnon, réactions, presets) est dans le fichier Figma « Landry - Direction artistique ».

Les presets sont des fichiers JSON dans `~/.config/landry/presets/`. On les modifie dans Réglages (formulaire avec aperçu en direct, ou JSON validé), ou à la main puis `landry reload`. Un preset invalide n'empêche jamais la barre de s'ouvrir : Landry revient au preset Défaut et le signale.

Wayland ne laisse pas une application placer sa fenêtre. Chaque preset choisit sa couche :

- `screen` : une couche transparente de la taille de l'écran, dans laquelle la barre est placée librement (ancre, marges, décalages). Ouverture mesurée entre 70 et 115 ms.
- `window` : une fenêtre de la taille de la barre, placée par GNOME. Ouverture mesurée entre 30 et 45 ms. C'est le choix du preset Minimal.

## Scripts

Tout fichier de `~/.config/landry/scripts` (shell, Python, Node, ou exécutable) devient une commande, rechargée à chaud. L'en-tête est facultatif :

```bash
#!/usr/bin/env bash
# @landry title: Vider le cache npm
# @landry subtitle: npm cache clean
# @landry argument: Paquet          (demandé avant de lancer, passé en $1)
# @landry mode: silent              (sinon la sortie s'affiche dans la barre)
```

## Données et confidentialité

- Réglages et presets : `~/.config/landry/`. Base (historique, extraits, notes, tâches, suivi) : `~/.local/share/landry/landry.db`.
- Jetons GitHub, Vercel et traduction : trousseau du système, jamais dans un fichier ni dans une sauvegarde.
- Aucune télémétrie. Réseau utilisé seulement pour les taux BCE, GitHub, Vercel, la traduction et `/meteo`, quand tu les demandes.
- Sauvegarde complète et import (sans suppression) : Réglages, onglet Général.

## Développement

Le guide complet est dans [docs/GUIDE_DEVELOPPEMENT.md](docs/GUIDE_DEVELOPPEMENT.md) : prérequis, lancement, architecture, arborescence, recettes pour ajouter un module, un réglage ou une traduction, paquets, dépannage et conventions.

L'essentiel :

```bash
npm install
npx tauri dev -- -- daemon        # résident en développement, interface rechargée à chaud
src-tauri/target/debug/landry toggle
(cd src-tauri && cargo test)      # tests du cœur
npm test                          # tests de l'interface
```

L'état d'avancement, les mesures et les décisions sont dans `STATUS.md`.
