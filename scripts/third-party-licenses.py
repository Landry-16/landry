#!/usr/bin/env python3
"""Regenerate THIRD_PARTY_LICENSES.md: every component shipped inside the Landry package, with
its license and the license texts it carries (their copyright notices included).

    python3 scripts/third-party-licenses.py > THIRD_PARTY_LICENSES.md

Rust: the crates compiled into the binary (normal dependencies only, Linux x86_64), read from
`cargo metadata` and the crate sources. Interface: the packages bundled by Vite. Data: the
emoji names. Run it again after changing dependencies.
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# Packages that end up in the interface bundle (the rest of node_modules is build tooling).
NPM = ["svelte", "@tauri-apps/api", "clsx", "esm-env"]
LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|notice|unlicense)", re.I)


def files_of(directory):
    """License texts found at the top of a package, by file name."""
    out = []
    for name in sorted(os.listdir(directory)):
        path = os.path.join(directory, name)
        if LICENSE_FILE.match(name) and os.path.isfile(path):
            with open(path, encoding="utf-8", errors="replace") as f:
                out.append(f.read().strip())
    return out


def rust():
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--filter-platform", "x86_64-unknown-linux-gnu", "--offline"],
        cwd=os.path.join(ROOT, "src-tauri")))
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = meta["resolve"]["root"]
    seen, todo = set(), [root]
    while todo:
        node = nodes[todo.pop()]
        for dep in node["deps"]:
            # kind None: a normal dependency, linked into the binary (not build or dev).
            if any(k["kind"] is None for k in dep["dep_kinds"]) and dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                todo.append(dep["pkg"])
    for pid in seen:
        p = packages[pid]
        yield p["name"], p["version"], p["license"] or p.get("license_file") or "?", files_of(os.path.dirname(p["manifest_path"]))


def npm():
    for name in NPM:
        directory = os.path.join(ROOT, "node_modules", name)
        if not os.path.isdir(directory):
            continue
        with open(os.path.join(directory, "package.json")) as f:
            p = json.load(f)
        yield name, p["version"], p.get("license", "?"), files_of(directory)


def main():
    components = sorted(rust(), key=lambda c: (c[0], c[1])) + list(npm())
    components.append(("emojibase-data (noms des emojis)", "-", "MIT", []))
    components.append(("Unicode CLDR (annotations des emojis, via emojibase)", "-", "Unicode-3.0", []))
    # Identical texts (the same Apache license in hundreds of crates) are printed once.
    texts = {}
    for name, version, _, found in components:
        for text in found:
            texts.setdefault(text, []).append(f"{name} {version}")

    out = [
        "# Licences des composants tiers",
        "",
        "Landry est distribué sous la licence MIT (fichier `LICENSE`). Le paquet contient aussi les",
        "composants ci-dessous, chacun sous sa propre licence. Fichier généré par",
        "`scripts/third-party-licenses.py` : ne pas le modifier à la main.",
        "",
        "Services en ligne interrogés à ta demande, dont le contenu n'est pas distribué avec Landry :",
        "les définitions et synonymes du Wiktionnaire (licence CC BY-SA 4.0, auteurs du Wiktionnaire),",
        "Datamuse (api.datamuse.com), les taux de change de la Banque centrale européenne, wttr.in (météo).",
        "",
        "| Composant | Version | Licence |",
        "| --- | --- | --- |",
    ]
    out += [f"| {n} | {v} | {l} |" for n, v, l, _ in components]
    out += ["", "## Textes des licences", ""]
    for text, users in texts.items():
        out += ["Utilisé par : " + ", ".join(users), "", "```text", text, "```", ""]
    out += [
        "Composants sous MPL-2.0 (cssparser, selectors, nucleo...) : utilisés sans modification, leur",
        "code source est disponible sur https://crates.io sous le nom et la version indiqués.",
        "",
        "Composants sans fichier de licence dans leur paquet : le texte standard de la licence indiquée",
        "dans le tableau s'applique (https://spdx.org/licenses/).",
        "",
    ]
    # House rule: no em dash in the repository. The texts keep their meaning with "--".
    sys.stdout.write("\n".join(out).replace(chr(0x2014), "--"))


main()
