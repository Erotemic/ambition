#!/usr/bin/env python3
"""Name the samples of an Android profile, on a host that has no simpleperf.

    python3 scripts/symbolize_android_profile.py PROFILE_DIR [PROFILE_DIR ...] [--top N]
    python3 scripts/symbolize_android_profile.py PROFILE_DIR --lines   # by source line
    python3 scripts/symbolize_android_profile.py PROFILE_DIR --ours    # by OUR source line

A profile from ``scripts/profile_android.sh`` has the device's report
(``simpleperf-device-basic.txt``: each sampled address and its share), and,
when the app library was stripped, no Rust names in it: an address of the app
is ``libambition_app.so[+OFFSET]``. The profile's ``symfs/`` has the
unstripped library with the same build id. This script names each app address
with ``llvm-symbolizer`` against that library, adds the shares up by function
and by crate, and prints a table for each profile, and a table of the change
between the first profile and each other one.

``--lines`` adds a table by source line: the innermost (inlined) frame of each
sampled address, as ``file:line``. ``--ours`` adds the table that says which of
OUR lines caused the time, framework time included: for each sample of the
device's call graph (``simpleperf-device-callgraph.txt``) it walks from the
sampled frame toward the root, inlined frames included, and gives the sample's
share to the first frame whose file is in this repository (``crates/`` or
``game/``). Time in ``bevy_ecs``, ``alloc`` or ``core`` is then counted on the
line of ours that called it. A sample with no frame of ours is ``(no frame of
ours)``. The release build folds identical functions, so a function NAME can
be another function's; a ``file:line`` of ours is still where the call is.

It needs ``llvm-symbolizer`` (any version; ``llvm-symbolizer-18`` is tried
too). Shares are of all samples of the app's process, all threads.

``--thread NAME_OR_TID`` keeps only the samples of the threads with that name,
or of that thread id, in the ``--ours`` table (shares stay of the whole
process). With the sample dump, a table of threads is printed: each thread's
share, and how much of one core it used over the recording (about 100% = that
thread is saturated). ``--inclusive`` prints our functions (a frame whose file is in ``crates/`` or
``game/``) by inclusive share: each function counted once per sample whose
stack holds it, so a system's row includes the framework and allocator time
under it. ``--under NAME`` (repeatable) prints the share of samples whose stack, inlined
frames included, passes through a function whose name contains NAME: the
inclusive cost of that function, framework and allocator time included.
``--callers-of NAME`` (repeatable) answers "which of our lines lead into
NAME": for each sample whose stack passes through a function containing NAME,
it walks from that frame toward the root and gives the sample to the first
line of ours; with no line of ours (work queued as commands, for example), to
the first framework function above NAME, marked ``(not ours)``.
A thread is named by its id too: a thread made with
``std::thread::spawn`` and no name has its parent's name (Bevy's render thread
is a second ``android_main``).
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Tuple

LINE = re.compile(r"^\s*([0-9.]+)%\s+(\S+)\s+(.*?)\s*$")
#: A frame of the call graph report and of the basic report.
APP_PATH = re.compile(r"(/data/app/\S*libambition_app\.so)")
#: A source file of this repository, as the library's line table names it.
OURS = re.compile(r"(?:^|/)((?:crates|game)/[^:]+\.rs):(\d+)")
APP = re.compile(r"libambition_app\.so\[\+([0-9a-f]+)\]")


def symbolizer() -> str:
    for name in ("llvm-symbolizer", "llvm-symbolizer-18", "llvm-symbolizer-17", "llvm-symbolizer-19"):
        found = shutil.which(name)
        if found:
            return found
    sys.exit("no llvm-symbolizer on PATH")


def app_library(profile: Path) -> Path:
    """The unstripped library of the build that was sampled.

    `symfs/` can hold the libraries of more than one install (the device keeps
    one path per install); the samples name the device path of theirs, and the
    library under `symfs/` at that path is the one with the same build."""
    found = sorted(profile.glob("symfs/**/libambition_app.so"))
    if not found:
        sys.exit(f"{profile}: no symfs/**/libambition_app.so (was the profile made with symfs on?)")
    report = (profile / "simpleperf-device-basic.txt").read_text(errors="replace")
    sampled = {m.group(1) for m in APP_PATH.finditer(report)}
    for library in found:
        if any(str(library).endswith(path) for path in sampled):
            return library
    if len(found) > 1:
        sys.exit(f"{profile}: {len(found)} libraries under symfs/ and none at the sampled path {sorted(sampled)}")
    return found[0]


def samples(profile: Path) -> List[Tuple[float, str, str]]:
    """``(share, shared object, symbol)`` of each line of the device's basic report."""
    out = []
    for line in (profile / "simpleperf-device-basic.txt").read_text(errors="replace").splitlines():
        match = LINE.match(line)
        if match:
            share, dso, symbol = match.groups()
            out.append((float(share), dso, symbol))
    return out


