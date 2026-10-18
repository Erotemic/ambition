#!/usr/bin/env python3
"""Census of the LIVE strike volumes a fighter's moves throw, against what its
moveset authored.

Reads the manifests `smash_move_sheet.py` rendered (the engine's own volumes per
shot) and the `moveset_export` bundle (the authored volumes). Per move:

- the live volume's size and reach, in units of the fighter's standing height;
- whether it is the AUTHORED rect or something else (a vfx-tagged volume takes
  its sheet row's published hit area, `moveset/mod.rs` §7.1);
- flags: TINY (smaller than a fist, area < 0.06 h^2), CLOSE (reaches less than
  0.35 h past the body's centre on its strike side), BEHIND (a forward move whose
  volume is behind the body, or a back move whose volume is in front).

    python3 scripts/smash_hitbox_census.py SHEETS_DIR EXPORT.json [FIGHTER...]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

FORWARD = {"attack", "attack_forward", "attack_dash", "attack_air_forward", "smash_forward", "special_forward"}
BACK = {"attack_air_back"}


def main() -> int:
    sheets, export = Path(sys.argv[1]), json.loads(Path(sys.argv[2]).read_text())
    chars = {c["id"]: c for c in export["characters"]}
    wanted = sys.argv[3:] or sorted(p.name for p in (sheets / "renders").iterdir())
    print(f"{'fighter':28} {'verb':19} {'move':24} {'live w x h (h)':>15} {'reach':>6} {'src':>8}  flags")
    for fid in wanted:
        moves = {m["id"]: m for m in chars.get(fid, {}).get("moves", [])}
        for vdir in sorted((sheets / "renders" / fid).iterdir()):
            mf = vdir / "manifest.json"
            if not mf.exists():
                continue
            m = json.loads(mf.read_text())
            best = None
            for shot in m["shots"]:
                subj = next((b for b in shot["observation"]["bodies"] if b.get("role") == "subject"), None)
                if not subj:
                    continue
                for st in shot["observation"]["strikes"]:
                    if not st.get("subject_owned"):
                        continue
                    area = st["half"][0] * st["half"][1] * 4
                    if best is None or area > best[0]:
                        best = (area, st, subj, shot.get("move"))
            if not best:
                continue
            area, st, subj, move_id = best
            h = subj["collision"]["half"][1] * 2
            face = subj.get("facing", 1.0)
            dx = (st["pos"][0] - subj["collision"]["pos"][0]) * face
            reach = (dx + st["half"][0]) / h
            near = (dx - st["half"][0]) / h
            authored = None
            mv = moves.get(move_id or "")
            if mv:
                for w in mv["windows"]:
                    for v in w["volumes"]:
                        if v.get("half_extents"):
                            hw, hh = v["half_extents"]
                            if abs(hw - st["half"][0]) < 0.6 and abs(hh - st["half"][1]) < 0.6:
                                authored = True
                if authored is None:
                    authored = False
            flags = []
            if area / (h * h) < 0.06:
                flags.append("TINY")
            verb = vdir.name
            if verb in FORWARD and reach < 0.35:
                flags.append("CLOSE")
            if verb in FORWARD and reach < 0.0:
                flags.append("BEHIND")
            if verb in BACK and near > -0.05:
                flags.append("FRONT")
            src = "authored" if authored else ("SHEET" if authored is False else "?")
            print(f"{fid:28} {verb:19} {str(move_id):24} {st['half'][0]*2/h:5.2f} x {st['half'][1]*2/h:4.2f}   {reach:6.2f} {src:>8}  {' '.join(flags)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
