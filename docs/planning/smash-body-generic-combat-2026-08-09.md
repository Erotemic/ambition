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

## Execution order (mine, revise as measurements land)

0. **Stabilize** — compile the affected crates, run the focused suites, repair.
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