def name_addresses(library: Path, offsets: List[str]) -> Dict[str, str]:
    """The function at each address of the app library."""
    if not offsets:
        return {}
    proc = subprocess.run(
        [symbolizer(), f"--obj={library}", "--functions=linkage", "--no-inlines", "--demangle", "--output-style=GNU"],
        input="\n".join(f"0x{offset}" for offset in offsets) + "\n",
        capture_output=True,
        text=True,
        check=True,
    )
    names = [line for i, line in enumerate(proc.stdout.splitlines()) if i % 2 == 0]
    return {offset: (name or "??") for offset, name in zip(offsets, names)}


def inline_frames(library: Path, offsets: List[str]) -> Dict[str, List[Tuple[str, str]]]:
    """Each address's frames, innermost (inlined) first, as ``(function, file:line)``."""
    if not offsets:
        return {}
    proc = subprocess.run(
        [symbolizer(), f"--obj={library}", "--functions=linkage", "--inlines", "--demangle"],
        input="\n".join(f"0x{offset}" for offset in offsets) + "\n",
        capture_output=True,
        text=True,
        check=True,
    )
    blocks = proc.stdout.strip("\n").split("\n\n")
    out = {}
    for offset, block in zip(offsets, blocks):
        lines = block.splitlines()
        out[offset] = [(lines[i], lines[i + 1] if i + 1 < len(lines) else "??") for i in range(0, len(lines) - 1, 2)]
    return out


def our_line(frames: List[Tuple[str, str]]) -> str | None:
    """The first frame of an address (innermost first) whose file is ours."""
    for _function, place in frames:
        match = OURS.search(place)
        if match:
            return f"{match.group(1)}:{match.group(2)}"
    return None


def call_paths(profile: Path) -> List[Tuple[float, List[str]]]:
    """``(self share, frames root first)`` of each node of the device's call graph.

    The report prints each thread's tree under `__start_thread`; a node's share
    is of its parent, and its self share is what its children do not take."""
    text = (profile / "simpleperf-device-callgraph.txt").read_text(errors="replace").splitlines()
    heads = [i for i, line in enumerate(text) if re.match(r"^\d+\.\d+%\s+\d+\.\d+%", line)]
    paths: List[Tuple[float, List[str]]] = []
    for k, head in enumerate(heads):
        if not text[head].rstrip().endswith("__start_thread"):
            continue
        body = text[head + 1 : heads[k + 1] if k + 1 < len(heads) else len(text)]
        total = float(text[head].split("%")[0])
        root = {"col": -1, "share": total, "frames": [], "kids": 0.0}
        stack, nodes = [root], [root]
        for line in body:
            # A branch is `|--N%-- frame`; the last branch of a node is `--N%-- frame`.
            match = re.search(r"(?:\|)?--(\d+\.\d+)%-- (.*)$", line)
            if match:
                col = match.start(1) - 2
                while len(stack) > 1 and stack[-1]["col"] >= col:
                    stack.pop()
                parent = stack[-1]
                node = {"col": col, "share": parent["share"] * float(match.group(1)) / 100.0,
                        "frames": parent["frames"] + [match.group(2).strip()], "kids": 0.0}
                parent["kids"] += node["share"]
                stack.append(node)
                nodes.append(node)
                continue
            frame = line.strip().lstrip("|").strip()
            if frame.startswith("-- "):
                frame = frame[3:]
            if frame and "[hit in function]" not in frame and not set(frame) <= set("|- "):
                stack[-1]["frames"].append(frame)
        paths += [(max(0.0, n["share"] - n["kids"]), n["frames"]) for n in nodes if n["frames"]]
    return paths


#: ``(start, end)`` seconds from a profile's first sample, or None: set from
#: ``--window``, and read by [`sample_paths`] for every table made from the
#: sample dump.
WINDOW: Tuple[float, float] | None = None


