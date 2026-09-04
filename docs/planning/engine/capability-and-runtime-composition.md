# Capability and runtime composition

**State:** OPEN successor program.

## Goal

⭐ **THIS PROGRAM OWNS THE SECOND OF THE TWO ARCHITECTURAL GOALS.** Authority
decomposition — one authority per fact, dependencies pointing the right way — is
the actor-monolith program's. Independently installable capability composition,
the Bevy-like property, is this one's, and the durable statement of the criterion
with its user-facing examples and its named risks is
[`decomposition.md`](decomposition.md) ("Decomposition has two dimensions"). ⇒ A carve that lands
elsewhere does not close a row here; the question this program asks is whether
the capability can be INSTALLED alone.

Make engine composition reflect what a game actually chooses to use.

## The first measured baseline for this goal (2026-09-04)

⭐ **ONE RUNTIME FILE INITIALISES 40 RESOURCES BELONGING TO 14 OTHER CRATES.**
`crates/ambition_platformer2d_runtime/src/sim_core_resources.rs` calls
`init_resource` forty times, and `lib.rs` adds fifty-two plugins. That is the
runtime acting as the semantic owner of every capability's state — the failure
mode [`decomposition.md`](decomposition.md) names — rather than as the provider
of schedules and lifecycle seams.

⛔ **AND TEN OF THE FOURTEEN OWNING CRATES CONTAIN NO `impl Plugin` AT ALL:**
`ambition_boss_encounter`, `ambition_combat`, `ambition_encounter`,
`ambition_gameplay_trace`, `ambition_input`, `ambition_items`,
`ambition_persistence`, `ambition_platformer2d_core`,
`ambition_platformer2d_world`, `ambition_projectiles`. They have no way to
install themselves, so the runtime has to. ⇒ *"Capabilities register themselves
against stable seams"* is not one edit away from any of them; it is this
program's work, and this is its size.

⚠ **AND AUTHORITY STILL COMES FIRST, demonstrated by the cheapest-looking
candidate.** Projectiles read like a clean first move — one resource
(`ProjectileSeqCounter`), a named reusable capability. But
`runtime/src/projectile_schedule.rs` is seventeen lines of RE-EXPORTS whose
steppers live in `actor_monolith::projectile`, because they still touch
un-carved actor/player/boss/world state. A projectile plugin today would own a
counter and none of its systems. ⇒ Pick the first capability to self-install by
where its SYSTEMS already live, not by how few resources it has.

⚠ Not a crate classification and not a carve list: some of the fourteen are
foundation (`ambition_time`, `ambition_platformer2d_core`) where central
initialisation is appropriate. The number is the baseline, not the target.

✔ **FIRST STEPS TAKEN 2026-09-04, chosen by the rule above rather than by size,
and the count is 40 → 36.**
`7f666117a`: `ambition_sim_view`'s `FeatureViewIndex`, `ActorRenderIndex` and
`BossRenderIndex` now initialise in `FeatureViewSyncSchedulePlugin`, which
schedules their rebuilds and already stated the rule in its own doc — *"the
plugin that rebuilds the index initializes it; consumers only read"* — while
those three of its twenty siblings were initialised by the runtime.
Then `NewGameResetRequested` moved into `NewGameResetPlugin`, whose
`process_new_game_reset_request` is its sole consumer.
⚠ **Both were the same shape and it is the only cheap one: a resource whose
CONSUMER is already a plugin.** `PendingLifecycleCommit` is the counter-example
and was left alone — `shrine.rs` and `world/rooms/systems.rs` both write it, so
there is no single plugin to move it to, and inventing one would be
composability reasoning driving an authority decision. Each step here is
verified by `app_it` (551 passed / 0 failed), because a resource that stops being
initialised is a whole app failing, not a unit test.

