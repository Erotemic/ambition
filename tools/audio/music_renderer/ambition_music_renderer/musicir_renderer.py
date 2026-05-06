#!/usr/bin/env python3
"""Ambition MusicIR renderer.

A data-driven, non-ML music renderer for compact YAML music assets.

The renderer intentionally keeps composition out of Python code.  New cues should
be authored by changing YAML: instruments, motifs, sections, harmony, and layers.
The Python library interprets those declarative layers, emits MIDI events, renders
through either FluidSynth or a built-in orchestral/synth fallback, post-processes,
and exports OGG Vorbis section/stem assets plus a full soundtrack preview.
"""
from __future__ import annotations

import argparse
import copy
import dataclasses as dc
import hashlib
import json
import math
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Iterable
import wave
import gc
import os
import sys

import numpy as np
import pretty_midi
import soundfile as sf
import yaml
from scipy import signal

RENDERER_VERSION = "ambition-musicir-renderer-v0.6.4-reverb-drums-previews"
DEFAULT_SOUNDFONTS = [
    "/usr/share/sounds/sf2/TimGM6mb.sf2",
    "/usr/share/sounds/sf2/default-GM.sf2",
    "/usr/share/sounds/sf3/default-GM.sf3",
]

NOTE_CLASS = {
    "C": 0, "C#": 1, "Db": 1, "D": 2, "D#": 3, "Eb": 3,
    "E": 4, "F": 5, "F#": 6, "Gb": 6, "G": 7, "G#": 8,
    "Ab": 8, "A": 9, "A#": 10, "Bb": 10, "B": 11,
}

GM_PROGRAMS = {
    "acoustic_grand_piano": 0, "bright_piano": 1, "electric_grand_piano": 2,
    "honky_tonk_piano": 3, "electric_piano_1": 4, "electric_piano_2": 5,
    "harpsichord": 6, "clavinet": 7, "celesta": 8, "glockenspiel": 9,
    "music_box": 10, "vibraphone": 11, "marimba": 12, "xylophone": 13,
    "tubular_bells": 14, "dulcimer": 15, "drawbar_organ": 16,
    "church_organ": 19, "accordion": 21, "nylon_guitar": 24,
    "steel_guitar": 25, "jazz_guitar": 26, "clean_guitar": 27,
    "muted_guitar": 28, "overdrive_guitar": 29, "distortion_guitar": 30,
    "acoustic_bass": 32, "fingered_bass": 33, "picked_bass": 34,
    "fretless_bass": 35, "slap_bass_1": 36, "synth_bass_1": 38,
    "synth_bass_2": 39, "violin": 40, "viola": 41, "cello": 42,
    "contrabass": 43, "tremolo_strings": 44, "pizzicato_strings": 45,
    "orchestral_harp": 46, "timpani": 47, "string_ensemble_1": 48,
    "string_ensemble_2": 49, "synth_strings_1": 50, "synth_strings_2": 51,
    "choir_aahs": 52, "voice_oohs": 53, "synth_voice": 54,
    "orchestra_hit": 55, "trumpet": 56, "trombone": 57, "tuba": 58,
    "muted_trumpet": 59, "french_horn": 60, "brass_section": 61,
    "synth_brass_1": 62, "synth_brass_2": 63, "soprano_sax": 64,
    "alto_sax": 65, "tenor_sax": 66, "baritone_sax": 67, "oboe": 68,
    "english_horn": 69, "bassoon": 70, "clarinet": 71, "piccolo": 72,
    "flute": 73, "recorder": 74, "pan_flute": 75, "blown_bottle": 76,
    "shakuhachi": 77, "whistle": 78, "ocarina": 79, "lead_square": 80,
    "lead_saw": 81, "lead_calliope": 82, "lead_chiff": 83,
    "lead_charang": 84, "lead_voice": 85, "lead_fifths": 86,
    "lead_basslead": 87, "pad_new_age": 88, "pad_warm": 89,
    "pad_poly": 90, "pad_choir": 91, "pad_bowed": 92, "pad_metallic": 93,
    "pad_halo": 94, "pad_sweep": 95, "fx_rain": 96, "fx_soundtrack": 97,
    "fx_crystal": 98, "fx_atmosphere": 99, "fx_brightness": 100,
    "fx_goblins": 101, "fx_echoes": 102, "fx_scifi": 103, "sitar": 104,
    "banjo": 105, "shamisen": 106, "koto": 107, "kalimba": 108,
    "bagpipe": 109, "fiddle": 110, "shanai": 111, "tinkle_bell": 112,
    "agogo": 113, "steel_drums": 114, "woodblock": 115, "taiko_drum": 116,
    "melodic_tom": 117, "synth_drum": 118, "reverse_cymbal": 119,
}

DRUMS = {
    "kick": 36, "concert_bass_drum": 35, "side_stick": 37, "snare": 38,
    "hand_clap": 39, "electric_snare": 40, "floor_tom": 41, "closed_hat": 42,
    "low_tom": 43, "pedal_hat": 44, "mid_tom": 45, "open_hat": 46,
    "high_tom": 48, "crash": 49, "ride": 51, "china": 52, "ride_bell": 53,
    "tambourine": 54, "splash": 55, "cowbell": 56, "vibraslap": 58,
    "bongo_hi": 60, "bongo_low": 61, "conga_hi": 62, "conga_low": 64,
    "timbale_hi": 65, "timbale_low": 66, "agogo_hi": 67, "agogo_low": 68,
    "shaker": 70, "whistle_short": 71, "whistle_long": 72, "guiro_short": 73,
    "guiro_long": 74, "claves": 75, "woodblock_hi": 76, "woodblock_low": 77,
    "triangle_mute": 80, "triangle": 81,
}

ARTICULATION_GATE = {
    "staccato": 0.40, "spiccato": 0.34, "pluck": 0.46, "marcato": 0.68,
    "normal": 0.86, "tenuto": 0.98, "legato": 1.10, "pad": 1.02,
    "hit": 0.28, "bell": 1.40,
}

CC_NUMBERS = {
    "modulation": 1, "breath": 2, "volume": 7, "pan": 10,
    "expression": 11, "sustain": 64, "reverb": 91, "chorus": 93,
}

@dc.dataclass
class RenderContext:
    spec: dict[str, Any]
    sample_rate: int
    bpm: float
    beats_per_bar: float
    rng: np.random.Generator
    pm: pretty_midi.PrettyMIDI
    instruments: dict[str, pretty_midi.Instrument]
    groups: dict[str, str]
    section_starts: dict[str, int]
    motifs: dict[str, dict[str, Any]]

    def beat_to_time(self, beat: float) -> float:
        return beat * 60.0 / self.bpm

    def bar_to_beat(self, bar: float, beat: float = 0.0) -> float:
        return bar * self.beats_per_bar + beat

    def bar_to_time(self, bar: float, beat: float = 0.0) -> float:
        return self.beat_to_time(self.bar_to_beat(bar, beat))


