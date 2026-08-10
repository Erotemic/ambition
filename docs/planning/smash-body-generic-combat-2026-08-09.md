# Smash is the engine test, not a mode with exceptions — campaign, 2026-08-09

**Authority: Jon, 2026-08-09, verbatim below.** This file is the charter for the
combat campaign. `docs/concepts/one-body-one-path.md` is the durable concept and
records what is unified TODAY; this file records what is still owed and in what
order. When a roadmap item lands, move its statement into the concept doc and
strike it here.

The one-sentence version, in Jon's words:

> **Smash is not a special mode that gets exceptions. It is the engine test
> proving that Ambition works when there is no privileged protagonist.**

and the framing that decides every judgement call in this campaign:

> getting Smash working is **not permission to add Smash-specific exceptions**;
> Smash is the test case that should force the engine toward the body-generic
> architecture you wanted from the beginning.

---

## Where this stands before the campaign starts (2026-08-09, measured)

Three commits landed the first slice, and the concept doc already carries them:

| commit | what |
|---|---|
| `78fffd933` | body melee hit resolution unified — one victim loop, owner exclusion, relationship policy, published hurtbox geometry, per-hitbox dedup, victim-specific knockback |
| `aa52b3cce` | combat geometry and facing made body-generic — `ambition_sim_view::CombatGeometryView`, F1 reads it, movement stopped reversing facing on velocity loss, `BodyWallState` exposed to brains |
| `af5dd1ced` | pogo resolves from `LandedBodyHit`; world rebound stays a separate world-contact path; rollback schema bumped |

So roadmap items 4 (resolve overlap once) and 8 (read models read-only) are
**partly** discharged for melee/pogo and **not** discharged generally. Items 1,
2, 3, 5, 6, 7 are untouched. The feel layer (hitstop, hitstun, landing lag,
aerial policy, move-facing snapshot) is entirely unbuilt.

---

## Jon's brief, verbatim

