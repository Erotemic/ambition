# Developer notes — `dev/`

Long-running engineering memory for the Ambition project. Anything
that **isn't** the game itself (`crates/`), authoring tooling
(`tools/`), or design docs that describe current behavior (`docs/`)
lives here. The contents are deliberately agent-readable: an agent
arriving cold should be able to read this folder, understand the
project's pattern of past mistakes, and take fewer of them.

The search entry point is [`SEARCH.md`](SEARCH.md). It gives grep patterns and routing rules so agents search this memory instead of reading it all.

The subtrees today:

```text
dev/
  SEARCH.md                  # How to search engineering memory
  benchmark-candidates/      # Distilled hard questions from real refactor mistakes
  journals/                  # >1hr-debug-time bug postmortems
  ambition_dev_measurements/ # SUBMODULE — the append-only cost ledgers (below)
```

`dev/` is **not** a TODO list (use `TODO.md`), **not** a feature log
(use current system/concept docs or archive notes), **not** a code documentation tree
(use `docs/`), and **not** a place to dump WIP scratch (those go in
the working directory and get gitignored / cleaned up).

---

## `dev/benchmark-candidates/`

**What it is.** A growing corpus of self-contained Rust /
Bevy / game-engineering questions distilled from real maintenance
mistakes made while building this project. Each question captures
a *pre-error* setup — the context an agent had at the moment of
the mistake — so a different model facing the same setup can be
tested for the same failure mode. The corpus is intended to be
NeurIPS-dataset-track quality if the project ever ships a
benchmark; for now its first job is making future Ambition agents
better.

**Why an agent should care.**

- **Read** these before tackling a refactor that resembles one
  catalogued here. The "Why this was easy to miss" section on
  each question is the single most useful one — it names the
  cognitive trap so you can recognise it in your own reasoning.
- **Write** here when you cause (or watch the user cause, or
  resolve) a mistake whose root cause is a transferable
  invariant. The bar is "another model in the same situation
  could plausibly make the same mistake without this question
  written down."

**Layout.**

```text
benchmark-candidates/
  README.md                          # Workflow, quality bar, prompt levels (Levels A/B/C)
  rust-questions.md                  # Main Rust corpus (most entries land here)
  compositions.md                    # Multi-invariant questions that compose single-issue Qs
  sprite-generator-schema-questions.md
  ui-nav-refactor-questions.md
  ui-nav-test-questions.md
  warning-cleanup-questions.md
```

The smaller topic-scoped files exist because parallel agents have
been editing the main `rust-questions.md` and we want to avoid
merge-conflict ping-pong. When in doubt, add to `rust-questions.md`;
spin off a topic file only when you know another agent is touching
the same section concurrently.

`compositions.md` is special — it catalogues *combinations* of
single-issue questions that test capabilities (enumeration,
synthesis, error-attribution, interference detection) which the
component questions can't measure on their own.

[`benchmark-candidates/README.md`](benchmark-candidates/README.md)
spells out the workflow (write the failure evidence first, then
distil; pick the right Level A/B/C prompt; tag by failure-class
invariant rather than by surface technology) and is required
reading before adding a new question. [`benchmark-candidates/index.md`](benchmark-candidates/index.md)
routes by invariant/failure class.

---

## `dev/journals/`

**What it is.** Postmortem journal of bugs that effort to
diagnose. Newest-first, written in the moment so the symptom
language matches what a future debugger would search for.

**Why an agent should care.**

- **Read** here first when you encounter a confusing symptom in
  the same area as a past entry. The grep-target is the symptom
  description (e.g. "duplicate sprite", "staircase smear",
  "down_pressed every frame", "two music sources audible").
  The entries are deliberately written so the symptom keywords
  match what you'd search for from inside the bug, not the
  technically correct vocabulary that comes after diagnosis.
- **Write** here after a fix that took effort to diagnose. The
  goal is for the next person to recognise the bug 10× faster.
  Skip the narrative — the format at the end of the file shows
  the canonical shape: Symptom, Root cause, Fix, Takeaway.

**Layout.**

```text
journals/
  index.md             # Symptom router
  lessons_learned.md   # Aggregate entries, newest first
```

Standalone per-incident files are preferred when a lesson has a focused symptom. Add rows to `journals/index.md` so future agents can find them by symptom language.

---

## How `dev/` relates to the rest of the repo

```text
docs/planning/     -> what's in flight
docs/current/      -> active architecture and implementation state
docs/              -> how things work today
crates/            -> the game itself
tools/             -> authoring + build tooling
dev/               -> long-running engineering memory (you are here)
  benchmark-candidates/     -> distilled "hard question" corpus from real mistakes
  journals/                 -> >1hr-debug-time bug postmortems
  ambition_dev_measurements/ -> SUBMODULE: the cost ledgers (see below)
  compile_ratchet_baseline.json -> the ratchet's frozen input; NOT in the submodule
  compile_telemetry_schema.md   -> the ledgers' field-by-field contract
  audio_loudness_report.md      -> every sound's level, ranked; REWRITTEN by
                                   `scripts/audio_levels.py`, not appended
```

