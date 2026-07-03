# resource_tally — measured LLM resource accounting (per commit)

A small, **self-contained, copy-pasteable** module that records the compute each
commit cost an LLM agent, so a repo's lifetime *resource utilization* can be estimated.
Token counts and model are **measured** verbatim from the agent's own session
transcript; energy and carbon can be estimated later from the recorded
tokens/time and the commit timestamp — the timestamp can be used to estimate
how clean the grid's energy was at that moment).

```
dev/resource_tally/
├── resource_tally.py     # the tool (stdlib only; no pip installs)
├── hooks/post-commit     # auto-records after each commit
├── data/
│   ├── resource-ledger.jsonl   # append-only ledger — the source of truth
│   └── lifetime-totals.yaml    # rollup output (generated)
└── README.md             # you are here
```

## Install (per repo, ~1 minute)

1. **Copy this folder** to `dev/resource_tally/` in the target repo.
2. **Enable the auto-recording hook** (repo-local; not committed):
   ```bash
   git config core.hooksPath dev/resource_tally/hooks
   ```
   Now every `git commit` records usage automatically — zero effort thereafter.
   *(Caveat: `core.hooksPath` sets the hooks dir for the whole repo. If you already
   use other git hooks, point this at a dir that also contains them, or call
   `dev/resource_tally/resource_tally.py record` from your existing `post-commit`.)*
3. **Tell agents about it** — paste the snippet below into the repo's `AGENTS.md`
   (or `CLAUDE.md` / `README`), so any agent (Claude, Codex, …) maintains it.
4. *(Optional)* If the repo keeps a manifest with these two marker lines, `rollup`
   will also refresh the totals there in place (otherwise it just writes
   `data/lifetime-totals.yaml`):
   ```yaml
       # BEGIN lifetime_totals (auto-generated; do not edit by hand)
       # END lifetime_totals
   ```
   (Currently wired to `formalization.yaml`; change the filename in
   `_write_yaml_totals` for other manifests.)

## Usage

```bash
python3 dev/resource_tally/resource_tally.py record       # attribute new turns -> HEAD
python3 dev/resource_tally/resource_tally.py rollup        # refresh lifetime totals
python3 dev/resource_tally/resource_tally.py show          # print the ledger
python3 dev/resource_tally/resource_tally.py reconcile     # sweep un-committed turns
```
With the hook installed you normally only ever run `rollup` (at session end).
**Codex / non-Claude agents:** point the tool at your own log with
`record --transcript <path/to/session.jsonl>`.

## What's measured vs estimated vs deferred

| field | status | source |
|---|---|---|
| model, input/cache-write/cache-read/output tokens, server-tool calls | **measured** | session transcript `usage` (deduped by message id) |
| wall-clock span of attributed turns | **measured** | transcript timestamps |
| inference seconds | **estimated** | `output_tokens ÷ throughput` (assumption `time-v0`, unvalidated) |

> **Dedup:** A transcript logs each assistant message several times with
> *identical* usage; summing raw records overcounts (~2.6× on cache reads here).
> The tool dedups by `message.id` — do not hand-count tokens.

## Correctness guarantees

- **No double-count under concurrency.** Usage is attributed **per session** (each
  agent = its own transcript file = disjoint turns), and rows are keyed
  `(session_id, commit)`. The ledger is append-only under an `flock`; re-recording a
  `(session, commit)` pair is a no-op without `--force`.
- **No undercount.** `record` sweeps a session's turns in
  `(last-watermark, commit_ts]`; the next commit continues from that watermark, so no
  turn is dropped or counted twice. `reconcile` sweeps any un-committed trailing
  turns into a `pending@…` row so work that never produced a commit is still counted.
- **The ledger tip trails the commit tip by one row, by design** — recording commit
  *N* modifies the ledger, which needs commit *N+1*. This is a fixed point, not a
  bug; land the trailing row with your next commit (or bypass the hook once for a
  pure bookkeeping commit, as noted in git history).

## AGENTS.md snippet (paste into the target repo)

```markdown
## Resource accounting — the resource cost of the LLM work (CRITICAL: DO THIS EVERY COMMIT)

For every commit produced by an LLM agent; we keep a measured record of the
compute each commit cost. It is near-zero effort:

- Install once: `git config core.hooksPath dev/resource_tally/hooks` — then every
  commit auto-records. (Or run `python3 dev/resource_tally/resource_tally.py record`
  right after committing.)
- At the end of a work session: `python3 dev/resource_tally/resource_tally.py rollup`.
- Codex/other agents: `record --transcript <path/to/session.jsonl>`.

Tokens/model are MEASURED from your session transcript (deduped by message id — do
not hand-count). The ledger (`dev/resource_tally/data/resource-ledger.jsonl`) 
is append-only, per-session, concurrency-safe. See `dev/resource_tally/README.md`.
```
