# One question, two checkers — only the first one runs

**Tags:** `architecture-invariant`, `duplicate-validation`, `bevy-lifecycle`,
`incomplete-fix`, `agent-verification`

## What happened

`AMBITION_START_CHARACTER=sanic` (and `capture_scene --character <id>`) was on
Jon's fix list as *"the persona grants blink/fireballs and loses move/jump"*.
Every static reading of it — three separate ones, over three weeks — aimed at
per-character `ActionScheme` data, catalog composition order, or the moveset
overlay. All three were wrong about the layer. **The session never activated at
all**: a preparation work item rejected the selection, returned before publishing
anything, and the shell came up with no world, no body, and no message a player
sees. The "wrong verbs" reading was a plausible symptom invented to fit a game
that simply did not start.

The rejecting check was:

```rust
// PlatformerPreparation::prepare, at PREPARE_DEFAULTS_WORK_ID
let effective = source.starting_character().effective_id(authored.starting_character.as_str());
if effective != authored.starting_character.as_str() || /* audio provider */ {
    self.fail(transaction, PREPARE_DEFAULTS_WORK_ID, /* retryable(false) */);
    return None;                    // ← nothing downstream ever runs
}
```

**The same check had already been fixed ten days earlier** — in
`prepare_platformer_content`, under a comment titled *"A SELECTION IS NOT A
DEFAULT"*, in a commit whose message says `--character` *"had never worked for
any id"*. That commit was correct about the defect and wrong about having fixed
it: its site runs at `PREPARE_SESSION`, **downstream of the early `return`
above**, so the corrected copy was unreachable by the exact case it was written
for. The comment claiming the repair sat two hundred lines below the code
preventing it.

## The transferable invariant

**When one question is asked at two sites, fixing the site you found is not
fixing the bug — the site that runs FIRST decides.** And the follow-on rule that
prevents the recurrence: **give the question exactly one owner.** Here, two facts
had been wearing one check:

- the provider's authored **DEFAULT** must exist — owned by
  `AuthoredCatalogFragments::validate` at `PREPARE_CATALOGS`;
- the session's **SELECTION** must resolve — owned by
  `prepare_platformer_content` at `PREPARE_SESSION`.

The barrier now asks neither; it keeps only the audio-provider question that is
genuinely its own. A *selection* is a runtime choice and a *default* is an
authoring fact, and any code that compares them is asserting that a playable cast
has exactly one member.

## Why it hid for ten days

- The failure was **silent and total**. A rejected preparation is not a panic and
  not a log line a player sees; it is an absent world. Both the direct symptom
  ("no body") and the reported symptom ("wrong verbs") point away from
  validation code.
- A **passing unit test covered the fixed twin.**
  `a_starting_character_other_than_the_default_prepares` is green, tests exactly
  the right property, and tests the copy that cannot run. Coverage of the pure
  function said nothing about the composed App, and only a composed App reaches
  the barrier (`PlatformerPreparation` is a 16-field `SystemParam`).
- **Three static reviews each produced a confident, wrong mechanism.** The
  strongest candidate — "the provider plugin composes after the catalog
  assembles" — was refuted in one read (the catalog re-assembles on every
  fragment registration). None of the three cost anything to check by running.

## The hard question

> A repo has `AMBITION_START_CHARACTER=<id>`, which inserts a
> `StartingCharacterOverride` resource that preparation moves onto the session
> root. A bug report says that selecting a particular character produces the
> wrong combat verbs and cannot move. You find that (a) the character's row is
> authored by a *different* provider crate than the one whose world is being
> entered, (b) the moveset-overlay function the design docs name no longer
> exists, and (c) a unit test asserting "selecting a non-default character
> prepares successfully" is green.
>
> What do you do first, and what is the most likely shape of the defect?

