"""Enemy beat cues, heard in their timing: a reel per creature and one of all.

Jon, 2026-10-30: "Do the enemies have sound effects to help their telegraph
and for their attacks?" A catalog row's `cues` (notice, tell, swing, hit)
play on the creature's beats (docs/planning/game/enemy-patterns.md, "What
the sound gives a character"). This reads the cues and windups from the
catalog and lays them out as the game plays them, so they can be judged by
ear in context:

    notice, 0.6 s, tell, its windup, swing, 60 ms, hit

The windup is the one its brain times (a flyer's hang before its dive, a
volley's coil) when it has one, else its action set's melee `windup_s`. A
cue the row does not state is left out. The overall reel starts with the
player's own `player.slash` and `player.hit`, the level the enemy cues sit
under.

Run with the SFX renderer's venv, from the repo root, after the cues are
rendered (`scripts/regen/sfx.sh`):

    ~/.cache/ambition-tool-venvs/ambition_sfx_renderer/bin/python scripts/enemy_cue_reels.py

Writes `untracked/sfx-candidates/enemy_cues/` (untracked): `solo/<cue>.wav`
(every cue a row names, and every recipe whose notes say it is an enemy
beat cue),
`reel/<row>.wav`, `reel/all.wav` and `INDEX.md`. Levels are as rendered:
nothing is normalized, so the reels play the cues at their shipped levels.
"""

from __future__ import annotations

import re
from pathlib import Path

import numpy as np
import soundfile as sf
import yaml

REPO = Path(__file__).resolve().parents[1]
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"
RENDERER = REPO / "tools/ambition_sfx_renderer"
OUTPUT = RENDERER / "output"
ACTIVE = RENDERER / "sounds/active"
HERE = REPO / "untracked/sfx-candidates/enemy_cues"
SR = 48000
NOTICE_GAP_S = 0.6
HIT_AFTER_S = 0.06
BEATS = ("notice", "tell", "swing", "hit")


def load(cue: str) -> np.ndarray:
    for ext in ("wav", "ogg"):
        path = OUTPUT / cue / f"{cue}.{ext}"
        if path.exists():
            audio, sr = sf.read(path, always_2d=True, dtype="float32")
            assert sr == SR, f"{cue}: {sr} Hz"
            return audio
    raise FileNotFoundError(f"{cue} is not rendered: run scripts/regen/sfx.sh")


def blocks(text: str, indent: int) -> dict[str, str]:
    """Each `"name": (` block at `indent` spaces, up to its closing `),`."""
    pad = " " * indent
    out = {}
    for m in re.finditer(rf'^{pad}"([A-Za-z0-9_]+)": \($', text, re.M):
        end = text.find(f"\n{pad}),", m.end())
        out[m.group(1)] = text[m.end():end]
    return out


def rows_with_cues() -> list[dict]:
    text = CATALOG.read_text()
    rows = blocks(text, 8)
    rows_out = []
    for row_id, body in rows.items():
        cues_m = re.search(r"^\s*cues: \((.*)\),$", body, re.M)
        if not cues_m:
            continue
        cues = dict(re.findall(r'(\w+): Some\("([^"]+)"\)', cues_m.group(1)))
        set_m = re.search(r'default_action_set: "([^"]+)"', body)
        melee = None
        if set_m and set_m.group(1) in rows:
            w = re.search(r"windup_s: ([0-9.]+)", rows[set_m.group(1)])
            melee = float(w.group(1)) if w else None
        # A named profile (`named_autonomous_profile`) states the brain's windup for the rows that wear it.
        prof_m = re.search(r'named_autonomous_profile: Some\("([^"]+)"\)', body)
        mind = body + (rows.get(prof_m.group(1), "") if prof_m else "")
        timed = re.search(r"dive_tell_s: ([0-9.]+)", mind) or re.search(r"ranged_volley: Some\(\(windup_s: ([0-9.]+)", mind)
        windup = float(timed.group(1)) if timed else (melee or 0.3)
        name = re.search(r'display_name: "([^"]+)"', body)
        rows_out.append({"id": row_id, "name": name.group(1) if name else row_id, "cues": cues, "windup": windup})
    return rows_out


