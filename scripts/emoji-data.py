#!/usr/bin/env python3
"""Regenerate src-tauri/data/emoji.tsv from emojibase-data (French and English names).

    npm pack emojibase-data && tar -xzf emojibase-data-*.tgz
    python3 scripts/emoji-data.py package > src-tauri/data/emoji.tsv

Columns: emoji, order, French name, French keywords, English name, English keywords.
Skin tone variants and components (group 2) are left out; regional letters too (no order).
"""
import json
import sys

root = sys.argv[1]
fr = {e["hexcode"]: e for e in json.load(open(f"{root}/fr/compact.json"))}
en = {e["hexcode"]: e for e in json.load(open(f"{root}/en/compact.json"))}
rows = []
for code, e in fr.items():
    if "order" not in e or e.get("group") == 2:
        continue
    o = en.get(code, {})
    clean = lambda s: s.replace("\t", " ").replace("\n", " ")
    rows.append((e["order"], e["unicode"], clean(e["label"]), clean(",".join(e.get("tags", []))), clean(o.get("label", "")), clean(",".join(o.get("tags", [])))))
for order, uni, fl, ft, el, et in sorted(rows):
    print(f"{uni}\t{order}\t{fl}\t{ft}\t{el}\t{et}")
