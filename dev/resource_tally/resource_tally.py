#!/usr/bin/env python3
"""resource_tally.py — measured LLM-usage accounting for this repo.

WHY
    Every commit here is produced by an LLM agent. To make the *climate cost* of
    that work legible, we record — per commit — the **measured** token usage and
    model, plus a time estimate, into an append-only ledger. Energy and carbon are
    *deferred*: they are derived later from the recorded tokens/time and the commit
    timestamp (which fixes "how dirty was the grid at that moment"). See the
    `automation.resource_accounting` block in `formalization.yaml` and AGENTS.md.

WHAT IS MEASURED vs ESTIMATED
    measured  : model, input/cache-write/cache-read/output tokens, server-tool
                calls, wall-clock span — all read verbatim from the agent's own
                session transcript (Claude Code writes a `usage` object per turn).
    estimated : inference_seconds (output_tokens ÷ throughput, assumption 'time-v0').
    deferred  : energy_kwh, carbon_gco2e (left null; computed in a later pass).

DESIGN (the constraints this satisfies)
    * No double-count under concurrency: usage is attributed **per session**. Each
      agent has its own transcript file (filename = session id), so two agents in
      the same repo touch disjoint turns. The ledger is append-only JSONL keyed by
      (session_id, commit); re-running for the same pair is a no-op without --force.
    * No undercount: a run sweeps *all* of the session's turns since that session's
      last recorded watermark — committed or not. `reconcile` attributes any
      trailing un-recorded turns to a pending bucket so nothing is lost.
    * Minimal agent tokens: the agent runs ONE command (or nothing, via the
      post-commit hook in dev/resource_tally/hooks/). Language-agnostic: a Codex agent
      can pass `--transcript <path>` to its own log.

    This whole `resource_tally/` folder is self-contained and copy-pasteable between
    repos: the ledger and rollup live under `resource_tally/data/`, relative to this
    file — no host-repo layout is assumed. See resource_tally/README.md to install.

USAGE
    python dev/resource_tally/resource_tally.py record        # attribute new turns -> HEAD
    python dev/resource_tally/resource_tally.py record --commit <sha>
    python dev/resource_tally/resource_tally.py rollup        # refresh lifetime totals
    python dev/resource_tally/resource_tally.py show          # print the ledger
    python dev/resource_tally/resource_tally.py reconcile     # catch un-recorded turns

    Optional: --transcript PATH | --session ID | --tps N | --force | --projects-dir DIR
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys
from datetime import datetime, timezone

# ---- assumptions (versioned; see formalization.yaml automation.resource_accounting) ----
TIME_ASSUMPTIONS_VERSION = "time-v0"
DEFAULT_TPS = 55.0  # placeholder decode throughput (tok/s) for Opus-class; UNVALIDATED
SCHEMA = "resource-ledger/v1"

TOKEN_KEYS = ("input_tokens", "cache_creation_input_tokens",
              "cache_read_input_tokens", "output_tokens")


# --------------------------------------------------------------------------- git
def git(*args: str, cwd: str | None = None) -> str:
    return subprocess.run(["git", *args], cwd=cwd, check=True,
                          capture_output=True, text=True).stdout.strip()


def repo_root() -> str:
    return git("rev-parse", "--show-toplevel")


def superproject_root() -> str:
    """Parent repo working tree if we are a submodule, else our own toplevel."""
    sp = git("rev-parse", "--show-superproject-working-tree")
    return sp or repo_root()


def commit_meta(ref: str) -> tuple[str, str]:
    """Return (full_sha, committer_date_iso) for `ref`."""
    sha = git("rev-parse", ref)
    ts = git("show", "-s", "--format=%cI", sha)
    return sha, ts


# ------------------------------------------------------------------- transcripts
def munged_project_dir(path: str) -> str:
    """Claude Code stores transcripts under ~/.claude/projects/<cwd with / -> ->."""
    return path.replace("/", "-")


def default_projects_dir() -> str:
    return os.path.expanduser(os.environ.get(
        "CLAUDE_PROJECTS_DIR", "~/.claude/projects"))


def find_session_transcript(projects_dir: str, session: str | None) -> str:
    """Locate the transcript for the current (or named) session.

    Prefers the project dir munged from the *superproject* cwd (the agent's cwd is
    typically the top-level repo, not this submodule). Falls back to a repo-wide
    scan. Picks the most-recently-modified `.jsonl` unless `--session` is given.
    """
    proj = os.path.join(projects_dir, munged_project_dir(superproject_root()))
    candidates = sorted(glob.glob(os.path.join(proj, "*.jsonl")),
                        key=os.path.getmtime, reverse=True)
    if not candidates:  # fall back: any transcript under projects_dir
        candidates = sorted(glob.glob(os.path.join(projects_dir, "**", "*.jsonl"),
                                      recursive=True),
                            key=os.path.getmtime, reverse=True)
    if session:
        for c in candidates:
            if os.path.splitext(os.path.basename(c))[0] == session:
                return c
        sys.exit(f"error: no transcript for session {session} under {projects_dir}")
    if not candidates:
        sys.exit(f"error: no session transcripts found under {projects_dir}")
    return candidates[0]


def parse_turns(transcript: str) -> list[dict]:
    """Extract assistant turns bearing a usage object, deduped by message id.

    Returns list of {id, ts, model, usage{...}, web_search, web_fetch}. Streaming
    can emit a message id more than once; the last occurrence wins.
    """
    by_id: dict[str, dict] = {}
    with open(transcript, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            if rec.get("type") != "assistant":
                continue
            msg = rec.get("message") or {}
            usage = msg.get("usage") or {}
            if not usage:
                continue
            mid = msg.get("id") or rec.get("uuid") or rec.get("timestamp")
            st = usage.get("server_tool_use") or {}
            by_id[mid] = {
                "id": mid,
                "ts": rec.get("timestamp"),
                "model": msg.get("model", "?"),
                "usage": {k: int(usage.get(k, 0) or 0) for k in TOKEN_KEYS},
                "web_search": int(st.get("web_search_requests", 0) or 0),
                "web_fetch": int(st.get("web_fetch_requests", 0) or 0),
            }
    turns = [t for t in by_id.values() if t["ts"]]
    turns.sort(key=lambda t: t["ts"])
    return turns


# ------------------------------------------------------------------------ ledger
def module_dir() -> str:
    """Directory of this module; the ledger/totals live under it so the whole
    `resource_tally/` folder is self-contained and copy-pasteable between repos."""
    return os.path.dirname(os.path.abspath(__file__))


def ledger_path() -> str:
    return os.path.join(module_dir(), "data", "resource-ledger.jsonl")


def totals_path() -> str:
    return os.path.join(module_dir(), "data", "lifetime-totals.yaml")


def read_ledger() -> list[dict]:
    p = ledger_path()
    if not os.path.exists(p):
        return []
    rows = []
    with open(p, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def session_watermark(rows: list[dict], session_id: str) -> str:
    """Max turn timestamp already recorded for this session ('' if none)."""
    hi = ""
    for r in rows:
        if r.get("session_id") == session_id:
            rng = r.get("turn_ts_range") or [None, None]
            if rng[1] and rng[1] > hi:
                hi = rng[1]
    return hi


def append_row(row: dict) -> None:
    """Append one JSONL row under an flock so concurrent agents don't interleave."""
    import fcntl
    p = ledger_path()
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "a", encoding="utf-8") as fh:
        fcntl.flock(fh, fcntl.LOCK_EX)
        fh.write(json.dumps(row, ensure_ascii=False) + "\n")
        fcntl.flock(fh, fcntl.LOCK_UN)