## Cost ledgers — `dev/ambition_dev_measurements/` (a submodule)

Append-only JSONL, one row per measured run. They exist so "this got slower" is
a diff rather than an impression, and so an optimisation can be shown to have
worked instead of asserted to have.

⭐ **they live in a submodule** (`https://github.com/Erotemic/ambition_dev_measurements.git`)
because they grow monotonically and every checkout paid for the growth: on
2026-08-08 these five files were **3.88 MB of a 43.95 MB tracked tree**. A cost
ledger's value IS its history, so there is no pruning policy that would have
fixed it. Get them with:

```sh
git submodule update --init dev/ambition_dev_measurements
```

⛔ **without that, the directory still EXISTS and is empty** — git creates the
mount point on any clone. So every writer checks before it appends and refuses
loudly rather than creating a stray file that the next `git submodule update`
deletes and `git status` never mentions. Readers degrade instead, the same way
they already do on a fresh clone that has run no collector. The paths, the check,
and the full reasoning are declared once in
[`scripts/lib/measurement_paths.py`](../scripts/lib/measurement_paths.py) — ⛔ do
not re-declare a ledger path in a script; three scripts used to and that is what
made this move expensive.

⚠ **`dev/compile_ratchet_baseline.json` deliberately stayed in this repo.** It is
a GATE INPUT — the bare `python3 scripts/compile_ratchet.py` gate reads it on
every run — and a gate whose baseline sits behind an uninitialised submodule
cannot run. It is bounded
too: one frozen snapshot, rewritten rather than appended.

| ledger (under `dev/ambition_dev_measurements/`) | `kind` | written by | measures |
|---|---|---|---|
| `run_tests_cost.jsonl` | `job` | `scripts/run_tests.py` | wall + per-job cost of the suite |
| `compile_cost.jsonl` | `scenario` | `scripts/compile_cost.py` | what an EDIT costs to rebuild |
| `compile_units.jsonl` | `unit` | `scripts/compile_collect.py` | seconds, frontend/codegen split and LOC per compile unit |
| `compile_graph.jsonl` | `graph` | `scripts/compile_ratchet.py --update` | the deterministic build-shape numbers the gate guards |
| `carve_lineage.jsonl` | `carve` | `scripts/compile_ratchet.py --record-carve` | what a module split from, and why |

**Every row in all five carries the same seven-key envelope, and `kind` is the
discriminator that lets them be read as one table.** The field-by-field contract
— including which columns are populated, which are reserved, and how the four
pre-schema rows in `compile_cost.jsonl` normalise — is
[`dev/compile_telemetry_schema.md`](compile_telemetry_schema.md). Read it before
adding a column.

`compile_cost.py` perturbs a real source file, runs a real cargo command, and
reverts — because the number that matters is the edit→feedback loop, not a cold
build. `compile_collect.py` does the same thing at the scale of the whole graph
and records one row per crate. Read their module docstrings before comparing
rows: a run is only comparable to another on the same machine, linker, profile
and `CARGO_INCREMENTAL` setting, and all four are recorded on every row for
exactly that reason. ⛔ **never run two of these at once** — two builds sharing a
target dir made a warm no-op read 222s, which looks like a slow machine rather
than a mistake.

`python3 scripts/compile_collect.py --analyze` reads the ledgers back and builds
nothing.

`python3 scripts/compile_report.py` is the other reader: it renders all five
ledgers into one self-contained HTML page at `dev/compile_report.html` (gitignored
— it regenerates in under a second and would otherwise churn a 236 KB diff on
every append), and `--print-summary` gives the same digest as text. It runs no
build and invokes no cargo. ⚠ **it labels its own thin data**: where a ledger has
one row the page draws one point and says so, because a trend line through a
single sample is the prettiest way this instrument could lie.

The auto-memory at
`/home/agent/.claude/projects/-home-joncrall-code-ambition/memory/`
is a parallel layer for **per-conversation** continuity (user
preferences, recent project state, feedback rules). It cross-
references `dev/` entries when relevant; the two layers don't
duplicate each other. If a fact is true across many sessions, it
goes in auto-memory. If it's a question or a postmortem, it goes
in `dev/`.

When in doubt about *where* to write something, ask: would a
brand-new agent landing in this repo benefit from reading it
cold, without a conversation context? If yes → `dev/`. If it's
only useful when the user is in the loop → auto-memory.

---

## Quality bar (one paragraph)

Don't add entries that record trivia. Both subtrees are
deliberately curated; an over-long file is a worse signal than a
shorter one, because future readers won't believe the
"important" entries hidden between filler. If you're unsure
whether something belongs, write it in your scratch notes
first; if a week later you still think the lesson is durable,
move it in.
