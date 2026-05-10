# Developer notes — `dev/`

Long-running engineering memory for the Ambition project. Anything
that **isn't** the game itself (`crates/`), authoring tooling
(`tools/`), or design docs that describe current behavior (`docs/`)
lives here. The contents are deliberately agent-readable: an agent
arriving cold should be able to read this folder, understand the
project's pattern of past mistakes, and take fewer of them.

The two subtrees today:

```text
dev/
  benchmark-candidates/   # Distilled hard questions from real refactor mistakes
  journals/               # >1hr-debug-time bug postmortems
```

`dev/` is **not** a TODO list (use `TODO.md`), **not** a feature log
(use `FEATURES.md`), **not** a code documentation tree
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
reading before adding a new question.

---

## `dev/journals/`

**What it is.** Postmortem journal of bugs that took >1 hour to
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
- **Write** here after a fix that took >1 hour to diagnose. The
  goal is for the next person to recognise the bug 10× faster.
  Skip the narrative — the format at the end of the file shows
  the canonical shape: Symptom, Root cause, Fix, Takeaway.

**Layout.**

```text
journals/
  lessons_learned.md   # All entries, newest first
```

(Plus eventually per-area journals if the file grows past
~50 entries.)

---

## How `dev/` relates to the rest of the repo

```text
TODO.md            -> what's in flight
FEATURES.md        -> what's shipped
docs/              -> how things work today
crates/            -> the game itself
tools/             -> authoring + build tooling
dev/               -> long-running engineering memory (you are here)
  benchmark-candidates/   -> distilled "hard question" corpus from real mistakes
  journals/               -> >1hr-debug-time bug postmortems
```

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
