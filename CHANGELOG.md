# Changelog

## 0.5.1 (2026-10-15)

Ubuntu 26.04 et GNOME 50.

### Corrigé
- Historique du presse-papiers et fenêtres ouvertes sous Ubuntu 26.04 : l'extension GNOME de Landry déclare maintenant GNOME 50 et 51. GNOME 50 la refusait comme incompatible, donc le presse-papiers ne marchait pas. Après la mise à jour, reconnecte-toi une fois à ta session.
- La barre pouvait s'ouvrir décalée par rapport au centre de l'écran (dock d'Ubuntu, barre du haut) : elle peut maintenant être déplacée, voir plus bas.

### Ajouté
- La barre se déplace à la souris : on l'attrape par une zone libre (autour du champ, le nom du preset) et on la glisse. Elle se rouvre ensuite à cet endroit, quel que soit le preset. La position (en pixels depuis le centre) se modifie aussi à la main dans Réglages, Général, avec un bouton Recentrer. Elle est enregistrée dans `settings.json` (`bar_offset`).
- Ctrl+Alt+Espace est installé comme raccourci dès le premier démarrage sous GNOME, même sans passer par l'assistant. Un raccourci déjà choisi n'est pas touché, et un raccourci supprimé ou changé ensuite n'est pas remis.

## 0.5.0

Paquets pour les grandes distributions Linux (.deb, .rpm, AppImage) et installateur Windows.

## 0.4.0

Première version publiée.
