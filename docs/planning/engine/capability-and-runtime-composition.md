# Capability and runtime composition

**State:** OPEN successor program.

## Goal

Make engine composition reflect what a game actually chooses to use.

A consumer building a small platformer should not inherit portal rendering, boss
orchestration, networking integration, persistence, debug presentation or
Ambition-only content merely because a broad historical crate sits in the middle
of the dependency graph.

## Why this program still matters

Current measurements changed the rationale.

Removing several non-Smash experiences from a measured Smash composition did not
materially improve representative frame time, and the associated plugin/system
removal did not improve plugin-registration startup in the measured probe.
Therefore capability composition is **not currently a funded generic runtime CPU
or startup optimization**.

Its demonstrated value is:

- dependency closure;
- coherent ownership;
- smaller/minimal consumers;
- compile/change and test isolation;
- host/platform composition;
- public SDK quality;
- making optional domains actually optional.

> **RE-MEASURED against `f32eb7274` (2026-09-02): this program has a LIVE MECHANICAL
> GUARD, and the page does not mention it.**
>
> `scripts/check_absence_contracts.py` runs
> **`capability-footprint-may-not-grow`**, which reports the program's headline
> number every time the gate runs: **43 crates linked, 16 of them a
> movement-only game never asked for.** It ratchets — a new crate entering the
> minimal consumer's closure turns it RED, naming each one — and its failure
> text is this page's §2e in one sentence: *"a perfectly semantic API can still
> force a movement-only game to compile and link every unrelated gameplay
> domain — no forbidden path is named and the footprint is still wrong."*
>
> Four sibling contracts pin the public-SDK half — `outlander-`,
> `minimal-game-`, `sim-harness-` and `capability-demo-names-only-the-public-sdk`
> — each at 0 of 0 baseline modules still naming internals. All 37 absence
> contracts hold at this commit.
>
> ⇒ **So "dependency closure" and "smaller/minimal consumers", two of the six
> values listed above, are no longer arguments — they are a number with a
> ratchet under it.** ⛔ And the number says the work is real and unfinished: 16
> of 43. A slice here has a ready-made acceptance condition (that count falls)
> and a ready-made regression guard (it may not rise), neither of which this page
> currently tells a reader exists.
>
> ⛔⛔ **BUT THE RATCHET COUNTS CRATES, NOT BYTES, AND A CARVE RAISES IT BY
> CONSTRUCTION.** Read "that count falls" as an acceptance condition without
> this and a decomposition slice looks like a regression on the very page that
> recommends decomposition. The closure is **44** as of 2026-09-02, not 43,
> because `ambition_world_items` entered it — the touched-collectible domain
> carved OUT of the actor monolith (D33). ⭐ **No new code entered the closure:
> the same `WorldItem` and `ItemMotion` were linked the day before, inside the
> monolith, and the monolith shed ~1,044 lines doing it.** The same is true of
> the `ambition_mount` and `ambition_damage` rows that preceded it.
>
> ⇒ **So the number answers "how many `ambition_*` crates does a movement-only
> game link", and NOT "how much does it link".** Both are worth knowing and they
> move in opposite directions under a carve. A row whose acceptance is "the
> count falls" is asking for edges to be made OPTIONAL or removed — the
> `ldtk_left_the_closure_2026_08_22` and `settings_menu_left_the_closure_2026_08_22`
> shape — and a carve cannot satisfy it, so the two lines of work must not be
> scored against each other. The baseline file records the reason beside every
> entry that arrived this way; ⚠ read those before quoting the delta.
>
> **Two more guards belong to this program and were also unnamed here:**
>
> - `scripts/check_capability_ships.py` — *"a capability whose only installer is
>   behind a DEV feature does not ship."* Green at `ce25540b1`: every Option-read
>   capability has at least one shipping writer, across 1437 files and 177
>   optional-read types. This is the mechanical form of "dependency closure and
>   installed runtime behavior should agree", the second principle below.
> - `scripts/check_engine_systems_are_engine_installed.py` — finds reusable
>   engine systems whose registration has leaked into a game host, so
>   headless/demo consumers cannot accidentally omit required behaviour. Green
>   at `ce25540b1`.
>
> **A fourth, and it is a PROHIBITION rather than a number:**
> `engine-crates-do-not-consume-the-umbrella-facade`
> (`check_absence_contracts.py`) forbids
> `ambition_platformer2d_actor_monolith` from depending on `ambition_platformer2d`.
> Its stated reason is this program's thesis in one sentence — an engine crate
> reaching back through the facade "is circular by construction, and it is how a
> headless consumer ends up compiling the render stack".
> ⚠ **And it records a deliberate exception that belongs on this page:** the rule
> is scoped to ENGINE crates because `ambition_content` DOES depend on the facade
> today, and whether that should stop is "a MEASUREMENT question the campaign
> defers rather than a rule". Green at `ec6d5150b`.
>
> ⛔ **A program with three green guards and one red-capable number is in a very
> different state from one with none, and a reader could not tell that from this
> page.** Name a guard where its program lives, or the next session re-derives
> the check that already exists.