> You are taking over an ongoing Ambition engine/Smash combat refactor. Work directly from the **current repository state**, including any intentional uncommitted work. Inspect the latest commits and working tree before changing anything. First compile and repair the current work rather than reverting architectural changes just to get green tests.
>
> The recent work has deliberately moved Ambition away from a privileged-protagonist architecture. Preserve and extend that direction.
>
> Current intended invariants:
>
> * Human-controlled, CPU-controlled, possessed, scripted, remote, and RL-controlled bodies all use the same simulation/combat paths.
> * Controller kind supplies intent. It does not choose physics, damage, collision, targeting, or debug behavior.
> * No engine system should require a `PrimaryPlayerOnly`/privileged protagonist in order to function.
> * A live moveset hitbox is authoritative gameplay geometry.
> * F1/debug combat geometry must visualize that same authoritative geometry, not reconstruct approximations from animation/read-model state.
> * Collision reports semantic facts such as side contact; autonomous control policy decides whether to turn around.
> * Body strikes are resolved once into an identified attacker/victim contact. Damage, hit confirms, and on-hit effects consume that fact instead of independently rediscovering overlap.
> * Ordinary combat bodies must not be flattened into anonymous world collision objects in order for combat systems to interact with them.
> * Genuine world rebound/pogo surfaces remain a separate world-contact concept.
> * Prefer affirmative body-generic abstractions over adding Smash-specific branches.
>
> Recent refactors that should remain conceptually intact:
>
> * Player/body melee was routed through the same direct victim resolver as other bodies, fixing self-hit and preserving authored knockback.
> * Body-generic combat geometry was added to `ambition_sim_view`, and Smash's F1 visualization was connected to it.
> * Movement integration stopped implicitly reversing facing based on velocity loss; semantic wall facts were exposed to autonomous brains instead.
> * Synthetic `BodyMelee` attack geometry was removed from F1; live `Hitbox::world_volume()` is the attack geometry.
> * Damage resolution stopped requiring the derived `BodyMelee.swing` projection.
> * Pogo/on-hit work is being refactored so a shared `LandedBodyHit` fact drives body on-hit effects, while explicit world rebound surfaces use the world geometry path. The rollback schema was bumped accordingly.
>
> **First task: stabilize the current tree.**
>
> Compile the affected crates and run the focused tests. Repair any compile failures, rollback/schema errors, scheduling problems, test-fixture mistakes, or behavior regressions you find. Do not weaken meaningful regressions to make them pass. If a fixture is unrealistic, make it model production construction correctly.
>
> Do not run or recommend `cargo fmt` commands or git-diff checking commands.
>
> Pay particular attention to these behaviors:
>
> * attack volume can hit a victim while attacker and victim collision bodies remain separated;
> * attacker never damages itself;
> * authored launch direction/base/growth survive the complete path;
> * a down-air in empty space never pogo-bounces from its own body;
> * a down-air that actually lands on another pogo-enabled body can bounce;
> * explicit world rebound surfaces still work;
> * repeated active ticks do not repeatedly damage/pogo the same victim unless the move intentionally supports multihit;
> * F1 shows the exact authoritative strike/hurtbox geometry;
> * attacking or stopping movement does not unexpectedly reverse facing;
> * changing a body's controller from human to CPU does not alter combat physics.
>
> After the current work is stable, continue the larger combat cleanup. The desired end state is approximately:
>
> ```text
> input/control intent
>         ↓
> MovePlayback / move timeline
>         ↓
> authoritative Strike
>         ↓
> shared body contact resolver
>         ↓
> LandedBodyHit / ResolvedBodyHit
>    attacker
>    victim
>    cause
>    strike data
>    contact
>         ↓
>    ┌───────────────┬────────────────┬──────────────┐
>    ↓               ↓                ↓              ↓
> damage        HitReaction       hit confirm    on-hit effects
>                                  / cancels      / pogo / etc.
> ```
>
> World attacks/contact should have an equally explicit path rather than masquerading as unidentified body hits.
>
> **Complete the following architectural roadmap.**
>
> 1. **Eliminate legacy knockback side channels.**
>    Remove `PlayerSlash { knock_x }` as a physics channel. Migrate dive or any remaining users to the same explicit typed knockback/reaction representation used by normal moveset strikes. A successful authored launch strike must not be representable as a hit with silently missing knockback.
>
> 2. **Make hit cause independent of architectural direction.**
>    `PlayerSlash`, `EnemyAttack`, `BossAttack`, and similar vocabulary currently encode old player-versus-world routing assumptions. Move toward a cause vocabulary such as melee/projectile/contact/hazard plus explicit attacker/victim identity. Controller/faction should not determine which damage consumer handles an event.
>
> 3. **Generalize body targeting.**
>    Prefer an explicit body victim such as `HitTarget::Body(Entity)` / `ResolvedBodyHit { victim }` over `Player(Entity)` versus `Actor(Entity)`. Retain volume/world targeting only where a target genuinely has not yet been resolved.
>
> 4. **Resolve overlap exactly once.**
>    Damage, pogo, lifesteal, status effects, hit confirms, etc. should not each rescan attack geometry against victims. The combat resolver decides that a strike landed and publishes a resolved fact. Downstream effects consume it.
>
> 5. **Unify relationship semantics.**
>    At present MatchTeam/faction semantics have historically differed between “may damage” and AI target selection. Extract one combat-relationship policy used by both targeting and damage:
>
>    ```text
>    attacker + candidate + match rules
>              ↓
>         ally / foe / neutral
>    ```
>
>    Match/team semantics should outrank authored world faction where appropriate for a match.
>
> 6. **Remove Smash seat-parity faction hacks.**
>    Smash should not need `Player, Enemy, Player, Enemy` faction assignment so everybody can fight. FFA and teams should be represented by `MatchTeam`/match relationship state. Authored faction should remain an independent world/character property.
>
> 7. **Move authoritative combat types out of VFX.**
>    If `Hitbox`, knockback, launch direction, damage geometry, owner identity, etc. still live in `ambition_vfx`, move simulation-authoritative vocabulary into `ambition_combat`. VFX should consume presentation observations/events, not own gameplay truth. Avoid introducing dependency cycles; make the ownership boundary clean.
>
> 8. **Keep read models read-only.**
>    `BodyMelee`, animation pose projections, HUD state, debug views, etc. may observe authoritative combat state but must never gate whether combat occurs. Search for any remaining places where read-model/presentation state acts as gameplay authority and remove that coupling.
>
> **Then improve the combat architecture specifically so Smash starts to feel good rather than merely function.**
>
> I recommend implementing these as coherent engine concepts rather than one-off character tweaks:
>
> * **Move-facing snapshot / attack orientation authority.** Capture the intended attack orientation when appropriate at move startup, rather than allowing mutable locomotion facing to accidentally flip an active hitbox. Moves that intentionally track/turn can opt into that explicitly.
> * **One authoritative move timeline.** Startup, active windows, recovery, cancel windows, and any per-tick hitbox keyframes should come from `MovePlayback`/moveset authoring. Derived `BodyMelee` state should only project that timeline.
> * **Hitstop/hitlag.** Add deterministic short hit pause for attacker/victim, authored or derived from hit strength. It should pause the appropriate move/body simulation without becoming a presentation-only effect. This will dramatically improve perceived impact.
> * **Explicit hitstun/reaction state.** Separate “velocity was changed” from “this body is in hitstun/tumble.” A typed `HitReaction` should carry launch plus reaction duration/state. Movement control should know when authority is suppressed or reduced.
> * **Landing behavior for aerial moves.** Add authored landing lag and auto-cancel windows. Landing in an aerial's recovery should not be identical to ordinary landing unless the move says so.
> * **Aerial locomotion as an explicit policy.** Make air drift, acceleration, maximum air speed, fast-fall, and momentum conservation clearly authored body/movement capabilities rather than accidental reuse of ground locomotion behavior.
> * **Input buffering and jump startup/jump-squat semantics.** Platform-fighter controls benefit heavily from deterministic short buffering for jumps/attacks and an explicit jump-start phase. Keep this at the intent/action boundary rather than special-casing input devices.
> * **Hitbox tracks rather than one coarse box where useful.** Support several authored shapes/keyframes across an attack's active portion. F1 must display exactly whichever shape is live.
> * **Pose-aware hurtboxes where justified.** Combat bodies should be able to expose authored hurtbox changes during crouch/jump/attacks without changing their fundamental body identity or collision envelope.
> * **Knockback feel as one system.** Base knockback, growth with percent, weight response, launch angle, hitstun and hitstop should flow through a single calculation. Avoid source-specific formulas.
> * **Directional influence as a later layer.** Once launch/hitstun are clean, add DI/launch influence in the reaction phase rather than contaminating strike resolution.
>
> Do not rush into shields, grabs, parries, ledges, or a huge move roster until movement, strike contact, hitstop, launch, hitstun, aerial control, and landing behavior feel coherent. Those are higher-value foundations.
>
> **Improve observability alongside the simulation.**
>
> F1 should be useful enough to tune combat without reading logs. Prefer extending the shared simulation-view/debug system to expose:
>
> * exact body collision envelope;
> * exact effective hurtboxes;
> * exact live strike volumes;
> * current move and startup/active/recovery phase;
> * facing/attack orientation;
> * current percent/damage;
> * launch vector from the most recent resolved hit;
> * hitstop/hitstun state;
> * semantic grounded/wall/contact state.
>
> Do not make any of those require a designated primary protagonist.
>
> **Regression matrix / definition of done**
>
> Add or preserve tests for:
>
> * no body can hit itself with normal melee;
> * no body can pogo from itself;
> * separated collision bodies can hit when strike geometry reaches the victim;
> * exact authored strike geometry drives both F1 and damage;
> * controller substitution human ↔ CPU leaves the same combat outcome;
> * 4-way FFA: every distinct opponent relation is targetable/damageable;
> * 2v2: allies and foes are consistent between target selection and damage;
> * multihit/dedup behavior is explicitly authored rather than accidental;
> * startup/active/recovery timing comes from one move timeline;
> * hitstop and hitstun are deterministic;
> * landing-lag/autocancel tests for aerial attacks;
> * actual wall contact is a fact; turning is control policy;
> * engine debug systems work with zero, one, or many human-controlled bodies;
> * ordinary combat bodies never need anonymous world projections for body combat semantics.
>
> Prefer end-to-end slice tests where practical: input/move playback → live strike → contact resolution → damage/reaction/on-hit, rather than synthesizing legacy halfway-state events.
>
> Run targeted `cargo test` and `cargo check` commands as you work, plus the repository architecture/absence-contract checker where relevant. **Do not run or recommend `cargo fmt` or git-diff checks.**
>
> If rollback-visible types/components/events change, update rollback registration, checksums, mapping, schema evidence, and schema version coherently. Same-frame derived events that should not survive load should be modeled explicitly that way rather than accidentally serialized.
>
> Keep changes elegant even if that means touching several crates. Do not preserve a bad boundary merely to minimize lines changed. Conversely, do not rewrite unrelated systems just because they are nearby.
>
> The guiding rule is:
>
> **Bodies own physical/combat state. Controllers provide intent. Moves author strikes. One resolver establishes contact. Resolved combat facts drive reactions and effects. Match relationships determine opponents. Presentation observes all of this without controlling it.**
>
> Smash is not a special mode that gets exceptions. It is the engine test proving that Ambition works when there is no privileged protagonist.
>
> When finished, summarize:
>
> * architectural changes;
> * remaining legacy seams you intentionally left;
> * feel changes visible in Smash;
> * tests/checks run and their results;
> * any follow-up work you think still has unusually high leverage.