def aggregate(turns: list[dict], tps: float) -> dict:
    tok = {k: 0 for k in TOKEN_KEYS}
    by_model: dict[str, dict] = {}
    web_search = web_fetch = 0
    out_tokens = 0
    for t in turns:
        for k in TOKEN_KEYS:
            tok[k] += t["usage"][k]
        bm = by_model.setdefault(t["model"], {k: 0 for k in TOKEN_KEYS})
        for k in TOKEN_KEYS:
            bm[k] += t["usage"][k]
        web_search += t["web_search"]
        web_fetch += t["web_fetch"]
        out_tokens += t["usage"]["output_tokens"]
    ts_lo = turns[0]["ts"] if turns else None
    ts_hi = turns[-1]["ts"] if turns else None
    wall = _span_seconds(ts_lo, ts_hi)
    return {
        "turns": len(turns),
        "models": sorted(by_model),
        "tokens": {
            "input": tok["input_tokens"],
            "cache_write": tok["cache_creation_input_tokens"],
            "cache_read": tok["cache_read_input_tokens"],
            "output": tok["output_tokens"],
            "billable_input": (tok["input_tokens"]
                               + tok["cache_creation_input_tokens"]
                               + tok["cache_read_input_tokens"]),
        },
        "by_model": {m: {"input": v["input_tokens"],
                         "cache_write": v["cache_creation_input_tokens"],
                         "cache_read": v["cache_read_input_tokens"],
                         "output": v["output_tokens"]}
                     for m, v in by_model.items()},
        "server_tools": {"web_search": web_search, "web_fetch": web_fetch},
        "time": {
            "wall_clock_s": wall,
            "inference_s_est": round(out_tokens / tps, 1) if tps else None,
            "assumptions_version": TIME_ASSUMPTIONS_VERSION,
            "throughput_tok_per_s": tps,
        },
        "turn_ts_range": [ts_lo, ts_hi],
    }