def place(track: np.ndarray, audio: np.ndarray, at_s: float) -> np.ndarray:
    s = int(round(at_s * SR))
    need = s + len(audio)
    if need > len(track):
        track = np.concatenate([track, np.zeros((need - len(track), 2), np.float32)])
    track[s:need] += audio
    return track


def reel(row: dict) -> np.ndarray:
    cues = row["cues"]
    track = np.zeros((0, 2), np.float32)
    t = 0.0
    if "notice" in cues:
        notice = load(cues["notice"])
        track = place(track, notice, t)
        t += len(notice) / SR + NOTICE_GAP_S
    if "tell" in cues:
        track = place(track, load(cues["tell"]), t)
    t += row["windup"]
    if "swing" in cues:
        track = place(track, load(cues["swing"]), t)
    if "hit" in cues:
        track = place(track, load(cues["hit"]), t + HIT_AFTER_S)
    return np.concatenate([track, np.zeros((int(0.4 * SR), 2), np.float32)])


def loud100(audio: np.ndarray) -> float:
    x = audio.mean(axis=1)
    w = min(len(x), int(0.1 * SR))
    return float(10 * np.log10(np.convolve(x ** 2, np.ones(w) / w, mode="valid").max() + 1e-12))


def write(path: Path, audio: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(path, np.clip(audio, -1, 1), SR, subtype="PCM_16")


def main() -> None:
    rows = rows_with_cues()
    named = {cue for row in rows for cue in row["cues"].values()}
    # The beat cues authored for this (their notes say so), named by a row or not yet.
    authored = {p.name[: -len(".sfx.yaml")] for p in ACTIVE.glob("enemy.*.sfx.yaml")
                if "Enemy beat cue" in str(yaml.safe_load(p.read_text()).get("notes", ""))}
    for old in (HERE / "solo").glob("*.wav"):
        old.unlink()
    lines = ["# Enemy beat cues", "",
             "Written by `scripts/enemy_cue_reels.py`. Each reel: notice, 0.6 s, tell, its windup, swing, "
             "60 ms, hit (a cue the row does not state is left out). `reel/all.wav` starts with the player's "
             "`player.slash` and `player.hit` for level. Levels are as shipped.", "",
             "| row | windup s | notice | tell | swing | hit | reel |", "|---|---:|---|---|---|---|---|"]
    whole = np.zeros((0, 2), np.float32)
    t = 0.0
    for ref in ("player.slash", "player.hit"):
        whole = place(whole, load(ref), t)
        t += 0.5
    t += 0.8
    for row in rows:
        audio = reel(row)
        write(HERE / "reel" / f"{row['id']}.wav", audio)
        whole = place(whole, audio, t)
        t += len(audio) / SR + 0.8
        cells = [f"`{row['cues'][b]}`" if b in row["cues"] else "-" for b in BEATS]
        lines.append(f"| {row['name']} (`{row['id']}`) | {row['windup']:.2f} | " + " | ".join(cells)
                     + f" | `reel/{row['id']}.wav` |")
    write(HERE / "reel" / "all.wav", whole)
    lines += ["", "## Every cue", "", "| cue | ms | loudest 100 ms dBFS | named by a row | description |",
              "|---|---:|---:|---|---|"]
    for cue in sorted(named | authored):
        audio = load(cue)
        write(HERE / "solo" / f"{cue}.wav", audio)
        recipe = ACTIVE / f"{cue}.sfx.yaml"
        notes = yaml.safe_load(recipe.read_text()).get("notes", "") if recipe.exists() else ""
        notes = str(notes).split(" Enemy beat cue")[0]
        used = "yes" if cue in named else "no"
        lines.append(f"| `{cue}` | {len(audio) * 1000 // SR} | {loud100(audio):.1f} | {used} | {notes} |")
    (HERE / "INDEX.md").write_text("\n".join(lines) + "\n")
    print(f"{len(rows)} reels, {len(named | authored)} solos -> {HERE}")


if __name__ == "__main__":
    main()
