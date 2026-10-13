#!/usr/bin/env bash
# Landry: install, update or reinstall in one command, every dependency included.
#
#   ./install.sh               install or update from this folder (run it again after a git pull)
#   ./install.sh --uninstall   remove Landry; settings, notes and history are kept
#
# Without a clone: curl -fsSL https://raw.githubusercontent.com/Landry-16/landry/dev/install.sh | bash
# Environment: LANDRY_BRANCH (dev), LANDRY_SRC (~/.local/src/landry), LANDRY_DRY_RUN=1 prints the
# commands that need sudo instead of running them.
set -euo pipefail

REPO=https://github.com/Landry-16/landry.git
BRANCH=${LANDRY_BRANCH:-dev}
SRC=${LANDRY_SRC:-$HOME/.local/src/landry}
NODE_DIR=$HOME/.local/share/landry/node
SHORTCUT=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/landry/

step() { printf '\n\033[1;33m%s\033[0m\n' "$*"; }
die() { printf '\033[1;31mErreur : %s\033[0m\n' "$*" >&2; exit 1; }
root() { if [[ ${LANDRY_DRY_RUN:-} == 1 ]]; then echo "  [dry-run] sudo $*"; else sudo "$@"; fi; }

[[ $(id -u) -ne 0 ]] || die "lance ce script avec ton utilisateur, pas en root : il demande sudo quand il en a besoin."
command -v apt-get >/dev/null || die "Landry s'installe sur Ubuntu ou Debian (apt-get introuvable)."

stop_landry() {
  [[ ${LANDRY_DRY_RUN:-} == 1 ]] && return
  if pgrep -u "$(id -u)" -x landry >/dev/null; then
    landry quit >/dev/null 2>&1 || true
    sleep 0.5
    pkill -u "$(id -u)" -x landry 2>/dev/null || true
  fi
}

if [[ ${1:-} == --uninstall ]]; then
  step "Désinstallation de Landry"
  stop_landry
  if [[ -f $HOME/.config/systemd/user/landry.service ]]; then
    systemctl --user disable landry.service >/dev/null 2>&1 || true
    rm -f "$HOME/.config/systemd/user/landry.service"
    systemctl --user daemon-reload || true
  fi
  rm -f "$HOME/.config/autostart/landry.desktop"
  ext=$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo "@as []")
  ext=$(sed -e "s#, 'landry@landry-16.github.io'##; s#'landry@landry-16.github.io', ##; s#'landry@landry-16.github.io'##" <<<"$ext")
  [[ $ext == "[]" ]] && ext="@as []"
  gsettings set org.gnome.shell enabled-extensions "$ext" 2>/dev/null || true
  list=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "@as []")
  if [[ $list == *"$SHORTCUT"* ]]; then
    list=$(sed -e "s#, '$SHORTCUT'##; s#'$SHORTCUT', ##; s#'$SHORTCUT'##" <<<"$list")
    [[ $list == "[]" ]] && list="@as []"
    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$list"
  fi
  if dpkg-query -W landry >/dev/null 2>&1; then root apt-get remove -y landry; fi
  echo "Landry est désinstallé. Tes réglages restent dans ~/.config/landry et tes données dans ~/.local/share/landry."
  exit 0
fi

# ---------- 1. System packages, only the missing ones ----------
PKGS=(build-essential curl git file pkg-config xz-utils libwebkit2gtk-4.1-dev libssl-dev libxdo-dev libdbus-1-dev
  libayatana-appindicator3-dev librsvg2-dev)
missing=()
for p in "${PKGS[@]}"; do
  dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || missing+=("$p")