And Jon's own ranking of the additions beyond the named roadmap:

> A few of those additions go beyond the roadmap we had explicitly named. The ones I think have the **highest extra leverage** are the move-facing snapshot, authoritative move timeline, hitstop/hitstun, and landing-lag/autocancel model. Those are places where good architecture and good platform-fighter feel align unusually well: each removes ambiguous ownership from the code while also making attacks feel much more deliberate.
>
> I'd also strongly encourage the agent to build the **combat debug view as a tuning instrument**, not just a collision-box renderer. Once you can see `startup → active → recovery`, current strike shape, launch vector, hitstop and hitstun directly in F1, combat iteration becomes much faster and hidden spaghetti becomes much harder to maintain.

---

## ⭐⭐ A GENRE'S MECHANICS ARE NOT A QUESTION FOR JON (2026-08-09, verbatim)

> note that the feel goal is to make it feel like smash and the concepts of what
> the engine needs to do things like hitstun, knock back, techs, and other smash
> things are things that are objective and you should not need my input. You
> should be able to write it so I can tweak numbers and get the right feel, but
> smash games are so well documented getting the framework to make them work
> right and elegantly in this engine should not need my input.
>
> and the which game was the death in issue has been resolved. Note the same
> thing goes for mario (maryo). the mechanics of the game are standard.

