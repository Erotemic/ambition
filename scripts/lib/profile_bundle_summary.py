#!/usr/bin/env python3
"""Write `summary.md`, the front page of a profiling bundle.

Reports MEASUREMENTS. Where a diagnostic is absent it says which of three
things happened, because those three call for different next steps:

* **measured** — the number is here;
* **unavailable on this machine/backend** — the tool or the adapter could not
  produce it, and a different machine would;
* **not applicable** — this was a headless/CPU-only run and the measurement is
  about a GPU that was never asked to draw.

An omitted section reads as "free", which is the one conclusion the absence of
a measurement can never support.
"""

from __future__ import annotations

import csv
import json
import os
import re
import sys

ADAPTER = re.compile(r"AdapterInfo \{[^}]*\}")
PERCENT = re.compile(r"\s*[0-9]+(\.[0-9]+)?%")
SOFTWARE_MARKERS = ("device_type: Cpu", "llvmpipe", "lavapipe", "swiftshader", "softpipe")

DSO_BUCKETS = [
    ("software rasterizer (CPU emulating a GPU)",
     ("llvmpipe", "lvp", "lavapipe", "swiftshader", "softpipe", "[JIT]")),
    ("GPU driver / graphics stack",
     ("nvidia", "radeonsi", "iris", "amdgpu", "i965", "libvulkan", "libGLX", "libEGL", "libdrm", "zink")),
    ("kernel", ("[kernel", "[kvm", "[nvidia_uvm", "[snd", "[vdso")),
    ("audio", ("pipewire", "pulse", "alsa", "libspa")),
]


class Bundle:
    def __init__(self, path: str) -> None:
        self.path = path

    def read(self, name: str) -> str:
        try:
            with open(os.path.join(self.path, name), encoding="utf-8", errors="replace") as handle:
                return handle.read()
        except OSError:
            return ""

    def rows(self, name: str) -> list[dict]:
        path = os.path.join(self.path, name)
        if not os.path.exists(path):
            return []
        with open(path, newline="", encoding="utf-8", errors="replace") as handle:
            return list(csv.DictReader(handle))

    def exists(self, name: str) -> bool:
        return os.path.exists(os.path.join(self.path, name))

    def metadata(self) -> dict:
        text = self.read("metadata.json")
        if text:
            try:
                return json.loads(text)
            except json.JSONDecodeError:
                pass
        data = {}
        for line in self.read("metadata.txt").splitlines():
            key, sep, value = line.partition("=")
            if sep:
                data[key] = value
        return data

    def status(self, name: str) -> str:
        return self.read(f"{name}.status").strip() or "not run"


def number(row: dict, key: str, default: float = 0.0) -> float:
    try:
        return float(row.get(key, default))
    except (TypeError, ValueError):
        return default


def section(lines: list[str], title: str) -> None:
    lines += [f"## {title}", ""]