⭐ **AND ONE CRATE ALREADY SHIPS AN INSTALL PATH THE RUNTIME BYPASSES, which is a
different shape worth recognising before someone "fixes" it.**
`ambition_time::TimePlugin` initialises `ClockState` and `WorldTime` and
schedules `refresh_world_time` — and **nothing in this repository installs it**;
the runtime does both halves itself, scheduling that system in `player_schedule`
with an explicit ordering. ⛔ Installing the plugin alongside the runtime would
add a SECOND, UNORDERED copy of `refresh_world_time`. ⇒ It is not dead code and
not a gap: its audience is an external frame-stepped host that is not using the
Ambition runtime, which is exactly the kind of install path a foundation
capability should offer. The hazard is now documented on the plugin itself.
⚠ The lesson for this program: "the crate has no plugin" and "the crate has a
plugin nobody installs" want different answers, and the second can be correct.

✔ **THE FIRST MINIMUM-HOST PROBE EXISTS AND IT PASSES (2026-09-04).**
`a_host_that_omits_cutscenes_still_builds_and_steps` builds the engine group with
`.disable::<CutsceneSchedulePlugin>()` and steps eight frames, against a CONTROL
arm that installs the same group whole. *"A platformer without cutscenes"* is one
of [`decomposition.md`](decomposition.md)'s named target compositions, and it is
reachable today.
⭐ **AND THE PROBE'S TWO FAILED ATTEMPTS ARE THE MORE USEFUL RESULT.** Both
failed in the CONTROL: first inside `bevy_asset`, then in
`finalize_unpresented_room_transition_failure_system` for want of
`NextState<GameMode>`. Neither says anything about cutscenes. ⇒ **The engine's
prerequisites are DECLARED — `add_headless_foundation` is exactly this set
(MinimalPlugins, asset, image, transform, states, `init_engine_states`) and its
doc calls itself "the minimal Bevy foundation for a HEADLESS engine app"** — they
are simply not what `MinimalPlugins` gives you. A probe that hand-rolls the host
measures the host.
⚠ What it does NOT prove, stated on the test: that a cutscene-free composition is
USEFUL, or that content triggering a cutscene degrades gracefully, or that any
other capability can be omitted. Each is its own probe.

⭐ **THREE OF THE DOCTRINE'S NAMED COMPOSITIONS, PROBED — and the two that could
not be probed are the more useful half.**

```text
without cutscenes   ✔ builds and steps, against a whole-group control arm
without portals     ✔ the same
without dialogue    ⚠ NOT PROBED — the plugin cannot be NAMED from this test
                       target: the facade re-exports `ambition_dialog` only
                       behind a feature this target does not enable, and
                       `ambition_app` has no direct dependency on it. A
                       `#[cfg]`-guarded test would have compiled to nothing and
                       reported success.
encounters without  ⛔ NOT EXPRESSIBLE — `ambition_boss_encounter` contains no
boss encounters        `impl Plugin` at all, so there is no seam to omit it
                       through. A stronger statement than a failing probe.