⛔ **So "what should hitstun do?", "how does teching work?", "how much knockback
growth?", "what does a Mary-O block do when struck?" are RESEARCH, not decisions
owed to the maintainer.** The genre is documented; look it up, implement the
standard mechanic, and expose the numbers as authored tuning so Jon can dial
feel without touching structure.

⇒ **the deliverable shape this implies**: every feel quantity is an authored
number in one place, with a defensible platform-fighter default already in it.
A mechanic Jon cannot retune by editing a value is not finished.

⇒ and **D68's "which game was the death in" is CLOSED** — it was the only
blocked-on-Jon item that blocked work.

## Standing constraints for this campaign

- ⛔ **No `cargo fmt` and no git-diff-checking commands.** Jon said it twice.
  Format a single file with `rustfmt` if a file needs it
  (`rustfmt --config skip_children=true <file>`), never the workspace command.
- ⛔ **Do not weaken a meaningful regression to get green.** If a fixture is
  unrealistic, make it model production construction.
- ⛔ **Rollback coherence is part of every change**: registration, checksums,
  mapping, schema evidence, schema version, together in one commit. A same-frame
  derived event that must not survive a load is modelled as such, not
  accidentally serialized.
- ⭐ Prefer the end-to-end slice test (intent → playback → strike → contact →
  damage/reaction) over synthesising a halfway-state event.

## ✔ Step 0 — STABILIZE: done 2026-08-09

`app_it` was **red on three tests**, and they had one cause.

> `boss_contact_iframes::face_tanking_player_swings_back_and_is_recoil_locked`
> — *"frames a swing reached the boss: 0"* in 300 frames ·
> `rollback_exit_oracle::combat_equipment_switch_and_breakable_survive_forced_rollback_identically`
> — *"the brick was never broken in 2400 frames"* ·
> `rollback_lifecycle_reset::a_player_death_reset_survives_the_rollback_window`.

