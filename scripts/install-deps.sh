#!/usr/bin/env bash
# Dépendances système de Landry sur Ubuntu 24.04. À lancer soi-même : demande sudo.
set -euo pipefail

# A broken third-party repository makes `apt-get update` fail without affecting the Ubuntu
# packages below: warn and keep going with the package lists already known.
if ! sudo apt-get update; then
  echo
  echo "Attention : apt-get update a signalé une erreur (souvent un dépôt tiers cassé, voir ci-dessus)."
  echo "L'installation continue avec les listes de paquets déjà connues."
  echo
fi

sudo apt-get install -y \
  build-essential curl wget file pkg-config \
  libwebkit2gtk-4.1-dev libssl-dev libxdo-dev libdbus-1-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  tesseract-ocr tesseract-ocr-fra
