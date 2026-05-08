#!/usr/bin/env python3
"""Pack the SFX renderer's per-id output directory into a single .sfxbank file.

Reads ``<input_dir>/<id>/<id>.render.json`` for each entry, locates the
audio payload via the JSON's ``outputs`` map, and emits a binary bank
file consumable by the ``ambition_sfx_bank`` Rust crate.

Format spec (v1, little-endian):

    Header (40 bytes):
        magic           [u8; 8]   = b"AMBNDSFX"
        version         u32       = 1
        entry_count     u32
        entries_offset  u64       (start of entry table)
        payloads_offset u64       (start of payload region)
        names_offset    u64       (start of names section)

    Entry table (entry_count * 64 bytes, sorted ascending by id_hash):
        id_hash         u64       (FNV-1a 64 of the dot-separated id)
        offset          u64       (from start of file)
        length          u32       (payload bytes)
        codec           u8        (0=Wav, 1=Ogg, 2=Flac reserved)
        channels        u8
        _pad0           u16
        sample_rate     u32
        duration_ms     u32
        default_gain_db f32
        peak_db         f32
        rms_db          f32
        flags           u32       (bit0=streamable_hint, bit1=looping)
        _reserved       [u8; 16]

    Payloads: concatenated, in entry order, no padding required.

    Names section (debug; runtime may skip):
        per entry, in id_hash order:
            len: u16, bytes: [u8; len]   (UTF-8, the dot-separated id)

The format is designed to be mmap-friendly: fixed-size header, fixed-size
entry records, sorted by hash for binary search.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

MAGIC = b"AMBNDSFX"
VERSION = 1
HEADER_SIZE = 40
ENTRY_SIZE = 64

CODEC_WAV = 0
CODEC_OGG = 1
CODEC_FLAC = 2

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
U64_MASK = (1 << 64) - 1

HEADER_FMT = "<8sIIQQQ"
ENTRY_FMT = "<QQIBBHIIfffI16x"
assert struct.calcsize(HEADER_FMT) == HEADER_SIZE
assert struct.calcsize(ENTRY_FMT) == ENTRY_SIZE


def fnv1a_64(s: str) -> int:
    h = FNV_OFFSET
    for b in s.encode("utf-8"):
        h = ((h ^ b) * FNV_PRIME) & U64_MASK
    return h


@dataclass
class Entry:
    id: str
    id_hash: int
    payload: bytes
    codec: int
    channels: int
    sample_rate: int
    duration_ms: int
    default_gain_db: float
    peak_db: float
    rms_db: float
    flags: int


def codec_for(resolved_format_policy: str, output_keys: Iterable[str]) -> int:
    if resolved_format_policy:
        if resolved_format_policy.endswith(":wav"):
            return CODEC_WAV
        if resolved_format_policy.endswith(":ogg"):
            return CODEC_OGG
        if resolved_format_policy.endswith(":flac"):
            return CODEC_FLAC
    keys = set(output_keys)
    if "wav" in keys:
        return CODEC_WAV
    if "ogg" in keys:
        return CODEC_OGG
    if "flac" in keys:
        return CODEC_FLAC
    raise ValueError(f"could not determine codec from policy={resolved_format_policy!r} keys={keys}")


def collect_entries(input_dir: Path) -> list[Entry]:
    entries: list[Entry] = []
    for child in sorted(input_dir.iterdir()):
        if not child.is_dir():
            continue
        render_json = child / f"{child.name}.render.json"
        if not render_json.exists():
            print(f"  skip {child.name}: no render.json", file=sys.stderr)
            continue
        meta = json.loads(render_json.read_text())
        sfx_id = meta["id"]
        if meta.get("skipped"):
            print(f"  skip {sfx_id}: marked skipped in render.json", file=sys.stderr)
            continue

        outputs = meta.get("outputs", {}) or {}
        codec = codec_for(meta.get("resolved_format_policy", ""), outputs.keys())
        codec_key = {CODEC_WAV: "wav", CODEC_OGG: "ogg", CODEC_FLAC: "flac"}[codec]
        payload_path_str = outputs.get(codec_key)
        if not payload_path_str:
            print(f"  skip {sfx_id}: no payload for codec={codec_key}", file=sys.stderr)
            continue
        # Renderer writes absolute paths into render.json. Tolerate that as
        # well as relative paths by trying both.
        candidate = Path(payload_path_str)
        if not candidate.is_absolute():
            candidate = (render_json.parent / payload_path_str).resolve()
        if not candidate.exists():
            local = render_json.parent / Path(payload_path_str).name
            if local.exists():
                candidate = local
            else:
                print(f"  skip {sfx_id}: payload missing at {candidate}", file=sys.stderr)
                continue

        payload = candidate.read_bytes()
        duration_seconds = float(meta.get("duration_seconds", 0.0))
        duration_ms = int(round(duration_seconds * 1000.0))
        peak_db = float(meta.get("peak_db", 0.0))
        rms_db = float(meta.get("rms_db", 0.0))
        # default_gain_db: not set by renderer today; reserve at 0 dB and let
        # callers override via spec/manifest layers in the future.
        default_gain_db = 0.0
        channels = int(meta.get("channels", 2))
        sample_rate = int(meta.get("sample_rate", 48000))
        # flags: streamable_hint when the renderer chose ogg (longer clips).
        flags = 0
        if codec == CODEC_OGG:
            flags |= 0b0000_0001

        entries.append(
            Entry(
                id=sfx_id,
                id_hash=fnv1a_64(sfx_id),
                payload=payload,
                codec=codec,
                channels=channels,
                sample_rate=sample_rate,
                duration_ms=duration_ms,
                default_gain_db=default_gain_db,
                peak_db=peak_db,
                rms_db=rms_db,
                flags=flags,
            )
        )
    return entries


def assert_no_collisions(entries: list[Entry]) -> None:
    seen: dict[int, str] = {}
    for entry in entries:
        prior = seen.get(entry.id_hash)
        if prior is not None and prior != entry.id:
            raise SystemExit(
                f"FNV-1a 64 collision between {prior!r} and {entry.id!r} "
                f"(hash=0x{entry.id_hash:016x}). Pick a different id."
            )
        seen[entry.id_hash] = entry.id


def write_bank(out_path: Path, entries: list[Entry]) -> None:
    entries = sorted(entries, key=lambda e: e.id_hash)
    entry_count = len(entries)
    entries_offset = HEADER_SIZE
    payloads_offset = entries_offset + entry_count * ENTRY_SIZE

    cursor = payloads_offset
    payload_offsets: list[int] = []
    for entry in entries:
        payload_offsets.append(cursor)
        cursor += len(entry.payload)
    names_offset = cursor

    header = struct.pack(
        HEADER_FMT,
        MAGIC,
        VERSION,
        entry_count,
        entries_offset,
        payloads_offset,
        names_offset,
    )

    entry_records = bytearray()
    for entry, offset in zip(entries, payload_offsets):
        entry_records += struct.pack(
            ENTRY_FMT,
            entry.id_hash,
            offset,
            len(entry.payload),
            entry.codec,
            entry.channels,
            0,  # _pad0
            entry.sample_rate,
            entry.duration_ms,
            entry.default_gain_db,
            entry.peak_db,
            entry.rms_db,
            entry.flags,
        )

    payload_blob = b"".join(entry.payload for entry in entries)

    names_blob = bytearray()
    for entry in entries:
        encoded = entry.id.encode("utf-8")
        if len(encoded) > 0xFFFF:
            raise SystemExit(f"id too long for u16 length prefix: {entry.id!r}")
        names_blob += struct.pack("<H", len(encoded))
        names_blob += encoded

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("wb") as fh:
        fh.write(header)
        fh.write(entry_records)
        fh.write(payload_blob)
        fh.write(names_blob)


def write_dump(dump_path: Path, entries: list[Entry], bank_path: Path) -> None:
    """Write a human-readable, diff-friendly summary alongside the bank."""
    entries = sorted(entries, key=lambda e: e.id)
    lines = [
        f"# {bank_path.name} dump",
        f"# version={VERSION} entries={len(entries)}",
        "#",
        "# id_hash            codec channels  sr  ms     bytes  peak_db  rms_db  flags  id",
    ]
    for entry in entries:
        codec_name = {CODEC_WAV: "wav", CODEC_OGG: "ogg", CODEC_FLAC: "flac"}[entry.codec]
        lines.append(
            f"0x{entry.id_hash:016x} {codec_name:>5} {entry.channels:>8} "
            f"{entry.sample_rate:>5} {entry.duration_ms:>5} "
            f"{len(entry.payload):>8} "
            f"{entry.peak_db:>7.2f} {entry.rms_db:>7.2f} "
            f"0x{entry.flags:04x}  {entry.id}"
        )
    dump_path.write_text("\n".join(lines) + "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    here = Path(__file__).resolve().parent
    repo_root = here.parent.parent
    default_input = repo_root / "tools" / "ambition_sfx_renderer" / "output"
    default_output = (
        repo_root / "crates" / "ambition_sandbox" / "assets" / "audio" / "sfx.bank"
    )
    parser.add_argument(
        "--input", type=Path, default=default_input, help=f"renderer output dir (default: {default_input})"
    )
    parser.add_argument(
        "--output", type=Path, default=default_output, help=f"bank file path (default: {default_output})"
    )
    parser.add_argument(
        "--dump", action="store_true", help="also emit a human-readable .txt sibling file"
    )
    args = parser.parse_args(argv)

    if not args.input.is_dir():
        parser.error(f"input dir not found: {args.input}")

    entries = collect_entries(args.input)
    if not entries:
        parser.error(f"no entries found under {args.input}")
    assert_no_collisions(entries)

    write_bank(args.output, entries)
    print(
        f"wrote {len(entries)} entries to {args.output} "
        f"({args.output.stat().st_size:,} bytes)"
    )
    if args.dump:
        dump_path = args.output.with_suffix(args.output.suffix + ".txt")
        write_dump(dump_path, entries, args.output)
        print(f"wrote dump to {dump_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
