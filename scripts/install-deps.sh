#!/usr/bin/env bash
# Dépendances système de Landry sur Ubuntu 24.04. À lancer soi-même : demande sudo.
set -euo pipefail

sudo apt-get update
sudo apt-get install -y \
  build-essential curl wget file pkg-config \
  libwebkit2gtk-4.1-dev libssl-dev libxdo-dev libdbus-1-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  tesseract-ocr tesseract-ocr-fra