def load_yaml(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf8") as f:
        return yaml.safe_load(f)


def choose_soundfont(path: str | None = None) -> str:
    if path:
        p = Path(path).expanduser()
        if not p.exists():
            raise FileNotFoundError(f"soundfont does not exist: {p}")
        return str(p)
    for candidate in DEFAULT_SOUNDFONTS:
        if Path(candidate).exists():
            return candidate
    return ""


def note_to_midi(note: str) -> int:
    note = note.strip()
    m = re.fullmatch(r"([A-G](?:#|b)?)(-?\d+)", note)
    if not m:
        raise ValueError(f"bad note name: {note!r}")
    return 12 * (int(m.group(2)) + 1) + NOTE_CLASS[m.group(1)]


def midi_to_note(num: int) -> str:
    return pretty_midi.note_number_to_name(int(num))


def clamp(v: float, lo: float, hi: float) -> float:
    return min(max(v, lo), hi)


def chord_intervals(chord_symbol: str) -> tuple[str, list[int], str | None]:
    raw = chord_symbol.strip()
    if "/" in raw:
        main, slash_bass = raw.split("/", 1)
    else:
        main, slash_bass = raw, None
    m = re.match(r"^([A-G](?:#|b)?)(.*)$", main)
    if not m:
        raise ValueError(f"cannot parse chord root from {chord_symbol!r}")
    root = m.group(1)
    suffix = m.group(2).lower()
    if "dim" in suffix or "o" in suffix:
        intervals = [0, 3, 6]
    elif "aug" in suffix or "+" in suffix:
        intervals = [0, 4, 8]
    elif "sus2" in suffix:
        intervals = [0, 2, 7]
    elif "sus4" in suffix or "sus" in suffix:
        intervals = [0, 5, 7]
    elif suffix.startswith("m") and not suffix.startswith("maj"):
        intervals = [0, 3, 7]
    else:
        intervals = [0, 4, 7]
    if "maj7" in suffix or "Δ" in suffix:
        intervals.append(11)
    elif "7" in suffix or "9" in suffix or "13" in suffix:
        intervals.append(10)
    if "6" in suffix and 9 not in intervals:
        intervals.append(9)
    if "9" in suffix or "add9" in suffix:
        intervals.append(14)
    if "#11" in suffix:
        intervals.append(18)
    elif "11" in suffix:
        intervals.append(17)
    if "b13" in suffix:
        intervals.append(20)
    elif "13" in suffix:
        intervals.append(21)
    if "b9" in suffix:
        intervals.append(13)
    if "#9" in suffix:
        intervals.append(15)
    seen = set()
    intervals = [i for i in intervals if not (i in seen or seen.add(i))]
    return root, intervals, slash_bass


def chord_pitches(chord_symbol: str, octave: int = 4, *, voicing: str = "closed") -> list[int]:
    root, intervals, slash_bass = chord_intervals(chord_symbol)
    root_midi = note_to_midi(f"{root}{octave}")
    notes = [root_midi + i for i in intervals]
    if voicing in {"open", "spread"} and len(notes) >= 4:
        notes = [notes[0] - 12, notes[2], notes[1] + 12, notes[3]] + [n + 12 for n in notes[4:]]
    elif voicing == "wide" and len(notes) >= 3:
        notes = [notes[0] - 12, notes[2], notes[1] + 12] + [n + 12 for n in notes[3:]]
    elif voicing == "drop2" and len(notes) >= 4:
        notes = notes[:]
        notes[-2] -= 12
        notes.sort()
    if slash_bass:
        bass_root = re.match(r"^([A-G](?:#|b)?)", slash_bass.strip())
        if bass_root:
            notes.insert(0, note_to_midi(f"{bass_root.group(1)}{octave - 1}"))
    return notes


def section_starts(sections: list[dict[str, Any]]) -> dict[str, int]:
    starts: dict[str, int] = {}
    cursor = 0
    for sec in sections:
        starts[sec["id"]] = cursor
        sec["start_bar"] = cursor
        cursor += int(sec["bars"])
    return starts


def add_cc(inst: pretty_midi.Instrument, number: int, value: int, time: float) -> None:
    inst.control_changes.append(pretty_midi.ControlChange(number=int(number), value=int(clamp(value, 0, 127)), time=float(time)))



def _program_family(program: int) -> str | None:
    """Classify a GM program into a fast-renderer family.

    Names returned here must match the branches in `_synth_note_fast`.
    Specific programs (harp, timpani) need to be checked before the
    string range that nominally contains them.
    """
    program = int(program)
    if program == 46:
        return "harp"
    if program == 47:
        return "timpani"
    if 9 <= program <= 15 or 112 <= program <= 119:
        return "mallet"
    if 0 <= program <= 7:
        return "piano"
    if 32 <= program <= 39:
        return "bass"
    if 40 <= program <= 45 or 48 <= program <= 51:
        return "string"
    if 52 <= program <= 54:
        return "choir"
    if 56 <= program <= 63:
        return "brass"
    if 64 <= program <= 79:
        return "wind"
    if 80 <= program <= 87:
        return "lead"
    if 88 <= program <= 103:
        return "pad"
    return None

def add_instrument(ctx: RenderContext, spec: dict[str, Any]) -> None:
    name = spec["name"]
    if spec.get("is_drum", False):
        inst = pretty_midi.Instrument(program=0, is_drum=True, name=name)
    else:
        program_name = spec.get("program", "string_ensemble_1")
        program = int(program_name) if isinstance(program_name, int) else GM_PROGRAMS[program_name]
        inst = pretty_midi.Instrument(program=program, is_drum=False, name=name)
    ctx.pm.instruments.append(inst)
    ctx.instruments[name] = inst
    ctx.groups[name] = spec.get("group", name)
    add_cc(inst, 7, int(spec.get("volume", 100)), 0.0)
    add_cc(inst, 10, int(spec.get("pan", 64)), 0.0)
    add_cc(inst, 11, int(spec.get("expression", 100)), 0.0)
    for key, cc_num in CC_NUMBERS.items():
        if key in spec and key not in {"volume", "pan", "expression"}:
            add_cc(inst, cc_num, int(spec[key]), 0.0)


def resolve_instruments(ctx: RenderContext, layer: dict[str, Any]) -> list[str]:
    if "instrument" in layer:
        return [layer["instrument"]]
    if "instruments" in layer:
        return list(layer["instruments"])
    if "group" in layer:
        return [name for name, group in ctx.groups.items() if group == layer["group"]]
    raise KeyError(f"layer needs instrument/instruments/group: {layer}")


def add_note(ctx: RenderContext, inst_name: str, pitch: int | str, bar: float, beat: float, dur_beats: float, vel: float, *, articulation: str = "normal", humanize_ms: float = 0.0, gate: float | None = None, pitch_scoop_cents: float = 0.0) -> None:
    if inst_name not in ctx.instruments:
        raise KeyError(f"unknown instrument {inst_name!r}")
    inst = ctx.instruments[inst_name]
    pitch_num = note_to_midi(pitch) if isinstance(pitch, str) else int(pitch)
    start_beat = ctx.bar_to_beat(bar, beat)
    start = ctx.beat_to_time(start_beat)
    if humanize_ms:
        start += float(ctx.rng.normal(0.0, humanize_ms / 1000.0))
    dur_scale = gate if gate is not None else ARTICULATION_GATE.get(articulation, 0.86)
    end = start + max(0.025, ctx.beat_to_time(dur_beats * dur_scale))
    start = max(0.0, start)
    if end <= start:
        end = start + 0.025
    velocity = int(clamp(round(vel), 1, 127))
    inst.notes.append(pretty_midi.Note(velocity=velocity, pitch=pitch_num, start=start, end=end))
    if pitch_scoop_cents:
        bend_value = int(clamp(pitch_scoop_cents / 200.0 * 8192.0, -8192, 8191))
        inst.pitch_bends.append(pretty_midi.PitchBend(pitch=bend_value, time=start))
        inst.pitch_bends.append(pretty_midi.PitchBend(pitch=0, time=min(end, start + 0.10)))


def add_chord(ctx: RenderContext, inst_name: str, chord: str, bar: float, beat: float, dur_beats: float, vel: float, *, octave: int = 4, articulation: str = "pad", voicing: str = "open", humanize_ms: float = 0.0, gate: float | None = None) -> None:
    notes = chord_pitches(chord, octave=octave, voicing=voicing)
    for idx, p in enumerate(notes):
        add_note(ctx, inst_name, p, bar, beat, dur_beats, vel - idx * 2, articulation=articulation, humanize_ms=humanize_ms, gate=gate)


def add_drum(ctx: RenderContext, kit: str, drum_name: str, bar: float, beat: float, vel: float, *, dur_beats: float = 0.30, humanize_ms: float = 0.0) -> None:
    pitch = DRUMS[drum_name]
    add_note(ctx, kit, pitch, bar, beat, dur_beats, vel, articulation="normal", humanize_ms=humanize_ms, gate=1.0)


def chord_for_bar(section: dict[str, Any], local_bar: int) -> str:
    harmony = section.get("harmony") or ["C"]
    return harmony[local_bar % len(harmony)]


def root_for_chord(chord: str, octave: int = 2) -> int:
    root, _intervals, slash = chord_intervals(chord)
    bass = slash or root
    bass = re.match(r"^([A-G](?:#|b)?)", bass).group(1)  # type: ignore[union-attr]
    return note_to_midi(f"{bass}{octave}")


def transform_motif(notes: list[int], transform: str | dict[str, Any] | None, pivot: int | None = None) -> list[int]:
    out = list(notes)
    if not transform:
        return out
    if isinstance(transform, dict):
        kind = transform.get("kind")
    else:
        kind = transform
    if kind == "retrograde":
        out = list(reversed(out))
    elif kind == "invert":
        p = pivot if pivot is not None else out[0]
        out = [p - (n - p) for n in out]
    elif kind == "transpose":
        shift = int(transform.get("semitones", 0)) if isinstance(transform, dict) else 0
        out = [n + shift for n in out]
    elif kind == "up_octave":
        out = [n + 12 for n in out]
    elif kind == "down_octave":
        out = [n - 12 for n in out]
    return out


def motif_notes(ctx: RenderContext, motif_id: str, root: str | int | None = None, transform: Any = None, transpose: int = 0) -> tuple[list[int], list[float], list[float]]:
    motif = ctx.motifs[motif_id]
    if "notes" in motif:
        notes = [note_to_midi(n) if isinstance(n, str) else int(n) for n in motif["notes"]]
        if root is not None and isinstance(root, str):
            base = note_to_midi(root)
            motif_base = note_to_midi(motif.get("root", "C4"))
            notes = [base + (n - motif_base) for n in notes]
    else:
        base = note_to_midi(root if isinstance(root, str) else motif.get("root", "C4"))
        notes = [base + int(x) for x in motif.get("intervals", [0])]
    notes = [n + transpose for n in notes]
    if transform:
        if isinstance(transform, list):
            for tr in transform:
                notes = transform_motif(notes, tr)
        else:
            notes = transform_motif(notes, transform)
    rhythm = [float(x) for x in motif.get("rhythm", [1.0] * len(notes))]
    velocities = [float(x) for x in motif.get("velocities", [1.0] * len(notes))]
    return notes, rhythm, velocities


def apply_automation(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    for auto in layer.get("automation", []):
        inst_names = resolve_instruments(ctx, auto) if any(k in auto for k in ("instrument", "instruments", "group")) else resolve_instruments(ctx, layer)
        cc = auto.get("cc", "expression")
        cc_num = CC_NUMBERS.get(cc, int(cc) if isinstance(cc, int) or str(cc).isdigit() else 11)
        start_bar = section["start_bar"] + float(auto.get("start_bar", 0.0))
        dur_bars = float(auto.get("bars", section["bars"]))
        start_val = float(auto.get("from", 80))
        end_val = float(auto.get("to", 110))
        points = int(auto.get("points", 12))
        curve = auto.get("curve", "linear")
        for inst_name in inst_names:
            inst = ctx.instruments[inst_name]
            for i in range(points):
                a = i / max(1, points - 1)
                if curve == "smooth":
                    a2 = a * a * (3 - 2 * a)
                elif curve == "exp":
                    a2 = a * a
                else:
                    a2 = a
                val = round(start_val * (1 - a2) + end_val * a2)
                add_cc(inst, cc_num, val, ctx.bar_to_time(start_bar + dur_bars * a))


def render_layer_pad_chords(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    every = float(layer.get("every_bars", 1.0))
    dur = float(layer.get("duration_beats", ctx.beats_per_bar * every))
    octave = int(layer.get("octave", 4))
    velocity = float(layer.get("velocity", 60)) * float(section.get("intensity", 1.0))
    articulation = layer.get("articulation", "pad")
    voicing = layer.get("voicing", "open")
    for local in range(0, int(section["bars"]), max(1, int(every))):
        chord = chord_for_bar(section, local)
        for inst in insts:
            add_chord(ctx, inst, chord, section["start_bar"] + local, 0.0, dur, velocity, octave=octave, articulation=articulation, voicing=voicing, humanize_ms=float(layer.get("humanize_ms", 8.0)))


def render_layer_arpeggio(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    pattern = [int(x) for x in layer.get("pattern", [0, 2, 1, 2])]
    step = float(layer.get("step", 0.5))
    dur = float(layer.get("duration_beats", step))
    octave = int(layer.get("octave", 4))
    velocity = float(layer.get("velocity", 64))
    density = float(layer.get("density", section.get("density", 1.0)))
    articulation = layer.get("articulation", "staccato")
    inst_velocity_offsets = layer.get("instrument_velocity_offsets", {}) or {}
    inst_octave_offsets = layer.get("instrument_octave_offsets", {}) or {}
    for local in range(int(section["bars"])):
        if "every" in layer and local % int(layer["every"]) != int(layer.get("offset", 0)):
            continue
        tones = chord_pitches(chord_for_bar(section, local), octave=octave, voicing=layer.get("voicing", "closed"))
        count = int(ctx.beats_per_bar / step)
        for i in range(count):
            if ctx.rng.random() > density:
                continue
            base_pitch = tones[pattern[i % len(pattern)] % len(tones)]
            for inst in insts:
                p = base_pitch + 12 * int(inst_octave_offsets.get(inst, 0))
                v = velocity + float(inst_velocity_offsets.get(inst, 0.0))
                add_note(ctx, inst, p, section["start_bar"] + local, i * step, dur, v * float(section.get("intensity", 1.0)), articulation=articulation, humanize_ms=float(layer.get("humanize_ms", 4.0)))


def render_layer_ostinato(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    intervals = [int(x) for x in layer.get("intervals", [0, 7, 12, 7])]
    rhythm = [float(x) for x in layer.get("rhythm", [0.5] * len(intervals))]
    root_octave = int(layer.get("octave", 3))
    velocity = float(layer.get("velocity", 60))
    articulation = layer.get("articulation", "spiccato")
    bars = int(section["bars"])
    for local in range(bars):
        root = root_for_chord(chord_for_bar(section, local), root_octave)
        beat = 0.0
        idx = 0
        while beat < ctx.beats_per_bar - 1e-6:
            dur = rhythm[idx % len(rhythm)]
            p = root + intervals[idx % len(intervals)]
            for inst in insts:
                add_note(ctx, inst, p, section["start_bar"] + local, beat, dur, velocity * float(section.get("intensity", 1.0)), articulation=articulation, humanize_ms=float(layer.get("humanize_ms", 4.0)))
            beat += dur
            idx += 1


def render_layer_bassline(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    inst = resolve_instruments(ctx, layer)[0]
    pattern = layer.get("pattern", [[0, 0.0, 0.75], [7, 1.5, 0.5], [12, 2.5, 0.5], [7, 3.25, 0.4]])
    octave = int(layer.get("octave", 2))
    velocity = float(layer.get("velocity", 74))
    articulation = layer.get("articulation", "marcato")
    for local in range(int(section["bars"])):
        root = root_for_chord(chord_for_bar(section, local), octave)
        for item in pattern:
            interval, beat, dur = int(item[0]), float(item[1]), float(item[2])
            add_note(ctx, inst, root + interval, section["start_bar"] + local, beat, dur, velocity * float(section.get("intensity", 1.0)), articulation=articulation, humanize_ms=float(layer.get("humanize_ms", 5.0)))


def render_layer_motif(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    roots = layer.get("roots") or [layer.get("root", None)]
    starts = layer.get("starts") or [[0, 0.0]]
    repeats = int(layer.get("repeats", 1))
    every_bars = float(layer.get("every_bars", 2.0))
    velocity = float(layer.get("velocity", 78))
    articulation = layer.get("articulation", "normal")
    transpose = int(layer.get("transpose", 0))
    transform = layer.get("transform")
    inst_velocity_offsets = layer.get("instrument_velocity_offsets", {}) or {}
    inst_octave_offsets = layer.get("instrument_octave_offsets", {}) or {}
    inst_pitch_scoop = layer.get("instrument_pitch_scoop_cents", {}) or {}
    note_velocity_pattern = layer.get("note_velocity_pattern", None)
    for rep in range(repeats):
        root = roots[rep % len(roots)]
        for start in starts:
            local_bar, start_beat = float(start[0]) + rep * every_bars, float(start[1])
            if local_bar >= section["bars"]:
                continue
            notes, rhythm, velocities = motif_notes(ctx, layer["motif"], root=root, transform=transform, transpose=transpose)
            beat = start_beat
            for i, p0 in enumerate(notes):
                dur = rhythm[i % len(rhythm)] * float(layer.get("rhythm_scale", 1.0))
                vel_scale = velocities[i % len(velocities)]
                if note_velocity_pattern:
                    vel_scale *= float(note_velocity_pattern[i % len(note_velocity_pattern)])
                for j, inst in enumerate(insts):
                    p = p0 + 12 * int(inst_octave_offsets.get(inst, 0))
                    v = velocity + float(inst_velocity_offsets.get(inst, -8 * j))
                    scoop = float(inst_pitch_scoop.get(inst, layer.get("pitch_scoop_cents", 0.0)))
                    add_note(ctx, inst, p, section["start_bar"] + local_bar, beat, dur, v * vel_scale * float(section.get("intensity", 1.0)), articulation=articulation, humanize_ms=float(layer.get("humanize_ms", 6.0)), pitch_scoop_cents=scoop)
                beat += dur


def render_layer_chord_hits(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    hits = layer.get("hits", [[0, 0.0], [4, 0.0], [8, 0.0], [12, 0.0]])
    velocity = float(layer.get("velocity", 90))
    octave = int(layer.get("octave", 3))
    for local, beat in hits:
        if float(local) >= section["bars"]:
            continue
        chord = chord_for_bar(section, int(local))
        for inst in insts:
            add_chord(ctx, inst, chord, section["start_bar"] + float(local), float(beat), float(layer.get("duration_beats", 0.75)), velocity * float(section.get("intensity", 1.0)), octave=octave, articulation=layer.get("articulation", "marcato"), voicing=layer.get("voicing", "closed"), humanize_ms=float(layer.get("humanize_ms", 6.0)))


def render_layer_drums(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    kit = resolve_instruments(ctx, layer)[0]
    events = layer.get("events", [])
    if not events:
        return
    for local in range(int(section["bars"])):
        for ev in events:
            if "bars" in ev and local not in set(int(b) for b in ev["bars"]):
                continue
            if "every" in ev and local % int(ev["every"]) != int(ev.get("offset", 0)):
                continue
            beats = ev.get("beats", [ev.get("beat", 0.0)])
            for beat in beats:
                if ctx.rng.random() > float(ev.get("probability", 1.0)):
                    continue
                add_drum(ctx, kit, ev["drum"], section["start_bar"] + local, float(beat), float(ev.get("velocity", layer.get("velocity", 70))) * float(section.get("intensity", 1.0)), dur_beats=float(ev.get("duration_beats", 0.1)), humanize_ms=float(ev.get("humanize_ms", layer.get("humanize_ms", 2.0))))


def render_layer_texture(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    scale = [int(x) for x in layer.get("scale", [0, 2, 3, 5, 7, 10, 12])]
    root = note_to_midi(layer.get("root", "D5"))
    count_per_bar = float(layer.get("events_per_bar", 1.0))
    velocity = float(layer.get("velocity", 38))
    for local in range(int(section["bars"])):
        count = int(math.floor(count_per_bar)) + (1 if ctx.rng.random() < count_per_bar % 1 else 0)
        for _ in range(count):
            beat = float(ctx.rng.uniform(0.0, ctx.beats_per_bar))
            p = root + int(ctx.rng.choice(scale)) + 12 * int(ctx.rng.integers(-1, 2))
            inst = str(ctx.rng.choice(insts))
            add_note(ctx, inst, p, section["start_bar"] + local, beat, float(layer.get("duration_beats", 0.25)), velocity * float(section.get("intensity", 1.0)), articulation=layer.get("articulation", "bell"), humanize_ms=float(layer.get("humanize_ms", 2.0)))


def render_layer_pedal(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    pitch = layer.get("note")
    if pitch is None:
        pitch = root_for_chord(chord_for_bar(section, 0), int(layer.get("octave", 2)))
    velocity = float(layer.get("velocity", 45)) * float(section.get("intensity", 1.0))
    for inst in insts:
        add_note(ctx, inst, pitch, section["start_bar"], 0.0, section["bars"] * ctx.beats_per_bar, velocity, articulation=layer.get("articulation", "pad"), humanize_ms=float(layer.get("humanize_ms", 8.0)))



def render_layer_root_hits(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    insts = resolve_instruments(ctx, layer)
    hits = layer.get("hits", [[0, 0.0, 0, 0.75]])
    velocity = float(layer.get("velocity", 76))
    octave = int(layer.get("octave", 2))
    articulation = layer.get("articulation", "marcato")
    for item in hits:
        local = float(item[0]); beat = float(item[1]); interval = int(item[2]) if len(item) > 2 else 0; dur = float(item[3]) if len(item) > 3 else float(layer.get("duration_beats", 0.75))
        if local >= section["bars"]:
            continue
        root = root_for_chord(chord_for_bar(section, int(local)), octave)
        for inst in insts:
            add_note(ctx, inst, root + interval, section["start_bar"] + local, beat, dur, velocity * float(section.get("intensity", 1.0)), articulation=articulation, humanize_ms=float(layer.get("humanize_ms", 2.0)))


def render_layer_automation(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    # Note-free layer used to express section-wide CC ramps in YAML.
    apply_automation(ctx, section, layer)

def render_layer(ctx: RenderContext, section: dict[str, Any], layer: dict[str, Any]) -> None:
    kind = layer["kind"]
    if kind == "pad_chords":
        render_layer_pad_chords(ctx, section, layer)
    elif kind == "arpeggio":
        render_layer_arpeggio(ctx, section, layer)
    elif kind == "ostinato":
        render_layer_ostinato(ctx, section, layer)
    elif kind == "bassline":
        render_layer_bassline(ctx, section, layer)
    elif kind == "motif":
        render_layer_motif(ctx, section, layer)
    elif kind == "chord_hits":
        render_layer_chord_hits(ctx, section, layer)
    elif kind == "drums":
        render_layer_drums(ctx, section, layer)
    elif kind == "texture":
        render_layer_texture(ctx, section, layer)
    elif kind == "pedal":
        render_layer_pedal(ctx, section, layer)
    elif kind == "root_hits":
        render_layer_root_hits(ctx, section, layer)
    elif kind == "automation":
        render_layer_automation(ctx, section, layer)
        return
    else:
        raise KeyError(f"unknown layer kind {kind!r}")
    apply_automation(ctx, section, layer)


def merged_layers(spec: dict[str, Any], section: dict[str, Any]) -> list[dict[str, Any]]:
    templates = spec.get("layer_templates", {})
    out: list[dict[str, Any]] = []
    for item in section.get("layers", []):
        if isinstance(item, str):
            layer = copy.deepcopy(templates[item])
        elif "template" in item:
            layer = copy.deepcopy(templates[item["template"]])
            layer.update({k: v for k, v in item.items() if k != "template"})
        else:
            layer = copy.deepcopy(item)
        out.append(layer)
    return out


def build_score(spec: dict[str, Any]) -> tuple[pretty_midi.PrettyMIDI, dict[str, str], list[dict[str, Any]]]:
    bpm = float(spec.get("tempo", {}).get("bpm", spec.get("bpm", 120)))
    beats_per_bar = float(spec.get("meter", {}).get("beats_per_bar", 4))
    pm = pretty_midi.PrettyMIDI(initial_tempo=bpm)
    ctx = RenderContext(
        spec=spec,
        sample_rate=int(spec.get("render", {}).get("sample_rate", 48000)),
        bpm=bpm,
        beats_per_bar=beats_per_bar,
        rng=np.random.default_rng(int(spec.get("seed", 1))),
        pm=pm,
        instruments={},
        groups={},
        section_starts={},
        motifs={m["id"]: m for m in spec.get("motifs", [])},
    )
    for inst_spec in spec.get("instruments", []):
        add_instrument(ctx, inst_spec)
    starts = section_starts(spec["sections"])
    ctx.section_starts = starts
    section_meta = section_metadata_from_spec(spec)
    for section in spec["sections"]:
        for layer in merged_layers(spec, section):
            render_layer(ctx, section, layer)
    return pm, ctx.groups, section_meta


def spec_hash(spec_path: Path, soundfont_path: str, backend: str) -> str:
    payload = {
        "renderer_version": RENDERER_VERSION,
        "spec_text": spec_path.read_text(encoding="utf8"),
        "soundfont": str(soundfont_path),
        "backend": backend,
    }
    return hashlib.sha256(json.dumps(payload, sort_keys=True).encode("utf8")).hexdigest()[:16]


def _coerce_stereo(audio: np.ndarray) -> np.ndarray:
    audio = np.asarray(audio, dtype=np.float32)
    if audio.ndim == 1:
        audio = np.column_stack([audio, audio])
    if audio.shape[1] > 2:
        audio = audio[:, :2]
    return audio.astype(np.float32, copy=False)


def render_pretty_midi(pm: pretty_midi.PrettyMIDI, soundfont: str, sample_rate: int) -> np.ndarray:
    audio = pm.fluidsynth(fs=sample_rate, sf2_path=soundfont)
    return _coerce_stereo(audio)


def _pan_stereo(mono: np.ndarray, pan: float) -> np.ndarray:
    pan = float(clamp(pan, -1.0, 1.0))
    left = math.sqrt((1.0 - pan) / 2.0)
    right = math.sqrt((1.0 + pan) / 2.0)
    return np.column_stack([mono * left, mono * right]).astype(np.float32)


def _adsr_curve(n: int, sr: int, attack: float, decay: float, sustain: float, release: float) -> np.ndarray:
    a = max(1, int(attack * sr)); d = max(1, int(decay * sr)); r = max(1, int(release * sr))
    s = max(0, n - a - d - r)
    env = np.concatenate([
        np.linspace(0.0, 1.0, a, endpoint=False),
        np.linspace(1.0, sustain, d, endpoint=False),
        np.full(s, sustain, dtype=np.float32),
        np.linspace(sustain, 0.0, r, endpoint=True),
    ]).astype(np.float32)
    if len(env) < n:
        env = np.pad(env, (0, n - len(env)))
    return env[:n]


def _saw(phase: np.ndarray) -> np.ndarray:
    return (2.0 * (phase % 1.0) - 1.0).astype(np.float32)


def _tri(phase: np.ndarray) -> np.ndarray:
    return (4.0 * np.abs((phase % 1.0) - 0.5) - 1.0).astype(np.float32)


def _pulse(phase: np.ndarray, duty: float = 0.5) -> np.ndarray:
    return np.where((phase % 1.0) < duty, 1.0, -1.0).astype(np.float32)


def _lowpass_mono(signal_in: np.ndarray, amount: float) -> np.ndarray:
    # One-pole lowpass: y[n] = y[n-1] + amount * (x[n] - y[n-1]).
    # Implemented with scipy.signal.lfilter because this runs for every rendered
    # note and Python loops make long pad-heavy scores unacceptably slow.
    if len(signal_in) == 0:
        return signal_in
    amount = float(clamp(amount, 1e-5, 1.0))
    return signal.lfilter([amount], [1.0, -(1.0 - amount)], signal_in).astype(np.float32)


def _declick(sig: np.ndarray, sr: int, attack: float = 0.006, release: float = 0.018) -> np.ndarray:
    """Apply a tiny edge fade to synthetic notes/drums.

    The fast renderer is additive and section/stem based.  Hard synthetic
    starts/stops that are barely audible in isolation can become obvious when
    multiple stems line up.  This helper keeps the renderer deterministic while
    avoiding those edge discontinuities.
    """
    n = len(sig)
    if n == 0:
        return sig.astype(np.float32, copy=False)
    out = sig.astype(np.float32, copy=True)
    a = min(n, max(1, int(attack * sr)))
    r = min(n, max(1, int(release * sr)))
    if a > 1:
        out[:a] *= np.linspace(0.0, 1.0, a, endpoint=True, dtype=np.float32)
    if r > 1:
        out[-r:] *= np.linspace(1.0, 0.0, r, endpoint=True, dtype=np.float32)
    return out.astype(np.float32, copy=False)



def _instrument_family(inst: pretty_midi.Instrument) -> str:
    """Classify instruments for the fast renderer.

    Family names returned here must match the branches in `_synth_note_fast`.
    GM program is preferred over name; name fallback covers exotic / synthesised
    cues that don't carry a meaningful program.
    """
    if inst.is_drum:
        return "drum"

    family = _program_family(int(getattr(inst, "program", 0)))
    if family is not None:
        return family

    name = (inst.name or "").lower()
    if "harp" in name and "harpsichord" not in name:
        return "harp"
    if "timpani" in name:
        return "timpani"
    if any(k in name for k in ("marimba", "mallet", "xylo", "vibe", "glock", "celesta", "bell")):
        return "mallet"
    if any(k in name for k in ("violin", "viola", "celli", "cello", "cell", "contrabass", "string")):
        return "string"
    if any(k in name for k in ("trumpet", "trombone", "tuba", "brass")) or ("horn" in name and "english" not in name):
        return "brass"
    if any(k in name for k in ("flute", "oboe", "clarinet", "bassoon", "piccolo", "recorder", "english_horn", "english horn", "wind")):
        return "wind"
    if any(k in name for k in ("choir", "voice")):
        return "choir"
    if "pad" in name:
        return "pad"
    if any(k in name for k in ("piano", "keys")):
        return "piano"
    return "generic"

def _synth_note_fast(frequency: float, duration: float, velocity: int, family: str, sr: int, rng: np.random.Generator) -> np.ndarray:
    """Built-in fallback instrument model.

    Clean demo-stem revision: reduce buzzy partials and hard synthetic edges.
    This is still not a replacement for a real sample library, but it avoids
    the string/wind fizz and constructive-interference clicks that were showing
    up when several OGG stems were layered in-game.
    """
    n = max(1, int(duration * sr))
    t = np.arange(n, dtype=np.float32) / sr
    vel = (velocity / 127.0) ** 1.22
    drift = 1.0 + float(rng.normal(0.0, 0.00035))
    f = frequency * drift
    phase = f * t
    # Voices use bandlimited additive synthesis (explicit harmonic series)
    # so we get rich, alias-free timbres without needing oversampling.
    # Harmonics above Nyquist/2 are dropped to keep the sound clean for
    # very high notes.
    nyquist = sr * 0.5
    twopi_f_t = 2 * np.pi * f * t

    def _harm_stack(weights: list[float]) -> np.ndarray:
        """Sum sin(2π * n * f * t) * weight for n=1..len(weights), skipping
        any harmonic that would alias above ~85% of Nyquist."""
        out = np.zeros(n, dtype=np.float32)
        cap = nyquist * 0.85
        for n_idx, w in enumerate(weights, start=1):
            if w == 0.0:
                continue
            if f * n_idx >= cap:
                break
            out += w * np.sin(twopi_f_t * n_idx).astype(np.float32)
        return out

    def _harm_saw(amp: float, n_max: int = 32, exponent: float = 1.0) -> np.ndarray:
        """Bandlimited sawtooth-flavored stack: amplitude = amp / n^exponent,
        truncated at Nyquist*0.85 so we never alias.

        exponent=1 is a true sawtooth (1/n falloff, broad-spectrum, buzzy).
        exponent>1 attenuates upper harmonics for warmer timbres.
        """
        out = np.zeros(n, dtype=np.float32)
        cap = nyquist * 0.85
        for n_idx in range(1, n_max + 1):
            if f * n_idx >= cap:
                break
            out += (amp / (n_idx ** exponent)) * np.sin(twopi_f_t * n_idx).astype(np.float32)
        return out

    if family == "string":
        # Warm bowed strings: bandlimited near-sawtooth body for natural
        # harmonic richness, mild waveshape for intermod, and a bow-noise
        # band in the bridge-resonance region so the voice carries presence
        # energy. Noise weight is conservative — multiple string stems sum
        # in the master preview and the noise floor stacks otherwise.
        raw = _harm_saw(0.45, n_max=28, exponent=1.05)
        body = np.tanh(raw * 1.50).astype(np.float32)
        bow = rng.normal(0.0, 0.40, n).astype(np.float32)
        bow_band = bow - _lowpass_mono(bow, 0.05)
        bow_band = _lowpass_mono(bow_band, 0.50)
        sig = _lowpass_mono(body, 0.70) + bow_band * 0.16
        env = _adsr_curve(n, sr, 0.085, 0.16, 0.66, 0.34)
    elif family == "brass":
        # Brass: bandlimited buzzy stack + drive + light lip-buzz noise.
        raw = _harm_saw(0.45, n_max=24, exponent=0.95)
        body = np.tanh(raw * 1.55).astype(np.float32)
        buzz = rng.normal(0.0, 0.30, n).astype(np.float32)
        buzz_band = buzz - _lowpass_mono(buzz, 0.03)
        buzz_band = _lowpass_mono(buzz_band, 0.45)
        sig = _lowpass_mono(body, 0.70) + buzz_band * 0.10
        env = _adsr_curve(n, sr, 0.045, 0.10, 0.72, 0.22)
    elif family == "wind":
        # Woodwinds: harmonic stack with steeper falloff than brass, light
        # breath layer for presence.
        raw = _harm_saw(0.55, n_max=22, exponent=1.20)
        body = np.tanh(raw * 1.30).astype(np.float32)
        breath = rng.normal(0.0, 0.36, n).astype(np.float32)
        breath_band = breath - _lowpass_mono(breath, 0.04)
        breath_band = _lowpass_mono(breath_band, 0.60)
        sig = _lowpass_mono(body, 0.70) + breath_band * 0.12
        env = _adsr_curve(n, sr, 0.060, 0.070, 0.78, 0.24)
    elif family == "pad":
        # Pad: detuned dual-osc with low harmonic emphasis. Stays warm.
        raw = (
            0.42 * np.sin(2 * np.pi * f * 0.997 * t)
            + 0.40 * np.sin(2 * np.pi * f * 1.003 * t)
            + 0.18 * np.sin(twopi_f_t * 2.0)
            + 0.08 * np.sin(twopi_f_t * 3.0)
            + 0.03 * np.sin(twopi_f_t * 4.0)
        )
        sig = _lowpass_mono(raw, 0.22)
        env = _adsr_curve(n, sr, 0.30, 0.35, 0.68, 0.90)
    elif family == "choir":
        # Choir/voice: vibrato fundamental + formant-shaped harmonics.
        vib = 0.0028 * np.sin(2 * np.pi * 5.2 * t)
        raw = (
            0.45 * np.sin(2 * np.pi * f * (1.0 + vib) * t)
            + 0.28 * np.sin(twopi_f_t * 2.0)
            + 0.20 * np.sin(twopi_f_t * 3.0)
            + 0.10 * np.sin(twopi_f_t * 4.0)
            + 0.04 * np.sin(twopi_f_t * 5.0)
        )
        sig = _lowpass_mono(raw, 0.28)
        env = _adsr_curve(n, sr, 0.20, 0.30, 0.78, 0.55)
    elif family == "mallet":
        raw = _harm_stack([1.00, 0.30, 0.14, 0.06, 0.03])
        sig = _lowpass_mono(raw, 0.55)
        env = np.exp(-t / max(0.18, duration * 0.50)).astype(np.float32)
        ramp = np.linspace(0.0, 1.0, min(n, max(8, int(0.014 * sr))), endpoint=True, dtype=np.float32)
        env[:len(ramp)] *= ramp
    elif family == "harp":
        raw = _harm_stack([0.70, 0.30, 0.16, 0.08, 0.04, 0.02])
        sig = _lowpass_mono(raw, 0.55)
        decay_tau = max(0.40, duration * 0.85)
        env = np.exp(-t / decay_tau).astype(np.float32)
        ramp = np.linspace(0.0, 1.0, min(n, max(6, int(0.005 * sr))), endpoint=True, dtype=np.float32)
        env[:len(ramp)] *= ramp
    elif family == "timpani":
        body_freq = max(40.0, frequency * 0.5)
        sweep_t = np.exp(-t / 0.045)
        f_sweep = body_freq + (frequency - body_freq) * sweep_t
        phase_int = 2 * np.pi * np.cumsum(f_sweep) / sr
        raw = 0.80 * np.sin(phase_int) + 0.18 * np.sin(2 * np.pi * frequency * 1.5 * t) + 0.08 * np.sin(2 * np.pi * frequency * 2.0 * t)
        rumble = rng.normal(0.0, 0.05, n).astype(np.float32) * np.exp(-t / 0.060)
        sig = _lowpass_mono(raw + rumble, 0.20)
        env = np.exp(-t / max(0.55, duration * 0.85)).astype(np.float32)
        ramp = np.linspace(0.0, 1.0, min(n, max(6, int(0.004 * sr))), endpoint=True, dtype=np.float32)
        env[:len(ramp)] *= ramp
    elif family == "piano":
        raw = _harm_stack([0.62, 0.28, 0.16, 0.10, 0.06, 0.03])
        sig = _lowpass_mono(raw, 0.40)
        env = np.exp(-t / max(0.34, duration * 0.70)).astype(np.float32)
        ramp = np.linspace(0.0, 1.0, min(n, max(8, int(0.010 * sr))), endpoint=True, dtype=np.float32)
        env[:len(ramp)] *= ramp
    elif family == "bass":
        raw = _harm_stack([0.65, 0.32, 0.18, 0.08, 0.04])
        sig = _lowpass_mono(raw, 0.28)
        env = _adsr_curve(n, sr, 0.018, 0.08, 0.72, 0.18)
    elif family == "lead":
        raw = 0.50 * np.sin(twopi_f_t) + 0.26 * _tri(phase) + 0.10 * _pulse(phase, 0.45) + 0.10 * np.sin(twopi_f_t * 2.0)
        sig = np.tanh(raw * 0.88).astype(np.float32)
        sig = _lowpass_mono(sig, 0.40)
        env = _adsr_curve(n, sr, 0.018, 0.06, 0.60, 0.16)
    else:
        raw = _harm_stack([0.70, 0.22, 0.10, 0.04])
        sig = _lowpass_mono(raw, 0.40)
        env = _adsr_curve(n, sr, 0.024, 0.06, 0.68, 0.18)
    return _declick(sig * env * vel, sr, 0.004, 0.012).astype(np.float32)

def _synth_drum_fast(pitch: int, duration: float, velocity: int, sr: int, rng: np.random.Generator) -> np.ndarray:
    n = max(1, int(duration * sr))
    t = np.arange(n, dtype=np.float32) / sr
    vel = (velocity / 127.0) ** 1.18
    noise = rng.normal(0, 1, n).astype(np.float32)
    if pitch in {35, 36}:
        f0, f1 = 74.0, 40.0
        sweep = f0 * ((f1 / f0) ** (t / max(duration, 1e-4)))
        phase = 2 * np.pi * np.cumsum(sweep) / sr
        sig = np.sin(phase).astype(np.float32) * np.exp(-t / 0.18)
        sig += 0.025 * noise * np.exp(-t / 0.018)
        sig = _lowpass_mono(sig, 0.060)
        sig = _declick(sig, sr, 0.010, 0.025)
    elif pitch in {38, 40, 37}:
        tone = np.sin(2 * np.pi * 160 * t).astype(np.float32) * np.exp(-t / 0.075)
        body = _lowpass_mono(noise, 0.060) * np.exp(-t / 0.060) * 0.22
        sig = tone * 0.48 + body
        sig = _declick(sig, sr, 0.008, 0.025)
    elif pitch in {41, 43, 45, 48, 47}:
        base = {41: 82, 43: 98, 45: 118, 48: 148, 47: 132}.get(pitch, 112)
        sig = np.sin(2 * np.pi * base * t).astype(np.float32) * np.exp(-t / 0.16)
        sig += _lowpass_mono(noise, 0.045) * np.exp(-t / 0.050) * 0.055
        sig = _declick(sig, sr, 0.008, 0.025)
    elif pitch in {42, 44, 46, 51, 49, 55, 52, 80, 81}:
        hp = noise - _lowpass_mono(noise, 0.035)
        hp = _lowpass_mono(hp, 0.090)
        sig = hp * np.exp(-t / (0.032 if pitch in {42, 44} else 0.18)) * 0.42
        sig = _declick(sig, sr, 0.006, 0.030)
    else:
        sig = _lowpass_mono(noise, 0.080) * np.exp(-t / 0.09) * 0.40
        sig = _declick(sig, sr, 0.006, 0.020)
    return (sig * vel).astype(np.float32)


def midi_content_seed(pm: pretty_midi.PrettyMIDI) -> int:
    """Stable pseudo-random seed derived from score content."""
    h = hashlib.sha256()
    for inst in pm.instruments:
        h.update(str(inst.program).encode())
        h.update(str(inst.is_drum).encode())
        h.update((inst.name or "").encode())
        for note in inst.notes[:2048]:
            h.update(f"{note.pitch}:{note.start:.4f}:{note.end:.4f}:{note.velocity}".encode())
    return int.from_bytes(h.digest()[:8], "big") & 0xFFFFFFFF

def _cc_track(inst: pretty_midi.Instrument, number: int) -> tuple[np.ndarray, np.ndarray]:
    """Return sorted (times, values) arrays for one CC number on `inst`."""
    events = [(c.time, c.value) for c in inst.control_changes if c.number == number]
    if not events:
        return np.empty(0, dtype=np.float64), np.empty(0, dtype=np.float32)
    events.sort(key=lambda tv: tv[0])
    times = np.fromiter((float(t) for t, _ in events), dtype=np.float64, count=len(events))
    values = np.fromiter((float(v) for _, v in events), dtype=np.float32, count=len(events))
    return times, values


def _cc_value(times: np.ndarray, values: np.ndarray, t: float, default: float) -> float:
    """Latest CC value at-or-before time `t`, with stairstep semantics."""
    if times.size == 0:
        return float(default)
    idx = int(np.searchsorted(times, t + 1e-6, side="right")) - 1
    if idx < 0:
        return float(default)
    return float(values[idx])


def render_fast(pm: pretty_midi.PrettyMIDI, sample_rate: int, *, minimum_duration: float | None = None) -> np.ndarray:
    end_time = pm.get_end_time()
    if minimum_duration is not None:
        end_time = max(end_time, minimum_duration)
    total_samples = int(math.ceil((end_time + 0.75) * sample_rate))
    mix = np.zeros((total_samples, 2), dtype=np.float32)
    rng = np.random.default_rng(midi_content_seed(pm))
    for inst in pm.instruments:
        family = _instrument_family(inst)
        # MIDI CC envelopes are stairstep. Sampling at note attack lets the
        # YAML expression / volume / pan ramps actually shape the rendered
        # audio instead of being silently dropped.
        vol_t, vol_v = _cc_track(inst, 7)
        pan_t, pan_v = _cc_track(inst, 10)
        expr_t, expr_v = _cc_track(inst, 11)
        for note in inst.notes:
            start = max(0, int(note.start * sample_rate))
            dur = max(0.025, note.end - note.start)
            vol_cc = _cc_value(vol_t, vol_v, note.start, 100.0)
            expr_cc = _cc_value(expr_t, expr_v, note.start, 100.0)
            pan_cc = _cc_value(pan_t, pan_v, note.start, 64.0)
            vol = (vol_cc / 127.0) * (expr_cc / 127.0)
            pan = (pan_cc - 64.0) / 63.0
            if inst.is_drum:
                mono = _synth_drum_fast(note.pitch, dur, note.velocity, sample_rate, rng)
            else:
                mono = _synth_note_fast(pretty_midi.note_number_to_hz(note.pitch), dur, note.velocity, family, sample_rate, rng)
            n = min(len(mono), total_samples - start)
            if n <= 0:
                continue
            mix[start:start + n] += _pan_stereo(mono[:n] * vol, pan)
    # Leave authored/stem relative loudness alone. Only protect the fast
    # renderer from obvious clipping; normalization-up happens later only if
    # the YAML master postprocess asks for it.
    peak = float(np.max(np.abs(mix)))
    if peak > 0.92:
        mix *= 0.92 / peak
    return mix.astype(np.float32, copy=False)


def render_with_fluidsynth_cli(midi_path: Path, soundfont: str, sample_rate: int, dry_wav_path: Path) -> np.ndarray:
    cmd = ["fluidsynth", "-ni", "-r", str(sample_rate), "-F", str(dry_wav_path), soundfont, str(midi_path)]
    subprocess.run(cmd, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    audio, sr = sf.read(dry_wav_path, dtype="float32", always_2d=True)
    if sr != sample_rate:
        audio = signal.resample_poly(audio, sample_rate, sr, axis=0).astype(np.float32)
    return _coerce_stereo(audio)


def render_synth_audio(pm: pretty_midi.PrettyMIDI, backend: str, soundfont: str, sample_rate: int, midi_path: Path, dry_wav_path: Path, minimum_duration: float) -> np.ndarray:
    if backend == "fast":
        return render_fast(pm, sample_rate, minimum_duration=minimum_duration)
    if backend == "fluidsynth-cli":
        if not soundfont:
            raise FileNotFoundError("fluidsynth-cli backend requires --soundfont or installed default SoundFont")
        if not shutil.which("fluidsynth"):
            raise FileNotFoundError("fluidsynth binary not found")
        return render_with_fluidsynth_cli(midi_path, soundfont, sample_rate, dry_wav_path)
    if backend == "pretty-midi":
        if not soundfont:
            raise FileNotFoundError("pretty-midi backend requires --soundfont or installed default SoundFont")
        return render_pretty_midi(pm, soundfont, sample_rate)
    if backend == "auto":
        if soundfont and shutil.which("fluidsynth"):
            try:
                return render_with_fluidsynth_cli(midi_path, soundfont, sample_rate, dry_wav_path)
            except Exception as ex:
                print(f"[WARN] fluidsynth-cli failed ({ex}); falling back to fast renderer")
        return render_fast(pm, sample_rate, minimum_duration=minimum_duration)
    raise ValueError(f"unknown backend {backend}")


def _one_pole_alpha(hz: float, sample_rate: int) -> float:
    hz = float(clamp(hz, 1.0, sample_rate * 0.49))
    return float(1.0 - math.exp(-2.0 * math.pi * hz / sample_rate))


def lowpass(audio: np.ndarray, sample_rate: int, hz: float = 12_000.0, order: int = 1) -> np.ndarray:
    if hz <= 0 or hz >= sample_rate * 0.49:
        return audio.astype(np.float32, copy=False)
    audio = _coerce_stereo(audio)
    alpha = _one_pole_alpha(hz, sample_rate)
    out = audio.astype(np.float32, copy=True)
    # Cascade cheap one-pole sections for steeper response when requested.
    for _ in range(max(1, int(order))):
        out[:, 0] = _lowpass_mono(out[:, 0], alpha)
        out[:, 1] = _lowpass_mono(out[:, 1], alpha)
    return out.astype(np.float32, copy=False)


def highpass(audio: np.ndarray, sample_rate: int, hz: float = 35.0) -> np.ndarray:
    if hz <= 0:
        return audio.astype(np.float32, copy=False)
    audio = _coerce_stereo(audio)
    return (audio - lowpass(audio, sample_rate, hz, order=1)).astype(np.float32)


def high_shelf(audio: np.ndarray, sample_rate: int, *, hz: float = 4_500.0, db: float = -2.0) -> np.ndarray:
    """Simple high-shelf using a high-passed side band."""
    if abs(db) < 1e-6:
        return audio.astype(np.float32, copy=False)
    hi = highpass(audio, sample_rate, hz)
    gain = 10 ** (db / 20.0)
    return (audio + hi * (gain - 1.0)).astype(np.float32)


def band_gain(audio: np.ndarray, sample_rate: int, *, low_hz: float, high_hz: float, db: float) -> np.ndarray:
    if abs(db) < 1e-6:
        return audio.astype(np.float32, copy=False)
    audio = _coerce_stereo(audio)
    low_hz = max(20.0, float(low_hz))
    high_hz = min(float(high_hz), sample_rate * 0.49)
    if high_hz <= low_hz:
        return audio.astype(np.float32, copy=False)
    band = lowpass(audio, sample_rate, high_hz, order=1) - lowpass(audio, sample_rate, low_hz, order=1)
    gain = 10 ** (db / 20.0)
    return (audio + band * (gain - 1.0)).astype(np.float32)



def simple_reverb(audio: np.ndarray, sr: int, wet: float = 0.08, decay: float = 0.9, damping_hz: float = 6500.0) -> np.ndarray:
    """Small deterministic reverb used by MusicIR post-processing.

    Uses a denser, lower-gain set of early-reflection taps so percussive
    transients don't read as slap-back echoes (the previous design's first
    tap was at +29 ms, gain ~0.97, which sounded like a duplicate hit).
    """
    wet = float(wet)
    decay = max(float(decay), 1e-3)
    if wet <= 0.0 or audio.size == 0:
        return audio.astype("float32", copy=False)

    y = np.asarray(audio, dtype="float32")
    if y.ndim == 1:
        y = y[:, None]
    acc = np.zeros_like(y)

    # Twelve taps spaced from ~7 ms to ~340 ms with smoothly falling gain.
    # The first tap is intentionally quiet so transients don't echo, and
    # the broader spread reads as diffuse ambience rather than discrete
    # delay lines.
    taps = (
        (0.007, 0.28), (0.013, 0.24), (0.019, 0.20), (0.029, 0.17),
        (0.041, 0.14), (0.057, 0.11), (0.079, 0.085), (0.103, 0.066),
        (0.137, 0.050), (0.181, 0.038), (0.241, 0.028), (0.331, 0.020),
    )

    for idx, (delay_seconds, base_gain) in enumerate(taps):
        delay = max(1, int(round(delay_seconds * sr)))
        if delay >= len(y):
            continue
        gain = base_gain * math.exp(-delay_seconds / decay)
        acc[delay:] += y[:-delay] * gain
        # Subtle stereo cross-feed for spread (kept very small so a panned
        # source doesn't bleed across channels in the reverb tail).
        if acc.shape[1] >= 2:
            acc[delay:, 1] += y[:-delay, 0] * gain * 0.06
            acc[delay:, 0] += y[:-delay, 1] * gain * 0.04

    if damping_hz and damping_hz > 0 and len(acc) > 16:
        cutoff = min(float(damping_hz), sr * 0.45)
        if cutoff > 20.0:
            b, a = signal.butter(1, cutoff / (sr * 0.5), btype="low")
            acc = signal.lfilter(b, a, acc, axis=0).astype("float32")

    out = y * (1.0 - wet) + acc * wet
    return out.astype("float32", copy=False)

def stereo_widen(audio: np.ndarray, amount: float = 0.12) -> np.ndarray:
    if amount <= 0:
        return audio.astype(np.float32, copy=False)
    mid = (audio[:, 0] + audio[:, 1]) * 0.5
    side = (audio[:, 0] - audio[:, 1]) * 0.5 * (1.0 + amount)
    return np.column_stack([mid + side, mid - side]).astype(np.float32)


def soft_limit(audio: np.ndarray, target_peak_db: float = -1.0, *, drive: float = 1.08, normalize: bool = True) -> np.ndarray:
    driven = np.tanh(audio * drive).astype(np.float32)
    peak = float(np.max(np.abs(driven)))
    target = 10 ** (target_peak_db / 20.0)
    if peak > 1e-8:
        # Master previews should normalize up to the target peak. Stems should
        # usually only be scaled down if too hot; otherwise quiet layers like
        # glimmer/mallets become unintentionally huge and shrill when mixed.
        if normalize or peak > target:
            driven *= target / peak
    return driven.astype(np.float32)


def post_process(audio: np.ndarray, sample_rate: int, settings: dict[str, Any]) -> np.ndarray:
    audio = _coerce_stereo(audio)
    if settings.get("gain_db", 0):
        audio = audio * (10 ** (float(settings["gain_db"]) / 20.0))
    if settings.get("highpass_hz", 0):
        audio = highpass(audio, sample_rate, float(settings["highpass_hz"]))
    # Tame very fast transients by blending toward a darker copy. This is most
    # useful for synthetic mallets, cymbals, and plucked/arpeggiated layers.
    tame = float(settings.get("transient_tame", 0.0))
    if tame > 0:
        dark = lowpass(audio, sample_rate, float(settings.get("transient_lowpass_hz", 6_500)))
        audio = (audio * (1.0 - tame) + dark * tame).astype(np.float32)
    if settings.get("presence_db", 0):
        audio = band_gain(audio, sample_rate, low_hz=float(settings.get("presence_low_hz", 2_000)), high_hz=float(settings.get("presence_high_hz", 4_500)), db=float(settings["presence_db"]))
    if settings.get("high_shelf_db", 0):
        audio = high_shelf(audio, sample_rate, hz=float(settings.get("high_shelf_hz", 4_500)), db=float(settings["high_shelf_db"]))
    if settings.get("lowpass_hz", 0):
        audio = lowpass(audio, sample_rate, float(settings["lowpass_hz"]))
    audio = simple_reverb(
        audio,
        sample_rate,
        wet=float(settings.get("reverb_wet", 0.18)),
        decay=float(settings.get("reverb_decay_seconds", 1.4)),
        damping_hz=float(settings.get("reverb_damping_hz", 6_000)),
    )
    # Apply one final brightness control after the room, because undamped
    # reverb can reintroduce fizz on synthetic sources.
    if settings.get("post_reverb_high_shelf_db", 0):
        audio = high_shelf(audio, sample_rate, hz=float(settings.get("post_reverb_high_shelf_hz", 5_000)), db=float(settings["post_reverb_high_shelf_db"]))
    audio = stereo_widen(audio, float(settings.get("stereo_width", 0.10)))
    return soft_limit(
        audio,
        float(settings.get("target_peak_db", -1.0)),
        drive=float(settings.get("limiter_drive", 1.08)),
        normalize=bool(settings.get("normalize", True)),
    )

def write_wav(path: Path, audio: np.ndarray, sample_rate: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(path, audio, sample_rate, subtype="PCM_16")


def encode_ogg(wav_path: Path, ogg_path: Path, quality: float = 5.0) -> None:
    ogg_path.parent.mkdir(parents=True, exist_ok=True)
    if not shutil.which("ffmpeg"):
        raise FileNotFoundError("ffmpeg is required to encode OGG Vorbis")
    cmd = ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error", "-i", str(wav_path), "-map_metadata", "-1", "-c:a", "libvorbis", "-q:a", str(quality), str(ogg_path)]
    subprocess.run(cmd, check=True)


def write_ogg_from_audio(audio: np.ndarray, sample_rate: int, ogg_path: Path, *, quality: float = 5.0, keep_wav: bool = False) -> Path:
    """Write OGG Vorbis, preferring ffmpeg pipe encoding for reliability/speed."""
    ogg_path.parent.mkdir(parents=True, exist_ok=True)
    pcm = np.nan_to_num(np.clip(_coerce_stereo(audio), -1.0, 1.0), nan=0.0, posinf=0.0, neginf=0.0).astype(np.float32, copy=False)
    if not shutil.which("ffmpeg"):
        # Fallback for minimal environments. Some libsndfile builds are slow on
        # many OGG writes, but this keeps the renderer usable if ffmpeg is absent.
        sf.write(ogg_path, pcm, sample_rate, format="OGG", subtype="VORBIS")
        if keep_wav:
            write_wav(ogg_path.with_suffix(".wav"), audio, sample_rate)
        return ogg_path
    cmd = [
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
        "-f", "f32le", "-ar", str(sample_rate), "-ac", "2", "-i", "pipe:0",
        "-map_metadata", "-1", "-c:a", "libvorbis", "-q:a", str(quality), str(ogg_path),
    ]
    proc = subprocess.run(cmd, input=pcm.tobytes(order="C"), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.decode("utf8", errors="replace"))
    if keep_wav:
        write_wav(ogg_path.with_suffix(".wav"), audio, sample_rate)
    return ogg_path

def copy_with_instruments(pm: pretty_midi.PrettyMIDI, instruments: list[pretty_midi.Instrument], bpm: float) -> pretty_midi.PrettyMIDI:
    new_pm = pretty_midi.PrettyMIDI(initial_tempo=bpm)
    new_pm.instruments = [copy.deepcopy(inst) for inst in instruments]
    return new_pm


def ensure_audio_length(audio: np.ndarray, target_samples: int) -> np.ndarray:
    if len(audio) < target_samples:
        audio = np.pad(audio, ((0, target_samples - len(audio)), (0, 0)))
    elif len(audio) > target_samples:
        audio = audio[:target_samples]
    return audio.astype(np.float32, copy=False)


def slice_audio(audio: np.ndarray, sample_rate: int, start_seconds: float, end_seconds: float) -> np.ndarray:
    a = max(0, int(round(start_seconds * sample_rate)))
    b = max(a, int(round(end_seconds * sample_rate)))
    return audio[a:b]


def section_metadata_from_spec(spec: dict[str, Any]) -> list[dict[str, Any]]:
    bpm = float(spec.get("tempo", {}).get("bpm", spec.get("bpm", 120)))
    beats_per_bar = float(spec.get("meter", {}).get("beats_per_bar", 4))
    seconds_per_beat = 60.0 / bpm
    cursor = 0
    out = []
    for section in spec["sections"]:
        bars = int(section["bars"])
        start_beat = cursor * beats_per_bar
        end_beat = (cursor + bars) * beats_per_bar
        out.append({
            "id": section["id"],
            "label": section.get("label", section["id"]),
            "kind": section.get("kind", "section"),
            "start_bar": cursor,
            "bars": bars,
            "start_beat": start_beat,
            "end_beat": end_beat,
            "start_seconds": start_beat * seconds_per_beat,
            "end_seconds": end_beat * seconds_per_beat,
            "duration_seconds": (end_beat - start_beat) * seconds_per_beat,
            "loopable": bool(section.get("loopable", False)),
            "valid_exit_local_bars": section.get("valid_exit_local_bars", []),
        })
        cursor += bars
    return out


def render_group_audio(pm: pretty_midi.PrettyMIDI, groups: dict[str, str], group: str, backend: str, soundfont: str, sample_rate: int, tempdir: Path, minimum_duration: float, bpm: float) -> np.ndarray:
    insts = [inst for inst in pm.instruments if groups.get(inst.name) == group]
    sub_pm = copy_with_instruments(pm, insts, bpm)
    midi_path = tempdir / f"group_{group}.mid"
    dry_wav = tempdir / f"group_{group}.dry.wav"
    # The built-in fast renderer consumes PrettyMIDI objects directly. Avoid
    # serializing stem MIDI unless an external backend actually needs it; this
    # keeps adaptive section x stem export snappy and avoids rare pretty_midi
    # writer stalls on sparse/empty instrument groups.
    if backend != "fast":
        sub_pm.write(str(midi_path))
    return render_synth_audio(sub_pm, backend, soundfont, sample_rate, midi_path, dry_wav, minimum_duration)


def build_manifest(spec: dict[str, Any], cue_hash: str, section_meta: list[dict[str, Any]], group_names: list[str], output_files: dict[str, Any], sample_rate: int) -> dict[str, Any]:
    bpm = float(spec.get("tempo", {}).get("bpm", spec.get("bpm", 120)))
    beats_per_bar = float(spec.get("meter", {}).get("beats_per_bar", 4))
    return {
        "schema": "ambition.adaptive_music_manifest.v2",
        "renderer_version": RENDERER_VERSION,
        "id": spec["id"],
        "title": spec.get("title", spec["id"]),
        "hash": cue_hash,
        "bpm": bpm,
        "beats_per_bar": beats_per_bar,
        "sample_rate": sample_rate,
        "stems": group_names,
        "sections": section_meta,
        "files": output_files,
        "playback": spec.get("playback", {}),
        "state_map": spec.get("state_map", {}),
        "notes": spec.get("notes", ""),
    }


def render_all(args: argparse.Namespace) -> dict[str, Any]:
    spec_path = Path(args.spec).resolve()
    spec = load_yaml(spec_path)
    render_cfg = spec.get("render", {})
    sample_rate = int(render_cfg.get("sample_rate", 48000))
    bpm = float(spec.get("tempo", {}).get("bpm", spec.get("bpm", 120)))
    beats_per_bar = float(spec.get("meter", {}).get("beats_per_bar", 4))
    output_root = Path(args.outdir).resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    soundfont = choose_soundfont(args.soundfont or render_cfg.get("soundfont"))
    backend = args.backend or render_cfg.get("backend", "auto")
    cue_hash = spec_hash(spec_path, soundfont, backend)
    quality = float(render_cfg.get("ogg_quality", 5.0))
    pm, groups, section_meta = build_score(spec)
    total_seconds = section_meta[-1]["end_seconds"] if section_meta else pm.get_end_time()
    target_samples = int(math.ceil(total_seconds * sample_rate))
    group_names = sorted(set(groups.values()))
    output_files: dict[str, Any] = {"preview": {}, "adaptive": {}}

    with tempfile.TemporaryDirectory() as d:
        tempdir = Path(d)
        # Render stems first, apply stem/bus tone controls without normalizing
        # them upward, write adaptive stem pieces, and sum the exact processed
        # stems to build the full preview. This guarantees that bus EQ and stem
        # gains affect both adaptive playback and the full soundtrack preview.
        full_stem_sum = np.zeros((target_samples, 2), dtype=np.float32)
        stem_base_settings = copy.deepcopy(spec.get("stem_postprocess", {}))
        group_post = spec.get("group_postprocess", {}) or {}
        for group in group_names:
            if getattr(args, "verbose", False): print(f"[render] stem {group}", flush=True)
            group_raw = render_group_audio(pm, groups, group, backend, soundfont, sample_rate, tempdir, total_seconds, bpm)
            group_raw = ensure_audio_length(group_raw, target_samples)
            group_settings = copy.deepcopy(stem_base_settings)
            group_settings.update(group_post.get(group, {}))
            # Stems should preserve authored relative gain. The default is no
            # upward normalization unless YAML explicitly asks for it.
            group_settings.setdefault("normalize", False)
            group_settings.setdefault("target_peak_db", -2.5)
            if getattr(args, "verbose", False): print(f"[post] stem {group} settings={group_settings}", flush=True)
            import time as _time
            _t0 = _time.time()
            group_audio = post_process(group_raw, sample_rate, group_settings)
            if getattr(args, "verbose", False): print(f"[post-done] stem {group} elapsed={_time.time() - _t0:.2f}s shape={group_audio.shape}", flush=True)
            _t0 = _time.time()
            full_stem_sum += ensure_audio_length(group_audio, target_samples)
            if getattr(args, "verbose", False): print(f"[sum-done] stem {group} elapsed={_time.time() - _t0:.2f}s", flush=True)
            for meta in section_meta:
                piece = slice_audio(group_audio, sample_rate, meta["start_seconds"], meta["end_seconds"])
                path = output_root / "adaptive" / meta["id"] / f"{spec['id']}_{cue_hash}.{meta['id']}.{group}.ogg"
                if getattr(args, "verbose", False): print(f"[write] stem {group} section {meta['id']}", flush=True)
                _t0 = _time.time()
                write_ogg_from_audio(piece, sample_rate, path, quality=quality, keep_wav=args.keep_wav)
                if getattr(args, "verbose", False): print(f"[write-done] stem {group} section {meta['id']} elapsed={_time.time() - _t0:.2f}s", flush=True)
                output_files["adaptive"].setdefault(meta["id"], {})[group] = str(path.relative_to(output_root))
            del group_raw, group_audio
            gc.collect()

        if getattr(args, "verbose", False): print("[post] master from processed stems", flush=True)
        full_audio = post_process(full_stem_sum, sample_rate, spec.get("postprocess", {}))
        preview_path = output_root / "preview" / f"{spec['id']}_{cue_hash}.full_soundtrack_preview.ogg"
        if getattr(args, "verbose", False): print("[write] preview", flush=True)
        write_ogg_from_audio(full_audio, sample_rate, preview_path, quality=quality, keep_wav=args.keep_wav)
        output_files["preview"]["full_soundtrack"] = str(preview_path.relative_to(output_root))

        # Full section renders are slices of the mastered stem sum.
        for meta in section_meta:
            section_dir = output_root / "adaptive" / meta["id"]
            section_dir.mkdir(parents=True, exist_ok=True)
            piece = slice_audio(full_audio, sample_rate, meta["start_seconds"], meta["end_seconds"])
            path = section_dir / f"{spec['id']}_{cue_hash}.{meta['id']}.full.ogg"
            if getattr(args, "verbose", False): print(f"[write] section full {meta['id']}", flush=True)
            write_ogg_from_audio(piece, sample_rate, path, quality=quality, keep_wav=args.keep_wav)
            output_files["adaptive"].setdefault(meta["id"], {})["full"] = str(path.relative_to(output_root))

        if args.keep_midi:
            midi_out = output_root / "debug" / f"{spec['id']}_{cue_hash}.mid"
            midi_out.parent.mkdir(parents=True, exist_ok=True)
            pm.write(str(midi_out))
            output_files["debug_midi"] = str(midi_out.relative_to(output_root))

    manifest = build_manifest(spec, cue_hash, section_meta, group_names, output_files, sample_rate)
    manifest_path = output_root / f"{spec['id']}_{cue_hash}.adaptive_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf8")
    return {"manifest": str(manifest_path), "preview": str(preview_path), "hash": cue_hash}

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render Ambition MusicIR YAML to adaptive OGG assets")
    parser.add_argument("spec", help="Path to .music.yaml source")
    parser.add_argument("--outdir", default="output", help="Output directory")
    parser.add_argument("--backend", choices=["auto", "fast", "fluidsynth-cli", "pretty-midi"], default=None)
    parser.add_argument("--soundfont", default=None)
    parser.add_argument("--keep-wav", action="store_true")
    parser.add_argument("--keep-midi", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(argv)
    result = render_all(args)
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.stdout.flush()
    sys.stderr.flush()
    os._exit(main())