def sample_paths(profile: Path, thread: str | None = None) -> List[Tuple[float, List[str]]] | None:
    """``(share, frames root first)`` of each sample, from the device's
    ``simpleperf report-sample --show-callchain`` dump, or None without one.

    Each sample is a block that starts with ``sample:``; its own frame is the
    first ``vaddr_in_file``/``file`` pair, and its callers follow
    ``callchain:``, innermost first."""
    dump = profile / "simpleperf-device-samples.txt"
    if not dump.is_file() or dump.stat().st_size == 0:
        return None
    weights: List[float] = []
    stacks: List[List[str]] = []
    threads: List[str] = []
    tids: List[str] = []
    times: List[float] = []
    vaddr = None
    for line in dump.read_text(errors="replace").splitlines():
        text = line.strip()
        if text == "sample:":
            weights.append(1.0)
            stacks.append([])
            threads.append("")
            tids.append("")
            times.append(0.0)
            vaddr = None
        elif not stacks:
            continue
        elif text.startswith("time:"):
            times[-1] = float(text.split(":", 1)[1]) * 1e-9
        elif text.startswith("thread_id:"):
            tids[-1] = text.split(":", 1)[1].strip()
        elif text.startswith("thread_name:"):
            threads[-1] = text.split(":", 1)[1].strip()
        elif text.startswith("event_count:"):
            weights[-1] = float(text.split(":", 1)[1])
        elif text.startswith("vaddr_in_file:"):
            vaddr = int(text.split(":", 1)[1].strip(), 16)
        elif text.startswith("file:") and vaddr is not None:
            path = text.split(":", 1)[1].strip()
            stacks[-1].append(f"libambition_app.so[+{vaddr:x}]" if path.endswith("libambition_app.so") else path)
            vaddr = None
    if WINDOW is not None and times:
        # Keep the samples between WINDOW's seconds from the first sample.
        start = min(times)
        keep = [WINDOW[0] <= t - start < WINDOW[1] for t in times]
        weights, stacks, threads, tids = (
            [x for x, k in zip(column, keep) if k] for column in (weights, stacks, threads, tids)
        )
    total = sum(weights) or 1.0
    if thread == "busiest":
        # The thread with the most samples: the game's main thread, whatever
        # its id in this run (the render thread has the same name).
        per_tid: Dict[str, float] = defaultdict(float)
        for w, tid in zip(weights, tids):
            per_tid[tid] += w
        thread = max(per_tid, key=per_tid.get) if per_tid else None
    return [(100.0 * w / total, list(reversed(frames)))
            for w, frames, name, tid in zip(weights, stacks, threads, tids) if thread in (None, name, tid)]


def thread_table(profile: Path) -> Dict[str, float] | None:
    """Per thread name: ``share% (busy% of one core)``, from the sample dump."""
    dump = profile / "simpleperf-device-samples.txt"
    if not dump.is_file() or dump.stat().st_size == 0:
        return None
    busy: Dict[str, float] = defaultdict(float)
    first = last = None
    name, tid, count = "", "", 0.0
    for line in dump.read_text(errors="replace").splitlines():
        text = line.strip()
        if text == "sample:":
            if name:
                busy[f"{name} (tid {tid})"] += count
            name, tid, count = "", "", 0.0
        elif text.startswith("thread_id:"):
            tid = text.split(":", 1)[1].strip()
        elif text.startswith("time:"):
            when = int(text.split(":", 1)[1])
            first = when if first is None else first
            last = when
        elif text.startswith("thread_name:"):
            name = text.split(":", 1)[1].strip()
        elif text.startswith("event_count:"):
            count = float(text.split(":", 1)[1])
    if name:
        busy[f"{name} (tid {tid})"] += count
    total = sum(busy.values()) or 1.0
    wall = max(1, (last or 0) - (first or 0))
    print(f"recorded {wall / 1e9:.1f} s of wall time")
    return {f"{k}, busy {100.0 * v / wall:.0f}% of one core": 100.0 * v / total for k, v in busy.items()}


