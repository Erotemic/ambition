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

## ✔ Roadmap item 2 — the cause vocabulary — CLOSED 2026-08-12

⭐ **all three parts of what this item said it "lands as" are in the tree**,
verified rather than assumed:

* **the variants are renamed.** `HitSource` is `Melee` · `Projectile` · `Contact`
  · `Hazard` · … — the fold this section licensed, with the swing/shot
  distinction kept because it is a real difference in the world and the direction
  half gone because it was not.
* **`is_attacker_side` DOES NOT EXIST.** Zero references in the workspace, tests
  included; the only survivors are prose in doc comments recording the history.
* **boss strength is re-sourced from the ATTACKER.** `damage_apply` takes
  `heavy_attacker: bool`, documented as *"asked of the attacker entity"* — not a
  flag on the event, which this section explicitly forbade as reinventing the
  side channel item 1 deleted.

⇥ the original measurement follows unedited: it is the table that made the fold
safe, and a fold justified by a census cannot be checked without it.

### ▢ (as written 2026-08-09) — the cause vocabulary, measured before touching it

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

### ✔ item 2, step 2 — nothing but description is left keyed to the words

Three more things were reading the cause vocabulary to answer a question about
somebody's identity. All three now ask the entity.

| was | is | why it blocked the fold |
|---|---|---|
| `matches!(source, PlayerSlash)` → scale by the human's damage slider | the ATTACKER is human-controlled | one `Melee` would scale ENEMY damage by the player's multiplier |
| `source.defaults_to_primary_attacker()` → credit the primary player | the event is an **unresolved broadcast** | an enemy shot that named its victim and carries no entity owner would credit the player with the confirm and the hitstop |
| `is_attacker_side` | **`seeks_victims`** | the name said "player versus world"; the question is the event's RESOLUTION state |

⭐ the slider one is worth stating plainly: `matches!(source, PlayerSlash)` reads
as *"a player's slash"* and actually means *"a slash filed under the player-side
spelling"*. A possessed enemy's swing carries that spelling and an empowered
ally's does not — **the slider already reached the wrong strikes in both
directions**, before any fold. Its poison — an uncontrolled body swinging the
same cause is not scaled — was unassertable while the spelling was the claim.

⚠ two more fixtures were modelling swings nobody threw (`attacker: None` plus a
player-spelled source). Both name a real attacker now.

### ✔ item 2, step 3 — the fold landed (2026-08-09)

**Nine variants became four causes** — `Melee` · `Projectile` · `Contact` ·
`Hazard` · `LeftTheWorld` · `Pogo`. `PlayerSlash`/`EnemyAttack`/`BossAttack` →
`Melee`; `PlayerProjectile`/`EnemyProjectile` → `Projectile`;
`EnemyBody`/`ContactHarm`/`EnemyChargeCrash` → `Contact`; `PogoBounce` → `Pogo`.
The half of each old name that said *who* is gone; the half that says *what kind
of thing happened* — which the HUD, the trace and the victim's reaction all
genuinely want — is kept.

⛔⛔ **and the fold found a real hole the direction words had been plugging.**
`apply_feature_hit_events`' actor scan **never excluded the event's own
attacker**. It never had to: a body-contact hit was filed victim-side, the drain
skipped every victim-side broadcast, and a self-hit could not arise. Take the
direction out and the protection leaves with it. The rule that was actually
wanted — *identity beats every relationship rule*, the body resolver's own first
line — is now stated there.

⭐ **two tests said so before the code did**, which is the argument for folding
rather than renaming around the problem:

* `victim_side_enemy_body_hit_does_not_damage_features` passed with
  `attacker: None`. A hit with no attacker has no self to exclude, so the
  fixture could not express its own claim; it named the emitter and went red.
* `player_faction_shot_damages_an_overlapping_enemy_and_expires` asserted
  `PlayerProjectile` present **and** `EnemyProjectile` absent — one cause turns
  that into a claim and its negation, which is the honest signal that the SOURCE
  was never what it cared about. It asserts reach now.

⚠ **one honest gap surfaced and is left open, marked**: the player-faction
projectile branch still broadcasts `HitTarget::Volume` instead of naming its
victim, unlike its own enemy-faction branch. That is roadmap item 3's remainder
for projectiles, and the test asserts the current truth with a `▢` saying which
assertion should change when it lands.

Rollback: checksum tags collapse to 0–4 plus 10. `LeftTheWorld` deliberately
keeps its number rather than compacting — a gratuitous renumber of a surviving
variant is a diff nobody can review.

<!-- historical: the plan this discharged -->
~~**what remains of item 2 is the rename itself. It is unblocked and
mechanical:**~~
`PlayerSlash`/`EnemyAttack`/`BossAttack` → `Melee`, `PlayerProjectile`/
`EnemyProjectile` → `Projectile`, `EnemyBody`/`ContactHarm`/`EnemyChargeCrash` →
`Contact`, `PogoBounce` → `Pogo`, `Hazard` and `LeftTheWorld` unchanged.
⛔ **and one live blocker, found by asking rather than assuming**:
`hitbox::apply_hitbox_damage` publishes its unresolved half only for
`PlayerSlash`. Under one `Melee` that gate cannot be spelled — so the rename
forces the body-generic answer, every body melee reaching breakables and bosses.