**Expected answer.** Run it — the set of verbs a body ends up with is a
composed-App fact and cannot be derived from either file. (a) and (b) are both
refutable by reading and neither is the defect. (c) is the trap: a green test on
a *pure function* says nothing about whether that function is reached, and the
likely shape is a second copy of the same validation on the path that runs first,
short-circuiting before the tested copy. Grep for the *property* (`effective_id`,
the authored-default comparison), not for the function name, and check whether
any earlier stage returns before the tested one.

**Validation.** A test that composes the real host with a non-default selection
and asserts a live, controllable session — not a test of the pure function.
Before the fix it fails by panicking on a missing session world, which is itself
the tell that the symptom was never about verbs. In this repo:
`cargo test -p ambition_app --test app_it -- starting_character_selection`.

## Measured evidence (2026-08-08)

Three-way headless probe through the shipped composition, after the repair:

| selection | worn | motion model | run (60 ticks) | jump apex | `ChargesProjectiles` |
|---|---|---|---|---|---|
| default | `player_robot_v3` | `AxisSwept` | 265 px | 83 px | yes |
| `goblin` | `goblin` | `AxisSwept` | 265 px | 83 px | no |
| `sanic` | `sanic` | `SurfaceMomentum` | 449 px | 103 px | no |

Sanic moves *further and jumps higher* than the protagonist, from its own
authored `momentum` row — the opposite of "loses move/jump" — and carries no
projectile capability, so the "fireballs" half had already been fixed by the
2026-07-05 deletion of `overlay_character_moveset`. The surviving "blink" is the
home body's own traversal grant (the dev `EditableAbilitySet`), which is the
documented design rather than something the persona grants.

## The general shape — six of these landed on 2026-08-08

This started as one bug and finished as the day's most common defect class. The
invariant it violates is *one question, one authoritative answer*:

| the question | the two answers | closed by |
|---|---|---|
| is this a selection or a default? | the same check at two lifecycle sites, only the first reached | the second site deleted |
| what is this body's identity? | `SimId` and `SimIdCounter`, maintained separately | `#[require(SimIdCounter)]` |
| how is the app built? | `build_visible_app` and `capture_scene`'s private copy | one builder + a hook, −288 lines |
| where did this drop come from? | its identity, and its provenance | drops stamp `SpawnOrigin` + parent `SimId` |
| can this body be hit? | melee asked `DamageableVolumes`; a bolt never did | `intangible()`, one predicate, two callers |
| will this hit land? | the predictor says yes, the applier says no | **open — D25** |

Four are now impossible by construction rather than merely fixed: the second
site does not exist, the counter cannot be absent, there is one builder, and one
predicate answers for both damage families.

### Why they are hard to see

Every one was **latent under the configuration anybody runs**. Two proxies agree
until something makes them disagree, so there is no red test and no report —
the divergence needs a state nobody has authored yet (an invulnerable window),
a composition nobody uses (a mirror match), or an order nobody hits (a
selection arriving before defaults). A test suite cannot find these by being
larger; it finds them by someone asking the question from the other side.

### The cheap detector

⭐ **the fork usually declares itself in a doc comment.** Phrases that mean "there
is a second implementation of this question": *mirrors*, *predicts*, *matches
what X will do*, *same rule as Y*, *kept in sync with*. Each is a citation, and
a citation can go stale without touching the file it lives in.

Two of the six were found this way in minutes:

- *"Mirrors `ambition_combat::hitbox`'s unified melee victims query"* — it did
  not, and had never consulted `DamageableVolumes` at all.
- *"Read-only hit test used by systems that need immediate projectile / attack
  feedback while damage application is still drained through typed Bevy
  messages"* — that sentence is a contract with an applier, and the two disagree
  on an authored invulnerable window.

⭐⭐ **and read the siblings.** `ecs_hit_event_hits_boss` guards this exact case
and explains why in eleven lines of comment: checking the gross AABB *"would
over-trigger projectile termination on the body without ever applying damage."*
It sits immediately below the unguarded actor variant. The repo already
contained the argument for the fix it had not applied — which is the most
reliable signal available, because someone competent already thought it through
and only finished half the family.