⭐⭐ **the melee unification took away the strike's reach to everything that is
not a combat body.** Before it, a player `FollowOwner` strike emitted one
broadcast `HitTarget::Volume` event and `apply_feature_hit_events` fanned it out
over actors, bosses and breakables. After it, the strike resolves bodies by
identity — and a boss keeps HP/phase on an encounter, a breakable is a feature,
so neither matches `StrikeVictim` and neither could be hit at all. Combat bodies
were fine, which is why 320 tests stayed green and the three that failed were
the only ones that hit a *non-body*.

⛔ **the fix is not the broadcast back.** The strike now publishes its unresolved
half as `HitTarget::UnresolvedFeatures` — an explicit "the bodies are already
resolved; scan only what a body resolver cannot name" — and the feature consumer
skips its actor scan for it, so a body cannot take a second anonymous copy of a
hit it was already named for. This is roadmap item 3's own sanctioned remainder
(*"retain volume/world targeting only where a target genuinely has not yet been
resolved"*), and the variant is scaffolding: it retires when bosses and
breakables become victims in their own right.

Two details worth keeping:

- dedup rides `MovePlayback.hit_targets`, the move's authoritative accumulator —
  **not** the `BodyMelee.swing` projection that used to gate the old emit. That
  gate was the read-model-as-authority bug item 8 names, and it is not restored.
- the guard is `a_body_owned_strike_publishes_its_unresolved_half_beside_the_resolved_body_hit`
  plus a **poison** asserted in four tests: a body-owned melee must never emit a
  `HitTarget::Volume`, because that is the exact event shape that re-scans bodies
  and, historically, let a swing reach its own owner.

Verified: `ambition_combat` 149/149 · monolith 1195/1195 · smash + mary-o +
runtime green · absence contracts 25/25 · **`app_it` 323 passed, 0 failed**.

## ✔ Roadmap item 1 — the legacy knockback side channel is gone (2026-08-09)

`HitSource::PlayerSlash { knock_x: f32 }` is now a unit variant. What the
measurement found, before any code moved:

| | |
|---|---|
| sites spelling `knock_x` | 33 across 10 files |
| producers that set it non-zero | **1** — the dive corridor, `local_dir.x * 1.4` |
| every other producer | `knock_x: 0.0`, written only to satisfy the pattern |
| what the consumer did with it | took `knock_x.signum()` and substituted `FeelScale(1.0)` |

⭐⭐ **so the one authored magnitude on the channel never reached a victim.**
`DIVE_KNOCKBACK = 1.4` had no effect on anything, and the hit was
indistinguishable from one with no shove authored at all — Jon's condition
exactly: *a successful authored launch strike must not be representable as a hit
with silently missing knockback.*

⚠ **the dive's shove is now 1.4× what shipped.** That is the authored number
finally being used, not a new one, and it is a named constant if it wants
tuning.

The guard is in `dive/tests.rs` and asserts the **magnitude**, not merely that
something is attached — a sign-only channel would pass the weaker assertion,
which is how this survived. The actor consumer's synthesizing arm is deleted, so
there is nowhere left to put a magnitude that evaporates.

## ✔ Roadmap item 3 — one body victim, named by entity (2026-08-09)

`HitTarget::Player(Entity)` and `HitTarget::Actor(Entity)` are one variant,
`HitTarget::Body(Entity)`.

⭐ **the split was a routing artifact, not a fact about the hit.** Every one of
the five producers computed it the same way — `if victim.is_player { Player }
else { Actor }` — so the stamp said nothing the victim entity did not already
say, and it said it at the moment a producer is least entitled to care. Its real
job was telling two consumers which one owned the event.

⇒ each consumer now asks the world instead of reading the stamp. The
controlled-body FIFO stages a resolved hit when the victim **is in its
population** (`With<PlayerEntity>`); the actor consumer matches its own entity.
That deleted, as dead weight, a `Has<PlayerEntity>` query column in two systems
and a `target_is_player: bool` parameter threaded through `ContactAttack`.

⚠ **one fixture was modelling a body production never builds** — it spawned a
bare entity and stamped it `HitTarget::Player`, which worked only because the
stamp was the whole claim. It now carries `PlayerEntity`, and a second
body-targeted hit on a body the resolver does *not* own is the poison beside it.

Verified: `ambition_combat` 149/149 · monolith 1195/1195 · absence contracts
25/25 · `app_it` 323 passed, 0 failed.

## ▢ Roadmap item 2 — the cause vocabulary, measured 2026-08-09 before touching it

⭐⭐ **the measurement that makes this safe, and it was not obvious**: after items
1 and 3, **every victim-side source in the tree already resolves to
`HitTarget::Body`.** Producer-by-producer:

| source | target it stamps |
|---|---|
| `ContactHarm`, `EnemyBody`, `EnemyProjectile`, `Hazard`, `EnemyAttack`, `BossAttack` | **`Body`** — resolved |
| `PlayerSlash` (5), `PlayerProjectile` (4), `EnemyChargeCrash` (2) | `Volume` — unresolved |

⇒ **`is_attacker_side` is consulted at exactly three sites and only ever for an
UNRESOLVED event**, and every unresolved producer in the tree is attacker-side.
So the predicate's victim-side branch is already dead for real traffic; the
direction words are load-bearing for nothing but legacy and test events. That is
what licenses the fold — without this table, collapsing `PlayerProjectile` and
`EnemyProjectile` into one `Projectile` would silently reroute hits.

**The vocabulary**: `Melee` · `Projectile` · `Contact` · `Hazard` ·
`LeftTheWorld` · `Pogo`. The HUD/trace distinction worth keeping (was it a swing
or a shot?) survives as `Melee` vs `Projectile`; the direction half of each name
is what goes.

⛔ **the one real casualty is `boss_hit`, and it must not be papered over.**
`matches!(source, BossBody | BossAttack)` currently selects heavier knockback and
a longer hitstun (`feel.boss_hitstun_time`), and the folded vocabulary cannot
express it. **Do not add a flag to the event** — that reinvents the side channel
item 1 just deleted. The fact belongs to the ATTACKER, the event already carries
`attacker: Option<Entity>`, and the consumer can ask the attacker whether it is a
boss. That also discharges a piece of the *knockback feel as one system* item:
one fewer source-specific formula.

⇒ **so item 2 lands as: rename the variants, rename `is_attacker_side` to say
what it now decides (does this unresolved volume hunt for victims?), and
re-source boss strength from the attacker entity.** Renaming without the last
part would be a half-fold that leaves `BossAttack` alive under a new name.

### ✔ item 2, step 1 — heavy strength left the vocabulary (2026-08-09)

`boss_hit` is no longer `matches!(source, BossBody | BossAttack)` at either
consumer. Both ask the attacker entity the event already names.

* ⛔ **`HitSource::BossBody` had ZERO producers** — a boss's body contact was
  always filed as `EnemyBody` by `ContactAttack`, so the "boss touched you"
  spelling described traffic that did not exist and the heavier feel only ever
  came from a boss *swing*. The variant is deleted.
* ⚠ **BLIND consequence, and it is a fix**: because heaviness now comes from the
  striker, a boss **body check** lands with boss weight for the first time. Two
  years of `EnemyBody` said otherwise only because nobody could spell it.
* the guard lands the **same `EnemyAttack` source twice from two attackers** and
  asserts the hitstun differs — a test that reads the cause cannot pass it. Its
  poison is the ordinary attacker, which must not inherit the heavy duration.

▢ **what remains of item 2**: the rename itself, and `is_attacker_side` →
a name for what it now decides.

## Execution order (mine, revise as measurements land)

0. ~~**Stabilize** — compile the affected crates, run the focused suites,
   repair.~~ ✔ done, above.
1. **1 + 2 + 3 together** — the knockback side channel, the cause vocabulary and
   the body victim are the same refactor seen from three sides; splitting them
   means migrating call sites twice.
2. **5 + 6** — one combat-relationship policy, then Smash's seat-parity faction
   hack deletes itself.
3. **7** — authoritative combat vocabulary out of `ambition_vfx`.
4. **Move-facing snapshot** and **one authoritative move timeline** — Jon's two
   highest-leverage feel items and both are ownership fixes.
5. **Hitstop/hitlag + typed `HitReaction`/hitstun.**
6. **Landing lag / autocancel**, then **aerial locomotion policy**, then input
   buffering / jump-squat.
7. **F1 as a tuning instrument**, extended as each of the above lands — the
   phase readout is worth building as soon as item 4 gives it a timeline to read.
8. Hitbox tracks, pose-aware hurtboxes, one knockback calculation, DI last.

⛔ shields, grabs, parries, ledges and roster breadth are explicitly deferred.