def to_dt(s: str | None) -> datetime | None:
    """Parse either transcript ('...Z') or git ('+00:00') ISO timestamps to aware
    datetimes. Never compare these formats as strings — 'Z' vs '+00:00' and the
    fractional-seconds '.' both break lexicographic order across the two sources."""
    if not s:
        return None
    return datetime.fromisoformat(s.replace("Z", "+00:00"))


def _span_seconds(lo: str | None, hi: str | None) -> float | None:
    dlo, dhi = to_dt(lo), to_dt(hi)
    if not dlo or not dhi:
        return None
    return round((dhi - dlo).total_seconds(), 1)


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


# ---------------------------------------------------------------------- commands
def cmd_record(args) -> None:
    transcript = args.transcript or find_session_transcript(
        args.projects_dir, args.session)
    session_id = os.path.splitext(os.path.basename(transcript))[0]
    sha, commit_ts = commit_meta(args.commit)
    rows = read_ledger()

    if not args.force and any(r.get("session_id") == session_id
                              and r.get("commit") == sha for r in rows):
        print(f"already recorded session {session_id[:8]} @ commit {sha[:8]} "
              f"(use --force to override); nothing to do.")
        return

    # Attribution window = (last watermark for this session, commit timestamp].
    # Bounding the top at commit_ts (not "now") keeps work done AFTER this commit
    # rolling forward to the next commit's record instead of misattributing here.
    wm = session_watermark(rows, session_id)
    wm_dt, cut_dt = to_dt(wm), to_dt(commit_ts)
    new = [t for t in parse_turns(transcript)
           if (wm_dt is None or to_dt(t["ts"]) > wm_dt) and to_dt(t["ts"]) <= cut_dt]
    if not new:
        print(f"no new turns for session {session_id[:8]} in "
              f"({wm or 'epoch'}, {commit_ts}].")
        return

    agg = aggregate(new, args.tps)
    row = {
        "schema": SCHEMA,
        "recorded_at": now_iso(),
        "repo": os.path.basename(repo_root()),
        "commit": sha,
        "commit_ts": commit_ts,
        "agent": "claude-code",
        "session_id": session_id,
        **agg,
        "energy_kwh": None,       # deferred
        "carbon_gco2e": None,     # deferred
        "factors_version": None,  # deferred
    }
    append_row(row)
    tk = agg["tokens"]
    print(f"recorded {agg['turns']} turns for {sha[:8]} "
          f"[{','.join(agg['models'])}]: "
          f"out={tk['output']} in={tk['input']} "
          f"cache_w={tk['cache_write']} cache_r={tk['cache_read']}; "
          f"wall={agg['time']['wall_clock_s']}s "
          f"infer~{agg['time']['inference_s_est']}s ({TIME_ASSUMPTIONS_VERSION}); "
          f"energy/carbon deferred.")
    print(trailer_line(row))