⚠ **that is not safe today, and the reason is the interesting part.** The boss
scan in `apply_feature_hit_events` applies **no relationship policy at all** —
it damages any boss an attacker-side volume reaches. It gets away with it
because only the player may broadcast. ⇒ **the boss's "who may hurt me" rule is
currently encoded as "who is allowed to emit a broadcast"**, and making the
broadcast body-generic removes the rule without replacing it: every enemy swing
near a boss would free-hit it.

⇒ **so the order changes.** Item 5 (one combat-relationship policy) comes BEFORE
the rename, and its first concrete bite is giving the boss scan the same
`damage_lands_between` the body resolver already uses. Then the unresolved half
goes body-generic with a guard, then the rename is mechanical.

### ✔ item 5, first bite — the boss is adjudicated like any other body (2026-08-09)

`boss_damage_allowed` is a named function, not a closure, so the policy can be
stated and tested rather than inferred from the shape of an `if`. It applies
`damage_lands_between` with **effective** allegiance, so a possessed boss fights
as its driver's side.

⚠ **one correction to the paragraph above, from reading `can_damage` rather than
assuming**: the engine's rule is *different faction, or friendly fire* — so an
Enemy hitting a Boss was always legitimate under it, and this change does not
forbid it. What was actually missing is that the scan asked **nothing at all**.
The free-hit worry was overstated; the real defect is a victim class with no
relationship policy, which is a fork whichever way the answer comes out.

⛔ **breakables are deliberately left open.** A crate has no allegiance, and
inventing one so the code looks symmetric would be a worse fork than the
asymmetry.

The guard pins four answers: the shipped player→boss case, the **poison**
(same-faction with friendly fire off — which landed before, because the scan had
no opinion), the possessed attacker, and the unattributed broadcast that must
still land because a hazard carries no entity to adjudicate.

### ✔ item 5, second bite — the broadcast is body-generic (2026-08-09)

Every body-owned melee publishes its unresolved half now, not just the player's.
The gate that stood there was the stand-in for the boss's missing relationship
policy; with the policy real, the permission went.

⛔ **lifting it desynced the rollback suite, and my predicted cause was wrong.**
I expected the `BossEncounter` phase machine — Enemy→Boss damage is legal by
`can_damage` and had zero rollback mileage on it, which is a good story. The
per-component localizer named a different resource in one run:

```
type_name: "ambition_combat::events::PendingPlayerHitEvents"
frames 19, 20, 21 · Resimulation · count 1, xor differs
```

⇒ **`stage_player_victim_hit_events` was staging the unresolved half into the
player-victim FIFO.** Its fallback arm reads `!seeks_victims()`, and an enemy
swing's cause is filed victim-side by the direction words — so a broadcast that
names no body was being queued as a hit ON the player, in rollback-registered
state. Barred explicitly: an `UnresolvedFeatures` event can never be a hit on
this resolver's population.

⭐⭐ **the lesson is the instrument, not the bug.** Two plausible stories, one
run, and the answer was neither of the two components I would have opened first.
`which_component_does_the_lifecycle_reset_divergence_live_in` is now in
`rollback_lifecycle_reset.rs` — `#[ignore]`d for cost, with the vacuity guard
its sibling has, and it is the first thing to run when that module goes red.

⇒ **the cause-vocabulary rename is unblocked.**

## ✔ items 5 + 6 — one relationship policy, and the seat-parity hack is deleted (2026-08-09)

⭐⭐ **the two rules had drifted exactly as Jon said, and the drift had a patch
holding it together.** "May this damage land" read faction difference with a team
override. "Is this worth chasing" read the `FactionRelations` hostility matrix
and **had never heard of a match team**. So a roster whose fighters share an
authored faction was damageable and untargetable at the same time — every CPU
seat would stand and stare — and the fix that had been applied was
`faction_for(index)`: `Player, Enemy, Player, Enemy` by seat, so the older of the
two rules would answer correctly.

`combat_relation` is the one policy both call. Precedence: **grudge → team →
faction**, allegiance effective on both sides.

⭐ **three answers, not two, and the third is the one a boolean could not
carry**:

| | targeting | damage |
|---|---|---|
| `Foe` — different team, hostile faction, or a grudge | chase it | hit it |
| `Neutral` — a different faction this one is not hostile *to* | leave it alone | **hit it** |
| `Ally` — same team or same faction | leave it alone | only with friendly fire |

Damage is **physical** (a swing that reaches a bystander hurts it); targeting is
**relational** (nobody goes hunting a bystander). Collapsing those is how a stray
hit stops landing, or a town NPC becomes prey. ⛔ this is why the damage side
passes `None` for the matrix rather than the live one — do not "fix" that.

