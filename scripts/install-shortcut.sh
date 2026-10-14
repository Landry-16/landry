#!/usr/bin/env bash
# Crée (ou met à jour) un raccourci GNOME personnalisé qui exécute `landry toggle`.
# Usage : scripts/install-shortcut.sh [chemin/vers/landry] [raccourci]
# Exemple : scripts/install-shortcut.sh "$PWD/src-tauri/target/debug/landry" '<Control><Alt>space'
set -euo pipefail

BIN="${1:-$(command -v landry || echo "$HOME/.local/bin/landry")}"
KEY="${2:-<Control><Alt>space}"
SCHEMA=org.gnome.settings-daemon.plugins.media-keys
ENTRY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/landry/

current=$(gsettings get "$SCHEMA" custom-keybindings)
if [[ "$current" != *"$ENTRY"* ]]; then
  if [[ "$current" == "@as []" || "$current" == "[]" ]]; then
    new="['$ENTRY']"
  else
    new="${current%]}, '$ENTRY']"
  fi
  gsettings set "$SCHEMA" custom-keybindings "$new"
fi

KB="$SCHEMA.custom-keybinding:$ENTRY"
gsettings set "$KB" name 'Landry'
gsettings set "$KB" command "$BIN toggle"
gsettings set "$KB" binding "$KEY"
echo "Raccourci $KEY -> $BIN toggle"