### ⛔ 16 of 43 CANNOT be cut by a manifest change — do not start there

Re-derived at `8621f2a7e` (2026-09-02) after being asked to cut the count, and the
answer is that the cheap version of this work is already finished.

- **Slice H already took every available facade cut**, 2026-07-30: closure 41 →
  38 by making facade edges optional, which removed `ambition_inventory_ui`,
  `ambition_portal2d_presentation` and `ambition_touch_input`.
- **Every one of the remaining 16 also arrives through
  `ambition_platformer2d_actor_monolith`**, so gating its facade edge cuts
  nothing. A game that needs actors needs the monolith, and the monolith brings
  them. That is the §4 carve condition, i.e. `actor-monolith-decomposition.md`
  (D33) — not a manifest edit.
- ⚠ **Two of the 16 were never classified**, and re-deriving them is what cost
  the time this note exists to save. `ambition_damage` and `ambition_mount`
  entered the closure on 2026-08-26, AFTER the baseline's
  `reachable_via_ambition_platformer2d_actor_monolith_alone` list was written, so
  that list has 16 entries which are not the same 16 as `never_asked_for`. Both
  are unconditional facade edges AND monolith dependencies
  (`ambition_platformer2d_actor_monolith/Cargo.toml`), so both behave like
  `ambition_audio`: cuttable at the facade, worthless to cut.

⭐ **And the baseline records why "make the edges optional" undersold itself the
first time.** The four facade-only crates are named through `ambition::` by
in-repo code 170 times (`render` alone 90). Making an edge optional means
cfg-gating its re-export, so every one of those call sites must gain the same
feature. The baseline's own words: *"calling it cheap was wrong"*, caught *"by
counting the call sites before starting, rather than after."*

⇒ **So the honest acceptance for this row is not "the count falls."** It is
either a carve slice under D33, or a facade-optionality migration with 170
consumer call sites. Both are real; neither is mechanical.
## Principles

- capability selection is a semantic engine API, not a Cargo-feature illusion;
- dependency closure and installed runtime behavior should agree;
- the easy default may install a broad useful engine while narrow composition
  remains real and tested;
- internal implementation crates are not public capability names;
- a capability owns its data/schema/install declarations close to its domain;
- headless, rendered, desktop and mobile hosts compose from one capability
  vocabulary with host-specific services layered on top;
- domain-owned rollback/content declarations compose through backend-neutral
  registrars/catalogs; the generic runtime does not own concrete domain-type
  censuses.

## Current pressure points

- `ambition_platformer2d_actor_monolith` still owns several unrelated domains
  and therefore acts as a dependency/composition hub;
- `ambition_platformer2d_shared_tangle` still has high fan-in and mixed ownership;
- the public facade can expose semantic APIs beside historical implementation
  topology;
- some optional domains remain reachable through dependency closure even when a
  consumer did not ask for them;
- construction/content capability installation can still have compile-time and
  runtime-install assumptions that need explicit closure proofs.

Rollback registration itself is no longer the earlier central-census problem:
concrete gameplay declarations are federated by domain and the GGRS backend is
separate from the generic runtime. Do not use that completed migration as the
justification for another capability layer.

### Measured 2026-09-03 — the closure bullet above, with its number and its paths

`fixtures/external_consumer` ("outlander") is the honest test: its own
`[workspace]`, its own lockfile, no workspace dependency table, and it asks the
facade for exactly TWO capabilities with `default-features = false` —
`ambition_render` and rollback. Its manifest even says so: *"What else still
links is the carve's problem, not this manifest's."*