done
if ((${#missing[@]})); then
  step "Paquets système manquants : ${missing[*]}"
  # A broken third-party repository must not stop the install: the known lists are enough.
  root apt-get update || echo "  apt-get update a échoué, l'installation continue avec les listes connues."
  root apt-get install -y "${missing[@]}"
fi

# ---------- 2. Sources: this folder, or a clone kept up to date ----------
HERE=$(cd "$(dirname "${BASH_SOURCE[0]:-.}")" 2>/dev/null && pwd || true)
if [[ -f $HERE/src-tauri/Cargo.toml ]]; then
  cd "$HERE"
else
  step "Sources dans $SRC (branche $BRANCH)"
  if [[ -d $SRC/.git ]]; then
    git -C "$SRC" fetch --quiet origin "$BRANCH" && git -C "$SRC" checkout --quiet "$BRANCH" && git -C "$SRC" merge --quiet --ff-only "origin/$BRANCH"
  else
    git clone --quiet --branch "$BRANCH" "$REPO" "$SRC"
  fi
  cd "$SRC"
fi

# ---------- 3. Rust ----------
export PATH=$HOME/.cargo/bin:$PATH
if ! command -v cargo >/dev/null; then
  step "Installation de Rust (rustup, pour ton utilisateur)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi

# ---------- 4. Node 22 or newer (Vite needs 20.19+ or 22.12+) ----------
node_ok() {
  command -v node >/dev/null && node -e '
    const [a, b] = process.versions.node.split(".").map(Number);
    process.exit(a > 22 || (a === 22 && b >= 12) || (a === 20 && b >= 19) ? 0 : 1)'
}
export PATH=$NODE_DIR/bin:$PATH
if ! node_ok; then
  step "Installation de Node 22 dans $NODE_DIR (sans toucher au Node du système)"
  case $(uname -m) in x86_64) arch=x64 ;; aarch64) arch=arm64 ;; *) die "processeur $(uname -m) non pris en charge" ;; esac
  base=https://nodejs.org/dist/latest-v22.x
  tmp=$(mktemp -d)
  curl -fsSL "$base/SHASUMS256.txt" -o "$tmp/SHASUMS256.txt"
  file=$(awk "/linux-$arch\\.tar\\.xz\$/ {print \$2}" "$tmp/SHASUMS256.txt")
  [[ -n $file ]] || die "archive Node introuvable sur $base"
  curl -fsSL "$base/$file" -o "$tmp/$file"
  (cd "$tmp" && grep " $file\$" SHASUMS256.txt | sha256sum -c --quiet -) || die "somme de contrôle de Node invalide"
  rm -rf "$NODE_DIR" && mkdir -p "$NODE_DIR"
  tar -xJf "$tmp/$file" -C "$NODE_DIR" --strip-components=1
  rm -rf "$tmp"
  node_ok || die "Node installé mais inutilisable"
fi

# ---------- 5. Build ----------
lock=$(sha256sum package-lock.json | cut -d' ' -f1)
if [[ ! -f node_modules/.landry-lock || $(cat node_modules/.landry-lock) != "$lock" ]]; then
  step "Dépendances JavaScript"
  npm ci --no-audit --no-fund --loglevel=error
  echo "$lock" >node_modules/.landry-lock
fi
VERSION=$(node -p "require('./src-tauri/tauri.conf.json').version")
step "Construction de Landry $VERSION"
echo "  Première fois : 5 à 10 minutes. Ensuite : environ 2 minutes."
echo "  La dernière étape, landry(bin), reste immobile 1 à 2 minutes : c'est normal, ne l'interromps pas."
npx tauri build --bundles deb
DEB=$PWD/src-tauri/target/release/bundle/deb/Landry_${VERSION}_$(dpkg --print-architecture).deb
[[ -f $DEB ]] || die "paquet introuvable : $DEB"

# ---------- 6. Install: apt adds the runtime and recommended tools (playerctl, tesseract...) ----------
step "Installation du paquet"
stop_landry
root apt-get install -y --reinstall --allow-downgrades "$DEB"

# ---------- 7. GNOME extension (open windows, clipboard history): starts at the next login ----------
UUID=landry@landry-16.github.io
list=$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo "@as []")
if [[ $list != *"'$UUID'"* && ${LANDRY_DRY_RUN:-} != 1 ]]; then
  step "Activation de l'extension GNOME de Landry"
  if [[ $list == "@as []" || $list == "[]" ]]; then list="['$UUID']"; else list="${list%]}, '$UUID']"; fi
  gsettings set org.gnome.shell enabled-extensions "$list"
  gnome-extensions enable "$UUID" 2>/dev/null || true
  echo "  Fenêtres ouvertes et historique du presse-papiers : reconnecte-toi une fois à ta session."
fi

# ---------- 8. Restart ----------
if [[ ${LANDRY_DRY_RUN:-} != 1 ]]; then
  step "Démarrage"
  if systemctl --user is-enabled landry.service >/dev/null 2>&1; then
    systemctl --user restart landry.service
  else
    setsid /usr/bin/landry daemon >/dev/null 2>&1 < /dev/null &
  fi
  for _ in $(seq 50); do landry ping >/dev/null 2>&1 && break; sleep 0.1; done
  landry doctor || true
fi

printf '\n\033[1;32mLandry %s est installé.\033[0m Ouvre la barre avec ton raccourci, ou tape : landry\n' "$VERSION"
echo "Pour mettre à jour plus tard : relance ce script. Pour désinstaller : $0 --uninstall"
