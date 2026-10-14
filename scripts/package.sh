#!/usr/bin/env bash
# Build the Landry packages into dist-packages/.
# With the system dependencies (scripts/install-deps.sh): .deb and AppImage through the Tauri bundler.
# Without them: a .deb built directly with dpkg-deb from the release binary.
set -euo pipefail

cd "$(dirname "$0")/.."
VERSION=$(grep -m1 '^version' src-tauri/Cargo.toml | cut -d'"' -f2)
OUT=dist-packages
mkdir -p "$OUT"

if pkg-config --exists ayatana-appindicator3-0.1 2>/dev/null; then
  npx tauri build --bundles deb appimage
  cp src-tauri/target/release/bundle/deb/*.deb src-tauri/target/release/bundle/appimage/*.AppImage "$OUT"/
  ls -1 "$OUT"
  exit 0
fi

echo "libayatana-appindicator3-dev absent : .deb construit avec dpkg-deb, pas d'AppImage."
echo "Pour l'AppImage : lance d'abord scripts/install-deps.sh, puis relance ce script."
npm run build
(cd src-tauri && cargo build --release --features custom-protocol)

ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT
install -Dm755 src-tauri/target/release/landry "$ROOT/usr/bin/landry"
for size in 32 128 256; do
  install -Dm644 "src-tauri/icons/${size}x${size}.png" "$ROOT/usr/share/icons/hicolor/${size}x${size}/apps/landry.png"
done
install -Dm644 src-tauri/icons/icon.png "$ROOT/usr/share/icons/hicolor/512x512/apps/landry.png"
install -Dm644 /dev/stdin "$ROOT/usr/share/applications/landry.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Landry
Comment=Compagnon du quotidien
Exec=landry toggle
Icon=landry
Categories=Utility;
Keywords=launcher;lanceur;palette;
StartupNotify=false
DESKTOP
mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<CONTROL
Package: landry
Version: $VERSION
Architecture: $(dpkg --print-architecture)
Maintainer: Landry-16
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libayatana-appindicator3-1, xdg-desktop-portal-gnome, zip
Recommends: tesseract-ocr, tesseract-ocr-fra
Section: utils
Priority: optional
Description: Landry, compagnon du quotidien pour Ubuntu GNOME
 Barre de commande au clavier : applications, fichiers, projets, calculs,
 extraits, notes, tâches et outils, avec presets personnalisables.
CONTROL
dpkg-deb --root-owner-group --build "$ROOT" "$OUT/landry_${VERSION}_$(dpkg --print-architecture).deb"
ls -1 "$OUT"