**Item 6 falls out**: `faction_for(index)` is deleted. Every seat fights as
itself, and every seat carries a team — an authored one where given, otherwise
its own, which is the literal statement of free-for-all. Authored faction goes
back to meaning what the character says.

⚠ **one behaviour deliberately changed**: friendly fire now also frees same-TEAM
damage. The old team arm returned a bare *different team?* and ignored the
toggle, so a teams match could never turn friendly fire on — a real
platform-fighter setting, and the flag says what it means.

⭐ **the fold made the 2v2 vacuity guard stronger.** It used to demand teammates
hold *different* factions and opponents the *same*, so neither half of its loop
could be the faction rule in a team's clothes — and arranging that took the very
hack being deleted. Every seat is one faction now, so the faction rule has
exactly one answer for every pair in the match (**Ally**), and anything the loop
finds is the team rule in both directions at once. The seating test likewise
stopped asserting opposing factions and now asserts the OUTPUT: these two can
damage each other.

## ✔ items 7 + 8 — combat truth leaves the VFX crate (2026-08-09)

`Hitbox`, `HitboxAnchor`, `HitboxLifetime`, `HitboxHits`, `HitboxKnockback`,
`DamageBox`, `spawn_damage_box` and `apply_effects` now live in
`ambition_combat::strike`. A `Hitbox` carries damage, authored knockback, launch
direction, owner identity and the per-strike dedup set — a presentation crate
owning that is how a read model ends up gating whether combat happens.

⛔⛔ **the hoist was BLOCKED, and by an item-8 defect.** `ambition_render`'s
unauthored-volume stand-in held a `Query<(Entity, &Hitbox)>` — a render system
naming live simulation state, which `engine.render-never-names-live-sim-state`
exists to forbid — and it does **not** depend on `ambition_combat`. Moving
`Hitbox` would have forced a render → combat edge, which is the wrong direction
for the graph and the wrong direction for the architecture.