def ours_table(profile: Path, library: Path, thread: str | None = None) -> Dict[str, float]:
    """Each sample's self share on the first line of ours from its frame toward the root."""
    paths = sample_paths(profile, thread)
    if paths is None:
        if thread:
            raise SystemExit(f"--thread needs {profile.name}/simpleperf-device-samples.txt")
        print(f"note: {profile.name} has no simpleperf-device-samples.txt; the -g report drops "
              "branches under 0.25%, so most time lands on a frame near the root")
        paths = call_paths(profile)
    offsets = sorted({m.group(1) for _s, frames in paths for f in frames if (m := APP.search(f))})
    frames_at = inline_frames(library, offsets)
    by_line: Dict[str, float] = defaultdict(float)
    for share, frames in paths:
        line = None
        for frame in reversed(frames):
            match = APP.search(frame)
            if match and (line := our_line(frames_at.get(match.group(1), []))):
                break
        by_line[line or "(no frame of ours)"] += share
    return by_line


def under_table(profile: Path, library: Path, patterns: List[str], thread: str | None = None) -> Dict[str, float]:
    """Per pattern, the share of samples with a frame whose function name contains it."""
    paths = sample_paths(profile, thread)
    if paths is None:
        raise SystemExit(f"--under needs {profile.name}/simpleperf-device-samples.txt")
    offsets = sorted({m.group(1) for _s, frames in paths for f in frames if (m := APP.search(f))})
    frames_at = inline_frames(library, offsets)
    hits: Dict[str, float] = {pattern: 0.0 for pattern in patterns}
    for share, frames in paths:
        names = {function for f in frames if (m := APP.search(f)) for function, _place in frames_at.get(m.group(1), [])}
        for pattern in patterns:
            if any(pattern in name for name in names):
                hits[pattern] += share
    return {f"under {pattern}": share for pattern, share in hits.items()}


def callers_table(profile: Path, library: Path, pattern: str, thread: str | None = None) -> Dict[str, float]:
    """For the samples under a function containing PATTERN: the first line of
    ours from that frame toward the root, or the first other function above it."""
    paths = sample_paths(profile, thread)
    if paths is None:
        raise SystemExit(f"--callers-of needs {profile.name}/simpleperf-device-samples.txt")
    offsets = sorted({m.group(1) for _s, frames in paths for f in frames if (m := APP.search(f))})
    frames_at = inline_frames(library, offsets)
    by_caller: Dict[str, float] = defaultdict(float)
    for share, frames in paths:
        # Inlined frames innermost first, then each caller: leaf to root.
        chain = [frame for f in reversed(frames) if (m := APP.search(f)) for frame in frames_at.get(m.group(1), [])]
        hit = next((i for i, (function, _place) in enumerate(chain) if pattern in function), None)
        if hit is None:
            continue
        above = chain[hit + 1:]
        caller = our_line(above)
        if caller is None:
            other = next((function for function, _place in above if pattern not in function), "(root)")
            caller = f"(not ours) {other[:110]}"
        by_caller[caller] += share
    return by_caller


def _last_generic_argument(text: str, head: str) -> str | None:
    """The last top-level type argument of ``head<...>`` in ``text``."""
    start = text.find(head + "<")
    if start < 0:
        return None
    i, depth, current, arguments = start + len(head) + 1, 1, "", []
    while i < len(text) and depth:
        ch = text[i]
        if ch in "<([":
            depth += 1
        elif ch in ">)]":
            depth -= 1
        if depth == 1 and ch == ",":
            arguments.append(current)
            current = ""
        elif depth:
            current += ch
        i += 1
    arguments.append(current)
    return arguments[-1].strip()


def system_table(profile: Path, library: Path, thread: str | None = None) -> Dict[str, float]:
    """Each sample run by a schedule executor, given to the system it ran in:
    the framework time inside a system (its parameter fetch, query iteration,
    command buffers) counts for that system, which is what caused it. A
    system's name is the last type argument of its ``FunctionSystem<..>`` or
    ``ExclusiveFunctionSystem<..>``; a run condition is marked ``COND``."""
    paths = sample_paths(profile, thread)
    if paths is None:
        raise SystemExit(f"--by-system needs {profile.name}/simpleperf-device-samples.txt")
    offsets = sorted({m.group(1) for _s, frames in paths for f in frames if (m := APP.search(f))})
    frames_at = inline_frames(library, offsets)
    executors = ("single_threaded", "multi_threaded")
    by_system: Dict[str, float] = defaultdict(float)
    for share, frames in paths:
        # Root to leaf, inlined frames outermost first.
        chain: List[str] = []
        for f in frames:
            match = APP.search(f)
            chain.extend([fn for fn, _place in reversed(frames_at.get(match.group(1), []))] if match else [f])
        at = max((i for i, name in enumerate(chain) if any(e in name for e in executors)), default=None)
        if at is None:
            by_system["(outside a schedule executor)"] += share
            continue
        name = None
        for function in chain[at + 1:]:
            for head in ("ExclusiveFunctionSystem", "FunctionSystem"):
                if (name := _last_generic_argument(function, head)):
                    break
            if name:
                break
        if name is None:
            rest = chain[at + 1:at + 2]
            name = "(the executor: " + (rest[0][:70] if rest else "itself") + ")"
        condition = any("readonly_run::<bool>" in function for function in chain[at + 1:at + 3])
        by_system[("COND " if condition else "") + re.sub(r"<.*", "<…>", name)[:120]] += share
    return by_system


