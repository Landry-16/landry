#!/usr/bin/env bash
# Landry: install, update or reinstall in one command, every dependency included.
#
#   ./install.sh               from a clone: build these sources and install them
#   ./install.sh --release     install the package of the latest GitHub release (no build)
#   ./install.sh --source      always build from source
#   ./install.sh --uninstall   remove Landry; settings, notes and history are kept
#
# Without a clone (public repository): curl -fsSL https://raw.githubusercontent.com/Landry-16/landry/dev/install.sh | bash
# installs the latest release, and builds from source only when no package is available.
# Environment: LANDRY_BRANCH (dev), LANDRY_SRC (~/.local/src/landry), LANDRY_DRY_RUN=1 prints the
# commands that need sudo instead of running them.
set -euo pipefail

REPO_SLUG=Landry-16/landry
BRANCH=${LANDRY_BRANCH:-dev}
SRC=${LANDRY_SRC:-$HOME/.local/src/landry}
NODE_DIR=$HOME/.local/share/landry/node
SHORTCUT=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/landry/
UUID=landry@landry-16.github.io

step() { printf '\n\033[1;33m%s\033[0m\n' "$*"; }
die() { printf '\033[1;31mErreur : %s\033[0m\n' "$*" >&2; exit 1; }
root() { if [[ ${LANDRY_DRY_RUN:-} == 1 ]]; then echo "  [dry-run] sudo $*"; else sudo "$@"; fi; }
dry() { [[ ${LANDRY_DRY_RUN:-} == 1 ]]; }

[[ $(id -u) -ne 0 ]] || die "lance ce script avec ton utilisateur, pas en root : il demande sudo quand il en a besoin."
command -v apt-get >/dev/null || die "Landry s'installe sur Ubuntu ou Debian (apt-get introuvable)."

# Remove $2 from the gsettings string list $1, keeping a valid empty list.
list_without() {
  local list
  list=$(sed -e "s#, '$2'##; s#'$2', ##; s#'$2'##" <<<"$1")
  [[ $list == "[]" ]] && list="@as []"
  echo "$list"
}

stop_landry() {
  dry && return
  if pgrep -u "$(id -u)" -x landry >/dev/null; then
    landry quit >/dev/null 2>&1 || true
    sleep 0.5
    pkill -u "$(id -u)" -x landry 2>/dev/null || true
  fi
}

uninstall() {
  step "Désinstallation de Landry"
  stop_landry
  if [[ -f $HOME/.config/systemd/user/landry.service ]]; then
    systemctl --user disable landry.service >/dev/null 2>&1 || true
    rm -f "$HOME/.config/systemd/user/landry.service"
    systemctl --user daemon-reload || true
  fi
  rm -f "$HOME/.config/autostart/landry.desktop"
  local keys ext
  keys=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "@as []")
  if [[ $keys == *"$SHORTCUT"* ]]; then
    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$(list_without "$keys" "$SHORTCUT")"
  fi
  ext=$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo "@as []")
  if [[ $ext == *"$UUID"* ]]; then
    gsettings set org.gnome.shell enabled-extensions "$(list_without "$ext" "$UUID")"
  fi
  if dpkg-query -W landry >/dev/null 2>&1; then root apt-get remove -y landry; fi
  echo "Landry est désinstallé. Tes réglages restent dans ~/.config/landry et tes données dans ~/.local/share/landry."
}

