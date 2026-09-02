#!/usr/bin/env python3
"""Which FX sheet ROWS can any content actually request, and WHO asks for them?

`--owners` answers the second question. The residency plan for these sheets
(asset open work 2) proposed demanding `<character>_vfx` beside `<character>`'s
pages -- which reads as if the sheet name derives from the character. IT DOES
NOT, and three different conventions are in play: `noether_vfx` belongs to
`npc_emmy_noether`, `carl_stargan_vfx` sits beside a sheet target `carl_runga`,
and `pca_vfx` / `patent_clerk_vfx` / `george_booul_vfx` use bare ids with no
`npc_` prefix. So ownership cannot be derived from the filename and must not be
hand-guessed either. What CAN be measured is which files name a sheet's rows --
a sheet whose rows are named only by one character's moveset belongs to that
character, and the evidence says so rather than the name.

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
corpus_files = collections.defaultdict(set)
for rel in tracked:
    if not rel.endswith((".rs", ".ron", ".yarn", ".json")):
        continue
    if "/assets/sprites/" in rel:      # the baked manifests are the definition, not a request
        continue
    try:
        text = (repo / rel).read_text(errors="ignore")
    except OSError:
        continue
    for name in re.findall(r'"([a-z_0-9]{3,40})"', text):
        corpus_names.add(name)
        corpus_files[name].add(rel)

owners_mode = "--owners" in sys.argv

total_rows = total_named = 0
dead_sheets = []
owner_report: list[tuple[str, list[tuple[str, int]]]] = []
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
    if owners_mode:
        # Who names this sheet's rows, by how many of them. A single dominant
        # file IS the ownership claim; several unrelated ones mean the sheet is
        # shared and must stay resident.
        askers: collections.Counter = collections.Counter()
        for r in named:
            for rel in corpus_files[r]:
                askers[rel] += 1
        # ⛔ NOT `most_common`. Ties there break in insertion order, which comes
        # from a SET and therefore from Python's per-run string hashing: two runs
        # printed different top-4 orders for the same tree. The counts were
        # stable and the ORDER was not, which is the shape that gets published as
        # a finding. Sort by count then path.
        ranked = sorted(askers.items(), key=lambda kv: (-kv[1], kv[0]))[:4]
        owner_report.append((t, ranked))

print()
print(f"{len(targets)} sheets, {total_rows} rows, {total_named} named, "
      f"{total_rows - total_named} named by nothing")
print(f"sheets with NO row named by anything: {len(dead_sheets)} {dead_sheets}")

if owners_mode:
    print()
    print("WHO NAMES EACH SHEET'S ROWS (top 4 files, by rows named)")
    print("⛔ A sheet with no dominant single asker is SHARED and must stay resident.")
    for t, askers in owner_report:
        print(f"\n  {t}")
        if not askers:
            print("      (nothing names any row)")
        for rel, count in askers:
            print(f"      {count:>3}  {rel}")