def inclusive_table(profile: Path, library: Path, thread: str | None = None) -> Dict[str, float]:
    """Each function of ours: the share of samples with it anywhere on the stack."""
    paths = sample_paths(profile, thread)
    if paths is None:
        raise SystemExit(f"--inclusive needs {profile.name}/simpleperf-device-samples.txt")
    offsets = sorted({m.group(1) for _s, frames in paths for f in frames if (m := APP.search(f))})
    frames_at = inline_frames(library, offsets)
    by_function: Dict[str, float] = defaultdict(float)
    for share, frames in paths:
        ours = {
            short(function)
            for f in frames
            if (m := APP.search(f))
            for function, place in frames_at.get(m.group(1), [])
            if OURS.search(place)
        }
        for function in ours:
            by_function[function] += share
    return by_function


def short(name: str) -> str:
    """A function name without its generic arguments and hash."""
    name = re.sub(r"::h[0-9a-f]{16}$", "", name)
    depth, out = 0, []
    for ch in name:
        if ch == "<":
            depth += 1
            if depth == 1:
                out.append("<…>")
            continue
        if ch == ">":
            depth = max(0, depth - 1)
            continue
        if depth == 0:
            out.append(ch)
    return "".join(out)


def crate(name: str) -> str:
    """The crate a function is in, as far as its name says."""
    name = name.lstrip("<").lstrip("&").strip()
    match = re.match(r"([A-Za-z_][A-Za-z0-9_]*)::", name)
    if match:
        return match.group(1)
    if name.startswith("mi_") or name.startswith("_mi_"):
        return "mimalloc"
    return "(other)"


def table(profile: Path, lines: bool = False):
    rows = samples(profile)
    library = app_library(profile)
    offsets = sorted({m.group(1) for _s, _d, symbol in rows if (m := APP.search(symbol))})
    names = name_addresses(library, offsets)
    frames_at = inline_frames(library, offsets) if lines else {}
    by_function: Dict[str, float] = defaultdict(float)
    by_crate: Dict[str, float] = defaultdict(float)
    by_object: Dict[str, float] = defaultdict(float)
    by_line: Dict[str, float] = defaultdict(float)
    for share, dso, symbol in rows:
        match = APP.search(symbol)
        obj = Path(dso).name if dso.startswith("/") else dso
        by_object[obj] += share
        if match:
            name = names.get(match.group(1), "??")
            by_function[short(name)] += share
            by_crate[crate(name)] += share
            if lines:
                frames = frames_at.get(match.group(1), [])
                place = frames[0][1] if frames else "??"
                by_line[re.sub(r"^.*?/((?:crates|game|src|library)/)", r"\1", place)] += share
        else:
            by_function[f"[{obj}] {symbol[:60]}"] += share
    return by_function, by_crate, by_object, by_line, library