def build_summary(bundle: Bundle) -> str:
    meta = bundle.metadata()
    lines: list[str] = ["# Profiling bundle", ""]

    # `all-run` writes one sub-bundle per mode and this top level holds only
    # the shared metadata; say so instead of reporting a page of UNAVAILABLE.
    children = sorted(
        name
        for name in os.listdir(bundle.path)
        if os.path.isdir(os.path.join(bundle.path, name))
        and os.path.exists(os.path.join(bundle.path, name, "summary.md"))
    )
    if children:
        lines += [
            "This run captured several modes. Each has its own complete bundle:",
            "",
        ]
        lines += [f"- [`{name}/summary.md`]({name}/summary.md)" for name in children]
        lines += [
            "",
            "The sections below describe only what this top level holds — the shared",
            "build, host, and capture settings.",
            "",
        ]

    headless = meta.get("headless") == "yes"
    log = bundle.read("game-stderr-stamped.txt") or bundle.read("game-stdout-stamped.txt")
    adapter_match = ADAPTER.search(log) or ADAPTER.search(bundle.read("perf-record.stderr"))
    adapter = adapter_match.group(0) if adapter_match else ""
    software = any(marker in adapter for marker in SOFTWARE_MARKERS)

    # ── What was measured ────────────────────────────────────────────────
    section(lines, "What this measured")
    lines += [
        "| fact | value |",
        "| --- | --- |",
        f"| git commit | `{meta.get('git_head_short', '?')}` on `{meta.get('git_branch', '?')}` |",
        f"| working tree | {'clean' if meta.get('git_clean', True) else 'DIRTY — the binary is not this commit alone'} |",
        f"| cargo profile | `{meta.get('cargo_profile', '?')}` (`target/{meta.get('profile_dir', '?')}`) |",
        f"| cargo features | `{meta.get('cargo_features') or '<none>'}` |",
        f"| executable | `{meta.get('binary_path', '?')}` |",
        f"| package / bin | `{meta.get('package', '?')}` / `{meta.get('binary', '?')}` |",
        f"| rust target | `{meta.get('rust_target', '?')}` |",
        f"| rustc | `{meta.get('rustc_version', '?')}` |",
        f"| capture mode | `{meta.get('mode', '?')}` |",
        f"| run command | `{meta.get('run_command', '?')}` |",
        f"| host | `{meta.get('hostname', '?')}` |",
        f"| kernel | `{meta.get('uname', '?')}` |",
        f"| workload census | {'on at ' + meta.get('census_hz', '?') + ' Hz' if meta.get('census_enabled') == 'yes' else 'OFF (--no-census)'} |",
        f"| headless | {'yes, ' + meta.get('headless_ticks', '?') + ' ticks' if headless else 'no'} |",
        f"| scenario | `{meta.get('headless_scenario', 'n/a')}` |",
        "",
    ]
    if meta.get("cargo_profile") == "dev":
        lines += [
            "> **This is the DEVELOPMENT build.** Workspace crates are built at low",
            "> optimization on purpose (see `[profile.dev]` in Cargo.toml), so these",
            "> numbers answer *why is my edit/play build slow*, NOT *why is the",
            "> optimized runtime slow*. For the second question re-run without",
            "> `--dev-build`.",
            "",
        ]
    elif meta.get("cargo_profile") == "profiling":
        lines += [
            "Release-level optimization with symbols and line tables kept, so this",
            "is representative of shipped runtime performance and still attributable.",
            "",
        ]

    for line in bundle.read("host-environment.txt").splitlines():
        if line.startswith("model name") or line.startswith("logical_cpus") or line.startswith("MemTotal"):
            lines.append(f"- {line.strip()}")
    lines.append("")

    # ── Renderer ─────────────────────────────────────────────────────────
    section(lines, "Renderer")
    if headless:
        lines += [
            "**NOT APPLICABLE — headless run.** The game ran its supported headless",
            "path (`--headless`), which selects `backends: None`: no window, no wgpu",
            "adapter, and therefore no render app and no GPU work at all. The",
            "presentation composition itself may still exist — where camera and view",
            "rows appear below, they are real and they are what a windowed run would",
            "have drawn — but nothing here measures drawing.",
            "",
            "Every GPU and render-pass measurement below is marked not-applicable",
            "rather than missing. This run is not evidence that rendering is cheap.",
            "",
        ]
    elif adapter:
        lines += ["```text", adapter, "```", ""]
        if software:
            lines += [
                "**SOFTWARE RENDERING — READ THIS BEFORE THE NUMBERS BELOW.**",
                "",
                "This run had no GPU: every pixel was rasterized on the CPU. Expect the",
                "bulk of samples in llvmpipe/lavapipe threads and in unsymbolized `[JIT]`",
                "frames, which are the rasterizer's runtime-compiled shaders -- perf can",
                "never attribute those to a pass, a material, or a draw call. Game-code",
                "symbols in this profile describe only the few percent left over.",
                "",
                "This is NOT a measurement of GPU rendering performance, and it must not",
                "be reported as one. Check `host-environment.txt` for why no GPU adapter",
                "was selected (missing ICD, `VK_*`/`WGPU_*` override, no DRM render node,",
                "headless session).",
                "",
            ]
        else:
            lines += ["Hardware rendering was available and used.", ""]
    else:
        lines += [
            "UNAVAILABLE — no `AdapterInfo` line was found in the captured logs, so",
            "this bundle cannot say which adapter drew. See `host-environment.txt`.",
            "",
        ]

    # ── Session and frame time ───────────────────────────────────────────
    stamps = [float(m.group(1)) for m in re.finditer(r"^\[\s*([0-9.]+)s\]", log, re.M)]
    duration = max(stamps) if stamps else 0.0
    section(lines, "Session")
    if duration:
        lines.append(f"Observed span of the game's own log: **{duration:.1f}s**.")
    else:
        lines.append("No stamped game log in this bundle (attach modes do not capture stdio).")
    lines.append("")

    frames = bundle.rows("frame_times.csv")
    spikes = bundle.rows("frame_spikes.csv")
    windows = bundle.rows("frame_windows.csv")
    section(lines, "Frame time")
    if frames:
        worst = sorted(frames, key=lambda row: -number(row, "max"))[:8]
        lines += [
            f"{len(frames)} census windows at {meta.get('census_hz', '?')} Hz. Worst windows by max frame:",
            "",
            "```text",
            f'{"t":>9} {"frames":>7} {"mean":>7} {"p50":>7} {"p95":>7} {"p99":>7} {"max":>8}',
        ]
        for row in worst:
            lines.append(
                f"{number(row, 't'):9.1f} {number(row, 'frames'):7.0f} {number(row, 'mean'):7.1f} "
                f"{number(row, 'p50'):7.1f} {number(row, 'p95'):7.1f} {number(row, 'p99'):7.1f} "
                f"{number(row, 'max'):8.1f}"
            )
        lines += ["```", "", "Full series: `frame_times.csv`.", ""]
    elif windows:
        lines += [
            "The 1 Hz census series is absent; the always-on 5s windows are in",
            "`frame_windows.csv`.",
            "",
        ]
    else:
        lines += ["UNAVAILABLE — no frame-time rows were captured.", ""]

    if spikes:
        ranked = sorted(spikes, key=lambda row: -number(row, "frame_ms"))[:10]
        lines += [
            f"{len(spikes)} frames over the 33.4ms spike threshold. Worst, with the",
            "wall-clock second to look up in the other CSVs:",
            "",
            "```text",
        ]
        for row in ranked:
            lines.append(f"{number(row, 'wall_s'):9.3f}s  {number(row, 'frame_ms'):8.1f} ms")
        lines += ["```", "", "Full list: `frame_spikes.csv`.", ""]
    else:
        lines += ["No frames crossed the 33.4ms spike threshold.", ""]

    # ── Views and cameras ────────────────────────────────────────────────
    section(lines, "Cameras and views")
    view_totals = bundle.rows("view_totals.csv")
    cameras = bundle.rows("camera_views.csv")
    if view_totals:
        peak = max(view_totals, key=lambda row: number(row, "world_rendering"))
        lines += [
            "```text",
            f'{"t":>9} {"cameras":>8} {"active":>7} {"world":>6} {"offscr":>7} {"views":>6}',
        ]
        for row in view_totals[:: max(1, len(view_totals) // 20)]:
            lines.append(
                f"{number(row, 't'):9.1f} {number(row, 'cameras'):8.0f} {number(row, 'active'):7.0f} "
                f"{number(row, 'world_rendering'):6.0f} {number(row, 'offscreen'):7.0f} "
                f"{number(row, 'local_views'):6.0f}"
            )
        lines += ["```", ""]
        lines += [
            f"Peak world-rendering cameras: **{number(peak, 'world_rendering'):.0f}** at "
            f"t={number(peak, 't'):.1f}s. Each one draws the world again.",
            "",
        ]
        if cameras:
            roles: dict[str, set[str]] = {}
            for row in cameras:
                roles.setdefault(row.get("role", "?"), set()).add(row.get("name", "?"))
            lines += ["Distinct cameras seen, by role:", "", "```text"]
            for role, names in sorted(roles.items()):
                lines.append(f"{role:>16}  {', '.join(sorted(names))[:100]}")
            lines += ["```", "", "Per-sample rows: `camera_views.csv`.", ""]
    elif meta.get("census_enabled") != "yes":
        lines += ["OFF — the workload census was disabled with `--no-census`.", ""]
    elif headless:
        lines += ["NOT APPLICABLE — a headless run composes no cameras.", ""]
    else:
        lines += ["UNAVAILABLE — no camera census rows reached this bundle.", ""]

    # ── Portals / offscreen ──────────────────────────────────────────────
    section(lines, "Portal and offscreen workload")
    portals = bundle.rows("portal_activity.csv")
    targets = bundle.rows("render_target_census.csv")
    if portals:
        peak = max(portals, key=lambda row: number(row, "active"))
        lines += [
            f"Peak active portal capture rigs: **{number(peak, 'active'):.0f}** of "
            f"{number(peak, 'rigs'):.0f} at t={number(peak, 't'):.1f}s.",
            "",
            "```text",
            f'{"t":>9} {"rigs":>5} {"active":>7}  budget',
        ]
        for row in portals[:: max(1, len(portals) // 12)]:
            budget = (
                f"res<={row.get('max_resolution', '?')} depth={row.get('recursion_depth', '?')} "
                f"captures<={row.get('max_active_captures', '?')} "
                f"updates/frame<={row.get('max_updates_per_frame', '?')}"
            )
            lines.append(
                f"{number(row, 't'):9.1f} {number(row, 'rigs'):5.0f} {number(row, 'active'):7.0f}  {budget}"
            )
        lines += ["```", "", "Full series: `portal_activity.csv`.", ""]
    elif headless:
        lines += ["NOT APPLICABLE — headless runs compose no portal capture rigs.", ""]
    elif meta.get("census_enabled") != "yes":
        lines += ["OFF — the workload census was disabled with `--no-census`.", ""]
    else:
        lines += ["No portal capture rigs were reported in this run.", ""]

    if targets:
        peak = max(targets, key=lambda row: number(row, "image_targets"))
        lines += [
            f"Peak offscreen image render targets: **{number(peak, 'image_targets'):.0f}** "
            f"(largest dimension {number(peak, 'largest_dim'):.0f}px) at t={number(peak, 't'):.1f}s.",
            "",
            "Full series: `render_target_census.csv`.",
            "",
        ]

    # ── Scene scale ──────────────────────────────────────────────────────
    section(lines, "Scene and ECS workload")
    ecs = bundle.rows("runtime_census.csv")
    draws = bundle.rows("draw_census.csv")
    schedules = bundle.rows("schedule_census.csv")
    if ecs:
        first, last = ecs[0], ecs[-1]
        lines += [
            "```text",
            f'{"":>10} {"entities":>10} {"archetypes":>11} {"bodies":>8} {"players":>8}',
            f'{"start":>10} {number(first, "entities"):10.0f} {number(first, "archetypes"):11.0f} '
            f'{number(first, "bodies"):8.0f} {number(first, "players"):8.0f}',
            f'{"end":>10} {number(last, "entities"):10.0f} {number(last, "archetypes"):11.0f} '
            f'{number(last, "bodies"):8.0f} {number(last, "players"):8.0f}',
            f'{"peak":>10} {max(number(r, "entities") for r in ecs):10.0f} '
            f'{max(number(r, "archetypes") for r in ecs):11.0f} '
            f'{max(number(r, "bodies") for r in ecs):8.0f} '
            f'{max(number(r, "players") for r in ecs):8.0f}',
            "```",
            "",
        ]
        growth = number(last, "entities") - number(first, "entities")
        if growth > 0:
            lines += [
                f"Entity count rose by {growth:.0f} over the session. A rise that never",
                "falls across room transitions is the shape of a lifecycle leak; check",
                "`runtime_census.csv` against the room markers in `timeline.md`.",
                "",
            ]
    elif meta.get("census_enabled") != "yes":
        lines += ["OFF — the workload census was disabled with `--no-census`.", ""]
    else:
        lines += ["UNAVAILABLE — no ECS census rows reached this bundle.", ""]

    if headless and ecs and max(number(row, "bodies") for row in ecs) == 0:
        lines += [
            "> **No simulated bodies existed in this run.** The scenario named in the",
            f"> table above (`{meta.get('headless_scenario', 'n/a')}`) never reached a",
            "> state with a body in it, so the numbers here describe composition rather",
            "> than gameplay, and no simulation cost can be read off them. A capture",
            "> that was supposed to exercise the sim and reports this either did not",
            "> run long enough (`--headless-ticks`) or never activated a session.",
            "",
        ]

    if draws:
        peak = max(draws, key=lambda row: number(row, "sprites"))
        lines += [
            f"Peak sprites: **{number(peak, 'sprites'):.0f}** "
            f"({number(peak, 'sprites_visible'):.0f} visible), "
            f"text2d {number(peak, 'text2d'):.0f}, "
            f"per-view projections {number(peak, 'per_view_projections'):.0f} "
            f"at t={number(peak, 't'):.1f}s. Full series: `draw_census.csv`.",
            "",
        ]
    if schedules:
        peak = max(schedules, key=lambda row: number(row, "systems"))
        lines += [
            f"Peak registered systems across visible schedules: **{number(peak, 'systems'):.0f}** "
            f"in {number(peak, 'schedules'):.0f} schedules.",
            "",
        ]

    # ── Render passes ────────────────────────────────────────────────────
    section(lines, "Render passes")
    passes = bundle.rows("render_diagnostics.csv")
    pass_status = bundle.rows("render_diagnostics_status.csv")
    if headless:
        lines += [
            "NOT APPLICABLE — a headless run has no render app, so there are no",
            "passes to time. This is not a claim that rendering is cheap.",
            "",
        ]
    elif passes:
        totals: dict[str, list[float]] = {}
        for row in passes:
            path = row.get("path", "")
            try:
                value = float(row.get("value", "nan"))
            except ValueError:
                continue
            totals.setdefault(path, []).append(value)
        # Times and counts are different units and must not share a ranking:
        # sorted together, a million shader invocations always outranks a 20ms
        # pass and the expensive pass falls off the bottom of the table.
        def emit(out: list[str], title: str, paths: list[str], unit: str) -> None:
            if not paths:
                return
            ranked = sorted(paths, key=lambda path: -(sum(totals[path]) / len(totals[path])))
            out += [title, "", "```text", f'{"mean":>14} {"max":>14} {"samples":>8}  {unit}']
            for path in ranked[:30]:
                values = totals[path]
                out.append(
                    f"{sum(values) / len(values):14.3f} {max(values):14.3f} {len(values):8d}  {path}"
                )
            out += ["```", ""]

        lines += ["Mean and max over the sampled frames, from Bevy's `RenderDiagnosticsPlugin`.", ""]
        emit(
            lines,
            "Pass time, milliseconds:",
            [path for path in totals if path.endswith(("/elapsed_cpu", "/elapsed_gpu"))],
            "diagnostic (ms)",
        )
        emit(
            lines,
            "Pipeline statistics, counts per frame:",
            [path for path in totals if not path.endswith(("/elapsed_cpu", "/elapsed_gpu"))],
            "diagnostic (count)",
        )
        gpu = sum(1 for path in totals if path.endswith("/elapsed_gpu"))
        stats = sum(1 for path in totals if not path.endswith(("/elapsed_cpu", "/elapsed_gpu")))
        lines.append(f"- CPU pass timings: **measured** ({sum(1 for p in totals if p.endswith('/elapsed_cpu'))} spans).")
        if gpu:
            lines.append(f"- GPU pass timings: **measured** ({gpu} spans).")
        else:
            lines.append(
                "- GPU pass timings: **supported by Bevy, unavailable on this adapter/backend.** "
                "Timestamp queries are Vulkan/DX12 only, and a software adapter has none."
            )
        if stats:
            lines.append(f"- Pipeline statistics: **measured** ({stats} diagnostics).")
        else:
            lines.append(
                "- Pipeline statistics (primitive and shader-invocation counts): "
                "**supported by Bevy, unavailable on this adapter/backend.**"
            )
        lines += ["", "Full series: `render_diagnostics.csv`.", ""]
    elif pass_status:
        lines += [
            "UNAVAILABLE — the render diagnostics store held no `render/*` entries.",
            "That happens when the build has no render app or no pass recorded a span.",
            "",
        ]
    else:
        lines += [
            "UNAVAILABLE — no render-pass rows reached this bundle. `RenderDiagnosticsPlugin`",
            "is installed by the presentation census plugin; a bundle with no rows either",
            "ran with `--no-census` or never rendered a frame.",
            "",
        ]

    # ── Bevy systems (Tracy) ─────────────────────────────────────────────
    section(lines, "Bevy systems and zones (Tracy)")
    tracy = bundle.read("tracy_summary.md")
    skipped = bundle.read("tracy.skipped").strip()
    caveat = bundle.read("tracy.caveat").strip()
    if caveat:
        lines += [f"> **Timer caveat.** {caveat}", ""]
    if tracy:
        body = [line for line in tracy.splitlines() if not line.startswith("# ")]
        lines += body[:70]
        lines += ["", "Full report: `tracy_summary.md`. Raw trace: `tracy.trace`.", ""]
    elif skipped:
        lines += [
            "UNAVAILABLE — no Tracy capture:",
            "",
            "```text",
            skipped,
            "```",
            "",
            "Without it there are no per-Bevy-system or per-render-pass zone timings;",
            "`perf` reports native symbols, which cannot be mapped back to a system.",
            "",
        ]
    else:
        lines += ["UNAVAILABLE — no Tracy artifacts in this bundle.", ""]

    # ── Native profile ───────────────────────────────────────────────────
    section(lines, "Where the native time went")
    dso = bundle.read("perf-report-by-dso.txt")
    if dso:
        tally: dict[str, float] = {}
        for line in dso.splitlines():
            match = re.match(r"\s+([0-9.]+)%\s+(\S.*?)\s*$", line)
            if not match:
                continue
            percent, name = float(match.group(1)), match.group(2)
            label = "game binary + its Rust/C deps"
            for bucket, needles in DSO_BUCKETS:
                if any(needle.lower() in name.lower() for needle in needles):
                    label = bucket
                    break
            tally[label] = tally.get(label, 0.0) + percent
        if tally:
            lines += ["```text"]
            for label, percent in sorted(tally.items(), key=lambda item: -item[1]):
                lines.append(f"{percent:6.1f}%  {label}")
            lines += [
                "```",
                "",
                "From `perf-report-by-dso.txt`. If the top bucket is not the game binary,",
                "ranking game symbols is ranking the wrong machine layer.",
                "",
            ]
    report = bundle.read("perf_report.txt")
    if report:
        rows = [line[:200] for line in report.splitlines() if PERCENT.match(line)][:35]
        if rows:
            lines += ["Top native symbols:", "", "```text"] + rows + ["```", ""]
    elif not dso:
        lines += ["UNAVAILABLE — no `perf` report in this bundle.", ""]

    # ── Assets ───────────────────────────────────────────────────────────
    section(lines, "Assets and render resources")
    assets = bundle.rows("asset_activity.csv")
    decodes = bundle.rows("image_decodes.csv")
    if assets:
        first, last = assets[0], assets[-1]
        lines += [
            f"- Decoded images: {number(first, 'decoded_images'):.0f} → "
            f"{number(last, 'decoded_images'):.0f} "
            f"({number(last, 'decoded_megapixels'):.1f} MP, "
            f"{number(last, 'decoded_bytes') / 1e6:.1f} MB of decode work).",
            f"- Images resident at end: {number(last, 'images_resident'):.0f}.",
            "",
            "Decode counts only ever rise. A rise with no new room is the same asset",
            "being decoded again; `image_decodes.csv` names which.",
            "",
        ]
    if decodes:
        seen: dict[str, int] = {}
        for row in decodes:
            seen[row.get("path", "?")] = seen.get(row.get("path", "?"), 0) + 1
        repeats = {path: count for path, count in seen.items() if count > 1}
        if repeats:
            lines += ["Textures decoded more than once:", "", "```text"]
            for path, count in sorted(repeats.items(), key=lambda item: -item[1])[:20]:
                lines.append(f"{count:4d}x  {path}")
            lines += ["```", ""]
        else:
            lines += [f"{len(decodes)} notable texture decodes, none repeated.", ""]
    if not assets and not decodes:
        lines += ["UNAVAILABLE — no asset census rows in this bundle.", ""]

    strace = bundle.read("asset-trace-summary.md")
    if strace:
        lines += ["Repeated on-disk opens (`asset-run`): see `asset-trace-summary.md`.", ""]

    # ── Status and file map ──────────────────────────────────────────────
    section(lines, "Collection status")
    for name in ("warm-build", "perf-record", "perf_report", "perf-report-by-dso",
                 "perf-stat", "strace"):
        if bundle.exists(f"{name}.status"):
            lines.append(f"- `{name}`: {bundle.status(name)}")
    if bundle.exists("perf.data"):
        lines.append(f"- `perf.data`: {os.path.getsize(os.path.join(bundle.path, 'perf.data'))} bytes")
    lines.append("")

    section(lines, "Files in this bundle")
    known = [
        ("summary.md", "this file"),
        ("metadata.txt / metadata.json", "build, commit, host, and capture settings"),
        ("host-environment.txt", "CPU, GPU, DRM nodes, Vulkan ICDs, graphics env overrides"),
        ("timeline.md", "per-window perf symbols labelled with the game's own log markers"),
        ("frame_times.csv", "per-census-window frame-time percentiles"),
        ("frame_spikes.csv", "every frame over 33.4ms, with its wall-clock second"),
        ("frame_windows.csv", "the always-on 5s frame census"),
        ("camera_views.csv", "one row per camera per sample: role, target, size, layers"),
        ("view_totals.csv", "camera/active/world-rendering/offscreen counts per sample"),
        ("runtime_census.csv", "entity, archetype, component, body, and player counts"),
        ("draw_census.csv", "sprite/text/projection population and visibility"),
        ("render_target_census.csv", "offscreen image targets and their bytes"),
        ("render_diagnostics.csv", "Bevy per-pass CPU/GPU times and pipeline statistics"),
        ("portal_activity.csv", "portal capture rigs and the budget bounding them"),
        ("asset_activity.csv", "cumulative decode work and resident images"),
        ("image_decodes.csv", "every notable texture decode, with its path"),
        ("schedule_census.csv", "registered system counts per sample"),
        ("tracy_summary.md / tracy_zones.csv", "per-Bevy-system and per-render-pass zones"),
        ("tracy_zone_windows.csv", "the same zones bucketed into time windows"),
        ("tracy.trace", "the raw Tracy trace, for the GUI"),
        ("perf_windows/", "one flat perf report per time slice"),
        ("perf_report.txt", "whole-run flat perf report"),
        ("perf-report-by-dso.txt", "which shared object owned the CPU"),
        ("game-stderr-stamped.txt", "the game's own log, stamped with seconds since launch"),
        ("perf.data", "the raw perf capture"),
    ]
    lines += ["| file | contents | present |", "| --- | --- | --- |"]
    for name, description in known:
        first = name.split(" / ")[0].rstrip("/")
        present = "yes" if bundle.exists(first) else "no"
        lines.append(f"| `{name}` | {description} | {present} |")
    lines.append("")
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: profile_bundle_summary.py <bundle-dir>", file=sys.stderr)
        return 2
    bundle = Bundle(argv[1])
    with open(os.path.join(argv[1], "summary.md"), "w", encoding="utf-8") as handle:
        handle.write(build_summary(bundle))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