⇒ **so item 8 came first, and the read model it needed already existed.**
`CombatStrikeGeometryView` gained `strike`, `owner`, `anchored_to_body` and
`owner_anchor`; the render system reads the observation. ⭐ `owner_anchor` is the
subtle one: presentation draws at the PRESENTED pose, and the old code
re-evaluated `hitbox.world_volume(drawn)` to get there. Publishing the anchor
the volume was resolved against turns that into one translation, so an observer
never needs to re-run authoritative geometry to place a picture. (Those same
four fields are what F1's tuning readout will hang off.)

⚠ **what did NOT move, and why it is the orphan rule rather than taste**:
`HitSide` and the `Effect` / `EffectRequest` / `DamageBoxEffect` / `SummonSpec`
vocabulary stay in `ambition_vfx`, because `ambition_projectiles` names `Effect`
and sits BELOW `ambition_combat`. Hoisting the request seam would hand a
projectile crate a dependency on all of combat. The tag is a small enum on a
message now, with no authoritative components beside it.

⭐ **zero new dependency edges** — `capability-footprint-may-not-grow` still
reports 41 crates, and `ambition_render` no longer names anything from
`ambition_vfx` but the message vocabulary.

## The feel layer — measured 2026-08-09, and three of the four already exist

⛔ **grep before building.** Jon's four highest-leverage feel items are not four
green fields:

| item | what is actually there |
|---|---|
| **move-facing snapshot** | ✔ **already correct.** `MovePlayback` captures `facing` at move start, every strike volume mirrors through it, nothing writes it afterwards. What was missing was a GUARD — now `a_live_strike_keeps_the_facing_its_move_started_with`, whose vacuity half asserts the body genuinely turned. |
| **one authoritative move timeline** | ✔ largely there — `MovePlayback` owns startup/active/recovery windows and `BodyMelee` is already a projection of it (that was item 8's melee slice). |
| **hitstop/hitlag** | ✔ **now one law** — see below. ⚠ my first reading said "asymmetric, and an actor victim gets none"; the victim side was already body-generic (`apply_body_hit_reaction` is the ONE reaction). The real defect was two unscaled constants at two sites. |
| **landing lag / autocancel** | ✔ **built** — the one item that was genuinely absent. See below. |

### ✔ hitstun scales with the launch (2026-08-09) — the one that was WRONG

⭐⭐ **`reaction_scale` returned a flat `1.0` for every authored `LaunchSpeed`.**
So a jab and a fully-grown smash armed *identical* hitstun: a launched fighter
recovered as fast at 150% as at 0%, and the knockback growth the strike had
already computed reached the victim's VELOCITY and stopped there.

⇒ **there was no combo game, structurally.** Bigger launch → longer stun →
victim cannot act → follow-up connects is the whole platform-fighter loop, and
per Jon's ruling it is documented genre mechanics rather than a taste call.

The launch now scales against `STANDARD_LAUNCH_SPEED` (150.0, chosen from what
the tree authors — shipped melee bases sit in the 40–200 band), capped at
`MAX_HITSTUN_SCALE` (4.0) because past that a launch is a kill, not a starter.
Both are named constants on the tuning row: **Jon dials feel by editing a
number, which is the test of whether this is finished.**

⚠ **BLIND feel change, stated**: a reference-strength hit is unchanged, a poke
stuns less (floored at 0.35), a grown smash stuns up to 4×.

⭐ the test that pinned the old behaviour was *right about its own concern* — a
launch SPEED must never be read as a bare scale, or 120 px/s arms 120× the
hitstun. That poison is kept; only the flatness went.

### ✔ hitlag is one law for both bodies (2026-08-09)

`attack_hitstop_time` (0.055) and `player_damage_hitstop_time` (0.070) are one
`hitlag_time` (0.070), and both sites call `hitlag_duration`, which rides the
same `reaction_scale` hitstun does. **A connect is one event, so it buys one
freeze** — and it scales, which is most of what "weight" feels like.

The guard's poison is the interesting half: hitlag and hitstun must read the
*same* scale off the *same* hit, compared only where neither is clamped (the
floors differ deliberately — 0.5 vs 0.35). If one grows and the other does not,
the pause and the stun have drifted and the connect stops reading as one event.

### ⇥ and a tripwire retired, by its own pre-registered rule

`rollback_exit_oracle`'s coverage pin — an EXACT session count — went red: the
longer freeze lengthens the walk, and a longer walk crosses one more lifecycle
commit. **The pin's own comment had already written the rule**: *"an EXACT count
pinned to a walk whose timing depends on content tuning is a tripwire that fires
on tuning. It has now cost two investigations and caught no defect. If it moves a
third time, ask whether the oracle should assert the checksum identity and merely
REPORT the session count."*

This was the third time. ⇒ the count is reported, with a **ceiling** kept so a
genuine runaway still fails; the checksum identity the file exists for is
untouched and still asserted. ⭐ following a decision rule the earlier self wrote
down cost one commit instead of a fourth investigation — the argument for writing
the rule at the time rather than the doubt.

⚠ **one wrong turn, recorded**: I first blamed the walker dying and added a deep-HP
guard — which `wear_oracle_armor` already had, twenty lines up. Reverted. Reading
the fixture before patching it would have cost less than the probe did.

### ✔ landing lag and auto-cancel (2026-08-09)

`MoveSpec` gains `landing_lag_s` and `autocancel_after_s`, both `Option` and
both defaulted, so **every shipped move keeps landing exactly as it does today**
— the mechanic is opt-in and nothing has opted in yet.

`resolve_aerial_landings` charges the lag when a body crosses the airborne →
grounded EDGE with a move still running, unless the move's clock is past its
auto-cancel point; the move then ends, so the lag is a cost rather than a delay.
It runs BEFORE `advance_move_playback` deliberately: advancing first would open
the next window of a move the ground has already cancelled.

`BodyCombat.landing_lag_timer` is a **separate field from `recoil_lock_timer`**,
even though both are hard control locks. Two different facts — *you were thrown*
versus *you landed out of a move you had not finished* — and a trace that cannot
tell them apart cannot explain why a fighter stood still. They join only at the
input gate, whose parameter is now honestly named `hard_lock_timer`.

⭐⭐ **the guard caught a real bug in the thing it guards, immediately.**
`a_grounded_move_never_pays_landing_lag` went red: `was_grounded` seeded `false`,
so a jab read its own first tick as a landing and paid an aerial's lag. The seed
is `true` now — *no airborne observation yet* — which reads backwards and is
correct, because the construction site cannot see the body. Price: a move that
starts airborne and lands within the same tick pays nothing, which describes a
grounded move.

⚠ **`MoveSpec` gained fields, so 13 files of literals needed them.** Mechanical,
but worth recording that the sweep mis-fired twice — inserting into `impl
MoveSpec {` and into `fn … -> MoveSpec {` signature lines. A brace-matching sweep
over a type name matches the type's *other* syntactic homes too.

## ✔ F1 is a tuning instrument (2026-08-09)

Jon's list — *move and phase, live strike shape, facing/attack orientation,
percent, launch vector, hitstop and hitstun, semantic contact* — is on the read
model and on the screen.

`CombatBodyGeometryView` now carries the body's identity, `damage_taken`
(percent), live `facing`, `velocity`, `hitstun_s`, `hitlag_s`,
`landing_lag_s`, `grounded`, `on_wall`/`wall_normal_x`, and a `CombatMoveView`
with the move's id, its authored **phase**, elapsed/duration, the **committed
attack facing**, and whether it has already landed a hit.

⭐ **`velocity` rather than a stored "last launch".** During hitstun a body's
velocity IS the launch it took, so the instrument reads the fact where it lives
instead of adding rollback state that exists only to be displayed.

**Drawn in gizmos only**, so it needs no font and works in every composition the
overlay already runs in:

* a **phase bar** above each body — the move's whole duration as a track, filled
  to the clock, coloured by authored window (Startup yellow, Active red,
  Recovery blue). *"Did that connect during active, or did I walk into them
  during recovery"* is unanswerable without it.
* a **launch arrow** while the body is in hitstun.
* **two facing ticks** — live facing and the move's committed orientation. They
  agree almost always; the times they do not are the times you need to see it.
* **three lock bars** — hitstun, hitlag, landing lag. On screen all three look
  like "the fighter is not moving", and they are three different reasons.

⛔ no controller, no faction, no primary-player check anywhere in it: the
readout draws whatever the read model published, which is every combat body.
The guard asserts exactly that — a body with no markers at all publishes the
whole readout, including a **body facing +1 while its move committed to -1**,
which is the disagreement a single-facing instrument could never show.

## Aerial locomotion — measured, and mostly already a policy (2026-08-09)

⛔ grep before building, again. Of Jon's list — *air drift, acceleration, maximum
air speed, fast-fall, momentum conservation* — **only one was missing**:

| | |
|---|---|
| air drift / acceleration | ✔ `air_accel`, `glide_air_accel`, `air_friction`, `air_stop_assist`, all authored |
| fast-fall | ✔ authored (`fast_fall_accel`, `fast_fall_speed`), double-tap-down at the intent boundary |
| momentum conservation | ✔ `carried_run` / `carried_decay`, and the momentum horizontal law |
| jump buffering + coyote | ✔ `buffer_jump`, `coyote_time` — tuned to **zero** for Mary-O's SMB1 convergence, which is the mechanism working, not missing |
| **maximum air speed** | ✔ `AxisLocomotion::max_air_speed` via `air_speed_cap()` — see the section directly below, which is what fixed it (marker corrected 2026-08-12) |

### ✔ air speed is its own authored number

`AxisLocomotion::max_air_speed`, read through `air_speed_cap()`, with `0.0`
meaning *inherit `max_run_speed`* — so every body drifts exactly as it did.

⭐ **air ACCELERATION was authored and air TOP SPEED was not**, which is the
"accidental reuse of ground locomotion behavior" Jon's item names, and it made a
slow-running heavy that drifts fast literally unspellable.

⚠ **the sentinel is deliberate**: `Option<f32>` costs a bool in the motion
codec's frozen wire layout for a value whose unset case is exactly *the other
number*, and `0.0` is not a meaningful air speed — a body that cannot drift
authors `air_accel: 0.0`. One accessor reads it, so the fallback cannot be
honoured on one horizontal law and forgotten on the other, which is the bug the
sentinel would otherwise invite.

## Jump-squat — the last structural piece of the movement model (2026-08-10)

Genuinely absent: `grep -rn 'jump_squat\|jumpsquat'` returned nothing.

### ✔ a jump can owe a grounded startup before it leaves the floor

`AxisLocomotion::jump_squat_time` (authored, default `0.0`) and
`AxisManeuverState::jump_squat_timer` (the committed crouch). ⭐ the timer is the
**opposite** of `buffer_jump`: a buffer means a press is waiting to be spent, a
squat means the press is already spent and the leap is owed. That is why the
squat is resolved BEFORE the buffer is even looked at — and it is also what stops
a mash from re-entering the crouch and pinning the body to the floor forever.

⭐ **this is what makes a jump committal.** It is the window a fighter can be
struck out of its own takeoff in, and the read an opponent reacts to. It is
authored per body rather than globally because a body without a squat is not a
badly-tuned fighter, it is a different game: Mary-O's SMB1 convergence requires
the leap on the press tick, and `0.0` preserves that byte-for-byte.

Three things the implementation had to get right, each guarded:

- **the leap is ONE rule.** `launch_ground_jump` was extracted so the squat's
  expiry and the instant press-tick leap share the launch band, the air-jump
  refill and the `Jump` op. ⛔ a second copy in the expiry branch is the
  bifurcation this doc exists to prevent.
- **losing the floor mid-crouch VOIDS the leap.** Otherwise the startup buys the
  attacker nothing, which is the whole reason it exists.
- **the release edge is swallowed by the crouch.** A tap comes up mid-squat,
  where there is no ascent to cut, so the takeoff replays it through the body's
  own `AxisJumpLaw` (`cut_ascent_now`, split out of `apply_jump_release`).
  ⛔ **not** a second "short hop" speed beside the variable-jump law — that would
  be two mechanisms for one feel knob. Without this, tapping jump on a squat body
  gives a *full* hop: a feel bug the feature itself introduces.

⛔ **an f32 timer does not land on zero.** A 3-frame squat at `3.0 * dt` leaves
~3e-9s after three subtractions and the body crouches *forever*. Found by the
guard on its first run. A remainder far below a tick is not a crouch frame; the
expiry test is `> dt * 1e-3`.

⚠ **the press tick is the FIRST crouch frame.** Charging the whole squat and
waiting for the next tick makes an authored N-frame squat cost N+1, and a squat
shorter than one tick cost a whole one instead of nothing — so entry calls the
same `tick_jump_squat` the later ticks call.

Authored at the character seam (`AxisTuningSpec`), so a RON row can spell it.
⛔ **and `max_air_speed` had no authoring path at all** — last commit added the
knob to the kernel and to nothing that could set it. Both are on `AxisTuningSpec`
now, and both round-trip through the live tuning editor, where
`max_air_speed: 0.0` was hardcoded and would have wiped an authored value the
moment the inspector wrote back.

F1 gained a fourth lock bar (green). ⚠ hitstun, hitlag, landing lag and a
jump-squat all look identical on screen — "the jump input did nothing" and "the
body is deliberately crouching" are the same picture — which is exactly why the
instrument names them apart. Read through `MotionModel::jump_squat_remaining()`,
a projection every body can be asked and only one policy variant can answer.

### ✔ and the fighters actually have one

`jump_squat_time: 3.0 / 60.0` on the Smash duelist's `movement_tuning` — the
universal three-frame squat of Smash Ultimate, authored beside `slash_recoil:
0.0` in the same block and for the same reason: ⛔ it is **this game's** kit, not
every game's. `DEFAULT_TUNING` stays `0.0`, because a squat is not a better jump,
it is a different game's jump.

## Pose-aware hurtboxes — the model was built, the vocabulary lied (2026-08-10)

⛔ grep before building, a fifth time. Pose-aware hurtboxes are **already wired
end to end**: `AuthoredHurtboxes` (a validated `HurtboxDoc` with default / pose /
move-clock timelines) → `resolve_body_hurtboxes` → `ResolvedHurtboxes` →
`refresh_body_damageable_volumes`, which publishes the authored silhouette in
place of the coarse envelope for **any** body, not just a boss. Move-clock
overrides — Jon's "attacks" case — work today, sampled on `MovePlayback.t`, the
same clock the move's hit windows use, so a move's hurtbox and hitbox timelines
cannot disagree about when they are.

### ⛔ what was actually broken: a doc block promising poses nothing writes

`BodyPoseClock`'s doc said *"the engine knows the vocabulary (`hitstun`,
`tumble`, `crouch`, `shield`, `airborne`, `ledge_hang`, `run`, `idle`)"*.
`advance_body_pose_clocks` wrote **three** of the eight.

⭐ this is worse than a missing feature and it fails silently in the most
expensive way: content authors a `crouch` profile, `validate_hurtbox_timeline`
**accepts it**, the catalog publishes it, and it is never selected. Nothing warns
— the fallback to the default shapes is indistinguishable from an unauthored
body. Five names in a doc comment, each one an invitation to author work that
does nothing. *(A doc block naming your invariant is a dependency.)*

### ✔ the vocabulary is now a contract

`crouch` is written, from `BodyMode::Crouching` — an authoritative simulation
fact the body-mode driver already maintains, and Jon's named case. The selection
rule moved into a pure `body_pose(hitstun, crouching, airborne)`, and `BODY_POSES`
is pinned equal to that function's reachable set by an **exhaustive** test over
its whole input space. A pose cannot be named without a branch producing it, and
a deleted branch cannot leave its name behind.

The other four are **removed from the vocabulary rather than left as
aspirations**. `shield` and `ledge_hang` have crisp facts and are cheap to add —
but Jon said not to rush shields and ledges, and a name in a list is precisely
the thing that was doing damage. `tumble` has no simulation concept at all, and
`run` would need a speed threshold nobody has authored.

⚠ **marked**: `update_body_mode` matches on `Brain::Player(slot)`, so only a
CONTROLLED body (human or possessed) can crouch. That is an intent gap, not a
fork — an AI brain emits no crouch intent, so lifting the gate would change
nothing today — but a CPU fighter cannot use a crouch profile until its brain
can ask for one.

⚠ **also marked**: F1 shows effective hurtboxes but not WHICH timeline produced
them, which is the "the box is wrong — which of three sources won" question
`HurtboxSelection` exists to answer. It stays out because `ambition_sim_view`
would have to name the monolith, and a new dep edge fails the contracts job.

## Hitbox tracks — authorable already, and they dealt quadruple damage (2026-08-10)

⛔ grep first, a sixth time. A move already carries `windows: Vec<MoveWindow>`,
each `Active` window with its own `volumes`, spawned and despawned exactly while
the owner's clock is inside it. `MoveFrameData` already models `active_spans` as
a **Vec**, deriving startup from the earliest and recovery from the latest. F1
already shows whichever shape is live, because it reads real strike entities
rather than re-deriving the timeline. So a track — several authored shapes across
an attack's active portion — was expressible and observable today.

### ⛔ and unusable, because every keyframe was its own strike

Hit dedup (`HitboxHits`) is per BOX, and each window spawns its own box. A sword
arc sampled at four keyframes hit the same victim **four times**. Measured, not
reasoned: the falsifier run reports exactly `3 hits` for a three-keyframe track.

⭐ this is Smash's own model, and the standard answer is the one shipped:
hitboxes carry an **id**, and boxes sharing an id share a hit list across frames.

### ✔ contiguity IS the id

A window that ends exactly where the next begins hands its hit set forward. ⛔
**not a guess about authoring intent** — it is the literal continuity of the
volume in time. The box never left, so the strike never ended, so the victim is
still struck. A GAP means the box went away and came back, which is precisely
what a genuine multi-hit move (a drill, a rapid jab) is, and it rehits.

⭐ **and it costs no wire format.** The carry only has to survive within one
tick, because contiguous windows hand off on the single tick where the clock
crosses their shared edge. No new rollback state, no `MoveWindow` field, no
schema bump — an authored `track: u8` would have bought a `MovePlayback` entity
set, a snapshot change and a checksum projection to express something the
timeline already says.

Two guards, and the second is the one that matters: a swept three-keyframe track
lands **once**, and a track with a GAP lands **twice or more**. Without the
poison this mechanism is a silent damage nerf on every multi-hit move in the
game, wearing a bug fix's clothes.

⚠ **zero moves author more than one Active window today**, so nothing in the
game changes behaviour. This is a capability being made correct before it has
users, which is the cheap moment to do it.

## DI — built, correct, and switched off in the game that needed it (2026-08-10)

Jon put DI last, after launch and hitstun are clean. They are. And the grep says
the mechanic was finished before the campaign started: `di_adjust` is the real
Smash law (rotate the launch toward the held stick, weighted by how
PERPENDICULAR the hold is and by the throttle, capped at an authored budget),
`ResolvedCombatTuning` carries the budget, and both damage resolvers already feed
it the victim's live `ActorControl.locomotion`.

### ⛔ and the Smash demo declared no combat rules at all

`di_max_angle` therefore fell to the engine baseline `0.0` and DI was **off on
the one stage built to need it**. The versus route has declared its budget since
AE6; the game that IS the platform-fighter test case never did. Nothing failed —
a launched fighter simply had no say, and a knock-off was a coin flip instead of
a read.

`SMASH_DI_MAX_ANGLE = 0.31` rad (~18°, Ultimate's budget), declared beside the
route in the same flush that publishes the roster, and **released with the
experience**. The release is the more dangerous half: left standing, this stage's
budget follows the player into Ambition's PvE, which answers `0.0` on purpose
because being hit there is a punishment, not the opening of a negotiation.

### ⛔ the law itself had ZERO tests

Live in versus for weeks, unguarded. Five now, and the first is the shape of the
mechanic rather than a detail: **you cannot DI along your own launch line.** A
victim who holds straight away steers nothing, because the influence is the
perpendicular part of the stick. Without that, DI would be a speed dial and
holding away from the blast zone would be strictly correct — the opposite of the
read it exists to create. The others pin that DI rotates without changing SPEED
(else it becomes damage mitigation), that a partial stick spends a partial
budget, that a zero budget is byte-identical (the poison every PvE body is in),
and that the whole thing conjugates under flipped gravity.

The release guard was falsified both ways: deleting the `releasing` line turns it
red.

### ⭐⭐ and the second declarer arrived, which a contract was waiting for

`a-second-writer-of-a-match-global-must-answer-ownership` went RED on the new
declaration, exactly as designed: it watches `DeclaredCombatRules` *because* it
carried no owner and exactly one stage wrote it, and its own text says **"if this
list has to grow, growing it IS the review."**

So the review happened and its answer is an owner field. `DeclaredCombatRules`
carries `declared_by`; both stages name themselves; both give it back with
`releasing_owned`. ⛔ this is the **third** time this repo has learned the same
lesson — the participant roster, the prepared match, now the rules — and each
earlier time it was learned from a bug: Versus deleting another game's match
every frame, a stage opening with one fighter instead of two. This time it was
learned from a checker, before a player ever saw it.

The `app_it::experience_scope_ownership` contract caught the same collision from
the other side (two scopes removing one type), which is why both exist: one reads
the source text, the other asks the composed scope registry.

⚠ **the contract was NARROWED, not waived.** A type that can answer the
ownership question does not need a contract asking whether anyone will, so
`DeclaredCombatRules` left the pattern. `ActiveMatch` still cannot answer and
stays under watch.

## The projectile fork — item 3's remainder, closed (2026-08-10)

`step_projectiles` branched on `firer_faction == Player`. The two sides had
drifted into different games:

| | player shot | every other shot |
|---|---|---|
| victim | `HitTarget::Volume` — a broadcast, resolved downstream by "iterate and take primary" | `HitTarget::Body(entity)`, named |
| knockback | none | resolved, per victim |
| published silhouette | not consulted | `is_intangible` respected |
| parry | impossible | a timed shield re-owns the shot |
| grudge / friendly fire | never asked | `damage_lands` |

⛔ **four rules that existed on one side of a fork whose entire content was who
pulled the trigger.** That is the bifurcation `one-body-one-path.md` is about,
and it is item 3's marked remainder.

### ✔ one victim loop, whoever fired

The fork is deleted. Every shot runs the loop that already existed, and then
publishes the strike's **unresolved half** — `HitTarget::UnresolvedFeatures`,
exactly as a body-owned melee does — for the breakables and boss encounters no
body resolver can name. ⛔ not `Volume`: `Volume` means "scan everything" and
would damage every body a second time on top of the identified hit it just took.
The consumer already skips its actor scan on that target, machinery melee paid
for and this reuses rather than re-derives.

The actor hit PREDICTION went with the fork that needed it, and `ecs_actors`
with it — one fewer system param, and one fewer place for "does this hit an
actor" to grow a second answer.

⚠ **what changes in play**: a player's bolt now launches what it hits, can be
parried, respects an authored invulnerable window, and honours a grudge. And a
hostile bolt can break a crate — which it could not do before, not by policy but
because the other side of the fork was the only one that looked.

### ⛔ the fixture had no faction

`player_faction_shot_damages_an_overlapping_enemy_and_expires` went red with
*zero* hits, not wrong ones. Its enemy carried no `ActorFaction` — invisible to
`StrikeVictim` — because the branch it was written against broadcast a volume and
never asked whose side anyone was on. Production builds no such body, so the
fixture was corrected rather than the reach requirement weakened. Its assertion
then moved from `Volume` to `Body(enemy)`, which is what its own `▢` note said
should happen the day this landed.

## Retiring `UnresolvedFeatures` — measured, and it BLOCKS on a call Jon owns

The last marked remainder. I estimated a data migration and the measurement says
otherwise, in both directions.

**A boss is already a body.** ⛔ the claim in `one-body-one-path.md` that its "HP
and phase live on an encounter rather than on a body carrying the combat cluster"
is **false and now corrected**: the boss spawn inserts the shared actor cluster
precisely so `apply_hitbox_damage`'s non-`Option` victim query still matches, it
carries `BodyHealth` / `BodyCombat` / `ActorFaction`, and melee's victim query has
**no boss exclusion at all**. A swing names `HitTarget::Body(boss)` today.

**And that event lands nowhere.** The damage consumer's boss branch is gated
`bosses.iter_mut().filter(|_| actor_target.is_none())`, so an event that names an
actor — including the boss itself — skips it, while the boss is excluded from the
actor query as a disjoint family. The boss's HP is moved by the **unresolved
half** instead. ⭐ so the same swing identifies its victim and then damages it
anonymously, and the identified event is inert.

### ⛔ ~~"and every test would still pass"~~ — FALSIFIED 2026-08-12, three ways

This section ended *"if the broadcast ever stopped, a boss would stop taking
melee damage and every test would still pass."* That was the reason to be
nervous about the seam, and it is **wrong**. Measured by poisoning each link and
running the suites, rather than by reading:

| link | poison | caught by |
|---|---|---|
| the **broadcast** | `if false {` around the `UnresolvedFeatures` emit | 2 tests in `ambition_combat`, incl. `a_body_owned_strike_publishes_its_unresolved_half_beside_the_resolved_body_hit` |
| the **routing** to `apply_boss_hit` | early `return false` before the volume scan | `face_tanking_player_swings_back_and_is_recoil_locked` |
| the **HP mutation** | restore `health.current` after the shared resolve, so feel fires and HP does not move | 4 unit tests, incl. `damage_decreases_hp_in_a_vulnerable_phase` |

⚠ **the middle one is the interesting result.** `boss_contact_iframes` says in
its own doc that it measures the swing CONNECTING (`hit_flash`) *"rather than
boss HP"*, because intro invulnerability and stray room-edge resets make an HP
assertion flaky there — so it looks like it is not covering this. It covers it
anyway, because `hit_flash` is written INSIDE `apply_boss_hit`: a test that
deliberately declined to assert the consequence still fails when the consequence
stops being reachable. ⭐ that is luck rather than design, and it is worth
knowing which one it is before someone "simplifies" the flash.

⇒ **the seam is fragile in ARCHITECTURE and covered in FACT.** The nervousness
this paragraph created was the reason to build an end-to-end guard; three
measurements say the links are each pinned and a fourth test would be
redundant. What remains is the design defect — one swing naming its victim and
then damaging it anonymously — and that still blocks on D23.

**⛔ the fix blocks on D23, which is Jon's.** Making the consumer honour
`Body(boss)` requires both producers to name bosses, and the projectile victim
query excludes `BossConfig` on purpose: that loop still tests the COARSE box
(`strict_intersects`), and a boss's coarse AABB is a giant composite envelope.
Including bosses without first adopting `victim.reached_by(..)` would let bolts
hit the bounding rectangle instead of the authored head/hand volumes — the
GNU-ton seam, undone. And adopting `reached_by` for projectiles retires
`strict_intersects` and changes how every shot connects, which the code already
records as a FEEL call reserved for Jon (queue row D23).

So the boss half is one small consumer change sitting behind one authored
decision, not a migration. **The breakable half is the real one**: no faction, no
combat cluster, nothing `StrikeVictim` can see — that is where a genuine
migration lives, and it is not worth starting with the ledger's answered-but-
unfixed rows still open.

✔ **the feel list is complete.** Move-facing snapshot, one authoritative move
timeline, hitstop/hitstun, landing lag / auto-cancel, aerial locomotion, air
speed, jump-squat, hitbox tracks, pose-aware hurtboxes, DI.

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
