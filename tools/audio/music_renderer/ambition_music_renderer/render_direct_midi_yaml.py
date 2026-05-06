#!/usr/bin/env python3
"""Render a direct-MIDI YAML diagnostic asset.

This module exists to answer one narrow production question: can the newer
YAML-based pipeline reproduce the first goblin cue exactly enough when the
original MIDI event stream is expressed as data?  It intentionally stores note,
CC, and pitch-bend events in YAML, then uses the same SoundFont and legacy
post-processing style as the first renderer.

If this sounds like the original, remaining differences live in the higher-level
MusicIR translation / orchestration rules.  If this does not, the issue is in
rendering, SoundFont selection, or post-processing.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any

import numpy as np
import pretty_midi
import soundfile as sf
import yaml
from scipy import signal

RENDERER_VERSION = "ambition-direct-midi-yaml-renderer-v0.2.0"
DEFAULT_SOUNDFONTS = [
    "/usr/share/sounds/sf2/TimGM6mb.sf2",
    "/usr/share/sounds/sf2/default-GM.sf2",
    "/usr/share/sounds/sf3/default-GM.sf3",
]


def load_yaml(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf8") as f:
        return yaml.safe_load(f)


def choose_soundfont(path: str | None) -> str:
    if path:
        p = Path(path).expanduser()
        if not p.exists():
            raise FileNotFoundError(f"soundfont does not exist: {p}")
        return str(p)
    for candidate in DEFAULT_SOUNDFONTS:
        if Path(candidate).exists():
            return candidate
    raise FileNotFoundError("No default SoundFont found; install TimGM6mb.sf2 or pass --soundfont")


def build_pretty_midi(spec: dict[str, Any]) -> pretty_midi.PrettyMIDI:
    bpm = float(spec.get("tempo", {}).get("bpm", spec.get("bpm", 120.0)))
    pm = pretty_midi.PrettyMIDI(initial_tempo=bpm)
    for inst_spec in spec.get("instruments", []):
        inst = pretty_midi.Instrument(
            program=int(inst_spec.get("program", 0)),
            is_drum=bool(inst_spec.get("is_drum", False)),
            name=str(inst_spec.get("name", f"instrument_{len(pm.instruments)}")),
        )
        for start, end, pitch, velocity in inst_spec.get("notes", []):
            inst.notes.append(pretty_midi.Note(
                velocity=int(velocity),
                pitch=int(pitch),
                start=float(start),
                end=float(end),
            ))
        for time, number, value in inst_spec.get("control_changes", []):
            inst.control_changes.append(pretty_midi.ControlChange(
                number=int(number), value=int(value), time=float(time)
            ))
        for time, pitch in inst_spec.get("pitch_bends", []):
            inst.pitch_bends.append(pretty_midi.PitchBend(pitch=int(pitch), time=float(time)))
        pm.instruments.append(inst)
    return pm


def _coerce_stereo(audio: np.ndarray) -> np.ndarray:
    audio = np.asarray(audio, dtype=np.float32)
    if audio.ndim == 1:
        if len(audio) % 2 == 0:
            audio = audio.reshape(-1, 2)
        else:
            audio = np.column_stack([audio, audio])
    if audio.ndim == 2 and audio.shape[1] == 1:
        audio = np.column_stack([audio[:, 0], audio[:, 0]])
    if audio.ndim != 2 or audio.shape[1] != 2:
        raise ValueError(f"expected stereo audio, got shape={audio.shape}")
    return audio.astype(np.float32, copy=False)


def ensure_audio_length(audio: np.ndarray, samples: int) -> np.ndarray:
    """Pad or trim stereo float audio to an exact sample count."""
    audio = _coerce_stereo(audio)
    if len(audio) == samples:
        return audio
    if len(audio) > samples:
        return audio[:samples].astype(np.float32, copy=False)
    out = np.zeros((samples, 2), dtype=np.float32)
    out[: len(audio)] = audio
    return out


def db_to_amp(db: float) -> float:
    return float(10 ** (db / 20.0))


def smooth_envelope(values: np.ndarray, sample_rate: int, attack_ms: float, release_ms: float) -> np.ndarray:
    """One-pole attack/release smoothing for mix automation envelopes."""
    values = np.asarray(values, dtype=np.float32)
    out = np.zeros_like(values, dtype=np.float32)
    attack = math.exp(-1.0 / max(1.0, sample_rate * attack_ms / 1000.0))
    release = math.exp(-1.0 / max(1.0, sample_rate * release_ms / 1000.0))
    y = 0.0
    for i, x in enumerate(values):
        coeff = attack if x > y else release
        y = coeff * y + (1.0 - coeff) * float(x)
        out[i] = y
    return out


def midi_velocity_scale_for_db(db: float) -> float:
    """A conservative musical mapping from dB gain to MIDI velocity scale.

    Full audio gain is applied when rendering instruments separately.  This
    function is used only when a single full MIDI render is requested, where the
    renderer cannot rebalance instruments after synthesis.
    """
    return float(10 ** (db / 30.0))


def group_for_instrument(spec: dict[str, Any], inst_name: str) -> str | None:
    mix = spec.get("mix", {}) or {}
    groups = mix.get("instrument_groups", {}) or {}
    for group_name, members in groups.items():
        if inst_name in set(members or []):
            return str(group_name)
    # Heuristic fallback keeps old YAML useful if group metadata is omitted.
    lower = inst_name.lower()
    if any(k in lower for k in ("violin", "viola", "cell", "contrabass")):
        return "strings"
    if any(k in lower for k in ("horn", "trumpet", "trombone", "tuba", "brass")):
        return "brass"
    if any(k in lower for k in ("flute", "oboe", "clarinet", "bassoon", "english")):
        return "winds"
    if any(k in lower for k in ("marimba", "xylophone", "harp")):
        return "mallets"
    if any(k in lower for k in ("timpani", "drum")):
        return "percussion"
    if any(k in lower for k in ("choir", "pad")):
        return "choir_pad"
    return None


def instrument_gain_db(spec: dict[str, Any], inst_name: str) -> float:
    """Return authored mix gain for an instrument.

    The direct-MIDI diagnostic format stores the exact note stream, but it still
    needs a production mix layer.  This function applies broad group gains and
    exact instrument overrides from YAML without changing the composition.
    """
    mix = spec.get("mix", {}) or {}
    group = group_for_instrument(spec, inst_name)
    gain_db = float((mix.get("group_gains_db", {}) or {}).get(group, 0.0)) if group else 0.0
    gain_db += float((mix.get("instrument_gains_db", {}) or {}).get(inst_name, 0.0))
    return gain_db


def clone_pm_with_instrument_subset(pm: pretty_midi.PrettyMIDI, indexes: list[int]) -> pretty_midi.PrettyMIDI:
    sub = pretty_midi.PrettyMIDI(initial_tempo=pm.estimate_tempo() if pm.instruments else 120.0)
    for idx in indexes:
        src = pm.instruments[idx]
        inst = pretty_midi.Instrument(program=src.program, is_drum=src.is_drum, name=src.name)
        inst.notes.extend(pretty_midi.Note(n.velocity, n.pitch, n.start, n.end) for n in src.notes)
        inst.control_changes.extend(pretty_midi.ControlChange(cc.number, cc.value, cc.time) for cc in src.control_changes)
        inst.pitch_bends.extend(pretty_midi.PitchBend(pb.pitch, pb.time) for pb in src.pitch_bends)
        sub.instruments.append(inst)
    return sub


def apply_single_pass_midi_mix(pm: pretty_midi.PrettyMIDI, spec: dict[str, Any]) -> None:
    """Apply conservative MIDI-domain mix controls for single-pass rendering.

    This is less accurate than separate-instrument rendering because SoundFont
    timbre changes with velocity.  It is kept for fast diagnostics and for
    environments where repeated FluidSynth calls are too expensive.
    """
    for inst in pm.instruments:
        gain_db = instrument_gain_db(spec, inst.name)
        if not gain_db:
            continue
        scale = midi_velocity_scale_for_db(gain_db)
        for note in inst.notes:
            note.velocity = max(1, min(127, int(round(note.velocity * scale))))
        # If a part has expression/volume CCs, nudge them too, but less than
        # velocity.  This avoids crushing dynamics while still making solos sit
        # forward on SoundFonts that ignore velocity on sustained instruments.
        cc_scale = midi_velocity_scale_for_db(gain_db * 0.45)
        for cc in inst.control_changes:
            if cc.number in {7, 11}:
                cc.value = max(1, min(127, int(round(cc.value * cc_scale))))


def render_instruments_separately(
    pm: pretty_midi.PrettyMIDI,
    spec: dict[str, Any],
    soundfont: str,
    sample_rate: int,
    backend: str,
    tempdir: Path,
) -> np.ndarray:
    """Render each MIDI instrument independently, apply YAML mix gains, then sum.

    Compared with the v7 diagnostic renderer, this path keeps more headroom and
    can duck accompaniment under foreground solo groups.  That matters because
    lifting solo velocities alone often makes SoundFonts brighter without
    actually making the line more readable; lowering beds around solo activity is
    usually a cleaner game-mix move.
    """
    duration_hint = spec.get("duration_seconds") or spec.get("source_reference", {}).get("original_duration_seconds")
    if duration_hint:
        target_samples = int(np.ceil((float(duration_hint) + 0.02) * sample_rate))
    else:
        end_seconds = max((n.end for inst in pm.instruments for n in inst.notes), default=pm.get_end_time())
        target_samples = int(np.ceil((end_seconds + 0.25) * sample_rate))
    foreground = np.zeros((target_samples, 2), dtype=np.float32)
    background = np.zeros((target_samples, 2), dtype=np.float32)
    misc = np.zeros((target_samples, 2), dtype=np.float32)
    mix_cfg = spec.get("mix", {}) or {}
    foreground_groups = set(mix_cfg.get("foreground_groups") or [])
    for idx, inst in enumerate(pm.instruments):
        if not inst.notes:
            continue
        sub = clone_pm_with_instrument_subset(pm, [idx])
        midi_path = tempdir / f"inst_{idx:02d}_{inst.name}.mid"
        sub.write(str(midi_path))
        dry = render_synth_audio(sub, midi_path, soundfont, sample_rate, backend, allow_separate_mix=False)
        dry = ensure_audio_length(dry, target_samples)
        gain = db_to_amp(instrument_gain_db(spec, inst.name))
        group = group_for_instrument(spec, inst.name)
        if group in foreground_groups:
            foreground += dry * gain
        elif group is None:
            misc += dry * gain
        else:
            background += dry * gain

    duck_cfg = mix_cfg.get("foreground_ducking", {}) or {}
    if duck_cfg.get("enabled", False) and np.max(np.abs(foreground)) > 1e-7:
        # Use a conservative RMS-ish mono envelope so ducking tracks musical
        # phrases rather than individual samples.  It is normalized by a high
        # percentile instead of the max, so one loud accent does not make the
        # whole cue pump.
        mono = np.sqrt(np.mean(foreground ** 2, axis=1)).astype(np.float32)
        env = smooth_envelope(
            mono,
            sample_rate,
            float(duck_cfg.get("attack_ms", 35.0)),
            float(duck_cfg.get("release_ms", 260.0)),
        )
        ref = float(np.percentile(env[env > 0], 92)) if np.any(env > 0) else 1.0
        env = np.clip(env / max(ref, 1e-6), 0.0, 1.0)
        threshold = float(duck_cfg.get("threshold", 0.08))
        if threshold > 0:
            env = np.clip((env - threshold) / max(1e-6, 1.0 - threshold), 0.0, 1.0)
        min_gain = db_to_amp(float(duck_cfg.get("depth_db", -2.5)))
        curve = float(duck_cfg.get("curve", 1.35))
        duck = 1.0 - (1.0 - min_gain) * (env ** curve)
        background *= duck[:, None]

    master_gain = db_to_amp(float(mix_cfg.get("master_gain_db", 0.0)))
    return ((background + misc + foreground) * master_gain).astype(np.float32, copy=False)


def render_pretty_midi(pm: pretty_midi.PrettyMIDI, soundfont: str, sample_rate: int) -> np.ndarray:
    raw = pm.fluidsynth(fs=sample_rate, sf2_path=soundfont, normalize=False)
    return _coerce_stereo(raw)


def render_with_fluidsynth_cli(midi_path: Path, soundfont: str, sample_rate: int, dry_wav_path: Path) -> np.ndarray:
    exe = shutil.which("fluidsynth")
    if exe is None:
        raise FileNotFoundError("fluidsynth CLI not found")
    dry_wav_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [exe, "-ni", str(soundfont), str(midi_path), "-F", str(dry_wav_path), "-r", str(sample_rate), "-g", "0.9"]
    subprocess.run(cmd, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    audio, sr = sf.read(str(dry_wav_path), always_2d=True, dtype="float32")
    if sr != sample_rate:
        raise RuntimeError(f"FluidSynth wrote sample rate {sr}, expected {sample_rate}")
    return _coerce_stereo(audio)


def render_synth_audio(
    pm: pretty_midi.PrettyMIDI,
    midi_path: Path,
    soundfont: str,
    sample_rate: int,
    backend: str,
    *,
    allow_separate_mix: bool = True,
    spec: dict[str, Any] | None = None,
    tempdir: Path | None = None,
) -> np.ndarray:
    backend = backend.lower()
    if backend not in {"auto", "fluidsynth-cli", "pretty_midi"}:
        raise ValueError(f"unknown backend {backend!r}")
    if allow_separate_mix and spec is not None and (spec.get("mix", {}) or {}).get("render_mode") == "separate_instruments":
        if tempdir is None:
            tempdir = midi_path.parent
        return render_instruments_separately(pm, spec, soundfont, sample_rate, backend, tempdir)
    if backend in {"auto", "fluidsynth-cli"}:
        try:
            return render_with_fluidsynth_cli(midi_path, soundfont, sample_rate, midi_path.with_suffix(".dry.wav"))
        except Exception:
            if backend == "fluidsynth-cli":
                raise
    return render_pretty_midi(pm, soundfont, sample_rate)


def highpass(audio: np.ndarray, sample_rate: int, hz: float = 35.0) -> np.ndarray:
    sos = signal.butter(2, hz, btype="highpass", fs=sample_rate, output="sos")
    return signal.sosfilt(sos, audio, axis=0).astype(np.float32)


def lowpass(audio: np.ndarray, sample_rate: int, hz: float) -> np.ndarray:
    if not hz or hz <= 0 or hz >= sample_rate * 0.48:
        return audio.astype(np.float32, copy=False)
    sos = signal.butter(2, hz, btype="lowpass", fs=sample_rate, output="sos")
    return signal.sosfilt(sos, audio, axis=0).astype(np.float32)


def high_shelf_like(audio: np.ndarray, sample_rate: int, hz: float, gain_db: float) -> np.ndarray:
    """Gentle high-band tilt built from a Butterworth split.

    This is intentionally simple and stable.  It is not a mastering-grade shelf,
    but it is useful for taming SoundFont fizz/buzz without requiring a binary
    EQ plugin.
    """
    if abs(gain_db) < 1e-6 or not hz or hz <= 0 or hz >= sample_rate * 0.48:
        return audio.astype(np.float32, copy=False)
    sos = signal.butter(2, hz, btype="highpass", fs=sample_rate, output="sos")
    high = signal.sosfilt(sos, audio, axis=0).astype(np.float32)
    low = audio - high
    return (low + high * db_to_amp(gain_db)).astype(np.float32)


def notch_like(audio: np.ndarray, sample_rate: int, hz: float, q: float, gain_db: float) -> np.ndarray:
    if abs(gain_db) < 1e-6 or not hz or hz <= 0 or hz >= sample_rate * 0.48:
        return audio.astype(np.float32, copy=False)
    # A soft subtractive notch: isolate a narrow band and attenuate it by gain_db.
    low = max(40.0, hz / max(1.1, q ** 0.5))
    high = min(sample_rate * 0.48, hz * max(1.1, q ** 0.5))
    if high <= low:
        return audio.astype(np.float32, copy=False)
    sos = signal.butter(2, [low, high], btype="bandpass", fs=sample_rate, output="sos")
    band = signal.sosfilt(sos, audio, axis=0).astype(np.float32)
    return (audio - band * (1.0 - db_to_amp(gain_db))).astype(np.float32)


def simple_reverb(audio: np.ndarray, sample_rate: int, *, wet: float = 0.18, decay: float = 1.4, damping_hz: float = 8500.0) -> np.ndarray:
    # This is intentionally the same convolution-style deterministic room used
    # in the first goblin renderer.
    rng = np.random.default_rng(20260505)
    ir_len = int(sample_rate * decay)
    t = np.arange(ir_len) / sample_rate
    env = np.exp(-t / (decay / 4.2))
    ir = rng.normal(0, 1, (ir_len, 2)).astype(np.float32) * env[:, None]
    for delay_ms, gain, pan in [(18, 0.55, -0.3), (31, 0.42, 0.25), (47, 0.32, -0.1), (73, 0.24, 0.2)]:
        idx = int(sample_rate * delay_ms / 1000)
        if idx < ir_len:
            ir[idx, 0] += gain * (1 - pan) * 0.5
            ir[idx, 1] += gain * (1 + pan) * 0.5
    sos = signal.butter(2, float(damping_hz), btype="lowpass", fs=sample_rate, output="sos")
    ir = signal.sosfilt(sos, ir, axis=0).astype(np.float32)
    ir /= max(1e-8, np.max(np.abs(ir)))
    wet_sig = np.column_stack([
        signal.fftconvolve(audio[:, 0], ir[:, 0], mode="full")[: len(audio)],
        signal.fftconvolve(audio[:, 1], ir[:, 1], mode="full")[: len(audio)],
    ]).astype(np.float32)
    return (audio * (1.0 - wet) + wet_sig * wet).astype(np.float32)


def stereo_widen(audio: np.ndarray, amount: float = 0.12) -> np.ndarray:
    mid = (audio[:, 0] + audio[:, 1]) * 0.5
    side = (audio[:, 0] - audio[:, 1]) * 0.5 * (1.0 + amount)
    return np.column_stack([mid + side, mid - side]).astype(np.float32)


def soft_limit(
    audio: np.ndarray,
    target_peak_db: float = -4.5,
    *,
    drive: float = 1.05,
    peak_mode: str = "ceiling",
) -> np.ndarray:
    """Very gentle safety limiter.

    v7 used a hotter tanh stage and always normalized up to the target peak.
    That made the cue loud and buzzy when many separate instrument renders were
    summed.  This version defaults to ceiling behavior: it only turns audio down
    when needed, and it uses much less saturation.
    """
    drive = max(0.75, float(drive))
    if abs(drive - 1.0) < 1e-3:
        driven = audio.astype(np.float32, copy=True)
    else:
        driven = np.tanh(audio * drive) / np.tanh(drive)
    peak = float(np.max(np.abs(driven)))
    target = 10 ** (target_peak_db / 20.0)
    if peak > 1e-9:
        if peak_mode == "normalize":
            driven *= target / peak
        else:
            driven *= min(1.0, target / peak)
    return driven.astype(np.float32)


def post_process_legacy_goblin_v1(audio: np.ndarray, sample_rate: int, settings: dict[str, Any]) -> np.ndarray:
    out = audio.astype(np.float32, copy=False) * db_to_amp(float(settings.get("pre_gain_db", 0.0)))
    out = highpass(out, sample_rate, float(settings.get("highpass_hz", 35.0)))
    for notch in settings.get("notches", []) or []:
        out = notch_like(
            out,
            sample_rate,
            float(notch.get("hz", 2600.0)),
            float(notch.get("q", 2.0)),
            float(notch.get("gain_db", -1.5)),
        )
    out = high_shelf_like(
        out,
        sample_rate,
        float(settings.get("high_shelf_hz", 6500.0)),
        float(settings.get("high_shelf_db", 0.0)),
    )
    out = lowpass(out, sample_rate, float(settings.get("lowpass_hz", 0.0)))
    if float(settings.get("reverb_wet", 0.0)) > 0:
        out = simple_reverb(
            out,
            sample_rate,
            wet=float(settings.get("reverb_wet", 0.12)),
            decay=float(settings.get("reverb_decay_seconds", 1.2)),
            damping_hz=float(settings.get("reverb_damping_hz", 6500.0)),
        )
    out = stereo_widen(out, float(settings.get("stereo_width", 0.08)))
    return soft_limit(
        out,
        float(settings.get("target_peak_db", -4.5)),
        drive=float(settings.get("limiter_drive", 1.05)),
        peak_mode=str(settings.get("peak_mode", "ceiling")),
    )


def write_ogg_from_audio(audio: np.ndarray, sample_rate: int, ogg_path: Path, quality: float, keep_wav: bool = False) -> Path:
    ogg_path.parent.mkdir(parents=True, exist_ok=True)
    audio = np.nan_to_num(np.clip(_coerce_stereo(audio), -1.0, 1.0), nan=0.0).astype(np.float32, copy=False)
    if keep_wav:
        sf.write(ogg_path.with_suffix(".wav"), audio, sample_rate, subtype="PCM_16")
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg:
        cmd = [
            ffmpeg, "-y", "-hide_banner", "-loglevel", "error",
            "-f", "f32le", "-ar", str(sample_rate), "-ac", "2", "-i", "pipe:0",
            "-map_metadata", "-1", "-c:a", "libvorbis", "-q:a", str(quality), str(ogg_path),
        ]
        proc = subprocess.run(cmd, input=audio.tobytes(order="C"), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if proc.returncode != 0:
            raise RuntimeError(proc.stderr.decode("utf8", errors="replace"))
    else:
        sf.write(ogg_path, audio, sample_rate, format="OGG", subtype="VORBIS")
    return ogg_path


def spec_hash(spec_path: Path, soundfont: str, backend: str) -> str:
    h = hashlib.sha256()
    h.update(RENDERER_VERSION.encode("utf8"))
    h.update(spec_path.read_bytes())
    h.update(str(Path(soundfont).resolve()).encode("utf8"))
    h.update(backend.encode("utf8"))
    return h.hexdigest()[:16]


def render_all(args: argparse.Namespace) -> dict[str, Any]:
    spec_path = Path(args.spec).expanduser().resolve()
    spec = load_yaml(spec_path)
    if spec.get("schema") != "ambition.musicir.direct_midi.v1":
        raise ValueError(f"expected schema ambition.musicir.direct_midi.v1, got {spec.get('schema')!r}")
    if getattr(args, "mix_render_mode", None):
        spec.setdefault("mix", {})["render_mode"] = args.mix_render_mode
    render_cfg = spec.get("render", {})
    sample_rate = int(args.sample_rate or render_cfg.get("sample_rate", 48000))
    backend = args.backend or render_cfg.get("backend", "pretty_midi")
    soundfont = choose_soundfont(args.soundfont or render_cfg.get("soundfont"))
    quality = float(render_cfg.get("ogg_quality", 5.0))
    cue_hash = spec_hash(spec_path, soundfont, backend)
    outdir = Path(args.outdir).expanduser().resolve()
    outdir.mkdir(parents=True, exist_ok=True)
    pm = build_pretty_midi(spec)
    # Keep the exact source events by default, but allow YAML to declare a mix
    # pass that lifts solo instruments or lowers beds without changing the
    # composition.  `separate_instruments` is the most faithful mix path;
    # `full_midi` applies conservative MIDI-domain gains for speed.
    mix_cfg = spec.get("mix", {}) or {}
    if mix_cfg.get("render_mode", "full_midi") != "separate_instruments":
        apply_single_pass_midi_mix(pm, spec)
    with tempfile.TemporaryDirectory() as d:
        temp = Path(d)
        midi_path = temp / f"{spec['id']}_{cue_hash}.mid"
        pm.write(str(midi_path))
        raw = render_synth_audio(pm, midi_path, soundfont, sample_rate, backend, spec=spec, tempdir=temp)
    duration_hint = spec.get("duration_seconds") or spec.get("source_reference", {}).get("original_duration_seconds")
    if duration_hint:
        raw = ensure_audio_length(raw, int(np.ceil(float(duration_hint) * sample_rate)))
    audio = post_process_legacy_goblin_v1(raw, sample_rate, spec.get("postprocess", {}))
    preview_dir = outdir / "preview"
    ogg_path = preview_dir / f"{spec['id']}_{cue_hash}.ogg"
    write_ogg_from_audio(audio, sample_rate, ogg_path, quality=quality, keep_wav=args.keep_wav)
    if args.keep_midi:
        debug_dir = outdir / "debug"
        debug_dir.mkdir(parents=True, exist_ok=True)
        midi_out = debug_dir / f"{spec['id']}_{cue_hash}.mid"
        pm.write(str(midi_out))
    manifest = {
        "id": spec["id"],
        "title": spec.get("title"),
        "schema": spec.get("schema"),
        "renderer_version": RENDERER_VERSION,
        "cache_key": cue_hash,
        "bpm": float(spec.get("tempo", {}).get("bpm", 120.0)),
        "sample_rate": sample_rate,
        "soundfont": soundfont,
        "backend": backend,
        "duration_seconds": len(audio) / sample_rate,
        "peak": float(np.max(np.abs(audio))),
        "rms": float(np.sqrt(np.mean(audio ** 2))),
        "ogg_path": str(ogg_path.relative_to(outdir)),
        "source_reference": spec.get("source_reference", {}),
        "mix": spec.get("mix", {}),
    }
    manifest_path = outdir / f"{spec['id']}_{cue_hash}.manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf8")
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return manifest


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("spec", help="Direct-MIDI YAML music asset")
    parser.add_argument("--outdir", default="output/goblin-short-v1", help="Output directory")
    parser.add_argument("--soundfont", default=None)
    parser.add_argument("--sample-rate", type=int, default=None)
    parser.add_argument("--backend", default=None, choices=["auto", "fluidsynth-cli", "pretty_midi"])
    parser.add_argument("--keep-wav", action="store_true", help="Also keep WAV master next to the OGG")
    parser.add_argument("--keep-midi", action="store_true", help="Also keep generated MIDI for debugging")
    parser.add_argument(
        "--mix-render-mode",
        choices=["full_midi", "separate_instruments"],
        default=None,
        help="Override spec.mix.render_mode for A/B diagnostics",
    )
    args = parser.parse_args(argv)
    render_all(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
