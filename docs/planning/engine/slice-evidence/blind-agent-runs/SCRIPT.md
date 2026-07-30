# The blind-agent script — fixed, so runs are comparable

[api-growth-method.md §2c](../../../planning/engine/api-growth-method.md) requires
a **fixed script**. A prompt improvised per run measures the prompt.

Change this file only with a recorded reason, and note the change in the run
that first used it — a script edited between runs makes the series
incomparable, which is the failure it exists to prevent.

## Preconditions

* **A FRESH agent.** §2c: *"An agent resumed from a session that touched engine
  internals measures its own memory. This is the single easiest way to get a
  falsely green result, and the result is falsely green in the direction that
  feels good."*
* The agent is **not** told what the answer is, which files matter, or that it
  is being measured on which engine file it opens.
* It is **not forbidden** from opening engine files. The measurement is
  *whether* and *which first* — a prohibition would produce a clean number and
  no information.

## The prompt

> You are a third-party game developer. You have just added the `ambition` game
> engine as a dependency and you want to stand up a minimal game on it.
>
> Your goal: **get a minimal game running against this engine — one that boots
> headless, and one that opens a window — and report whether you got there.**
>
> Start from `docs/sdk/README.md`. Work the way you normally would.
>
> Write your game at `<SCRATCH>/blind_run/`. Try to actually build it and
> report what happened.
>
> Keep a log as you go, and include it in your final report:
>
> 1. every file you opened that lives under `crates/`, `game/`, `fixtures/` or
>    `tools/`, **in the order you opened them**;
> 2. for each, one sentence on what you were trying to find out;
> 3. whether you achieved the headless boot and the windowed boot;
> 4. anything the documentation should have told you and did not.
>
> Be honest about failure. A report that says "I could not work out X without
> reading Y" is worth more than a report that says it succeeded.

## The recorded fields

Per §2c, plus provenance:

| Field | Why |
|---|---|
| `completed` | did each task complete |
| `first_engine_file_opened` | **the field that matters** — it names the next leak from the population the API is for |
| `engine_files_opened` | ordered; the tail is as informative as the head |
| `elapsed_context` | cost proxy |
| `wanted_and_missing` | the agent's own account of what the docs owed it |

## Known contamination, stated rather than hidden

A subagent in this repository inherits the project's `AGENTS.md`/`CLAUDE.md`
through its system prompt, which a genuine third party would not have. That
biases the run **toward** competence, so a bad result is trustworthy and a good
one is weaker than it looks. Runs that want the stricter measurement should be
driven from outside the repository.
