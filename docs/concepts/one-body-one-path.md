---
id: one-body-one-path
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-09
related_docs:
  - docs/concepts/invariants.md
  - docs/concepts/movement-collision.md
  - docs/adr/0020-mounts-and-vehicles.md
---

# One body, one path

**The player is an actor.** Every rule that fires for one controller kind fires
for all of them, through the same code, or it is a fork.

This is the repo's most-violated rule. It is stated in `AGENTS.md` because
everyone reads that; the detail lives here because the detail is long and the
rule is short.

## The bifurcation smell test

Before you write *anything* keyed to "player", or to "actor / enemy / boss" — an
attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state
machine, a brain hook — ask:

> **Does the other controller kind already do this on its own code path?**

If yes, you have found a **FORK**. Your job is to UNIFY onto the single shared
seam and DELETE the other side. It is not to add a second site.

⛔ **Adding a parallel emission site, state component, system, or spec for an
effect that already exists elsewhere is a BUG, not a fix — even if it compiles
and every test passes.** A green test on a forked path is worthless: it proves
the fork works, which is the problem.

⚠ **If you genuinely cannot complete the merge in one pass**, do NOT add the
parallel path "for now". Route the new caller *through the existing seam* —
extracting one shared fn / system / event if none exists — and log the remaining
merge in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word.

⭐ When a doc or a keystone says "unification", it means **delete one path**. It
does not mean "make the two behave similarly".

## What is unified today

This is a STATUS inventory, not the rule. It goes stale; the rule does not.

**Melee is unified end to end.** The STATE (`BodyMelee` / `MeleeSwing`), the
swing MODEL (`AttackSpec`), the slash VFX (`emit_melee_slash` in `combat::util`),
the strike SPAWN, and body CONTACT RESOLUTION are one path for the controlled
body and every actor. The spawn goes through the moveset:
`combat::moveset::trigger_moveset_moves` → `advance_move_playback` spawns ONE
gravity-resolved volume that drives both the damage `Hitbox` entity and the
slash, projected to body state by `project_moveset_melee_to_body_melee`.
`combat::hitbox::apply_hitbox_damage` then resolves every `FollowOwner` strike
through the same victim loop: owner exclusion, relationship/team policy,
published hurtbox geometry, per-hitbox dedup, and victim-specific knockback.
`Player` + `World` hitboxes remain a distinct world-AOE primitive rather than a
second melee path.

**A landed body strike is one fact.** `apply_hitbox_damage` publishes
`LandedBodyHit { hitbox, attacker, victim, volume, contact }` at the moment that
same shared resolver commits the targeted damage event. Move confirms and
authored `on_hit` techniques consume that fact; they do not independently
rediscover overlap, faction policy, self-exclusion, or victim identity. Pogo is
the keystone example: body pogo consumes the resolved victim plus its
`PogoPolicy`, while genuine entity-less rebound surfaces remain a separate world
contact path. Ordinary bodies may publish `PogoTargetVolumes` for affordance and
custom pogo silhouettes, but they are never flattened into anonymous
collision-world `PogoOrb` blocks. Only an explicit `PogoTargetContributor`
opts an ECS feature into world rebound geometry.

**A strike also has an UNRESOLVED half, and it says so.** The resolver names
every combat body it reaches. It cannot name a breakable: no faction, no combat
cluster, nothing `StrikeVictim` can see. ⚠ **this used to say the same of a boss
"whose HP and phase live on an encounter", and that is FALSE** — a boss carries
`BodyHealth`, `BodyCombat`, `ActorFaction` and the vulnerability trio through the
one bundle every body shares, melee's victim query has no boss exclusion, and it
is named `HitTarget::Body(boss)` today. What is still true is smaller and
stranger: the damage consumer's boss branch runs only when the event names NO
actor, so the `Body(boss)` event lands nowhere and the boss's HP is moved by the
unresolved half instead. The swing identifies its victim and then damages it
anonymously. Retiring that is blocked on the projectile loop adopting
`reached_by` (queue D23, a feel call Jon owns), because a boss's coarse AABB is a
giant composite envelope and switching producers without it would let bolts hit
the bounding rectangle instead of the authored head/hand volumes. So the same
strike publishes one
`HitTarget::UnresolvedFeatures` event carrying its geometry, deduped by the
attacker's `MovePlayback.hit_targets` (the move's own authoritative accumulator,
never the `BodyMelee.swing` projection). ⛔ **that is not `HitTarget::Volume`**:
`Volume` means "scan everything" and still belongs to the wielded world-AOE
primitive, while `UnresolvedFeatures` means "the bodies are already resolved —
scan only what a body resolver cannot see." A consumer that conflates them
damages every body twice. ⚠ **the variant is scaffolding with an expiry**: when
bosses and breakables become victims in their own right it goes away with them.

