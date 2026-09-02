#!/usr/bin/env python3
"""Which FX sheet ROWS can any content actually request?

`load_game_assets` loads every sheet in `FX_SHEETS` at boot, in every room. An
effect is drawn by NAME (`FxId::new(row)`), so a row nothing names is art the
running game has no way to ask for. Corpus: every tracked .rs/.ron/.yarn/.json
EXCEPT the baked sheet manifests themselves (which name every row by
construction, and would make the answer trivially "all of them").
"""
import re, subprocess, pathlib, sys, collections

repo = pathlib.Path(subprocess.run(["git","rev-parse","--show-toplevel"],
                                   capture_output=True, text=True).stdout.strip())
fx_rs = (repo / "crates/ambition_sprite_sheet/src/fx.rs").read_text()
targets = re.findall(r'target: "([a-z_0-9]+)"', fx_rs)
sheets = repo / "crates/ambition_platformer2d_actor_monolith/assets/sprites"

tracked = subprocess.run(["git","ls-files"], capture_output=True, text=True,
                         cwd=repo).stdout.split()
corpus_names = set()
for rel in tracked:
    if not rel.endswith((".rs", ".ron", ".yarn", ".json")):
        continue
    if "/assets/sprites/" in rel:      # the baked manifests are the definition, not a request
        continue
    try:
        text = (repo / rel).read_text(errors="ignore")
    except OSError:
        continue
    corpus_names.update(re.findall(r'"([a-z_0-9]{3,40})"', text))

total_rows = total_named = 0
dead_sheets = []
print(f"{'sheet':<32} {'rows':>5} {'named':>6} {'unnamed':>8}")
for t in targets:
    manifest = sheets / f"{t}_spritesheet.ron"
    if not manifest.exists():
        print(f"{t:<32} (no baked manifest)")
        continue
    rows = sorted(set(re.findall(r'animation: "([a-z_0-9]+)"', manifest.read_text())))
    named = [r for r in rows if r in corpus_names]
    total_rows += len(rows)
    total_named += len(named)
    if not named:
        dead_sheets.append(t)
    print(f"{t:<32} {len(rows):>5} {len(named):>6} {len(rows)-len(named):>8}")

print()
print(f"{len(targets)} sheets, {total_rows} rows, {total_named} named, "
      f"{total_rows - total_named} named by nothing")
print(f"sheets with NO row named by anything: {len(dead_sheets)} {dead_sheets}")