def cmd_reconcile(args) -> None:
    """Attribute any un-recorded trailing turns (per session) to a pending bucket,
    so a session that did work without committing is never dropped."""
    rows = read_ledger()
    projects = args.projects_dir
    proj = os.path.join(projects, munged_project_dir(superproject_root()))
    files = sorted(glob.glob(os.path.join(proj, "*.jsonl")))
    total = 0
    for f in files:
        sid = os.path.splitext(os.path.basename(f))[0]
        wm = session_watermark(rows, sid)
        new = [t for t in parse_turns(f) if (t["ts"] or "") > wm]
        if not new:
            continue
        agg = aggregate(new, args.tps)
        row = {
            "schema": SCHEMA, "recorded_at": now_iso(),
            "repo": os.path.basename(repo_root()),
            "commit": f"pending@{now_iso()[:10]}", "commit_ts": None,
            "agent": "claude-code", "session_id": sid, **agg,
            "energy_kwh": None, "carbon_gco2e": None, "factors_version": None,
            "note": "reconcile: un-committed turns swept so they are not undercounted",
        }
        append_row(row)
        total += agg["turns"]
        print(f"reconciled {agg['turns']} un-recorded turns for session {sid[:8]}.")
    if total == 0:
        print("nothing to reconcile; all session turns already accounted.")


def trailer_line(row: dict) -> str:
    tk = row["tokens"]
    return ("Resource-Estimate: model={m}; tok in={i} cw={cw} cr={cr} out={o}; "
            "wall={w}s infer~{inf}s ({v}); energy/carbon=deferred").format(
        m="+".join(row["models"]), i=tk["input"], cw=tk["cache_write"],
        cr=tk["cache_read"], o=tk["output"],
        w=row["time"]["wall_clock_s"], inf=row["time"]["inference_s_est"],
        v=TIME_ASSUMPTIONS_VERSION)


def cmd_show(args) -> None:
    rows = read_ledger()
    if not rows:
        print("ledger is empty.")
        return
    for r in rows:
        tk = r["tokens"]
        print(f"{(r.get('commit') or '')[:10]:12} {r['recorded_at'][:19]} "
              f"{','.join(r.get('models', [])):20} "
              f"out={tk['output']:>7} billable_in={tk['billable_input']:>9} "
              f"wall={r['time']['wall_clock_s']}s")


def cmd_rollup(args) -> None:
    rows = read_ledger()
    tot = {"input": 0, "cache_write": 0, "cache_read": 0, "output": 0,
           "billable_input": 0}
    by_model: dict[str, int] = {}
    turns = 0
    wall = 0.0
    infer = 0.0
    web_search = web_fetch = 0
    commits = set()
    for r in rows:
        tk = r["tokens"]
        for k in tot:
            tot[k] += tk.get(k, 0)
        turns += r.get("turns", 0)
        for m in r.get("models", []):
            by_model[m] = by_model.get(m, 0) + r["by_model"].get(m, {}).get("output", 0)
        w = r.get("time", {}).get("wall_clock_s") or 0
        i = r.get("time", {}).get("inference_s_est") or 0
        wall += w
        infer += i
        web_search += r.get("server_tools", {}).get("web_search", 0)
        web_fetch += r.get("server_tools", {}).get("web_fetch", 0)
        c = r.get("commit") or ""
        if c and not c.startswith("pending@"):
            commits.add(c)
    totals = {
        "generated_at": now_iso(),
        "ledger_rows": len(rows),
        "commits_accounted": len(commits),
        "turns": turns,
        "tokens": tot,
        "output_tokens_by_model": by_model,
        "server_tool_calls": {"web_search": web_search, "web_fetch": web_fetch},
        "time": {"wall_clock_s": round(wall, 1),
                 "inference_s_est": round(infer, 1),
                 "assumptions_version": TIME_ASSUMPTIONS_VERSION},
        "energy_kwh": None,
        "carbon_gco2e": None,
    }
    _write_totals_file(totals)                      # portable canonical output
    manifest = _write_yaml_totals(totals)           # optional repo-manifest integration
    print(json.dumps(totals, indent=2, ensure_ascii=False))
    print(f"# wrote {os.path.relpath(totals_path(), repo_root())}"
          + ("; refreshed formalization.yaml lifetime_totals" if manifest else ""))