def show(title: str, values: Dict[str, float], top: int) -> None:
    print(f"\n### {title}")
    for name, share in sorted(values.items(), key=lambda kv: -kv[1])[:top]:
        print(f"{share:6.2f}%  {name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("profiles", nargs="+", type=Path)
    parser.add_argument("--top", type=int, default=30)
    parser.add_argument("--lines", action="store_true", help="add a table by source line (innermost frame)")
    parser.add_argument("--ours", action="store_true", help="add a table by the first line of ours on each sample's stack")
    parser.add_argument("--thread", help="the --ours, --under, --inclusive and --by-system tables count only the samples "
                        "of this thread name or id; `busiest` is the thread with the most samples (the game's main thread)")
    parser.add_argument("--by-system", action="store_true",
                        help="add a table by the system each sample ran in (framework time inside a system included); "
                             "with two profiles, also the change")
    parser.add_argument("--inclusive", action="store_true",
                        help="add a table of our functions by inclusive share (framework time under them included)")
    parser.add_argument("--under", action="append", default=[],
                        help="share of samples whose stack passes through a function containing this name (repeatable)")
    parser.add_argument("--callers-of", action="append", default=[],
                        help="for samples under a function containing this name: the first line of ours above it (repeatable)")
    parser.add_argument("--window", help="START,END: the tables made from the sample dump count only the samples "
                        "this many seconds from the first one (to compare two scenes of one run)")
    parser.add_argument("--window-b", help="with --by-system and one profile: a second window, and the change from --window")
    args = parser.parse_args()
    global WINDOW
    parse_window = lambda text: tuple(float(x) for x in text.split(","))
    if args.window:
        WINDOW = parse_window(args.window)
    if args.window_b:
        if len(args.profiles) != 1 or not args.by_system:
            raise SystemExit("--window-b compares two windows of one profile, with --by-system")
        profile = args.profiles[0]
        library = table(profile, False)[4]
        first = system_table(profile, library, args.thread)
        WINDOW = parse_window(args.window_b)
        second = system_table(profile, library, args.thread)
        a, b = sum(first.values()) or 1.0, sum(second.values()) or 1.0
        print(f"## {profile.name}: systems, as a share of the thread's samples in each window ({args.window} vs {args.window_b})")
        change = {k: 100 * (second.get(k, 0.0) / b - first.get(k, 0.0) / a) for k in set(first) | set(second)}
        show(f"heavier in {args.window_b}", {k: v for k, v in change.items() if v > 0}, args.top)
        show(f"heavier in {args.window}", {k: -v for k, v in change.items() if v < 0}, args.top)
        return
    tables = []
    for profile in args.profiles:
        by_function, by_crate, by_object, by_line, library = table(profile, args.lines)
        tables.append((profile, by_function, by_crate, by_object))
        print(f"\n## {profile.name}")
        print(f"library: {library}")
        threads = thread_table(profile)
        if threads:
            show("threads", threads, 12)
        show("shared objects", by_object, 12)
        show("crates of the app", by_crate, 20)
        show("functions", by_function, args.top)
        if args.lines:
            show("source lines (innermost frame)", by_line, args.top)
        if args.ours:
            where = f", thread {args.thread}" if args.thread else ""
            show(f"lines of ours on the stack (framework time included{where})",
                 ours_table(profile, library, args.thread), args.top)
        if args.inclusive:
            show("our functions by inclusive share", inclusive_table(profile, library, args.thread), args.top)
        if args.under:
            show("inclusive share under a function", under_table(profile, library, args.under, args.thread), len(args.under))
        for pattern in args.callers_of:
            show(f"lines of ours that lead into {pattern}", callers_table(profile, library, pattern, args.thread), args.top)
        if args.by_system:
            systems = system_table(profile, library, args.thread)
            tables[-1] = tables[-1] + (systems,)
            where = f", thread {args.thread}" if args.thread else ""
            show(f"systems (framework time inside each included{where})", systems, args.top)
    if len(tables) > 1:
        base_name, base_fn, base_crate, _ = tables[0][:4]
        if args.by_system:
            base_systems = tables[0][4]
            for entry in tables[1:]:
                change = {k: entry[4].get(k, 0.0) - base_systems.get(k, 0.0) for k in set(entry[4]) | set(base_systems)}
                print(f"\n## {entry[0].name} minus {base_name.name}: systems (points of share)")
                show("systems that grew", {k: v for k, v in change.items() if v > 0}, args.top)
                show("systems that shrank", {k: -v for k, v in change.items() if v < 0}, args.top)
        for profile, by_function, by_crate, _ in (entry[:4] for entry in tables[1:]):
            print(f"\n## {profile.name} minus {base_name.name} (points of share)")
            for label, now, before in (("crates", by_crate, base_crate), ("functions", by_function, base_fn)):
                change = {k: now.get(k, 0.0) - before.get(k, 0.0) for k in set(now) | set(before)}
                show(f"{label} that grew", {k: v for k, v in change.items() if v > 0}, args.top if label == "functions" else 12)
                show(f"{label} that shrank", {k: -v for k, v in change.items() if v < 0}, 12)


if __name__ == "__main__":
    main()
