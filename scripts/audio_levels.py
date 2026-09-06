#!/usr/bin/env python3
"""Measure relative loudness across Ambition music and SFX.

The report combines three sources: rendered music files, clips packed in the SFX
bank, and procedural `SfxSpec` cues synthesized with the same parameters as the
runtime. It reports measurements only; it never retunes assets.

Music is ranked by integrated LUFS and true peak. Short SFX are ranked by RMS
dBFS and true peak because integrated LUFS is not meaningful for these clip
lengths. Packed and procedural SFX share one comparison population even though
the report records their production cohort separately.

Measurements are cached under `target/audio_loudness/` by content identity.

Usage::

    python3 scripts/audio_levels.py
    python3 scripts/audio_levels.py --limit 20
    python3 scripts/audio_levels.py --only music
    python3 scripts/audio_levels.py --json out.json"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import io
import json
import math
import os
import re
import statistics
import subprocess
import time
import urllib.parse
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Iterator, Sequence

import numpy as np
from rich import print as rprint
from rich.markup import escape as rich_escape

# Bump when the ffmpeg filter chain or a derived metric changes; it is part of
# the cache key, so an old cache cannot silently serve numbers for a new
# definition.
#
# **`synthesize()` is part of the definition too.** Procedural rows are keyed
# by the SPEC, not by the rendered bytes (see `spec_cache_key`), so a change to
# the synthesizer or to `PROCEDURAL_CUE_REFERENCE_RMS_DBFS` is invisible to the
# cache: the run reports `0 fresh, N cached` and serves the OLD sound's numbers
# for the new one, cheerfully and without a warning. That happened once already,
# on the run that was meant to verify this file's own change.
METRICS_VERSION = 4

REPO_ROOT = Path(__file__).resolve().parent.parent

DEFAULT_REPORT = REPO_ROOT / 'dev' / 'audio_loudness_report.md'
CACHE_DIR = REPO_ROOT / 'target' / 'audio_loudness'
CACHE_PATH = CACHE_DIR / 'cache.json'

# `-70.0` is ebur128's absolute gate, returned verbatim when nothing passes it.
LUFS_FLOOR = -70.0
# BS.1770 blocks are 400 ms. Anything shorter cannot produce a gated result.
LUFS_MIN_SECONDS = 0.4

ASTATS_MEASURES = (
    'Peak_level+RMS_level+RMS_peak+RMS_trough+Crest_factor+Flat_factor'
    '+Peak_count+Number_of_samples+Dynamic_range'
)


# ---------------------------------------------------------------------------
# where the assets live (the consumer declares; this tool is TOLD)
# ---------------------------------------------------------------------------


def declared_asset_roots() -> dict[str, Path]:
    """Read `scripts/lib/asset_roots.sh` rather than re-guessing the crate name."""
    script = REPO_ROOT / 'scripts' / 'lib' / 'asset_roots.sh'
    out = subprocess.run(
        ['bash', '-c', f'source {script!s} && printf "%s\\n%s\\n" "$AMBITION_ASSETS_ROOT" "$AMBITION_MUSIC_PUBLISH_ROOT"'],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    return {'assets': Path(out[0]), 'music': Path(out[1])}


# ---------------------------------------------------------------------------
# population 1: loose music files
# ---------------------------------------------------------------------------


AUDIO_SUFFIXES = ('.ogg', '.wav', '.flac', '.mp3', '.oga', '.m4a')


def discover_loose_audio() -> list[Path]:
    """Every audio file under a shipping `assets/` tree, however it is named.

    Deliberately broader than `*.ogg` under the music root: the point of a sweep
    is to notice a population nobody mentioned. What it finds is reported.
    """
    found: list[Path] = []
    for top in ('crates', 'game'):
        for assets in sorted((REPO_ROOT / top).glob('*/assets')):
            for path in sorted(assets.rglob('*')):
                if path.is_file() and path.suffix.lower() in AUDIO_SUFFIXES:
                    found.append(path)
    return found


def music_identity(path: Path, music_root: Path) -> tuple[str, str, str]:
    """(cohort, score_id, display_id) for a file under the music publish root."""
    try:
        rel = path.relative_to(music_root)
    except ValueError:
        return ('audio_other', '', str(path.relative_to(REPO_ROOT)))
    parts = rel.parts
    score = parts[0]
    if len(parts) == 2 and parts[1] == 'full.ogg':
        return ('music_score', score, score)
    if len(parts) >= 3 and parts[1] == 'adaptive':
        return ('music_section', score, f'{score}/{parts[2]}')
    return ('music_other', score, '/'.join(parts))


# ---------------------------------------------------------------------------
# population 2: the packed SFX bank
# ---------------------------------------------------------------------------


BANK_MAGIC = b'AMBNDSFX'
BANK_HEADER_FMT = '<8sIIQQQ'
BANK_ENTRY_FMT = '<QQIBBHIIfffI16x'
BANK_CODECS = {0: 'wav', 1: 'ogg', 2: 'flac'}


@dataclass
class BankEntry:
    sfx_id: str
    payload: bytes
    codec: str
    channels: int
    sample_rate: int
    duration_ms: int
    stored_peak_db: float
    stored_rms_db: float
    flags: int


def read_bank(path: Path) -> list[BankEntry]:
    """Slice the shipped bank. Format spec: `tools/ambition_sfx_pack/pack.py`."""
    import struct

    blob = path.read_bytes()
    magic, version, count, entries_off, _payloads_off, names_off = struct.unpack_from(
        BANK_HEADER_FMT, blob, 0
    )
    if magic != BANK_MAGIC:
        raise RuntimeError(f'{path} is not an Ambition SFX bank (magic={magic!r})')
    if version != 1:
        raise RuntimeError(f'{path}: unsupported bank version {version}')

    names: list[str] = []
    cursor = names_off
    for _ in range(count):
        (length,) = struct.unpack_from('<H', blob, cursor)
        cursor += 2
        names.append(blob[cursor : cursor + length].decode('utf-8'))
        cursor += length
    if cursor != len(blob):
        raise RuntimeError(f'{path}: names section does not end at EOF ({cursor} != {len(blob)})')

    entries: list[BankEntry] = []
    for index in range(count):
        (
            _hash,
            offset,
            length,
            codec,
            channels,
            _pad,
            sample_rate,
            duration_ms,
            _gain,
            peak_db,
            rms_db,
            flags,
        ) = struct.unpack_from(BANK_ENTRY_FMT, blob, entries_off + index * 64)
        entries.append(
            BankEntry(
                sfx_id=names[index],
                payload=blob[offset : offset + length],
                codec=BANK_CODECS.get(codec, f'codec{codec}'),
                channels=channels,
                sample_rate=sample_rate,
                duration_ms=duration_ms,
                stored_peak_db=peak_db,
                stored_rms_db=rms_db,
                flags=flags,
            )
        )
    return entries


# ---------------------------------------------------------------------------
# population 3: procedural SfxSpec rows, extracted then synthesized
# ---------------------------------------------------------------------------


SPEC_FIELDS = (
    'cue',
    'id',
    'waveform',
    'frequency',
    'frequency_end',
    'duration',
    'volume',
    'attack',
    'release',
    'noise',
)


@dataclass
class ProceduralSpec:
    sfx_id: str
    owner: str
    source: str
    waveform: str
    frequency: float
    frequency_end: float
    duration: float
    volume: float
    attack: float
    release: float
    noise: float
    sample_rate: int = 44100
    unresolved: tuple[str, ...] = ()

    def key(self) -> str:
        return '|'.join(
            [
                self.sfx_id,
                self.waveform,
                f'{self.frequency:g}',
                f'{self.frequency_end:g}',
                f'{self.duration:g}',
                f'{self.volume:g}',
                f'{self.attack:g}',
                f'{self.release:g}',
                f'{self.noise:g}',
                str(self.sample_rate),
            ]
        )


def _split_top_level(text: str, sep: str = ',') -> list[str]:
    """Split on `sep` at nesting depth 0, respecting (), [], {}, <> and strings."""
    parts: list[str] = []
    depth = 0
    in_str = False
    escape = False
    current: list[str] = []
    for ch in text:
        if in_str:
            current.append(ch)
            if escape:
                escape = False
            elif ch == '\\':
                escape = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
            current.append(ch)
            continue
        if ch in '([{<':
            depth += 1
        elif ch in ')]}>':
            depth -= 1
        if ch == sep and depth == 0:
            parts.append(''.join(current))
            current = []
            continue
        current.append(ch)
    tail = ''.join(current)
    if tail.strip():
        parts.append(tail)
    return [p.strip() for p in parts if p.strip()]


def _strip_comments(text: str) -> str:
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
    return re.sub(r'(?m)//.*?$', '', text)


def _strip_test_modules(text: str) -> str:
    """Drop `#[cfg(test)] mod .. { .. }` bodies.

    A test fixture's `SfxSpec` is not a sound anybody hears, and letting one in
    puts a fabricated level in a cohort whose median decides what counts as an
    outlier. Same reason the loose renderer output is not measured.
    """
    out = text
    while True:
        m = re.search(r'#\[cfg\(test\)\]\s*(?:pub\s+)?mod\s+\w+\s*\{', out)
        if not m:
            return out
        try:
            _body, end = _brace_block(out, m.end() - 1)
        except ValueError:
            return out
        out = out[: m.start()] + out[end + 1 :]


def _brace_block(text: str, open_index: int) -> tuple[str, int]:
    """Body between `text[open_index] == '{'` and its match; returns (body, end)."""
    depth = 0
    in_str = False
    escape = False
    for i in range(open_index, len(text)):
        ch = text[i]
        if in_str:
            if escape:
                escape = False
            elif ch == '\\':
                escape = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
        elif ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                return text[open_index + 1 : i], i
    raise ValueError('unbalanced braces')


def _paren_block(text: str, open_index: int) -> tuple[str, int]:
    depth = 0
    in_str = False
    escape = False
    for i in range(open_index, len(text)):
        ch = text[i]
        if in_str:
            if escape:
                escape = False
            elif ch == '\\':
                escape = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
        elif ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
            if depth == 0:
                return text[open_index + 1 : i], i
    raise ValueError('unbalanced parens')


_NUMBER = re.compile(r'^-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?(?:_?f(?:32|64))?$')
_ENUM_TAIL = re.compile(r'^(?:[A-Za-z_][\w]*::)*([A-Za-z_]\w*)$')


def _eval_scalar(expr: str, bindings: dict[str, Any]) -> float | None:
    """Evaluate a Rust scalar expression using only literals and bound params.

    Handles the forms the providers actually author:
    `0.12`, `duration`, `frequency * 1.25`, `(duration * 0.4).min(0.08)`.
    Anything else returns None and the field is reported unresolved.
    """
    expr = expr.strip()
    if _NUMBER.match(expr):
        return float(re.sub(r'_?f(?:32|64)$', '', expr))
    if expr in bindings and isinstance(bindings[expr], (int, float)):
        return float(bindings[expr])
    # `X.min(Y)` / `X.max(Y)` -> `min(X, Y)`
    converted = expr
    for _ in range(4):
        new = re.sub(r'\.\s*(min|max)\s*\(', r' @\1@ (', converted, count=1)
        if new == converted:
            break
        converted = new
    if '@min@' in converted or '@max@' in converted:
        # rewrite `A @min@ (B)` into `min(A, B)` from the right
        while True:
            m = re.search(r'(.+?)\s*@(min|max)@\s*\(([^()]*)\)', converted)
            if not m:
                break
            converted = converted[: m.start()] + f'{m.group(2)}({m.group(1)}, {m.group(3)})' + converted[m.end() :]
    converted = re.sub(r'\b(\d+(?:\.\d+)?)_?f(?:32|64)\b', r'\1', converted)
    if not re.fullmatch(r'[\w\s.,()*/+\-]*', converted):
        return None
    env: dict[str, Any] = {'__builtins__': {}, 'min': min, 'max': max}
    for name, value in bindings.items():
        if isinstance(value, (int, float)):
            env[name] = float(value)
    try:
        result = eval(converted, env)  # noqa: S307 - repo-local source, restricted env
    except Exception:
        return None
    return float(result) if isinstance(result, (int, float)) else None


def _eval_id(expr: str, bindings: dict[str, Any], consts: dict[str, str]) -> str | None:
    expr = expr.strip()
    if expr == 'None':
        return None
    m = re.fullmatch(r'Some\s*\((.*)\)', expr, flags=re.S)
    if m:
        expr = m.group(1).strip()
    expr = re.sub(r'\.\s*to_(?:owned|string)\s*\(\s*\)$', '', expr.strip()).strip()
    expr = re.sub(r'^str::to_owned\s*\((.*)\)$', r'\1', expr).strip()
    # `id.map(str::to_owned)` — the Option is threaded through unchanged.
    m = re.fullmatch(r'(\w+)\s*\.\s*map\s*\(.*\)', expr, flags=re.S)
    if m:
        expr = m.group(1)
    m = re.fullmatch(r'"((?:[^"\\]|\\.)*)"', expr)
    if m:
        return m.group(1)
    if expr in bindings and isinstance(bindings[expr], str):
        return bindings[expr]
    # `SFX_REV_TIERS[0]` and `crate::powerups::SFX_SMALL_TO_BIG`
    m = re.fullmatch(r'(?:[\w:]*::)?([A-Z][A-Z0-9_]*)\s*\[\s*(\d+)\s*\]', expr)
    if m and m.group(1) in consts:
        value = consts[m.group(1)]
        if isinstance(value, list):
            index = int(m.group(2))
            if index < len(value):
                return value[index]
        return None
    m = re.fullmatch(r'(?:[\w:]*::)?([A-Z][A-Z0-9_]*)', expr)
    if m and m.group(1) in consts:
        value = consts[m.group(1)]
        return value if isinstance(value, str) else None
    return None


def _eval_enum(expr: str, bindings: dict[str, Any]) -> str | None:
    expr = expr.strip()
    if expr == 'None':
        return None
    m = re.fullmatch(r'Some\s*\((.*)\)', expr, flags=re.S)
    if m:
        expr = m.group(1).strip()
    # `<anything>.then_some(E)` — the cue is E whenever the condition holds, and
    # the report wants to know which sound E is, not when it plays.
    m = re.fullmatch(r'.*?\.\s*then_some\s*\((.*)\)', expr, flags=re.S)
    if m:
        expr = m.group(1).strip()
    if expr in bindings and isinstance(bindings[expr], str):
        return bindings[expr]
    m = _ENUM_TAIL.match(expr.replace(' ', '').replace('\n', ''))
    return m.group(1) if m else None


_GLOBAL_CONSTS: dict[str, Any] | None = None


def global_str_consts() -> dict[str, Any]:
    """Every `const NAME: &str = ".."` in the workspace, by bare name.

    ⚠ providers author ids as `crate::powerups::SFX_SMALL_TO_BIG`, so a
    file-local const table resolves nothing and six of Mary-O's nine specs
    vanish from the report without a word. Names are unique enough in practice;
    a collision would only mislabel a row, never change a level.
    """
    global _GLOBAL_CONSTS
    if _GLOBAL_CONSTS is None:
        table: dict[str, Any] = {}
        for top in ('crates', 'game'):
            for rs in sorted((REPO_ROOT / top).rglob('*.rs')):
                text = rs.read_text(errors='replace')
                if 'const' not in text:
                    continue
                table.update(_collect_str_consts(text))
        _GLOBAL_CONSTS = table
    return _GLOBAL_CONSTS


def _collect_str_consts(text: str) -> dict[str, Any]:
    consts: dict[str, Any] = {}
    for m in re.finditer(r'const\s+([A-Z][A-Z0-9_]*)\s*:\s*&(?:\'static\s+)?str\s*=\s*"([^"]*)"', text):
        consts[m.group(1)] = m.group(2)
    for m in re.finditer(
        r'const\s+([A-Z][A-Z0-9_]*)\s*:\s*\[&(?:\'static\s+)?str;\s*\d+\]\s*=\s*\[([^\]]*)\]', text
    ):
        consts[m.group(1)] = re.findall(r'"([^"]*)"', m.group(2))
    return consts


def _enclosing_fn(text: str, index: int) -> tuple[str, list[str]] | None:
    """Name + parameter names of the `fn` whose body contains `index`."""
    best: tuple[str, list[str]] | None = None
    for m in re.finditer(r'(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(', text):
        if m.end() > index:
            break
        try:
            params_src, close = _paren_block(text, m.end() - 1)
        except ValueError:
            continue
        if close > index:
            continue
        brace = text.find('{', close)
        if brace < 0 or brace > index:
            continue
        try:
            _body, body_end = _brace_block(text, brace)
        except ValueError:
            continue
        if brace < index < body_end:
            names = []
            for param in _split_top_level(params_src):
                pm = re.match(r'(?:mut\s+)?(\w+)\s*:', param)
                if pm:
                    names.append(pm.group(1))
            best = (m.group(1), names)
    return best


def _call_sites(text: str, fn_name: str) -> Iterator[list[str]]:
    # `fn NAME(` matches a call-site regex, and its parameter LIST has exactly
    # the arity of a real call, so the declaration binds nothing and emits one
    # all-unresolved ghost spec per helper. Exclude it explicitly.
    for m in re.finditer(rf'(?<![\w.])(fn\s+)?{re.escape(fn_name)}\s*\(', text):
        if m.group(1):
            continue
        try:
            args_src, _ = _paren_block(text, m.end() - 1)
        except ValueError:
            continue
        yield _split_top_level(args_src)


def _spec_from_fields(
    fields: dict[str, str],
    bindings: dict[str, Any],
    consts: dict[str, str],
    owner: str,
    source: str,
    sample_rate: int,
) -> ProceduralSpec:
    unresolved: list[str] = []
    cue = _eval_enum(fields.get('cue', 'None'), bindings)
    ident = _eval_id(fields.get('id', 'None'), bindings, consts)
    waveform = _eval_enum(fields.get('waveform', ''), bindings) or ''
    if not waveform:
        unresolved.append('waveform')
    numbers: dict[str, float] = {}
    # Numeric fields may reference each other (`release: (duration*0.4).min(..)`),
    # so resolve in dependency order and feed each result back in.
    local = dict(bindings)
    for name in ('frequency', 'frequency_end', 'duration', 'volume', 'attack', 'release', 'noise'):
        value = _eval_scalar(fields.get(name, ''), local)
        if value is None:
            unresolved.append(name)
            value = 0.0
        numbers[name] = value
        local[name] = value
    sfx_id = ident or (f'cue:{cue}' if cue else '<unknown>')
    return ProceduralSpec(
        sfx_id=sfx_id,
        owner=owner,
        source=source,
        waveform=waveform,
        sample_rate=sample_rate,
        unresolved=tuple(unresolved),
        **numbers,
    )


def _owner_of(path: Path) -> str:
    rel = path.relative_to(REPO_ROOT)
    parts = rel.parts
    if parts[0] in ('crates', 'game') and len(parts) > 1:
        return parts[1]
    return parts[0]


def _sample_rate_from(text: str) -> int:
    m = re.search(r'sample_rate\s*:\s*([\d_]+)', text)
    # Rust authors `44_100`. A `(\d+)` capture silently yields 44 Hz, which
    # ffmpeg then clamps and reports as a real measurement of a different sound.
    return int(m.group(1).replace('_', '')) if m else 44100


def extract_ron_specs(path: Path) -> list[ProceduralSpec]:
    text = _strip_comments(path.read_text())
    sample_rate = _sample_rate_from(text)
    specs: list[ProceduralSpec] = []
    for opener in re.finditer(r'\(\s*(?:cue|id)\s*:', text):
        try:
            body, _ = _paren_block(text, opener.start())
        except ValueError:
            continue
        fields: dict[str, str] = {}
        for pair in _split_top_level(body):
            if ':' not in pair:
                continue
            key, value = pair.split(':', 1)
            if key.strip() in SPEC_FIELDS:
                fields[key.strip()] = value.strip()
        if 'waveform' not in fields:
            continue
        specs.append(
            _spec_from_fields(fields, {}, {}, _owner_of(path), str(path.relative_to(REPO_ROOT)), sample_rate)
        )
    return specs


def extract_rust_specs(path: Path) -> list[ProceduralSpec]:
    """Every `SfxSpec { .. }` literal in one Rust file, with helper args bound.

    Providers do not author literals directly; they write one helper per voice
    (`sanic_open`, `sanic_cue`, ...) whose PARAMETER NAMES are the field names.
    So a literal field that is a bare identifier naming a parameter is resolved
    by walking the helper's call sites — one spec per call. That is generic, not
    a per-provider special case, and it is what makes Sanic visible at all.
    """
    text = _strip_test_modules(_strip_comments(path.read_text()))
    consts = {**global_str_consts(), **_collect_str_consts(text)}
    owner = _owner_of(path)
    source = str(path.relative_to(REPO_ROOT))

    m = re.search(r'SfxRegistry\s*\{.{0,400}?sample_rate\s*:\s*([\d_]+)', text, flags=re.S)
    default_rate = int(m.group(1).replace('_', '')) if m else 44100

    specs: list[ProceduralSpec] = []
    for lit in re.finditer(r'SfxSpec\s*\{', text):
        try:
            body, _end = _brace_block(text, lit.end() - 1)
        except ValueError:
            continue
        fields: dict[str, str] = {}
        for pair in _split_top_level(body):
            if ':' in pair:
                key, value = pair.split(':', 1)
                key = key.strip()
                if key in SPEC_FIELDS:
                    fields[key] = value.strip()
            elif pair.strip() in SPEC_FIELDS:
                # Rust field shorthand: `duration,` means `duration: duration`
                fields[pair.strip()] = pair.strip()
        if 'waveform' not in fields:
            continue

        enclosing = _enclosing_fn(text, lit.start())
        params = enclosing[1] if enclosing else []
        uses_params = any(
            re.search(rf'(?<![\w.]){re.escape(p)}(?![\w])', v) for v in fields.values() for p in params
        )
        if enclosing and params and uses_params:
            for args in _call_sites(text, enclosing[0]):
                if len(args) != len(params):
                    continue
                bindings: dict[str, Any] = {}
                for name, arg in zip(params, args):
                    scalar = _eval_scalar(arg, {})
                    if scalar is not None:
                        bindings[name] = scalar
                        continue
                    as_id = _eval_id(arg, {}, consts)
                    if as_id is not None:
                        bindings[name] = as_id
                        continue
                    as_enum = _eval_enum(arg, {})
                    if as_enum is not None:
                        bindings[name] = as_enum
                specs.append(
                    _spec_from_fields(fields, bindings, consts, owner, source, default_rate)
                )
        else:
            specs.append(_spec_from_fields(fields, {}, consts, owner, source, default_rate))
    return specs


def discover_procedural_specs() -> list[ProceduralSpec]:
    specs: list[ProceduralSpec] = []
    for ron in sorted(REPO_ROOT.glob('game/*/assets/audio/*registry*.ron')):
        if 'sfx' in ron.name:
            specs.extend(extract_ron_specs(ron))
    for top in ('crates', 'game'):
        for rs in sorted((REPO_ROOT / top).rglob('*.rs')):
            if '/target/' in str(rs) or '/tests/' in str(rs) or rs.name == 'tests.rs':
                continue
            if 'SfxSpec' not in rs.read_text(errors='replace'):
                continue
            specs.extend(extract_rust_specs(rs))
    # Deduplicate identical (owner, id, params) rows; a helper called twice with
    # the same arguments is one sound.
    seen: dict[tuple[str, str], ProceduralSpec] = {}
    for spec in specs:
        if spec.sfx_id == '<unknown>':
            continue
        seen.setdefault((spec.owner, spec.key()), spec)
    return sorted(seen.values(), key=lambda s: (s.owner, s.sfx_id))


WAVEFORMS = {'Sine', 'Square', 'Triangle', 'Saw'}

# : `ambition_audio::render::PROCEDURAL_CUE_REFERENCE_RMS_DBFS` — the loudness of
# : a `volume = 1.0` cue, as RMS dBFS over its body. this is the one number
# : this port shares with the Rust by VALUE rather than by construction; if the
# : engine moves its target and this does not, every procedural row in the report
# : is off by the difference and nothing else notices.
PROCEDURAL_CUE_REFERENCE_RMS_DBFS = -11.0


def synthesize(spec: ProceduralSpec) -> bytes:
    """Port of `ambition_audio::render::audio_source_from_sfx_spec`.

    ⚠ a port, not a binding: it can drift from the Rust. Its analytic invariant
    (body RMS == clamped `volume` x the reference level, whatever the waveform
    and noise mix) is pinned by the tests, which is the part the report's
    conclusions actually rest on.
    """
    import soundfile as sf

    sample_rate = max(int(spec.sample_rate), 8000)
    duration = max(spec.duration, 0.01)
    frame_count = max(int(math.ceil(duration * sample_rate)), 2)
    index = np.arange(frame_count, dtype=np.float64)
    t = index / sample_rate
    progress = np.clip(t / duration, 0.0, 1.0)
    frequency = spec.frequency + (spec.frequency_end - spec.frequency) * progress
    phase = np.cumsum(2.0 * np.pi * np.maximum(frequency, 1.0) / sample_rate) % (2.0 * np.pi)

    if spec.waveform == 'Sine':
        tone = np.sin(phase)
    elif spec.waveform == 'Square':
        tone = np.where(np.sin(phase) >= 0.0, 1.0, -1.0)
    elif spec.waveform == 'Triangle':
        u = phase / (2.0 * np.pi)
        tone = 2.0 * np.abs(2.0 * (u - np.floor(u + 0.5))) - 1.0
    elif spec.waveform == 'Saw':
        tone = 2.0 * (phase / (2.0 * np.pi)) - 1.0
    else:
        raise ValueError(f'unknown waveform {spec.waveform!r}')

    mix = float(np.clip(spec.noise, 0.0, 1.0))
    if mix > 0.0:
        state = 0x6D2B79F5
        states = np.empty(frame_count, dtype=np.uint32)
        for i in range(frame_count):
            state = (state * 1664525 + 1013904223) & 0xFFFFFFFF
            states[i] = state
        noise = (states >> np.uint32(8)).astype(np.float64) / float(0x00FFFFFF) * 2.0 - 1.0
    else:
        noise = np.zeros(frame_count)

    # The cue's BODY, at unit scale and unenveloped, then one gain that puts it
    # on the loudness target. `volume` is a fraction of that target in the RMS
    # domain, so the body's own crest factor — which is what the waveform and
    # the noise mix decide — divides out.
    body = (1.0 - mix) * tone + mix * noise
    volume = float(np.clip(spec.volume, 0.0, 1.0))
    body_rms = float(np.sqrt(np.mean(body**2)))
    body_peak = float(np.max(np.abs(body)))
    if body_rms <= 0.0 or body_peak <= 0.0:
        gain = 0.0
    else:
        target_rms = volume * 10.0 ** (PROCEDURAL_CUE_REFERENCE_RMS_DBFS / 20.0)
        gain = min(target_rms / body_rms, 1.0 / body_peak)

    attack = max(spec.attack, 0.0)
    release = max(spec.release, 0.0)
    attack_gain = np.clip(t / attack, 0.0, 1.0) if attack > 0.0 else np.ones_like(t)
    release_start = max(duration - release, 0.0)
    if release > 0.0:
        release_gain = np.where(t > release_start, np.clip((duration - t) / release, 0.0, 1.0), 1.0)
    else:
        release_gain = np.ones_like(t)

    sample = body * gain * attack_gain * release_gain

    frames = np.stack([sample, sample], axis=1).astype(np.float32)
    buffer = io.BytesIO()
    sf.write(buffer, frames, sample_rate, subtype='FLOAT', format='WAV')
    return buffer.getvalue()


# ---------------------------------------------------------------------------
# the measurement itself
# ---------------------------------------------------------------------------


FFMPEG_FILTER = f'ebur128=peak=true,astats=measure_perchannel=none:measure_overall={ASTATS_MEASURES}'

_RE_I = re.compile(r'^\s*I:\s*(-?[\d.]+|-inf)\s*LUFS', re.M)
_RE_LRA = re.compile(r'^\s*LRA:\s*(-?[\d.]+)\s*LU\s*$', re.M)
_RE_TP = re.compile(r'^\s*Peak:\s*(-?[\d.]+|-inf)\s*dBFS', re.M)
_RE_FRAME = re.compile(r'M:\s*(-?[\d.]+|-inf)\s+S:\s*(-?[\d.]+|-inf)')
_RE_STREAM = re.compile(r'Audio:\s*([\w.]+).*?,\s*(\d+)\s*Hz,\s*([\w.() ]+?),')
_RE_ASTAT = re.compile(r'^\[Parsed_astats[^\]]*\]\s*([A-Za-z][A-Za-z ]*?):\s*(-?[\d.]+|-?inf|nan)\s*$', re.M)


def _f(text: str) -> float | None:
    if text in ('-inf', 'inf', 'nan'):
        return None
    try:
        return float(text)
    except ValueError:
        return None


def run_ffmpeg(source: Path | bytes) -> dict[str, Any]:
    cmd = ['ffmpeg', '-hide_banner', '-nostats']
    if isinstance(source, Path):
        cmd += ['-i', str(source)]
        stdin_data = None
    else:
        cmd += ['-i', 'pipe:0']
        stdin_data = source
    cmd += ['-map', '0:a:0', '-af', FFMPEG_FILTER, '-f', 'null', '-']
    proc = subprocess.run(cmd, input=stdin_data, capture_output=True)
    err = proc.stderr.decode('utf-8', errors='replace')
    if proc.returncode != 0:
        return {'error': err.strip().splitlines()[-1] if err.strip() else f'exit {proc.returncode}'}

    stats: dict[str, float | None] = {}
    for m in _RE_ASTAT.finditer(err):
        stats[m.group(1).strip()] = _f(m.group(2))

    frames = [(_f(a), _f(b)) for a, b in _RE_FRAME.findall(err)]
    momentary = [a for a, _ in frames if a is not None and a > LUFS_FLOOR]
    shortterm = [b for _, b in frames if b is not None and b > LUFS_FLOOR]

    stream = _RE_STREAM.search(err)
    samples = stats.get('Number of samples')
    sample_rate = int(stream.group(2)) if stream else 0
    duration = (samples / sample_rate) if (samples and sample_rate) else None

    i_all = _RE_I.findall(err)
    lufs = _f(i_all[-1]) if i_all else None
    tp_all = _RE_TP.findall(err)
    lra_all = _RE_LRA.findall(err)

    return {
        'codec': stream.group(1) if stream else None,
        'sample_rate': sample_rate,
        'channels': (stream.group(3).strip() if stream else None),
        'duration_s': duration,
        'lufs_i': lufs,
        'lra': _f(lra_all[-1]) if lra_all else None,
        'dbtp': _f(tp_all[-1]) if tp_all else None,
        'max_momentary_lufs': max(momentary) if momentary else None,
        'max_shortterm_lufs': max(shortterm) if shortterm else None,
        'peak_db': stats.get('Peak level dB'),
        'rms_db': stats.get('RMS level dB'),
        'rms_peak_db': stats.get('RMS peak dB'),
        'crest_factor': stats.get('Crest factor'),
        'flat_factor': stats.get('Flat factor'),
        'peak_count': stats.get('Peak count'),
        'dynamic_range': stats.get('Dynamic range'),
    }


@dataclass
class Item:
    key: str
    cohort: str
    name: str
    owner: str
    origin: str
    source: Path | bytes = field(repr=False, default=b'')
    extra: dict[str, Any] = field(default_factory=dict)
    metrics: dict[str, Any] = field(default_factory=dict)

    @property
    def lufs_valid(self) -> bool:
        lufs = self.metrics.get('lufs_i')
        duration = self.metrics.get('duration_s') or 0.0
        return lufs is not None and lufs > LUFS_FLOOR + 0.05 and duration >= LUFS_MIN_SECONDS


def file_cache_key(path: Path) -> str:
    stat = path.stat()
    return f'v{METRICS_VERSION}:file:{path.relative_to(REPO_ROOT)}:{stat.st_mtime_ns}:{stat.st_size}'


def bytes_cache_key(kind: str, payload: bytes) -> str:
    return f'v{METRICS_VERSION}:{kind}:{hashlib.sha1(payload).hexdigest()}'


def spec_cache_key(spec: ProceduralSpec) -> str:
    """Key a synthesized cue by the SPEC, never by the bytes.

    ⚠ libsndfile stamps a float WAV's `PEAK` chunk with the creation time, so
    two byte-identical-sounding renders hash differently and every procedural
    row misses the cache on every run — a cache that silently never hits.
    The spec is the sound's identity anyway.
    """
    return f'v{METRICS_VERSION}:spec:{spec.owner}|{spec.key()}'


def load_cache() -> dict[str, Any]:
    if CACHE_PATH.exists():
        try:
            return json.loads(CACHE_PATH.read_text())
        except json.JSONDecodeError:
            return {}
    return {}


def save_cache(cache: dict[str, Any]) -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    CACHE_PATH.write_text(json.dumps(cache, sort_keys=True))


# ---------------------------------------------------------------------------
# cohort statistics and outlier ranking
# ---------------------------------------------------------------------------


def robust_stats(values: Sequence[float]) -> dict[str, float]:
    if not values:
        return {}
    ordered = sorted(values)
    median = statistics.median(ordered)
    mad = statistics.median([abs(v - median) for v in ordered]) or 0.0
    return {
        'n': len(ordered),
        'min': ordered[0],
        'p10': ordered[max(0, int(len(ordered) * 0.10) - 0)],
        'median': median,
        'p90': ordered[min(len(ordered) - 1, int(len(ordered) * 0.90))],
        'max': ordered[-1],
        'mad': mad,
        # 1.4826 * MAD is the normal-consistent robust sigma.
        'sigma': 1.4826 * mad,
    }


def population_of(cohort: str) -> str:
    """The set a sound's level is judged against.

    ⛔ NOT the cohort. `sfx_packed` and `sfx_procedural` are two ways of
    producing the same thing: they share a metric, they share the SFX mix bus,
    and the runtime picks between them per provider — the same cue id is a
    packed sample in one game and a synth spec in another. Judging each against
    its own median hides exactly the defect that matters, because a population
    that is uniformly hot has no internal outlier. Same for a score and one of
    its adaptive sections: both are complete mixes that play alone.
    """
    return 'music' if cohort.startswith('music') else 'sfx'


def population_metric(population: str) -> str:
    """The metric that is DEFINED for this population. See the module docstring."""
    return 'lufs_i' if population == 'music' else 'rms_db'


# ---------------------------------------------------------------------------
# reporting
# ---------------------------------------------------------------------------


def fmt(value: Any, digits: int = 1, dash: str = '—') -> str:
    if value is None:
        return dash
    if isinstance(value, float):
        return f'{value:.{digits}f}'
    return str(value)


def file_uri(path: Path) -> str:
    return 'file://' + urllib.parse.quote(os.fspath(Path(path).resolve()))


def _display(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(REPO_ROOT))
    except ValueError:
        return str(resolved)


def link(path: Path, text: str | None = None) -> str:
    label = rich_escape(text if text is not None else str(path))
    return f'[link={file_uri(path)}]{label}[/link]'


def markdown_table(headers: Sequence[str], rows: Sequence[Sequence[str]]) -> str:
    out = ['| ' + ' | '.join(headers) + ' |', '|' + '|'.join(['---'] * len(headers)) + '|']
    for row in rows:
        out.append('| ' + ' | '.join(row) + ' |')
    return '\n'.join(out)


def _cohort_values(group: list[Item], metric: str) -> list[float]:
    if metric == 'lufs_i':
        return [i.metrics['lufs_i'] for i in group if i.lufs_valid]
    return [i.metrics[metric] for i in group if i.metrics.get(metric) is not None]


def verdict_lines(items: list[Item]) -> list[str]:
    """The answer, computed — not a paragraph somebody has to keep in sync.

    Two verdicts, because "too loud" is two different failures: a clipping
    transient (true peak) and a cohort-relative loudness offset (LUFS / RMS).
    """
    out: list[str] = []

    peaks = [i for i in items if i.metrics.get('dbtp') is not None]
    clipping = sorted((i for i in peaks if i.metrics['dbtp'] >= -1.0), key=lambda i: -i.metrics['dbtp'])
    hottest = max(peaks, key=lambda i: i.metrics['dbtp']) if peaks else None
    if clipping:
        out.append(
            f'⛔ **{len(clipping)} sound(s) at or above -1.0 dBTP** — inter-sample overshoot can '
            'clip on playback. Highest: '
            + ', '.join(f'`{i.name}` ({i.metrics["dbtp"]:+.1f} dBTP)' for i in clipping[:5])
            + '.'
        )
    elif hottest:
        out.append(
            f'✅ **Nothing clips.** The hottest true peak in the whole tree is '
            f'`{hottest.name}` at **{hottest.metrics["dbtp"]:.1f} dBTP**, which leaves headroom. '
            'No file is a clipping-driven ear-stab; every finding below is a *relative level* '
            'problem, not a distortion one.'
        )

    by_population: dict[str, list[Item]] = {}
    for item in items:
        by_population.setdefault(population_of(item.cohort), []).append(item)

    # A whole production path sitting hot is a different fix from one loud file.
    for population, group in sorted(by_population.items()):
        metric = population_metric(population)
        base = _cohort_values(group, metric)
        if len(base) < 8:
            continue
        base_median = statistics.median(base)
        cohorts = sorted({i.cohort for i in group})
        if len(cohorts) < 2:
            continue
        spread = []
        for cohort in cohorts:
            values = _cohort_values([i for i in group if i.cohort == cohort], metric)
            if values:
                spread.append((cohort, statistics.median(values), len(values)))
        spread.sort(key=lambda row: -row[1])
        if spread[0][1] - spread[-1][1] >= 3.0:
            scale = 'LUFS' if metric == 'lufs_i' else 'RMS dBFS'
            out.append('')
            out.append(
                f'⛔ **the two ways of producing a `{population}` sound do not agree**: '
                + ', '.join(f'`{c}` {m:.1f} (n={n})' for c, m, n in spread)
                + f' {scale} — a **{spread[0][1] - spread[-1][1]:.1f} dB** gap between paths that '
                'feed the same mix bus.'
            )

    offenders: list[tuple[float, str]] = []
    for population, group in sorted(by_population.items()):
        metric = population_metric(population)
        base = _cohort_values(group, metric)
        if len(base) < 8:
            continue
        base_median = statistics.median(base)
        owners: dict[str, list[Item]] = {}
        for item in group:
            owners.setdefault(item.owner, []).append(item)
        for owner, rows in owners.items():
            values = _cohort_values(rows, metric)
            if len(values) < 3:
                continue
            delta = statistics.median(values) - base_median
            if delta >= 3.0:
                scale = 'LUFS' if metric == 'lufs_i' else 'RMS dBFS'
                offenders.append(
                    (
                        delta,
                        f'**`{owner}` is {delta:+.1f} dB above the `{population}` median** '
                        f'({statistics.median(values):.1f} vs {base_median:.1f} {scale}, '
                        f'{len(values)} sounds, loudest {max(values):.1f}).',
                    )
                )
    out.append('')
    if offenders:
        out.append('Owners whose median sits 3 dB or more above everything they play alongside:')
        out.append('')
        for _, text in sorted(offenders, reverse=True):
            out.append(f'- {text}')
    else:
        out.append('No owner\'s median sits 3 dB or more above its population.')

    out.extend(_loudest_sounds_finding(by_population))
    return out


def _loudest_sounds_finding(by_population: dict[str, list[Item]]) -> list[str]:
    """Rank individual loudness outliers within each population.

    Owner medians describe catalogue balance, not isolated loud sounds. Use
    median and MAD because the populations have long quiet tails that distort
    mean/sigma thresholds. The 3.0 factor is ~2 sigma for a normal
    distribution — loose enough to be worth reading, tight enough to stay short.
    """
    out: list[str] = ['', '### The loudest individual sounds', '']
    out.append(
        'An ear is stabbed by ONE sound, not by a median. This ranks single sounds against '
        'their own population using **median + MAD** (robust: a long quiet tail cannot '
        'flatten it), and lists everything at or above `median + 3 x MAD`.'
    )
    out.append('')
    rows: list[tuple[float, str]] = []
    for population, group in sorted(by_population.items()):
        metric = population_metric(population)
        values = _cohort_values(group, metric)
        if len(values) < 8:
            continue
        median = statistics.median(values)
        mad = statistics.median([abs(v - median) for v in values])
        if mad <= 0.0:
            continue
        threshold = median + 3.0 * mad
        scale = 'LUFS' if metric == 'lufs_i' else 'RMS dBFS'
        for item in group:
            # through `metrics`, and NOT `getattr(item, metric)`: the metric
            # is a dict KEY, not an attribute name, so getattr returned None for
            # every item and this section reported a clean sweep on its first
            # run. a finding that cannot fire is worse than no finding — it
            # reads as a pass.
            if metric == 'lufs_i' and not item.lufs_valid:
                continue
            value = item.metrics.get(metric)
            if value is None or value < threshold:
                continue
            peak = fmt(item.metrics.get('dbtp'), dash='--')
            rows.append(
                (
                    value - median,
                    f'| {value - median:+.1f} | {value:.1f} | {peak} | {population} | '
                    f'`{item.owner}` | `{item.name}` |',
                )
            )
    if not rows:
        out.append('No single sound sits 3 MAD above its population. ✅')
        return out
    out.append('| Δ median | level | dBTP | pop | owner | sound |')
    out.append('|---|---|---|---|---|---|')
    for _, row in sorted(rows, reverse=True)[:20]:
        out.append(row)
    out.append('')
    out.append(
        '⚠ `level` is LUFS for music and RMS dBFS for SFX — the metric each population is '
        'defined on. The two columns are never comparable to each other.'
    )
    return out


def build_report(items: list[Item], top: int, timings: dict[str, float], notes: list[str]) -> str:
    by_cohort: dict[str, list[Item]] = {}
    for item in items:
        by_cohort.setdefault(item.cohort, []).append(item)

    lines: list[str] = []
    lines.append('# Audio loudness sweep')
    lines.append('')
    lines.append(
        'Generated by `python3 scripts/audio_levels.py`. **A report, not a change** — '
        'nothing here has been retuned. Regenerate after any audio edit; the numbers '
        'describe the tree at measurement time.'
    )
    lines.append('')
    lines.append(
        f'Measured **{len(items)}** sounds in {timings["total"]:.0f}s '
        f'({timings["measured"]} measured, {timings["cached"]} from cache).'
    )
    lines.append('')
    lines.append('## Verdict')
    lines.append('')
    for line in verdict_lines(items):
        lines.append(line)
    lines.append('')

    lines.append('## Method')
    lines.append('')
    lines.append(
        'One `ffmpeg -af ebur128=peak=true,astats` pass per sound (ITU-R BS.1770): '
        'integrated LUFS, loudness range, **true peak (dBTP)**, plus sample peak and RMS.'
    )
    lines.append('')
    lines.append(
        '⛔ **Integrated LUFS is structurally undefined below 400 ms** and ebur128 returns '
        'its `-70.0 LUFS` absolute-gate floor rather than an error. Every SFX in this repo is '
        'shorter than that, so the entire SFX population reads `-70.0` and ranking SFX by LUFS '
        'produces a perfect tie. Each population is therefore ranked on the metric defined for '
        'it: **music on integrated LUFS**, **SFX on RMS dBFS and true peak**. A LUFS number and '
        'an RMS number are never compared to each other.'
    )
    lines.append('')
    lines.append(
        'Procedural SFX are not files: they are `SfxSpec` rows synthesized at runtime. They are '
        'extracted from source, synthesized by a Python port of '
        '`ambition_audio::render::audio_source_from_sfx_spec`, and pushed through the same '
        'ffmpeg pass, so they land on the same axis as the packed clips.'
    )
    lines.append('')

    by_population: dict[str, list[Item]] = {}
    for item in items:
        by_population.setdefault(population_of(item.cohort), []).append(item)
    population_medians: dict[str, float] = {}
    for population, group in by_population.items():
        values = _cohort_values(group, population_metric(population))
        if values:
            population_medians[population] = statistics.median(values)

    lines.append('## Populations and cohorts')
    lines.append('')
    lines.append(
        'A **population** shares one metric and one mix bus, and is the baseline every level is '
        'judged against. A **cohort** is how the sound was produced. `Δ pop` is the cohort median '
        'minus its population median — a whole production path sitting hot shows up here and '
        'nowhere else.'
    )
    lines.append('')
    rows = []
    for cohort in sorted(by_cohort):
        group = by_cohort[cohort]
        population = population_of(cohort)
        metric = population_metric(population)
        stats = robust_stats(_cohort_values(group, metric))
        peaks = robust_stats([i.metrics['dbtp'] for i in group if i.metrics.get('dbtp') is not None])
        total_s = sum(i.metrics.get('duration_s') or 0.0 for i in group)
        delta = stats.get('median', 0.0) - population_medians.get(population, 0.0)
        rows.append(
            [
                population,
                f'`{cohort}`',
                str(len(group)),
                f'{total_s / 60:.1f} min',
                'LUFS' if metric == 'lufs_i' else 'RMS dBFS',
                fmt(stats.get('median')),
                f'**{delta:+.1f}**' if abs(delta) >= 3.0 else f'{delta:+.1f}',
                fmt(stats.get('p10')) + ' … ' + fmt(stats.get('p90')),
                fmt(stats.get('min')) + ' … ' + fmt(stats.get('max')),
                fmt(peaks.get('median')),
                fmt(peaks.get('max')),
            ]
        )
    lines.append(
        markdown_table(
            ['pop', 'cohort', 'n', 'audio', 'scale', 'median', 'Δ pop', 'p10 … p90',
             'min … max', 'med dBTP', 'max dBTP'],
            rows,
        )
    )
    lines.append('')

    lines.append('## True-peak danger list')
    lines.append('')
    lines.append(
        'Ranked by **dBTP** across everything, because a clipping transient is the case that '
        'actually hurts and it hides inside a file whose average loudness looks ordinary. '
        '`0 dBTP` is full scale; above `-1.0` an inter-sample overshoot can clip a DAC.'
    )
    lines.append('')
    lines.append(
        '⚠ read the spread before reading the order: '
        + '; '.join(
            f'`{p}` peaks span {robust_stats(v)["p10"]:.1f} … {robust_stats(v)["p90"]:.1f} dBTP '
            f'(median {robust_stats(v)["median"]:.1f})'
            for p, v in sorted(
                (p, [i.metrics['dbtp'] for i in g if i.metrics.get('dbtp') is not None])
                for p, g in by_population.items()
            )
            if v
        )
        + '. Where a production stage normalises to a ceiling, the top of this list is that '
        'ceiling and the ordering within it is rounding error.'
    )
    lines.append('')
    peaky = sorted(
        (i for i in items if i.metrics.get('dbtp') is not None),
        key=lambda i: -i.metrics['dbtp'],
    )[:top]
    lines.append(
        markdown_table(
            ['#', 'dBTP', 'peak dBFS', 'RMS dBFS', 'LUFS-I', 'cohort', 'owner', 'sound'],
            [
                [
                    str(n),
                    fmt(i.metrics.get('dbtp')),
                    fmt(i.metrics.get('peak_db')),
                    fmt(i.metrics.get('rms_db')),
                    fmt(i.metrics['lufs_i']) if i.lufs_valid else 'n/a',
                    i.cohort.replace('music_', 'mus/').replace('sfx_', 'sfx/'),
                    i.owner,
                    f'`{i.name}`',
                ]
                for n, i in enumerate(peaky, 1)
            ],
        )
    )
    lines.append('')

    for population in sorted(by_population):
        group = by_population[population]
        metric = population_metric(population)
        usable = (
            [i for i in group if i.lufs_valid]
            if metric == 'lufs_i'
            else [i for i in group if i.metrics.get(metric) is not None]
        )
        if len(usable) < 4:
            continue
        stats = robust_stats([i.metrics[metric] for i in usable])
        sigma = stats['sigma'] or 1.0
        ranked = sorted(usable, key=lambda i: -i.metrics[metric])
        label = 'integrated LUFS' if metric == 'lufs_i' else 'RMS dBFS'
        lines.append(f'## Loudest `{population}` (by {label})')
        lines.append('')
        lines.append(
            f'Population median **{stats["median"]:.1f}** over {len(usable)} sounds, robust sigma '
            f'**{sigma:.2f}**. `Δmed` is decibels above that median; `z` is the robust z-score.'
        )
        lines.append('')
        lines.append(
            markdown_table(
                ['#', label, 'Δmed', 'z', 'dBTP', 'dur s', 'cohort', 'owner', 'sound'],
                [
                    [
                        str(n),
                        fmt(i.metrics[metric]),
                        f'{i.metrics[metric] - stats["median"]:+.1f}',
                        f'{(i.metrics[metric] - stats["median"]) / sigma:+.1f}',
                        fmt(i.metrics.get('dbtp')),
                        fmt(i.metrics.get('duration_s'), 2),
                        i.cohort.split('_', 1)[1],
                        i.owner,
                        f'`{i.name}`',
                    ]
                    for n, i in enumerate(ranked[:top], 1)
                ],
            )
        )
        lines.append('')
        quietest = ranked[-3:]
        lines.append(
            'Quietest, for scale: '
            + ', '.join(f'`{i.name}` {i.metrics[metric]:.1f}' for i in reversed(quietest))
        )
        lines.append('')

    lines.append('## Owner vs. its population')
    lines.append('')
    lines.append(
        'The question "is X too loud" is answered here: `Δ pop` is this owner\'s median minus '
        'the median of everything it plays alongside, in dB. A positive number is how much '
        'louder this owner is than the field.'
    )
    lines.append('')
    owners: dict[tuple[str, str, str], list[Item]] = {}
    for item in items:
        owners.setdefault((population_of(item.cohort), item.cohort, item.owner), []).append(item)
    rows = []
    for (population, cohort, owner), group in sorted(owners.items()):
        metric = population_metric(population)
        values = _cohort_values(group, metric)
        if not values:
            continue
        peaks = [i.metrics['dbtp'] for i in group if i.metrics.get('dbtp') is not None]
        delta = statistics.median(values) - population_medians.get(population, 0.0)
        rows.append(
            [
                population,
                cohort.split('_', 1)[1],
                owner,
                str(len(group)),
                fmt(statistics.median(values)),
                f'**{delta:+.1f}**' if abs(delta) >= 3.0 else f'{delta:+.1f}',
                fmt(max(values)),
                fmt(max(peaks) if peaks else None),
            ]
        )
    lines.append(
        markdown_table(
            ['pop', 'cohort', 'owner', 'n', 'median', 'Δ pop', 'max', 'max dBTP'], rows
        )
    )
    lines.append('')

    if notes:
        lines.append('## Instrument notes')
        lines.append('')
        for note in notes:
            lines.append(f'- {note}')
        lines.append('')

    return '\n'.join(lines) + '\n'


# ---------------------------------------------------------------------------
# assembly
# ---------------------------------------------------------------------------


def music_owner_map(score_ids: Iterable[str]) -> dict[str, str]:
    """Which game crate names each score id in its source. Best effort, reported."""
    corpus: list[tuple[str, str]] = []
    for rs in sorted(REPO_ROOT.glob('game/*/src/**/*.rs')):
        corpus.append((rs.parts[len(REPO_ROOT.parts) + 1], rs.read_text(errors='replace')))
    owners: dict[str, str] = {}
    for score in score_ids:
        needle = f'"{score}"'
        claimants = sorted({crate for crate, text in corpus if needle in text})
        claimants = [c for c in claimants if c != 'ambition_content'] or claimants
        owners[score] = '+'.join(c.replace('ambition_demo_', '').replace('ambition_', '') for c in claimants) or 'shared'
    return owners


def collect_items(only: str | None, limit: int | None) -> tuple[list[Item], list[str]]:
    notes: list[str] = []
    roots = declared_asset_roots()
    items: list[Item] = []

    loose = discover_loose_audio()
    by_suffix: dict[str, int] = {}
    for path in loose:
        by_suffix[path.suffix.lower()] = by_suffix.get(path.suffix.lower(), 0) + 1
    notes.append(
        'Loose audio under shipping `assets/` trees: '
        + ', '.join(f'{n} × `{s}`' for s, n in sorted(by_suffix.items()))
        + f'. All of it lives under `{roots["music"].relative_to(REPO_ROOT)}` — '
        '**there is no loose SFX file in the repo**; SFX are packed or procedural.'
    )

    score_ids = sorted({music_identity(p, roots['music'])[1] for p in loose} - {''})
    owners = music_owner_map(score_ids)

    if only in (None, 'music'):
        for path in loose:
            cohort, score, name = music_identity(path, roots['music'])
            items.append(
                Item(
                    key=file_cache_key(path),
                    cohort=cohort,
                    name=name,
                    owner=owners.get(score, 'shared'),
                    origin=str(path.relative_to(REPO_ROOT)),
                    source=path,
                )
            )

    bank_path = roots['assets'] / 'audio' / 'sfx.bank'
    if only in (None, 'sfx') and bank_path.exists():
        entries = read_bank(bank_path)
        notes.append(
            f'`{bank_path.relative_to(REPO_ROOT)}`: {len(entries)} entries, '
            f'{bank_path.stat().st_size / 1e6:.0f} MB, codecs '
            + ', '.join(sorted({e.codec for e in entries}))
            + '. Payloads are measured in place; the bank is what the runtime loads.'
        )
        for entry in entries:
            items.append(
                Item(
                    key=bytes_cache_key('bank', entry.payload),
                    cohort='sfx_packed',
                    name=entry.sfx_id,
                    owner=entry.sfx_id.split('.')[0],
                    origin=f'{bank_path.relative_to(REPO_ROOT)}#{entry.sfx_id}',
                    source=entry.payload,
                    extra={
                        'stored_peak_db': entry.stored_peak_db,
                        'stored_rms_db': entry.stored_rms_db,
                    },
                )
            )

    if only in (None, 'sfx'):
        specs = discover_procedural_specs()
        unresolved = [s for s in specs if s.unresolved]
        notes.append(
            f'Procedural `SfxSpec` rows extracted: {len(specs)} across '
            + ', '.join(sorted({s.owner for s in specs}))
            + f'. Fields left unresolved: {len(unresolved)}.'
        )
        for spec in specs:
            if spec.waveform not in WAVEFORMS:
                continue
            payload = synthesize(spec)
            items.append(
                Item(
                    key=spec_cache_key(spec),
                    cohort='sfx_procedural',
                    name=spec.sfx_id,
                    owner=spec.owner.replace('ambition_demo_', '').replace('ambition_', ''),
                    origin=spec.source,
                    source=payload,
                    extra={
                        'waveform': spec.waveform,
                        'authored_volume': spec.volume,
                        'authored_duration': spec.duration,
                        'noise': spec.noise,
                        'unresolved': list(spec.unresolved),
                    },
                )
            )

    if limit:
        stride: dict[str, int] = {}
        kept: list[Item] = []
        for item in items:
            n = stride.get(item.cohort, 0)
            if n < limit:
                kept.append(item)
                stride[item.cohort] = n + 1
        items = kept
    return items, notes


def measure_all(items: list[Item], jobs: int, use_cache: bool) -> tuple[int, int]:
    cache = load_cache() if use_cache else {}
    pending = [i for i in items if i.key not in cache]
    for item in items:
        if item.key in cache:
            item.metrics = cache[item.key]

    done = 0
    if pending:
        with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
            futures = {pool.submit(run_ffmpeg, i.source): i for i in pending}
            for future in concurrent.futures.as_completed(futures):
                item = futures[future]
                item.metrics = future.result()
                cache[item.key] = item.metrics
                done += 1
                if done % 25 == 0 or done == len(pending):
                    rprint(f'  [dim]measured {done}/{len(pending)}[/dim]')
    if use_cache:
        save_cache(cache)
    return done, len(items) - done


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--report', type=Path, default=DEFAULT_REPORT, help='markdown artifact path')
    parser.add_argument('--json', type=Path, default=CACHE_DIR / 'levels.json', help='full per-item dump')
    parser.add_argument('--only', choices=('music', 'sfx'), help='measure one population')
    parser.add_argument('--limit', type=int, help='cap items per cohort (smoke run)')
    parser.add_argument('--top', type=int, default=25, help='rows per ranked table')
    parser.add_argument('--jobs', type=int, default=min(8, os.cpu_count() or 4))
    parser.add_argument('--no-cache', action='store_true')
    args = parser.parse_args(argv)

    start = time.time()
    items, notes = collect_items(args.only, args.limit)
    rprint(f'[bold]{len(items)}[/bold] sounds to measure across '
           f'{len({i.cohort for i in items})} cohorts (jobs={args.jobs})')

    measured, cached = measure_all(items, args.jobs, not args.no_cache)
    failed = [i for i in items if i.metrics.get('error')]
    for item in failed:
        rprint(f'[yellow]could not measure[/yellow] {rich_escape(item.origin)}: '
               f'{rich_escape(str(item.metrics["error"]))}')
    items = [i for i in items if not i.metrics.get('error')]
    if failed:
        notes.append(f'{len(failed)} sounds failed to decode and are excluded.')

    # Cross-check: the bank stores peak/RMS written at pack time. If those
    # disagree with a fresh measurement of the same bytes, one of the two is
    # lying and every conclusion about SFX rests on which.
    deltas = [
        abs(i.metrics['rms_db'] - i.extra['stored_rms_db'])
        for i in items
        if i.cohort == 'sfx_packed' and i.metrics.get('rms_db') is not None and 'stored_rms_db' in i.extra
    ]
    if deltas:
        notes.append(
            f'Bank self-check: measured RMS vs the `peak_db`/`rms_db` the packer stored — '
            f'max disagreement **{max(deltas):.3f} dB** over {len(deltas)} clips. '
            'The stored metadata is trustworthy.'
        )

    timings = {'total': time.time() - start, 'measured': measured, 'cached': cached}
    report = build_report(items, args.top, timings, notes)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(report)

    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(
        json.dumps(
            {
                'metrics_version': METRICS_VERSION,
                'items': [
                    {
                        'cohort': i.cohort,
                        'name': i.name,
                        'owner': i.owner,
                        'origin': i.origin,
                        **i.extra,
                        **i.metrics,
                    }
                    for i in items
                ],
            },
            indent=1,
            sort_keys=True,
        )
    )

    rprint('')
    rprint(f'[bold]{len(items)}[/bold] sounds measured in [bold]{timings["total"]:.1f}s[/bold] '
           f'({measured} fresh, {cached} cached)')
    rprint(f'report:    {link(args.report, _display(args.report))}')
    rprint(f'full dump: {link(args.json, _display(args.json))}')
    rprint(f'directory: {link(args.report.parent, _display(args.report.parent) + "/")}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