```

⇒ Two capabilities can be omitted today and it takes one line each to say so.
The third needs the probe to live where the plugin is nameable rather than a
feature flag added to make it nameable. The fourth needs an installation seam
that does not exist — and that seam is now SIZED rather than merely missing.

### The boss-encounter installation seam, sized 2026-09-04

⭐ **`ambition_boss_encounter` owns its systems already; what it does not own is
their INSTALLATION.** The driver is `update_boss_encounters` in that crate, and
the runtime's `progression_schedule.rs` schedules it alongside seven siblings
(`notify_bosses_on_mount_death`, `sync_boss_encounter_entities`,
`update_encounter_progress`, `tick_falling_hazards`, `tick_encounter_scripts`,
`release_payloads_on_death`, `boss_phase_transition_feedback`) and registers
three of its messages (`EncounterGate`, `PayloadReleased`, `BossPhaseChanged`).
Fourteen references in that one file, six more elsewhere in the runtime, plus two
resources the runtime initialises (`BossCatalog`, `BossEncounterRegistry`).

✔ **BUILT 2026-09-04 (`bd93f978f`), and it was a plugin.**
`BossEncounterSimulationPlugin` owns the eight systems, the three messages and
the two resources; `a_host_that_omits_boss_encounters_still_builds_and_steps` is
its acceptance, and the doctrine's *"generic encounters without boss
encounters"* is a `.disable::<_>()` today.

⭐ **THE MEASUREMENT THAT DECIDED THE SIZE IS THE REUSABLE PART, and it is the
check to run before proposing the next capability:** those eight systems' ordering
edges name only `ProgressionSet::BossAdvance` / `BossHazards`, which
`shared_tangle::schedule` already publishes and the crate already depended on. So
nothing moved and no ordering was renegotiated. ⛔ **A capability whose ordering
edges name another capability's SYSTEMS cannot be installed this way, however
coherent its authority is** — that is a carve with a negotiation inside it, and
the difference is knowable in one read.

⚠ **THE HOST STILL OWNS THE SETS.** The plugin does not `configure_sets`; the
runtime anchors `ProgressionSet` into the engine chain and the capability only
says which systems belong in two of its slots. A capability that configured the
ordering it runs in would be a second authority over the schedule — the failure
this page names for runtime and for shared scheduling alike.

⛔ And 87 files outside the crate still reference `ambition_boss_encounter`. That
number did not move and was never the point: this packet answered INSTALLATION,
not dependency reduction, and sizing it by that count would have asked the wrong
question.

### The sweep that rule enables, run 2026-09-04

⭐ **The runtime schedules systems from exactly FOUR crates**, and applying the
check above resolves every one of them without a further carve being proposed:

| crate | verdict |
|---|---|
| `ambition_boss_encounter` | ✔ installs itself since `bd93f978f` |
| `ambition_encounter_features` | ✔ already had `EncounterSimulationSchedulePlugin` |
| `ambition_time` | ✔ has `TimePlugin` for an external host; the runtime deliberately does not install it (see above) |
| `ambition_mount` | ⛔ **FAILS THE CHECK, and it is the worked example** |

⛔ **Mounts cannot be installed this way today.** `combat_schedule.rs` chains
`ambition_mount::enforce_mount_rider_link` with
`ambition_platformer2d_actor_monolith::features::rebuild_dismounted_rider_brains`
— one capability's system named directly in another's ordering, with a comment
explaining why the order is load-bearing ("a dismount request landing first would
remove the link it is relying on… the other order is silent"). ⇒ That is a CARVE
with an ordering negotiation inside it, not a plugin: the two systems' relation
would have to become a published set before either side could own its own
installation. Do not promote mounts as "the next easy one".

⭐ **AND THE CHECK RUN ON THE NEXT CANDIDATE COMES BACK CLEAN (yardrat,
2026-09-04): `ambition_combat` PASSES.** Every `.after(..)` / `.before(..)` in
`combat_schedule.rs` that names a foreign path resolves to one crate other than
combat itself — `ambition_mount` — and to a `SystemSet` rather than a system:
`MountRiderLinkEnforced`, whose own doc says it exists because *"the consumer is
itself in `Settle`, so pinning the parent would be a cycle — this is the shape
only a nested set can express."* Published vocabulary in exactly the sense
`ProgressionSet` is.
⚠ **That is one check passing, not a sizing.** `combat_schedule.rs` is ~500 lines
and installs a great deal; the count of what would move is not the count of what
is entangled, and combat's share of the 40 `init_resource` calls has not been
measured. ⇒ Worth knowing that combat is not the hard one; not yet a packet.
⚠ **The mount edge that FAILS is a different edge from the one that passes**, and
conflating them would lose both findings: combat's `.after` names a SET, while
the runtime separately CHAINS `ambition_mount::enforce_mount_rider_link` with the
monolith's `rebuild_dismounted_rider_brains` inside one `add_systems`. The first
is installable vocabulary; the second is two capabilities' systems ordered by a
third party.

⇒ **So the runtime's remaining knowledge of capabilities is now resources and
plugin lists, not scheduling.** 36 `init_resource` calls and ~53 `.add(...)`
lines. The next reduction comes from resources whose consumer is already a
plugin (three taken, see above) or from a capability gaining one — not from more
schedule moves, because there are none left to make.

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
> number every time the gate runs. ⛔ **THE NUMBER IS NOT QUOTED HERE ANY MORE —
> re-derive it:** `python3 scripts/check_absence_contracts.py | grep footprint`,
> which prints the live pair from
> `scripts/baselines/capability-footprint-baseline.json`, and that file's dated
> `*_entered_the_closure_*` rows say why each crate is there.
> ⚠ The quoted pair drifted FOUR times in a week — 43/16 → 44/16 → 44/17 →
> 45/18 — and then FOUR carves landed in one night (2026-09-03: body_seed,
> match, encounter_features, abilities: → 48/21 → 49/22 → 50/23), every one a
> crate boundary drawn through code the sentinel already linked. A number that
> moves faster than the paragraph quoting it is not a fact the paragraph can hold. A
>
> ⛔ **AND THE DIRECTION OF THAT DRIFT NEEDS SAYING OUT LOUD: 45→49 AND 18→22 IS
> +4 AND +4, THE SAME FOUR.** Every crate that joined the closure since
> `479f9d3e4` landed in `never_asked_for` — verified against the baseline JSON:
> `ambition_held_items`, `ambition_body_seed`, `ambition_encounter_features`,
> `ambition_match` and `ambition_registry_core` are all in that list. Each
> arrived by a carve commit whose subject is some form of *"X leaves the actor
> kernel"*.
>
> ⇒ **So the headline number gets WORSE as the decomposition succeeds, and that
> is arithmetic rather than regression.** `closure_size` counts CRATES. Splitting
> a monolith into a sibling the facade still pulls adds one; it does not add a
> line of linked code, and it is the work this program asked for. ⚠ **The
> consequence is that the pair cannot be read as a progress metric in the
> direction everyone will read it.** A reviewer seeing 22 where the page said 18
> will infer the footprint got worse, and what actually happened is that four
> domains became separately nameable.
>
> ⭐ **The number that WOULD mean progress is a crate LEAVING the closure, and
> that is a different act from carving.** It has happened and the log names it —
> `ab99e70aa` *"Portals leave the movement-only closure too: 43 -> 42"* — and
> `51600d168` records five leaving unnoticed because the ratchet *"only ever
> watched crates ENTER"*. ⇒ A carve makes a domain nameable; only cutting the
> facade's edge to it makes the footprint smaller. Both are needed and the
> ratchet counts only the first.
>
> ⛔ **RETRACTED 2026-09-03, SAME DAY, BY THE FILE I WAS ALREADY READING.** This
> block first claimed the lever was the facade's dependency list — that 21 of the
> 22 `never_asked_for` crates are named directly in
> `crates/ambition_platformer2d/Cargo.toml`, that 10 of them are unconditional
> there, and that making those optional would move the number. **The count is
> right and the conclusion was wrong.**
>
> `reachable_via_ambition_platformer2d_actor_monolith_alone` in the baseline JSON
> holds **all 22**. The sentinel links `ambition_platformer2d_actor_monolith`, and
> the monolith's own manifest names every one of them unconditionally — so each
> facade edge is REDUNDANT, and cutting it prints the same 49/22. The baseline
> even says so in a row I did not open: *"cuttable at the facade, worthless to
> cut, because the monolith brings them regardless"*
> (`damage_and_mount_classified_2026_09_02`). ⚠ **I read `never_asked_for` and
> `ambition_closure` out of that file and stopped at the two keys my hypothesis
> needed** — which is the failure mode this repo's own recipe warns about, one
> level up: not a missing instrument, an instrument read only as far as it agreed.
>
> ⇒ **What survives, and it is the useful part:** the +4/+4 identity above, and
> that the measurement is already minimal (`fixtures/minimal_game` uses
> `default-features = false`; the contract asks cargo's resolver, not a source
> walk). All 22 are linked by a game asking for as little as the facade permits,
> and no feature flag on the facade changes that.
>
> ⛔ **BUT THE FACADE EDGE IS NOT THE LEVER FOR THOSE TEN, and the baseline
> already says so** (`damage_and_mount_classified_2026_09_02`: *"cuttable at the
> facade, worthless to cut, because the monolith brings them regardless"*). Every
> one of the ten is an UNCONDITIONAL dependency of
> `ambition_platformer2d_actor_monolith`, which the sentinel links through the
> facade whatever the facade's own list says — the baseline's
> `reachable_via_ambition_platformer2d_actor_monolith_alone` list is the proof,
> and it is the same set. Making the facade's `ambition_match` edge optional would
> print 49/22 again. ⇒ The step that moves the number is the one the mount and
> damage rows name: *the closure should follow the plugin a game INSTALLS, not the
> dependency its crate declares* — which means the KERNEL's use of each domain
> becoming optional, and that is this program's stated non-goal ("scatter feature
> gates through the kernel merely to move a `cargo tree` number") until the
> domain's construction road has left the kernel as well. Carves land
> unconditional, by design, until then. (Ruled 2026-09-03 when the facade
> demonstration was offered.)
>
> ⇒ **This also explains the +4/+4 exactly.** `held_items`, `body_seed`, `match`,
> `encounter_features` and `world_items` are all recent carve outputs, and every
> one was added to the facade as an UNCONDITIONAL dependency. The carve creates
> the crate; the facade then names it the only way that guarantees a minimal
> game links it.
> ⛔ **DO NOT RETYPE IT — re-derive:**
> `python3 scripts/check_absence_contracts.py | grep footprint`, which prints
> the live pair from `scripts/baselines/capability-footprint-baseline.json`.
> ⚠ It has now drifted FOUR times — 43/16 → 44/16 → 44/17 → 45/18 — and the
> fourth happened INSIDE the edit that corrected the third, hours apart. A
> hand-copied ratchet value is a claim with nothing holding it, which is the very
> failure §2e is about; the command is the only form of this fact that stays
> true. It ratchets — a new crate entering the
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
- ✔ **CLOSED 2026-09-03 — the two lists are the same set again, and it now
  says something stronger.** `ambition_damage` and `ambition_mount` entered the
  closure on 2026-08-26 after the
  `reachable_via_ambition_platformer2d_actor_monolith_alone` list was written and
  were classified at `f1445c142`; the drift in the OTHER direction was found the
  same week and was larger — five crates that had LEFT the closure were still
  listed as reachable, because the ratchet reports crates that ENTER and nothing
  looked the other way. Pruned against a live re-measurement (46, zero entered,
  zero left), and guarded by
  `scripts/tests/test_capability_footprint_baseline_is_coherent.py`.
  ⇒ **All 23 that a movement-only game never asked for arrive through the
  monolith alone** — the two lists are literally equal — so no facade cut
  removes a single one. (19 when this was written on 2026-09-03; re-derived the
  same day at 23 after `ambition_abilities` and `ambition_encounter_features`
  landed. ⚠ The NUMBER moves with every carve; the EQUALITY is the claim, and
  `test_capability_footprint_baseline_is_coherent.py` is what keeps it honest.) `ambition_render` is the only crate reachable only
  through the facade, and it is asked for (90 in-repo call sites). Both of the
  2026-08-26 pair behave like `ambition_audio`: cuttable at the facade,
  worthless to cut.

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

### Carve recorded 2026-09-04 — `ambition_sprite_fx`, capability independence: **IMPROVED**

A new render-floor crate, made while giving the portal gun per-gun colours, and
recorded here because the doctrine asks every carve to say which way it moved
capability independence rather than only authority.

⭐ **What it is.** `SpriteEffect` — one component naming "this sprite, with one
simple visual manipulation": `Tint` (multiply), `HueShift`, `Saturate`,
`Silhouette`. The engine had **four** unrelated implementations of that idea and
no name for it: `Sprite.color`, the projectile catalog's `EnergyTinted` art
source, the hit-flash silhouette overlay, and `PortalClipMaterial`'s `tint`
uniform. A fifth caller had to pick one and copy it.

⇒ **IMPROVED, on each of the doctrine's four questions:**

- **Can it be absent?** Yes. Nothing depends on the effect existing; a caller
  that never adds `SpriteEffect` never pays for it, and a build without
  `SpriteFxPlugin` still compiles and draws every sprite.
- **Does the rest cohere without it?** Yes — it is additive. It was carved with
  a `pub use` where its one moved item used to live (`sprite_frame_basis` /
  `SpriteFrameBasis`, which the portal crate wrote and never owned), so no
  existing caller changed.
- **Does it declare only real prerequisites?** ⭐ **It declares NO `ambition_*`
  dependency at all** — bevy's 2D render, asset and image features and nothing
  else. It is the first crate in this workspace that a foreign game could take
  on its own.
- **Is the composition a convenience?** Yes. `SpriteFxPlugin` installs its own
  systems and its own material; `ambition_render` adds it in one line and
  initialises nothing on its behalf — the shape
  `capability-and-runtime-composition.md` asks for and the opposite of the 40
  `init_resource` calls the runtime makes for other crates.

⚠ **Where it sits, and why it is not somewhere cheaper.** Not in
`ambition_platformer2d_shared_tangle`: that crate refuses render features on
purpose (*"No render, audio, or asset features — this crate stays reusable and
headless"*) and a `Material2d` cannot be declared without them. Not in
`ambition_portal2d_presentation`: `ambition_render` depends on THAT crate, so a
general facility has to sit at or below it, and a sprite effect is not a portal
concept. ⇒ The render floor was genuinely empty and the crate is what fills it.

⚠ **What it did NOT do.** It did not migrate the four existing tint
implementations onto itself. Each is load-bearing and differently shaped — the
hit-flash overlay is a sibling mesh with its own pulse schedule, `EnergyTinted`
is an art-source variant resolved at spawn — and converting them is a separate
carve with its own before/after. The duplication is named here so the next
person does not have to rediscover that it is four things.

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

⛔ **CORRECTION, same day — my first causal split here was wrong, and the
repository already had a better one.** I wrote that six of the ten "arrive
through a capability it DID request", because `ambition_render`'s manifest names
them. That is one path among several and it is not the binding one:
`cargo tree -i ambition_cutscene` in the sentinel shows it arriving through
`ambition_boss_encounter` → `ambition_damage` → the facade AND the monolith.
Removing render's edge would not remove it.

✔ **The authoritative instrument is `scripts/baselines/capability-footprint-baseline.json`,
guarded by the `capability-footprint-may-not-grow` absence contract**, and its
split is by REACHABILITY rather than by manifest:

* **20 reachable via `ambition_platformer2d_actor_monolith` alone** — irreducible
  without moving code;
* **4 reachable only through the facade** (`ambition_inventory_ui`,
  `ambition_portal2d_presentation`, `ambition_render`, `ambition_touch_input`) —
  closable by making facade edges optional, a manifest change.

Its own note states the conclusion I arrived at independently and less precisely:
*"The second list can be closed by making facade edges optional — a manifest
change. The first cannot: a game that needs actors needs
`ambition_platformer2d_actor_monolith`, which brings them. That is what makes the
footprint irreducible without moving code."*

⇒ **So this pressure point is already measured, already guarded, and already
kept current** — the baseline records `registry_core_entered_the_closure_2026_09_03`,
the same day that edge landed. Anyone re-measuring should run
`scripts/check_absence_contracts.py` rather than a fresh `cargo tree`.

⭐ **What an independent measurement DID add**, having agreed with the baseline
at the `closure_size` of the day (45 then; 46 since `bbfa38a3d` added
`ambition_held_items` — the pair drifts with every carve, so
`python3 scripts/check_absence_contracts.py | grep footprint` prints the live
one rather than this sentence): the closure is insensitive to what a consumer
asks for.
`fixtures/external_consumer` requests TWO capabilities and
`fixtures/minimal_game` requests ONE, and both inherit the SAME ten optional
capability crates. The baseline uses only the one-capability sentinel, so this is
the half it does not cover.

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
