# Landry

Landry is a keyboard companion for Linux (built around GNOME) and Windows. One shortcut opens a bar: type a few letters to launch an app or switch to its window, open a file or a project, calculate, convert, paste a snippet or something you copied earlier, write a note, start a timer, control the music, or run a script. Everything stays on your computer, except the online services you choose to connect (GitHub, Vercel, translation, weather, dictionary).

Target: GNOME on Wayland (Ubuntu, Debian, Fedora, openSUSE, Arch…), and Windows 10 or 11. Stack: Tauri 2 (Rust) and Svelte 5. The interface is in French and English.

[Version française](README.md) · [Features and their state](FEATURES.md)

## Install

One command installs, updates or reinstalls Landry with everything it needs:

```bash
./install.sh
```

From a clone it builds your sources. `./install.sh --release` installs the package of the latest GitHub release instead, with nothing to build. Without a clone, once the repository is public:

```bash
curl -fsSL https://raw.githubusercontent.com/Landry-16/landry/dev/install.sh | bash
```

The script installs what is missing (system packages; Rust and Node 22 when it has to build), installs the package with its recommended tools (playerctl, FFmpeg, ImageMagick, Tesseract, ripgrep…), enables the GNOME extension, restarts Landry and ends with `landry doctor`. It only asks for your password for the package manager. `./install.sh --uninstall` removes Landry and keeps your settings and data.

Each tag `v*` pushed to GitHub builds the `.deb`, `.rpm`, AppImage and Windows installer and attaches them to a release (`.github/workflows/release.yml`).

### Distributions and Windows

`install.sh` detects the package manager: apt (Debian, Ubuntu, Mint, Pop!_OS) installs the `.deb`, dnf (Fedora) and zypper (openSUSE) the `.rpm`, pacman (Arch, Manjaro, EndeavourOS) a package made from the `.deb`. On any other distribution, take the AppImage of the [latest release](https://github.com/Landry-16/landry/releases/latest). Outside GNOME the bar works, but the shortcut, the extension and the portals (color picker, screenshot, auto-paste) need setting up by hand.

On Windows 10 or 11, run `Landry_*_x64-setup.exe` from the latest release, then press Ctrl+Alt+Space. Change the shortcut and the start with the session in `/bienvenue`. Apps come from the Start Menu. Not on Windows yet: the GNOME extension (windows, clipboard history), color picker, OCR, auto-paste, "Open with" and app icons; `capture` opens the Windows Snipping Tool.

## First launch

Run `landry`, or Landry from the app list. A one-screen assistant opens once, with five optional steps: the keyboard shortcut (press the keys you want, or keep Ctrl+Alt+Space), start with the session, the look in one click, the GNOME extension for windows and clipboard history, and the optional tools that are missing. Reopen it with `/bienvenue` or `landry setup`. Ctrl+Alt+Space is set up at the first start, even if you skip the assistant.

Drag the bar by any free spot (around the field, the preset name) to move it: it opens there from then on. The position can also be typed in Settings, General (Recenter puts it back in the middle).

## In the bar

- Enter: main action. Ctrl+K: every action of the item. Tab: complete. Up on an empty bar: history. Esc: clear, then close. Ctrl+,: settings. F1: help. Ctrl+plus and Ctrl+minus: text size.
- Dangerous actions (trash, stopping a process, suspend) need a second Enter.

| Type | For |
| --- | --- |
| `vsc`, `settings` | apps (initials, no accents, typos allowed); an app already open switches to its window |
| `invoice`, `pdf: invoice`, `img: <7j` | files, with type and date filters; Ctrl+K converts them |
| `nacre` | git projects: editor, terminal, pull, PR, npm scripts |
| `12*7%`, `5 km en m`, `10 EUR en USD` | calculations, units, currencies |
| `15h Paris en New York`, `dans 3 semaines` | times and dates |
| `todo call tomorrow 10h`, `minuteur 10 min`, `pomodoro` | tasks, timers |
| `note idea`, `notes` | Markdown notes |
| `uuid`, `b64 text`, `#ff8800`, `qr text` | toolbox |

## Slash commands

Type `/` to see them all. Any keyword above also works with a slash.

| Command | Does |
| --- | --- |
| `/fenetres`, `/presse-papiers` | switch to an open window; clipboard history (last 100 texts, in memory only, password managers ignored) |
| `/emoji red heart`, `/car dash` | emojis and special characters, searched in French or English |
| `/def house`, `/syn fast` | definitions and synonyms (Wiktionary, Datamuse); `/def en …` for English |
| `/contenu quote 2024` | files whose content contains a text |
| `/spotify daft punk`, `/media` | music: search in Spotify, current track and controls |
| `/volume 40`, `/muet`, `/luminosite 60` | sound and screen |
| `/sombre`, `/clair`, `/dnd`, `/wifi off` | system theme, do not disturb, Wi-Fi |
| `/verrouiller`, `/veille`, `/eteindre` | session |
| `/reglages sound`, `/tuer firefox`, `/meteo Lyon` | GNOME settings panels, processes, weather |

Aliases run whole commands: `pp = /spotify play` pauses the music in one word, and with `sp = /spotify`, typing `sp daft punk` searches Daft Punk.

## Windows and clipboard

Under Wayland only GNOME Shell can list windows or watch the clipboard. Landry ships a small extension (`landry@landry-16.github.io`) that does it on D-Bus and answers Landry only. It starts after the next login. The clipboard history is never written to disk, skips what password managers mark as secret, and can be paused or cleared.

## Presets and companion

The look of the bar comes from the active preset: position, size, light and dark colours, font, density, animation, modules, and the `style` block (the vouivre companion, ornaments, particles, secondary colour). Default, Minimal, Dense, Developer, Rose noire, Silk, Fresco and Pixel ship with Landry. The vouivre, a small pixel winged serpent, reacts to the bar: it listens while you type, looks at the results, gets lost when nothing matches, spits a flame when an action succeeds, and falls asleep. With `animation_ms` at 0 or reduced motion, nothing moves.

## Privacy

- Settings and presets: `~/.config/landry/`. Data (history, snippets, notes, tasks): `~/.local/share/landry/landry.db`.
- Tokens stay in the system keyring, never in a file or a backup. No telemetry.
- Network only when you ask: ECB rates, GitHub, Vercel, translation, `/meteo`, `/def`, `/syn`.

## Development

```bash
npm install
npx tauri dev -- -- daemon        # resident process in development, interface reloaded live
(cd src-tauri && cargo test)      # core tests
npm test                          # interface tests
```

The full guide is in French: [docs/GUIDE_DEVELOPPEMENT.md](docs/GUIDE_DEVELOPPEMENT.md). Progress and decisions: [STATUS.md](STATUS.md).

## License

Landry is released under the MIT license ([LICENSE](LICENSE)). The third-party components shipped in the package and their licenses are listed in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md), regenerated with `python3 scripts/third-party-licenses.py > THIRD_PARTY_LICENSES.md` after a dependency change.