# ---------- Prebuilt package from the latest GitHub release ----------
download_release() {
  [[ $(dpkg --print-architecture) == amd64 ]] || return 1
  local tmp url
  tmp=$(mktemp -d)
  # A private repository needs the GitHub CLI, logged in.
  if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
    gh release download --repo "$REPO_SLUG" --pattern '*_amd64.deb' --dir "$tmp" >/dev/null 2>&1 || true
  fi
  if ! ls "$tmp"/*.deb >/dev/null 2>&1; then
    url=$(curl -fsSL "https://api.github.com/repos/$REPO_SLUG/releases/latest" 2>/dev/null |
      sed -n 's/.*"browser_download_url": *"\([^"]*_amd64\.deb\)".*/\1/p' | head -1) || true
    if [[ -n $url ]]; then curl -fsSL "$url" -o "$tmp/landry_amd64.deb" || true; fi
  fi
  DEB=$(ls "$tmp"/*.deb 2>/dev/null | head -1 || true)
  [[ -n $DEB ]]
}

# ---------- Build from source ----------
build_from_source() {
  local pkgs=(build-essential curl git file pkg-config xz-utils libwebkit2gtk-4.1-dev libssl-dev libxdo-dev libdbus-1-dev
    libayatana-appindicator3-dev librsvg2-dev)
  local missing=() p
  for p in "${pkgs[@]}"; do
    dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || missing+=("$p")
  done
  if ((${#missing[@]})); then
    step "Paquets système manquants : ${missing[*]}"
    # A broken third-party repository must not stop the install: the known lists are enough.
    root apt-get update || echo "  apt-get update a échoué, l'installation continue avec les listes connues."
    root apt-get install -y "${missing[@]}"
  fi

  if [[ -f $HERE/src-tauri/Cargo.toml ]]; then
    cd "$HERE"
  else
    step "Sources dans $SRC (branche $BRANCH)"
    if [[ -d $SRC/.git ]]; then
      git -C "$SRC" fetch --quiet origin "$BRANCH" && git -C "$SRC" checkout --quiet "$BRANCH" && git -C "$SRC" merge --quiet --ff-only "origin/$BRANCH"
    else
      # HTTPS for a public repository, SSH (your GitHub key) for a private one.
      GIT_TERMINAL_PROMPT=0 git clone --quiet --branch "$BRANCH" "https://github.com/$REPO_SLUG.git" "$SRC" 2>/dev/null ||
        git clone --quiet --branch "$BRANCH" "git@github.com:$REPO_SLUG.git" "$SRC" ||
        die "impossible de récupérer les sources (dépôt privé : il faut une clé SSH GitHub)"
    fi
    cd "$SRC"
  fi

  export PATH=$HOME/.cargo/bin:$PATH
  if ! command -v cargo >/dev/null; then
    step "Installation de Rust (rustup, pour ton utilisateur)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi

  # Vite needs Node 20.19+ or 22.12+.
  node_ok() {
    command -v node >/dev/null && node -e '
      const [a, b] = process.versions.node.split(".").map(Number);
      process.exit(a > 22 || (a === 22 && b >= 12) || (a === 20 && b >= 19) ? 0 : 1)'
  }
  export PATH=$NODE_DIR/bin:$PATH
  if ! node_ok; then
    step "Installation de Node 22 dans $NODE_DIR (sans toucher au Node du système)"
    local arch base tmp file
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

  local lock version
  lock=$(sha256sum package-lock.json | cut -d' ' -f1)
  if [[ ! -f node_modules/.landry-lock || $(cat node_modules/.landry-lock) != "$lock" ]]; then
    step "Dépendances JavaScript"
    npm ci --no-audit --no-fund --loglevel=error
    echo "$lock" >node_modules/.landry-lock
  fi
  version=$(node -p "require('./src-tauri/tauri.conf.json').version")
  step "Construction de Landry $version"
  echo "  Première fois : 5 à 10 minutes. Ensuite : environ 2 minutes."
  echo "  La dernière étape, landry(bin), reste immobile 1 à 2 minutes : c'est normal, ne l'interromps pas."
  npx tauri build --bundles deb
  DEB=$PWD/src-tauri/target/release/bundle/deb/Landry_${version}_$(dpkg --print-architecture).deb
  [[ -f $DEB ]] || die "paquet introuvable : $DEB"
}

# ---------- Main ----------
MODE=auto
case ${1:-} in
  --uninstall) uninstall; exit 0 ;;
  --release) MODE=release ;;
  --source) MODE=source ;;
  "") ;;
  *) die "option inconnue : $1 (voir l'en-tête de ce script)" ;;
esac
HERE=$(cd "$(dirname "${BASH_SOURCE[0]:-.}")" 2>/dev/null && pwd || true)
# A clone is for working on Landry: build what is there. Otherwise prefer the released package.
if [[ $MODE == auto ]]; then
  if [[ -f $HERE/src-tauri/Cargo.toml ]]; then MODE=source; else MODE=release; fi
fi

DEB=
if [[ $MODE == release ]]; then
  step "Téléchargement du paquet de la dernière version"
  if download_release; then
    echo "  $(basename "$DEB")"
  else
    echo "  Aucun paquet publié accessible : construction depuis les sources."
    MODE=source
  fi
fi
if [[ $MODE == source ]]; then build_from_source; fi
VERSION=$(dpkg-deb -f "$DEB" Version)

# apt adds the runtime and the recommended tools (playerctl, ffmpeg, tesseract...).
step "Installation du paquet"
stop_landry
root apt-get install -y --reinstall --allow-downgrades "$DEB"

# GNOME extension (open windows, clipboard history): starts at the next login under Wayland.
ext=$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo "@as []")
if [[ $ext != *"'$UUID'"* ]] && ! dry; then
  step "Activation de l'extension GNOME de Landry"
  if [[ $ext == "@as []" || $ext == "[]" ]]; then ext="['$UUID']"; else ext="${ext%]}, '$UUID']"; fi
  gsettings set org.gnome.shell enabled-extensions "$ext"
  gnome-extensions enable "$UUID" 2>/dev/null || true
  echo "  Fenêtres ouvertes et historique du presse-papiers : reconnecte-toi une fois à ta session."
fi

if ! dry; then
  step "Démarrage"
  if systemctl --user is-enabled landry.service >/dev/null 2>&1; then
    systemctl --user restart landry.service
  else
    setsid /usr/bin/landry daemon >/dev/null 2>&1 </dev/null &
  fi
  for _ in $(seq 50); do landry ping >/dev/null 2>&1 && break; sleep 0.1; done
  landry doctor || true
fi

printf '\n\033[1;32mLandry %s est installé.\033[0m Ouvre la barre avec ton raccourci, ou tape : landry\n' "$VERSION"
echo "Pour mettre à jour plus tard : relance ce script. Pour désinstaller : $0 --uninstall"