It links **46 `ambition_*` crates**, including **10 of the 14** optional
capability crates it never named:

| inherited | not inherited |
|---|---|
| cutscene, dialog, encounter, items, menu, persistence, projectiles, sfx, ui_nav, vfx | inventory_ui, portal2d, settings_menu, touch_input |

⭐ **And the split by CAUSE is the useful part, because the two halves need
different fixes.**
* **Six arrive through a capability it DID request.** `cutscene`, `persistence`,
  `projectiles`, `sfx`, `ui_nav` and `vfx` are direct dependencies in
  `crates/ambition_render/Cargo.toml`. Asking for rendering and receiving these
  is a question about whether `ambition_render` is one capability or a bundle —
  not about feature plumbing.
* **Four arrive through the facade's NON-OPTIONAL core**, and no feature flag can
  remove them. `cargo tree -i` in the fixture:
  `ambition_items` ← `ambition_platformer2d_actor_monolith`;
  `ambition_menu` ← `ambition_platformer2d_host` **and**
  `ambition_platformer2d_runtime`. All three are unconditional dependencies of
  `ambition_platformer2d`.

⇒ **This is the decomposition campaign's customer-visible consequence, measured.**
Making a domain optional cannot remove it while the monolith, host or runtime
names it — so the pickup carve and its siblings are what move this number, and
turning more capabilities into features is not. ⚠ It also means the number is a
progress metric for that campaign: re-run the fixture's `cargo tree` after each
carve.

✔ One capability is now provably NOT inherited by this consumer:
`ambition_relativity` appears zero times in outlander's tree, which is the
external half of the cost contract guarded the same day
(`engine.facade-all-capabilities-omits-relativity`).

⭐⭐ **AND THE STRICTER SENTINEL GETS THE SAME TEN.** `fixtures/minimal_game`
asks the facade for exactly ONE capability — `ambition_render`, nothing else —
and links **45** `ambition_*` crates and the SAME ten capability crates as
outlander, which asked for two. ⇒ **The closure is not a function of what a
consumer requests.** Asking for less changes the count by one crate, because
what arrives is decided by the facade's non-optional core, not by the feature
list. That is the sharpest available statement of this pressure point, and it is
why more feature flags cannot answer it.

⚠ Fixture-quality note, not a finding: `minimal_game` has a committed
`Cargo.lock` and resolves `--offline`; `external_consumer` has neither and needs
`bevy_gltf`, which the main workspace never fetches, so it cannot be measured on
a fresh host without network.

## Target shape

```text
Game / Experience definition
    + authored content providers
    + capability declarations
    + host services
    + local presentation policy
          |
          v
Prepared capability plan
          |
          +--> headless runtime host
          +--> desktop rendered host
          +--> mobile rendered host
```

The plan is not a service locator. It records what is installed, validates
requirements/conflicts and lowers to ordinary Bevy plugins/resources.

## Phases

### C1 — inventory from actual consumers

Use Ambition, Mary-O, Sanic, TwinTrack, Smash and the external-consumer fixture to
identify capability families that have independent customers.

### C2 — choose one leaky capability

Pick a capability whose absence still drags in unrelated crates/runtime behavior.
Carve its declaration/installation boundary and prove a minimal consumer no
longer inherits the unwanted dependency.

Choose the slice for dependency/ownership value, not because system count is
expected to move frame time.

### C3 — align content/construction declarations

A capability that contributes authored schema/construction lanes must have a
coherent installation contract. Avoid states where compile-time support says a
room may build a feature while the runtime fingerprint says that capability is
absent.

### C4 — separate host services where substitution is real

Audio, persistence, networking transport, window/input devices and renderer
services may become explicit host/service contracts where actual consumers need
substitution. Do not abstract all of them preemptively.

### C5 — narrow the facade

Expose semantic capability names and stable game-author APIs. Keep internal crate
moves behind the facade.

## Acceptance

- a minimal external game selects a small capability set and its dependency tree
  reflects that choice;
- Ambition composes the rich engine without privileged hidden paths;
- adding an optional domain does not require edits in unrelated runtime/game
  crates merely to register its state/content;
- capability conflicts/missing requirements fail during preparation with useful
  diagnostics;
- internal decomposition can continue without forcing external game code to
  follow crate topology;
- any claimed performance/startup benefit is backed by a new comparable
  measurement rather than inferred from fewer plugins/crates.
