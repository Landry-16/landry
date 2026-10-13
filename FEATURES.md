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
| Alias (`nc` vers un projet, `ed` vers une recherche) | fait |
| Actions dangereuses confirmées par un deuxième Entrée | fait |
| Ligne de raccourcis en bas, indications sur barre vide ou sans résultat | fait |
| Aide F1 (raccourcis et exemples) | fait |

## Trouver

| Fonctionnalité | Exemple | État |
| --- | --- | --- |
| Applications (XDG, Flatpak, Snap) avec icônes | `vsc`, `paramètres` | fait |
| Index des applications mis à jour à l'installation et à la suppression | | fait |
| Désinstaller une application Flatpak ou Snap | Ctrl+K, « Désinstaller » | fait ; à vérifier |
| Fichiers (dossiers XDG, récents, favoris), filtres de type et de date | `pdf: facture`, `img: <7j` | fait |
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
| `/` : palette de toutes les commandes, Entrée ou Tab pour compléter | fait |
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
| Paquet .deb ; AppImage avec les dépendances de développement | fait |

## Accessibilité et confort

| Fonctionnalité | État |
| --- | --- |
| Rôles ARIA, focus visible, navigation entière au clavier | fait |
| Nombre de résultats annoncé aux lecteurs d'écran | fait |
| Taille du texte par preset, et Ctrl+plus / Ctrl+moins gardés d'une ouverture à l'autre | fait |
| Mouvement réduit respecté ; `animation_ms = 0` arrête toute animation | fait |
| Mode couleurs forcées (contraste élevé) | fait |
| Contrôle des contrastes (WCAG AA) dans l'éditeur de presets | fait |

## Confidentialité

- Tout reste en local. Réseau seulement pour les taux BCE, GitHub, Vercel, la traduction et `/meteo`, quand tu les demandes.
- Jetons dans le trousseau du système, jamais dans un fichier ni dans une sauvegarde.
- Aucune télémétrie.

## À faire

| Sujet | Détail |
| --- | --- |
| Essais sur le vrai bureau | Raccourci et service depuis l'assistant, vouivre et ornements dans la vraie fenêtre, désinstallation réelle, `/spotify`, `/media`, `/luminosite`, collage automatique, capture, OCR, pipette, jetons GitHub, Vercel et traduction |
| Extension GNOME Shell de Landry | Historique du presse-papiers (`/presse-papiers`) et sélecteur de fenêtres (`/fenetres`), impossibles sans extension sous Wayland ; proposée, pas encore retenue |
| Jeux Steam | Lire les `appmanifest_*.acf` de toutes les bibliothèques, lancer par `steam://rungameid/<id>` (idée reprise de Weft) |
| Applications de Weft | Trouver les programmes Windows installés par Weft, sans refaire son moteur |
| Figma | Section 03 « Barre vivante » à aligner sur les 9 réactions de la vouivre |
| Guide de développement | Ajouter les modules `slash` et `session`, l'assistant et le bloc `style` des presets |
| Paquet .deb | Ajouter `playerctl` aux paquets recommandés |
| Livraison | Fusionner la branche `feat/vouivre-direction-artistique` dans `dev`, puis passer en version 0.3.0 |
| Pistes à plus long terme | Extensions en TypeScript, intégration IA |