**A hit says WHAT KIND it was, never WHO.** `HitSource` is a cause —
`Melee`, `Projectile`, `Contact`, `Hazard`, `LeftTheWorld`, `Pogo` — and
`HitTarget::Body(Entity)` names the one victim a producer resolved. Nine
direction-spelled variants and two victim-classifying ones collapsed into these
because the who-half was carrying decisions that belong to identity: whose swing
it is, whether it hits heavy, whether the human's damage slider applies, which
consumer owns the event. **Every one of those is asked of the ATTACKER or of the
VICTIM's population, both of which the event names.** ⛔ do not reintroduce a
faction or controller word into the cause vocabulary; if a rule needs to know
who, it should ask the entity. ⚠ and note what the direction words were silently
buying: the feature drain had no attacker self-exclusion at all, because a
victim-side broadcast could never reach it. Identity beats every relationship
rule, and it is stated in both resolvers now.

**Who may fight whom is ONE policy.** `combat_relation` answers `Foe` / `Neutral`
/ `Ally` with precedence **grudge → match team → authored faction**, and both AI
target selection and damage resolution call it. ⛔ they used to be two rules:
damage read faction difference with a team override, targeting read the
`FactionRelations` matrix and had never heard of a team — so a match whose
fighters shared a faction was damageable and untargetable at once, and the patch
was to seat alternating FACTIONS (`Player, Enemy, Player, Enemy`). That hack is
deleted; every match seat carries a team, an authored one or its own, and a
character's faction means what the character says everywhere. ⭐ **the third
answer is load-bearing**: `Neutral` is hittable but not huntable, because damage
is physical and targeting is relational. A boolean cannot hold that, and
collapsing it makes a stray blow pass through a bystander or a town NPC become
prey.

**Simulation truth lives in the simulation crate.** `Hitbox` and its lifecycle
state are `ambition_combat::strike`, not `ambition_vfx` — a damage volume
carrying knockback, launch direction and owner identity is not a picture. ⚠ the
piece that stayed is `HitSide` and the `Effect` request vocabulary, because
`ambition_projectiles` names `Effect` and sits below combat; that is the orphan
rule, not a compromise. ⛔ and presentation must not reach back: the render
stand-in reads `CombatGeometryView`'s strike rows, which publish the strike
entity, its owner, whether it is body-anchored, and the owner position the
volume was resolved against — enough to re-place the geometry at the PRESENTED
pose without re-evaluating it.

⛔ **Do not reintroduce** a `PlayerAttackState` / `ActorAttackState` split, a
second slash emit, or a per-frame player damage loop. Every melee is an
`"attack"`-verb moveset move riding `MovePlayback`.

**A weapon in hand OWNS the Attack press, and one seam decides that.** With a
`HeldItem` on the body, `trigger_moveset_moves` resolves Attack through the ITEM
— its own melee verb if it authors one, otherwise nothing here, leaving the
press to the item's own subject-generic system (throw / bolt / gauntlet). The
wearer's `attack` verbs are NOT revoked and its timelines are NOT pruned: only
the resolution moves, and it moves back the moment the hand is empty. ⛔ **Do not
"fix" a double-fire by deleting the wearer's verbs** — the on-screen Attack
button is drawn (and made touchable) only while the action scheme carries an
Attack slot, so a verb revoke fires fine on a desktop and leaves the weapon
untappable on a phone. ⚠ an item system that ENDS the holding (the throw) must
also mark the press spent, because it removes `HeldItem` a schedule phase before
the trigger looks.

**The movement driver is unified at the engine entry.** Every body — driven or
not — integrates in ONE phase, `integrate_sim_bodies`
(`actor_monolith/src/features/ecs/actors/update.rs:1139`), and there is no
separate route for the controlled body.

⛔ **THIS PARAGRAPH DESCRIBED THE INTERMEDIATE STATE UNTIL 2026-09-03, AND THE
CODE NOW GUARDS AGAINST IT.** It said *"the player tick is ONE system
(`player_body_tick`) that calls the SAME combined body tick the actor uses
(`ae::update_player_with_tuning_clusters` ≈ the actor's
`update_body_with_tuning_clusters`)"* — a unification that kept a player-shaped
entry point. All three names are gone from production:
`update_body_with_tuning_clusters` exists nowhere,
`update_player_with_tuning_clusters` survives only in
`platformer2d_core/src/test_support.rs`, and `player_body_tick` survives only
inside the name of the test that FORBIDS it.

