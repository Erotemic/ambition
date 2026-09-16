#!/usr/bin/env python3
"""Peak anonymous RSS of every arm in a test binary, ONE ARM PER PROCESS.

⛔ WHY ONE PROCESS PER ARM, not `--test-threads=1` over the whole binary: a
single process reports ONE RSS curve for whatever ran, so a runaway is
attributable only to the arm that happened to trip the allocator, not to the arm
that owns the growth. `libtest` also never returns memory between arms. Peak RSS
is a property of a PROCESS; to make it a property of an ARM, give each arm one.

⛔ Track `RssAnon`, not `VmRSS` and not the cgroup's `memory.current`. Page cache
dominates both and is reclaimable, so they answer a question about the machine
rather than about the arm.

⛔ Every arm gets a hard wall-clock cap and is killed by PROCESS GROUP. An
unbounded allocator at ~24 MB/s reaches the box's whole memory in minutes; the
cap is what converts "this OOM-kills the agent session" into "this row says
TIMEOUT with a slope".
"""

import argparse, json, os, signal, subprocess, sys, time


def rss_anon_kb(pid):
    try:
        with open(f"/proc/{pid}/status") as f:
            for line in f:
                if line.startswith("RssAnon:"):
                    return int(line.split()[1])
    except OSError:
        pass
    return None


def run_arm(binary, arm, cap_s, interval_s):
    proc = subprocess.Popen(
        [binary, "--exact", arm, "--test-threads=1"],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        start_new_session=True,
    )
    started = time.monotonic()
    peak, samples, timed_out = 0, [], False
    while True:
        code = proc.poll()
        now = time.monotonic() - started
        kb = rss_anon_kb(proc.pid)
        if kb:
            peak = max(peak, kb)
            samples.append((round(now, 2), kb))
        if code is not None:
            break
        if now >= cap_s:
            timed_out = True
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
            break
        time.sleep(interval_s)
    elapsed = time.monotonic() - started
    stdout = "" if timed_out else (proc.stdout.read() if proc.stdout else "")
    # ⛔ `--exact` on a name libtest does not hold runs ZERO tests and exits 0.
    # Without this the report is a page of 0.0 MB rows that measured nothing.
    ran = None
    for line in stdout.splitlines():
        if line.startswith("test result:"):
            for part in line.split(";"):
                part = part.strip()
                if part.endswith("passed"):
                    ran = int(part.split()[0])
                elif part.endswith("failed"):
                    ran = (ran or 0) + int(part.split()[0])
    # Slope over the last half of the run: a healthy arm plateaus, a runaway does not.
    slope = None
    tail = [s for s in samples if s[0] >= elapsed / 2]
    if len(tail) >= 2 and tail[-1][0] > tail[0][0]:
        slope = (tail[-1][1] - tail[0][1]) / 1024.0 / (tail[-1][0] - tail[0][0])
    return {
        "arm": arm,
        "peak_anon_mb": round(peak / 1024.0, 1),
        "seconds": round(elapsed, 2),
        "exit": None if timed_out else proc.returncode,
        "timed_out": timed_out,
        "tail_slope_mb_per_s": None if slope is None else round(slope, 2),
        "tests_ran": ran,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("binary")
    ap.add_argument("--cap-seconds", type=float, default=90.0)
    ap.add_argument("--interval", type=float, default=0.05)
    ap.add_argument("--filter", default=None, help="substring; arms not containing it are skipped")
    ap.add_argument("--out", default=None, help="write JSONL here as rows complete")
    args = ap.parse_args()

    listed = subprocess.run(
        [args.binary, "--list", "--format", "terse"], capture_output=True, text=True, check=True
    ).stdout
    arms = [l[: -len(": test")] for l in listed.splitlines() if l.endswith(": test")]
    if args.filter:
        arms = [a for a in arms if args.filter in a]
    # ⛔ An empty corpus would print a clean report having measured nothing.
    if not arms:
        sys.exit("no arms matched — refusing to report on an empty corpus")
    print(f"{len(arms)} arms, cap {args.cap_seconds}s", file=sys.stderr, flush=True)

    out = open(args.out, "w") if args.out else None
    for i, arm in enumerate(arms, 1):
        row = run_arm(args.binary, arm, args.cap_seconds, args.interval)
        line = json.dumps(row)
        if out:
            out.write(line + "\n")
            out.flush()
        if row["timed_out"]:
            flag = "TIMEOUT"
        elif row["tests_ran"] == 0:
            flag = "NOTHING-RAN"
        elif row["exit"]:
            flag = "FAIL"
        else:
            flag = ""
        print(f"{i}/{len(arms)} {row['peak_anon_mb']:>8.1f}MB {row['seconds']:>6.2f}s {flag:<7} {arm}", flush=True)
    if out:
        out.close()


if __name__ == "__main__":
    main()
