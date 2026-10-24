#!/usr/bin/env python3
"""Rasterizer CPU a frame of one room, binary against binary.

On a machine with no GPU the renderer is llvmpipe: each fragment shader runs
on its `llvmpipe-N` threads. Their CPU time (from /proc, not wall time) over
a window of frames is what the frame's fragment work costs, and other load on
the machine moves it much less than it moves frame time.

    scripts/room_raster_cpu_ab.py BIN_A BIN_B ROUNDS QUALITY OUT_DIR ROOM [CAPTURE_ARGS...]

BIN_A and BIN_B are two `capture_scene` builds of one tree and profile. Runs
are interleaved A, B, A, B. Each measures from the census mark at FROM_S to
the one at TO_S. Used for the Mockingbird's space (2026-10-24):

    scripts/room_raster_cpu_ab.py A B 4 high out mockingbird_sky --boss-hp 0.2
"""
import os, re, subprocess, sys, time

FROM_S, TO_S = float(os.environ.get("FROM_S", 14)), float(os.environ.get("TO_S", 44))
TICK = os.sysconf("SC_CLK_TCK")


def thread_cpu(pid):
    raster = total = 0
    for tid in os.listdir(f"/proc/{pid}/task"):
        try:
            comm = open(f"/proc/{pid}/task/{tid}/comm").read().strip()
            fields = open(f"/proc/{pid}/task/{tid}/stat").read().rsplit(")", 1)[1].split()
        except OSError:
            continue
        cpu = (int(fields[11]) + int(fields[12])) * 1000.0 / TICK
        total += cpu
        if comm.startswith("llvmpipe"):
            raster += cpu
    return raster, total


def run(binary, quality, png, log_path, room, extra):
    env = dict(os.environ, AMBITION_PROFILE_CENSUS="1", AMBITION_QUALITY_PROFILE=quality)
    args = [binary, room, "player", png, "1280x720", *extra, "--warmup", "100000"]
    proc = subprocess.Popen(args, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, errors="replace")
    start = end = None
    frames = 0
    with open(log_path, "w") as log:
        for line in proc.stderr:
            log.write(line)
            m = re.search(r"\[census\] frame t=([\d.]+) frames=(\d+)", line)
            if not m:
                continue
            t, n = float(m.group(1)), int(m.group(2))
            if start is None and t >= FROM_S:
                start = thread_cpu(proc.pid)
            elif start is not None:
                frames += n
                if t >= TO_S:
                    end = thread_cpu(proc.pid)
                    break
    proc.kill()
    proc.wait()
    if end is None:
        return None
    return (end[0] - start[0]) / frames, (end[1] - start[1]) / frames, frames


def main():
    bin_a, bin_b, rounds, quality, out, room = sys.argv[1:3] + [int(sys.argv[3])] + sys.argv[4:7]
    extra = sys.argv[7:]
    os.makedirs(out, exist_ok=True)
    results = {"A": [], "B": []}
    for r in range(1, rounds + 1):
        for arm, binary in (("A", bin_a), ("B", bin_b)):
            got = run(binary, quality, f"{out}/{arm}.{quality}.r{r}.png", f"{out}/{arm}.{quality}.r{r}.raster.log", room, extra)
            load = open("/proc/loadavg").read().split()[0]
            if got is None:
                print(f"{arm} r{r} {quality}: no window", flush=True)
                continue
            results[arm].append(got)
            print(f"{arm} r{r} {quality}: rasterizer {got[0]:.1f} ms CPU/frame, whole process {got[1]:.1f} ms CPU/frame, {got[2]} frames (load {load})", flush=True)
    for arm, rows in results.items():
        if rows:
            rows = sorted(rows)
            print(f"{arm} {quality} median: rasterizer {sorted(x[0] for x in rows)[len(rows)//2]:.1f} ms CPU/frame, whole {sorted(x[1] for x in rows)[len(rows)//2]:.1f} ms CPU/frame")


main()