⇒ That test is `player_body_tick_is_not_the_gameplay_movement_route`
(`game/ambition_app/tests/unified_body_movement.rs:104`). It reads the real
`Update` schedule after a step and asserts `integrate_sim_bodies` IS registered
and that *"the old separate home-body movement route `player_body_tick` must be
gone"*, plus the same for `player_body_phase`. ⇒ **So an agent implementing what
this page said would have turned that guard red** — the doctrine described the
step before last, and the code had already forbidden it.

**Facing is control output, not a collision side effect.** The movement kernel
publishes semantic contacts such as `BodyWallState`; it never reverses a body
because velocity happened to stop. Autonomous Patrol/Wanderer policy may choose
to turn away from a real side contact. Human input, fighter brains, scripted
control, remote control and RL authority retain the facing they chose.

**Observers do not require a privileged protagonist.** Combat geometry is
projected through the `ambition_sim_view::CombatGeometryView` read-model for
every combat body: collision envelope, effective hurtboxes, and live strike
volumes. Debug rendering consumes that observation whether the session has zero,
one, or many bodies under human control. A `PrimaryPlayerOnly` body may still
have Ambition-specific diagnostics, but its existence is never the admission
ticket for engine-level observability.

**The two-clock precision-blink split is an INPUT affordance, not a simulation
structure.** Responsive aim during bullet-time is purely
`InputState::control_dt`: a human sets `control_dt = real frame dt`; a brain
leaves it `0` and runs everything at sim time. There is no second simulation.

## Six names for "player", and none of them is a body kind

These six exist, none is redundant, and confusing any two of them produces the
bifurcation above. The authoritative prose for each lives on its own definition;
this table is the map between them.

| Name | What it actually is | Lifetime | ⛔ never use it for |
| --- | --- | --- | --- |
| `ParticipantId` | the person in front of a controller | outlives every session, body and possession | anything a body does |
| `PlayerSlot` | which seat at the machine that participant occupies; `SlotControls[N]` is its control frame | the session's seating | "the protagonist" — slot 0 is a seat, not a role |
| `DrivingParticipant(slot)` | **control authority**: this body is driven by that seat | moves between bodies, which is what possession IS | a body kind; a boss carrying it is an ordinary controlled body. ⛔ and not `Brain` — that is AI policy, and the two shared one enum variant until it was split out |
| `PlayerEntity` | a body belonging to the player population | the body | assuming there is exactly one, or any |
| `PrimaryPlayer` | the **home avatar** — save identity, respawn anchor, inventory owner | the home body, and a session may have none | "the currently controlled body" (possession moves that away) |
| `ControlledSubject` | which body a local presentation/control context follows | the frame | a second global actor identity |

⛔ **zero of `PrimaryPlayer` is a legitimate steady state.** A match under
`InitialBodyPolicy::NoInitialBody` lowers no home avatar, so every reader must be
correct at a count of zero. The failure shape is not `With<PrimaryPlayer>`
itself — it is `single()` + `else { return }` around it, which disables a whole
subsystem for every entity. That shape has produced the same class of freeze at
least three times: the clock (2026-08-07, *"the characters are just stuck in
air"*), the camera, and the world's moving platforms (2026-08-14, frozen geometry
in every match). Read `markers.rs` before adding a seventh reader; it carries the
site-by-site classification.

⭐ **generic simulation may consult NONE of them.** A body decides, moves, fights
and rides because of its capabilities and its control authority. If a
body-generic system needs one of these six to function, that is the bug — the
question it is really asking is almost always "which body" (an `Entity`) or
"which seat" (a `PlayerSlot`), and both are available without a privileged
protagonist.

## What is deliberately SEPARATE, and why

⚠ **brain decision and body integration stay separate Bevy systems on purpose.**
`tick_actor_brains` decides and `integrate_sim_bodies` moves; what is shared is
the body-tick engine entry, not the orchestration. Merging phases into one
god-system is NOT the goal, and a change that does it is not an improvement — it
trades a legible seam for a large system that no longer says which phase it is in.

⚠ this paragraph used to name `player_body_tick` and `update_ecs_actors` as the
two systems to keep apart. Neither exists: the home body's separate tick was
deleted when one movement phase took every non-boss body, and the actor
orchestrator has since split by phase. The rule survived its examples, which is
why it is restated against the systems that are actually there.

## The next elevation

The unified action/ability timeline — cancel windows, movement locks,
armor/i-frames, resource costs, hurtbox swaps, anim binding — layered on the one
strike seam that already exists. Not a second seam beside it.