def _write_totals_file(totals: dict) -> None:
    """Portable, canonical rollup output at `<module>/data/lifetime-totals.yaml`.
    Repo-agnostic — this is what a copy-pasted module always produces, independent of
    whatever manifest (if any) the host repo keeps."""
    path = totals_path()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    lines = ["# Auto-generated by resource_tally.py rollup — do not edit by hand.",
             "lifetime_totals:"]
    lines += _yaml_indent(totals, 2)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")


def _write_yaml_totals(totals: dict) -> bool:
    """Best-effort manifest integration: if the repo has a formalization.yaml carrying
    the lifetime_totals markers, refresh that region in place and return True. Skips
    silently when the manifest or its markers are absent, so a copy-pasted module stays
    decoupled and never creates or pollutes a foreign repo's manifest."""
    yml = os.path.join(repo_root(), "formalization.yaml")
    if not os.path.exists(yml):
        return False
    begin = "    # BEGIN lifetime_totals (auto-generated; do not edit by hand)"
    end = "    # END lifetime_totals"
    with open(yml, encoding="utf-8") as fh:
        text = fh.read()
    if begin not in text or end not in text:
        return False
    block = [begin, "    lifetime_totals:"]
    block += _yaml_indent(totals, 6)
    block.append(end)
    pre = text[:text.index(begin)]
    post = text[text.index(end) + len(end):]
    with open(yml, "w", encoding="utf-8") as fh:
        fh.write(pre + "\n".join(block) + post)
    return True


def _yaml_indent(obj, indent: int) -> list[str]:
    pad = " " * indent
    out = []
    if isinstance(obj, dict):
        for k, v in obj.items():
            if isinstance(v, (dict,)) and v:
                out.append(f"{pad}{k}:")
                out += _yaml_indent(v, indent + 2)
            else:
                out.append(f"{pad}{k}: {_scalar(v)}")
    return out


def _scalar(v) -> str:
    if v is None:
        return "null"
    if isinstance(v, dict) and not v:
        return "{}"
    return str(v)


# ---------------------------------------------------------------------------- cli
def main() -> None:
    p = argparse.ArgumentParser(description="Measured LLM-usage accounting.")
    sub = p.add_subparsers(dest="cmd", required=True)

    def common(sp):
        sp.add_argument("--transcript", help="explicit session transcript path")
        sp.add_argument("--session", help="session id (transcript filename stem)")
        sp.add_argument("--projects-dir", default=default_projects_dir())
        sp.add_argument("--tps", type=float, default=DEFAULT_TPS,
                        help=f"decode throughput assumption (default {DEFAULT_TPS})")

    r = sub.add_parser("record", help="attribute new turns to a commit")
    r.add_argument("--commit", default="HEAD")
    r.add_argument("--force", action="store_true")
    common(r)
    r.set_defaults(func=cmd_record)

    rc = sub.add_parser("reconcile", help="sweep un-recorded trailing turns")
    common(rc)
    rc.set_defaults(func=cmd_reconcile)

    ru = sub.add_parser("rollup", help="refresh lifetime totals in formalization.yaml")
    ru.set_defaults(func=cmd_rollup)

    sh = sub.add_parser("show", help="print the ledger")
    sh.set_defaults(func=cmd_show)

    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
