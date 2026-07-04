# Fable review — 2026-07-02

## BULK REVIEW QUEUE (Jon: adjudicate these in one pass — the autonomous run did NOT stop for them)

The autonomous mandate: **do everything unblocked; never stop to ask; a decision that
can be parameterized/tweaked later is NOT a decision — implement it headless with a
sensible default and note it here for deferred tuning.** Two kinds of entries:

**DEFERRED TUNING** (already implemented headless; feel/value to sweep later):
- Autonomous special-firing CADENCE is OFF (`smash/action.rs` Engage arm). A naive
  "fire special while melee recharges" spammed the move + broke the duel regroup kit;
  needs a real special cooldown / spacing gate. Possession fires specials today.
- **Melee subsumption (actor swing → moveset `"attack"` move)** — an actor's authored
  `ActionSet.melee` is now folded into its `ActorMoveset` as a data-driven `"attack"`
  move (`attack_move_from_melee`), triggered on `melee_pressed` by the SAME runtime as
  its specials; the flat `BodyMelee` swing is skipped (`MovesetMelee` marker) and its
  read-model is PROJECTED from the live `MovePlayback`. Deferred-tuning knobs, all with
  sensible defaults, sweep-when-it-reads-wrong:
  - **Melee hit-volume geometry is a body-local forward rect** approximated from
    `reach_px` (offset `reach*0.6`, half-extents `reach*0.5 × 16`) — was the
    sprite-manifest per-animation box (`actor_attack_hitbox_world`). The manifest box
    is richer; if an actor's reach reads wrong, either tune the rect or teach the
    moveset to sample the manifest box.
  - **Melee knockback default = 120** (aggressor-push channel). Was faction-derived
    (`knockback_strength = 1.0`). Tune per-archetype if feel wants it.
  - **Melee swing SFX** now a `MoveEvent` cue `"melee_swing"` (→ `SfxMessage::Play`),
    was `SfxMessage::Slash`. Different bank entry; wire/point the cue if silent.
  - **Melee slash VFX is dropped** for moveset-melee bodies (the flat path's
    `spawn_melee_strike` emitted the slash flourish; the moveset hitbox does not yet).
    Presentation-only; add a slash event/effect to the attack move when the VFX pass runs.
  - **Lunge `step_px` / Slam `hop_height_px` self-motion is not carried** — the attack
    move is timing + a hit volume only. Add a self-impulse window if a lunging actor
    needs its forward step back.
  - **Possessed-body melee faction:** the moveset hitbox uses the body's raw
    `ActorFaction`, not `effective_faction(faction, brain)`. Possession tests pass
    (possessed actors flip faction), but verify a possessed enemy's swing hits the
    right side under every possession path.
  - **Uniform fold DONE:** both hostile AND peaceful-NPC spawn paths now fold melee →
    moveset (a possessed peaceful NPC's swing runs through the moveset too). Bosses
    (geometry strikes) are the only actor melee still off the moveset — a separate
    focused effort (see the boss-geometry fold note).
  - **Duel-arena AI cadence shifted:** routing the swing through the moveset move
    (0.42s player-robot / 0.62s PCA, gated `Without<MovePlayback>`) re-weighted the
    fighters — the player-robot now regroups flight-heavy (fly 731/1800 frames) and
    dashes/presses-melee less; the PCA is unchanged. Both still trade melee, take
    damage, shield, blink, fly. Two duel canaries were loosened to match (dash-wiring
    is now an aggregate proof; melee-intent threshold 3→2) — retune the fighters when
    the feel pass runs. Root cause is likely the lost `BodyMelee.cooldown` recovery
    floor (the flat path armed `ENEMY_ATTACK_COOLDOWN * mult`; the moveset move's own
    duration is the new floor) — if the cadence reads wrong, re-arm a per-archetype
    recovery floor on move trigger.
- **Ranged is now a moveset `"fire"` move (E54)** — deferred-tuning knobs, sensible
  defaults:
  - **Fire-move windup/recovery are NEW authored defaults** (`fire_move_from_ranged`:
    Pistol 0.08/0.15, Rock 0.12/0.18, Bolt 0.18/0.20, Arrow 0.28/0.22 s). Ranged had NO
    windup before (instant on cooldown); the draw/settle timeline is the expressivity win.
    Tune per weapon when it reads wrong.
  - **Two rate gates now stack:** the move's own duration (a re-trigger gate) AND the
    body-side `RANGED_REFIRE_S = 1.1s` cooldown (the hard floor, enforced in the projectile
    consumer). Since the move is shorter than the cooldown, an enemy may play the draw
    animation more often than it actually releases (2 of 3 draws rejected by the cooldown).
    Effective fire RATE is unchanged (still 1.1s); only the draw-anim cadence is loose. Align
    the fire-move duration with the refire cooldown, or move the cooldown onto the move, if
    the repeated-draw read looks wrong.
  - **Player ranged is untouched** — it flows through `ChargesProjectiles` /
    `PlayerProjectileTick` (the charge state machine + motion-gesture buffer), a separate
    per-actor capability, not `ActionSet.ranged`. This fold is enemy/boss/NPC ranged only.
  - **REAL BUG fixed en route:** `project_moveset_melee_to_body_melee` projected a phantom
    `BodyMelee.swing` from ANY live move on a `MovesetMelee` body — so a body playing its
    ranged (or special) move read as "mid-attack" and the movement pipeline FROZE it. Now it
    projects a swing only for the `"attack"` move. (Latent since the melee fold for specials;
    the ranged fold surfaced it — it froze the PCA's chase in `actor_phase_split`.) Pinned by
    `a_ranged_move_does_not_project_a_phantom_melee_swing`.
  - **Duel AI cadence shifted again (like the E49 melee fold):** routing the PCA's glider
    through the moveset `"fire"` move re-weighted both duel fighters — in the bout they now
    trade melee, block, roam, take real damage, and reach real air, but the `jump` and `blink`
    verb counts fell to 0 (they gain height flight/blink/knockback-driven and lunge less, so
    the lunge→blink-evade reaction rarely triggers). Two `duel_arena` canaries were loosened
    to the SPIRIT (verticality-by-rise; defense-by-block) with the jump/blink verb-counts
    dropped. Retune the fighters' hop/lunge/blink appetite in the feel pass.
- **Boss strike read-model projected from the move (E53 Slice B+C)** — while a boss move
  is inside its Active window, `BossAttackState`'s `active_*` fields are now DERIVED from the
  live `MovePlayback` (`project_boss_attack_state_from_move`), not trusted from the pattern
  cursor's mirror. DAMAGE is unchanged (the Active window's hitbox lifetime is identical —
  the move is triggered at `t0 = telegraph offset` so the strike is live the same frame as
  before). One sub-frame wart, deferred: because the move's clock starts fresh at the strike
  edge (`t0=tel`) rather than tracking the cursor's telegraph overshoot, the projected
  `active_remaining`/`active_elapsed` can differ from the old cursor value by `< one frame`
  (<16ms) at the strike boundary, and `active_profile` can clear ≤1 frame earlier/later.
  Read-model only (anim frame sampling / debug overlay / view index — none gate damage);
  imperceptible. NOTE: this wart is INHERENT to moving the read-model from cursor-time to
  move-time — the cursor carries a fractional step-boundary overshoot `∈ [0, dt)` that a
  freshly-inserted move's clock can't match, so it persists under Slice D's telegraph-edge
  trigger too (it is NOT `t0`-dependent). The honest resolution is to accept the move as the
  new authority (its timeline is equally valid); the deferred item is only tests/feel that
  key off exact old-cursor values. Damage is byte-identical either way.
- **Boss GEOMETRY strikes folded onto the moveset (E51)** — every boss strike now runs
  through the SAME moveset runtime an actor's swing does; `sync_boss_strike_hitboxes` +
  `FrameDrivenBossStrike` are DELETED. Deferred-tuning knobs (sensible defaults, sweep
  when a boss reads wrong):
  - **Boss strike geometry is now static body-local `HitVolume`s** (from
    `volumes_for_profile` at a ZERO origin), NOT the per-tick sprite-frame-tracked
    multi-part geometry. GNU-ton's two hands / the gradient sentinel's pose-tracked
    strike boxes lose their frame-tracking for GAMEPLAY (the debug overlay + hurtbox
    pose still use the rich path). Approximation blessed by the handoff; if a boss
    strike's hit region reads wrong, either tune the profile's `volumes_for_profile`
    rect or teach the moveset a frame-sampled volume kind.
  - **Strike move duration = the profile's first-seen strike window** (from the
    capability repertoire), so a profile authored with different durations in different
    pattern steps uses the first. Same limitation the special-fold already had. Tune by
    splitting into distinct profiles/keys if a boss needs per-step strike lengths.
  - **Boss strike knockback uses the body's LIVE facing** (`kin.facing`) via the move
    runtime; the retired sync hardcoded `facing: 1.0`. For symmetric/centered strikes
    this is a no-op; verify an off-center strike's knockback direction.
  - **`BossAttackState` still OWNS strike timing** (the pattern cursor writes it; the
    move is slaved to `active_profile`). The full target shape — pattern → pure
    move-SEQUENCER, `BossAttackState` → projection FROM the live `MovePlayback` (mirror
    of `project_moveset_melee_to_body_melee`) — is a recorded NEXT slice (see E51). It
    is a larger change (Telegraph/Strike/Rest steps → Startup/Active/Recovery windows,
    ~37 `BossAttackState` consumers migrate); the current slice already retires the
    bespoke hitbox poll and unifies the damage path, which is the load-bearing win.

**GENUINE FORKS** (shape-defining + expensive to reverse; Jon's call — the run picked a
defensible default and moved on):
- **Player-melee fold — how does a moveset express DIRECTIONAL variants + POGO?** The
  actor-melee subsumption (E49) folded the simple forward swing. The player's melee is
  richer: up / down / air-up / air-down / air-forward / air-back variants selected from
  `attack_axis` + on-ground, a POGO bounce on a down-air strike over a pogo target, and a
  sprite-manifest-authored per-animation hitbox. Folding it means the moveset must SELECT
  a move by attack intent and carry a pogo behavior — a schema shape, not a tuning value.
  Options: **(A)** one move per direction, verbs `attack`/`attack_up`/`attack_down`/… and
  the trigger resolves intent→verb (most explicit, most data); **(B)** one `attack` move
  with intent-tagged windows/volumes (compact, but new window semantics); **(C)** keep the
  player on the flat directional path indefinitely and only fold NON-directional bodies
  (the current state). Pogo needs a volume behavior flag (`pogo: bool`) that bounces the
  owner on contact with a pogo target — a new `HitVolume`/effect primitive either way.
  **Chosen default: (C) for now** (player stays flat; no feature lost), because (A)/(B)
  change the `MoveSpec` schema shape and want Jon's call. NOT implemented (reversible: it's
  the current state). Route: pick A or B and it's a bounded slice mirroring E49.
  **[REFRAMED 2026-07-04 — see JON'S DESIGN DIRECTION JD1]:** Jon does NOT want an A/B/C pick.
  The answer is the parameterized-effect ability model — directional variants become move
  SELECTION from input-mappings in the PUBLISHED character data, and POGO becomes a content
  Technique fired by an `Effect{key}` (so it never enters the content-free runtime — the
  pollution worry dissolves). fable to spec the params + dispatch + character-data schema.
- **Ranged subsumption — RESOLVED via option A, IMPLEMENTED (E54).** Was: fold actor ranged
  (Rock/Arrow/Pistol/Bolt) onto the moveset as a `"fire"` move whose event spawns a
  projectile, vs. the facing-lock problem (a `MovePlayback` locks facing at trigger, but the
  brain aims dynamically toward a strafing target). **Chose (A):** a content-free
  `MoveEventKind::Ranged` marker whose dispatch SAMPLES the owner's live aim
  (`ActorControl.fire`) at the event frame and BRIDGES to the SAME
  `ActorActionMessage::Ranged` the flat resolver emitted — so the mature enemy-projectile
  consumer (body-side fire-rate, recoil, muzzle, visual kind) is reused unchanged and the
  shot still tracks the target. `build_actor_moveset` folds `ActionSet.ranged` into a
  `"ranged"`-verb fire move (Startup draw → fire event → Recovery — ranged now has a real
  windup/recovery timeline, the expressivity win); a `MovesetRanged` marker suppresses the
  flat `frame.fire → Ranged` emission (no double-fire), the ranged analogue of `MovesetMelee`.
  Melee (E49–E53), specials (E47–48), and now ranged (E54) ALL run through the one moveset
  runtime — the flat combat paths are fully subsumed. Deferred-tuning (see below).

> Handoff for the continuing agent: **`docs/reviews/HANDOFF-2026-07-03-moveset-and-fable-review.md`**.

---

A read-only audit of the Rust codebase hunting **high-value, fable-hard refactors**
that move Ambition toward its design goal: a Unity/Unreal/Godot-class reusable 2D
platformer engine for Bevy, where the game is one content crate. No code was edited
(a portal agent was concurrently active). Four parallel deep audits, each verified
by reading code (no grep-only findings):

1. **Actor unification forks** — remaining player/actor/boss bifurcations
2. **Physics/gravity frame bugs** — relativity-principle violations
3. **Engine/content separation** — what blocks the "second game as a content crate" oracle
4. **Decomposition seams** — natural extractions inside the 95k-LOC `ambition_gameplay_core`

Cross-checked against `docs/planning/engine/unified-actors.md`,
`docs/current/{state,next}.md`, and `dev/journals/code_smells.md` so already-known
items are marked as such rather than re-discovered.

> **Provenance & contradiction convention.** This audit (sections A–D + the
> Synthesis) was authored by **fable**, a significantly stronger model with a
> wider view of the codebase — treat its findings as the high-confidence
> baseline. The EXECUTION LOG (E1+) is written by the various weaker executing
> agents. When an executing agent **contradicts, corrects, or reframes** a fable
> finding, it tags the claim with its model (e.g. `[opus-4.8[1m]]`) and flags it
> **`fable should re-check`** — the executing agent has the narrower, more
> focused scope and may be right, but fable saw things it may not, so the
> disagreement is surfaced for fable to adjudicate rather than silently
> overwritten. Search `fable should re-check` for all open disagreements.

---

## JON'S DESIGN FEEDBACK (2026-07-03) — FOR FABLE TO ADJUDICATE

Triggered by D3.2a moving `FeatureVisualKind` (variants `Hazard, Enemy,
TrainingDummy, Boss, Breakable, Chest, Pickup, Npc, Switch`) into a leaf crate.
Jon's verbatim direction, to be reconciled with the audit's design:

1. **The fundamental taxonomy should be `actors` (things WITH brains) vs `props`
   (things WITHOUT brains)** — not a 9-variant content enum. "Shouldn't there
   just be actors and props?"
2. **`Boss` / `Npc` / `Enemy` are THE SAME THING** (actors) and "must" render the
   same — distinguishing them (even as a placeholder color) is the smell. "boss,
   NPC, and Enemy should all be colored the same thing because they are the same
   thing (or should be, they must be!)."
3. **`TrainingDummy` should not exist as a kind** — it is "an NPC that just does
   nothing… the most NPC, because it should have no special components." A
   brainless actor with the empty component set, not a variant.
4. **A ~100-line leaf crate for the read-model smells** unless it earns real meat
   or the compile graph genuinely requires it. "we aren't planning on this just
   being a single 1 lib crate with 100 lines are we? Is this thing gonna have any
   extra meat on it? … maybe its necessary, but it seems like a smell to me."

`[opus-4.8[1m]]` **executing-agent analysis (fable should adjudicate):** Point (2)
is *partly* already honored — `FeatureVisualKind` for actors is DERIVED from
`disposition.is_hostile()` / `tuning.is_sandbag` at read-model-build time
(`features/ecs/view_index.rs:188`, comment: "a FUNCTION OF STATE, not an actor
type"), so it isn't a stored actor-type fork. But Jon's deeper point stands: the
actor variants (`Enemy`/`Npc`/`Boss`/`TrainingDummy`) should collapse to a single
`Actor` (brains) axis, with `props` (`Chest`/`Pickup`/`Switch`/`Breakable`/
`Hazard`, no brains) the other — and the placeholder-color/z tables + the
`entity_sprite_for_kind` resolvers reduce accordingly. This reshapes the D3
read-model (what `FeatureView.kind` even is) and touches §C (closed vocab →
`actors|props` + open prop ids) and §A (one more actor-unification collapse). It
also means the sim-view crate shouldn't be extracted until this shape is settled
— hence D3.2a was reverted (E24). **Open question for fable:** is `actors` vs
`props` the intended top-level render taxonomy, and should `FeatureVisualKind`
be replaced by it? (Also flow this into `docs/planning/engine/unified-actors.md`.)

> **[fable 2026-07-03] ADJUDICATED: YES — see FABLE ADJUDICATIONS (AD1) below**
> for the binding shape and the migration slices. D3 is unblocked.

---

## FABLE ADJUDICATIONS (2026-07-03) — every open fork, resolved

Written by fable after a fresh read of the landed code (four parallel deep
reads: the A1 archetype-swap commits, the A2 resolver, the full
`FeatureVisualKind` consumer surface, the boss attack geometry) plus a green
test pass (gameplay_core 1091, boss_lifecycle 8 / boss_contact_iframes 4 /
boss_possession_specials 1 / boss_motion_parity 2).

**Verdict on the execution so far: E1–E32 landed as logged.** Spot-checks
found no drift between the log and the code: the A2 one-resolver claim is
honest (all three body kinds route through `resolve_body_hit`; the old
hardcoded −90/−280 pop is gone; every emit site uses `body_vulnerable`; the
only residue is the expected `Without<BossConfig>` partitioning that AS4c
retires), and AS1/AS2/AS4a + the brain half are exactly as described, with
the AS2 cluster correctly inert. The E24 revert and the E25 hold were the
right calls — that discipline (parity net first, hold before relocating into
a fundamental crate) is precisely what to keep doing. The only debris found:
three stale doc lines (fixed this session, see AD5).

### AD1. actors|props IS the taxonomy — the four actor variants collapse to ONE `Actor`

Grounding facts (all verified): `FeatureVisualKind` is **presentation-only**
— zero sim/damage/AI logic branches on it. The actor variants are already
stamped from STATE at the single rebuild site (`view_index.rs`: `is_sandbag`
→ TrainingDummy, `is_hostile()` → Enemy, else Npc; Boss from its own query
family). Sprite resolution is already **name-first** (authored/catalog name
wins; kind is only the placeholder fallback). So the collapse is low-risk and
confined to render's color/z/gate tables — the taxonomy Jon named is what the
code already wants to be.

**The binding shape:**
- `FeatureVisualKind` becomes `{ Actor, Hazard, Breakable, Chest, Pickup,
  Switch }` (keep the type name; renaming is churn without meaning). ONE
  `Actor` variant — Enemy/Npc/Boss/TrainingDummy all stamp it.
- The five **prop kinds STAY closed variants**. They mirror genuine
  interaction-kit component families (`Chest`/`Breakable`/`Switch`/
  `HazardFeature`/pickups) with real view-state semantics (`switch_on`,
  opened-flash, cracking-flash). That is kit vocabulary, not Ambition
  content — an open prop-id string is a knob nobody needs yet
  (design-balance rule). This also answers Jon's point 4: the taxonomy does
  NOT require a new crate; it's an enum reshape in place, and `sim_view`
  returns only when read-model materialization gives it meat (AD-D3 below).
- **[REVISED per Jon, 2026-07-03]** ~~`FeatureView` gains `hostile: bool`~~ —
  Jon: "not hostile, hostile is player centric. hostile to what? relativity
  principle." The state axis is **fighting / not-fighting**, a fact about the
  actor itself, no reference frame. The model, in Jon's words: "FightingAble
  should be a component on all actors and some actors won't have it, and they
  can be in a fighting state or a not fighting state."
  - **Capability:** `FightingAble` — a component an actor carries or doesn't
    (a training dummy: doesn't — the empty component set, per point 3 of the
    feedback). Presence = this actor CAN fight. Same shape as every other
    capability in the kit.
  - **State:** fighting vs not-fighting, on that component (a provoked NPC
    *enters* the fighting state; an at-rest enemy is not-fighting until it
    engages). `FeatureView` gains `fighting: bool` = FightingAble present AND
    in the fighting state — a STATE fact exactly like `flash`.
  - The placeholder tint MAY modulate on `fighting` (an actor entering the
    fight shifting tint is information about state and honors "they are the
    same thing" — the TYPE is one; the state changed). Base placeholder color
    and z are ONE value for every actor. The Npc-draws-one-layer-higher nuance
    dies with the variant (fine pre-release; if actor draw order ever matters
    it must come from a real signal, not visual kind).
  - **Follow-up smell to sweep (entity-id-matches-label + relativity):** the
    sim-side vocabulary that stamps this today is itself frame-tainted —
    `disposition.is_hostile()`, `CombatCapabilities.attacks_player`. Interim:
    stamp `fighting` from the existing disposition signal so T1 doesn't
    balloon; then rename/reshape the disposition vocabulary onto the
    fighting model (its own slice — the aggro/provoke/grudge machinery is the
    natural home of the fighting-state transitions and is already relational).
- `TrainingDummy` dies entirely, per Jon: a sandbag is the most-NPC actor.
  The sandbag fallback sheet keys off `is_sandbag` tuning at the fallback
  resolver (the data is already on the entity; `enemy_visual_kind()` /
  `EnemyIntegration::visual_kind()` — the two DUPLICATE derivation helpers —
  get deleted, their logic surviving only in the one fallback-sprite pick).
- `Boss` keeps NO variant. The boss render path already partitions on its
  own query/view build (`render/actors/boss.rs` builds its own view;
  view_index excludes bosses via `Without<BossConfig>`) — nothing needs a
  `Boss` enum arm; re-key the boss upgrade gate off its own query data.

**Migration plan (opus-ready):**
- **T1 (one bold commit — pre-release, no dual-variant bridge):** reshape the
  enum + stamp `Actor`/`fighting` at the rebuild site + rewrite the render
  tables (`feature_z`, `feature_color`, `pick_placeholder_color`,
  `state_aware_entity_sprite`) + **merge the enemy/npc sprite-upgrade systems
  into ONE name-first actor upgrade system** (the enemy path's chain
  `override_name → enemy_name → npc_asset_for_name → state fallback` already
  subsumes the npc path; the two systems only existed because the variants
  did). Boss upgrade system stays separate until 3f, gated on its own query
  instead of the variant. Delete: the two duplicate `visual_kind` helpers,
  `entity_sprite_for_kind`'s actor arms (test-only today), `is_boss_kind`
  (dead). The compiler drives the sweep — every exhaustive match breaks,
  which is the point. Placeholder color/z changes ship in a `blind fix:`
  commit (visual-only).
- **T2 (the D3 re-opener):** materialize the read-model so render needs NO
  live-query accessors: the view index (already keyed by id string) grows
  the name/sprite-key + anim facts render currently pulls via `ecs_*`
  accessors. Note `FeatureView` is `Copy` today — adding a `String` breaks
  that; either keep identity as the index key with a side map, or accept
  non-Copy when materialization lands (decide there, not before). When T2 is
  real, re-create `ambition_sim_view` — it will have actual meat AND enable
  the edge-cut, which is the condition E24 set.
- Ordering: T1 is independent of AS4b/AS4c and can land now. The
  boss-upgrade-gate convergence piece completes in 3f
  (`BossAnim`→`CharacterAnim`), which is this same taxonomy wearing its
  render-animator face.

### AD2. E31 fork: per-frame sprite-driven attack volumes are CANONICAL — generalize the shared pipeline UP, never flatten the boss down

Grounding facts (verified): the per-frame data model (`AnimationBox.frames`)
lives in engine-neutral `ambition_sprite_sheet`; the HURTBOX consumer
(`CombatGeometry`/`damageable_volumes`) is already actor-general and
per-frame; only the attack-hitbox consumer is boss-only today, and the
actor/player melee path (`manifest_attack_hitbox_world`) samples the coarse
per-animation box ONCE at window entry and freezes it. GNU-ton's authored
10-frame hand/head trajectories (~200px of sweep) are real content that
static volumes would discard. So the fork resolves decisively:

- **(a) Static strike volumes are REJECTED.** Attack volumes that track the
  drawn pose frame-by-frame are exactly the actor-geometry-unification north
  star (ONE sprite-metadata pipeline driving collision/hurtbox/attack). The
  boss is the first consumer of the general mechanism, not a special case to
  be demoted.
- **(b) The general mechanism:** hitbox entities gain per-tick frame-driven
  geometry. A component (shape: `FrameDrivenHitbox { animation key, part }`)
  plus ONE shared system — in the combat layer, NOT boss code — that samples
  `AnimationBox.frames` via the drawn-frame sample each tick and writes
  `Hitbox.half_extent`/`local_offset`; spawned on the telegraph→strike edge,
  despawned at strike end. E31's recommended shape was right; the correction
  is PLACEMENT (generic over any body with sprite metrics, so actor melee /
  the moveset clip-by-phase seam can opt in later and eventually retire
  freeze-at-entry as the only actor mode).
- **(c) Dedup:** strike hitboxes carry `HitboxHits` like every other strike
  (per-swing hit-once). For any strike window shorter than the victim's
  post-hit invuln (0.75s) this is equivalent to today's receiver-side
  throttle — assert that equivalence in the test, don't assume it.
- **(d) The body-contact arm does NOT become a respawned-per-tick hitbox**
  (that shape fights the primitive). Boss contact damage folds onto the
  EXISTING shared body-contact system (`apply_actor_contact_damage`, already
  body-generic per §A4): set the boss actor-cluster's contact tuning from
  `behavior.body_damage` (spawn currently sets `body_contact_damage: false`
  precisely to avoid double-hit — flip it in the same commit that deletes
  the poll's contact arm). Receiver-side i-frames already gate continuous
  contact exactly like today; `boss_contact_iframes` pins the feel.
- **(e) End state: `boss_attack_damage` is DELETED.** Strikes flow through
  `apply_hitbox_damage`'s existing Boss-faction branch (§A3); contact flows
  through the shared contact system. Ships BLIND (feel-sensitive), gated on
  boss_contact_iframes + boss_motion_parity + a NEW frame-tracking test:
  assert the strike hitbox center follows the authored per-frame trajectory
  across a full swing (GNU-ton's `gnu_hand_sweep` is the natural fixture).

### AD3. AS4b/AS4c — the E32 plan is endorsed as written

Spec-parity pin FIRST (render `boss_asset.spec.render_size(kin.size)` vs
gameplay `sprite_metrics.sprite_render_size` for every real boss); if it
holds, the size flip is preserved-by-construction; if it diverges, that's a
latent render/hurtbox bug to fix regardless. Then AS4c with the golden
trajectory pin. Dropping AS5 is also confirmed — `BossRef`/`BossMut` view
encounter concerns, and deleting them is churn, not convergence.

### AD4. The [opus-4.8[1m]] contradiction tags — CONFIRMED, all of them

Each was checked against the code; in every case the executing agent's
narrower measurement beats the audit's wider estimate. Marked inline at each
tag; summary:
- **E19/D1 features hub:** 634 refs (not 271 — that was internal-only), a
  3-layer public facade stack. Family-by-family redirection as each family
  reaches its leaf home is the binding strategy; "one-file data migrations"
  was too sunny for the `components::` symbols.
- **E22/D3 render edge:** the edge is genuinely wider than read-model
  vocabulary — the world/rooms types (category C) and the registered
  presentation SYSTEMS (category D) are real surfaces the audit under-called.
  "Payoff is binary / multi-session" is the honest frame; the D3.2–D3.7
  slice order stands and is now UNBLOCKED by AD1 (T1 then T2).
- **E23 CameraSnapshot2d:** confirmed NOT a clean mover (settings/rooms/
  camera_ease imports). Move it LAST, or first invert those into a small
  camera-config value type.
- **E25/D4 outbound surface:** confirmed bigger than audited; the audit read
  the inbound surface. D4.1 resolved the linchpin correctly. The LDtk
  **converter extensibility** (content-registered entity converters,
  ADR-0009-shaped) remains the crux and is worth its multi-session cost —
  it IS the "second game ships its own world" oracle.
- **E32 AS5 drop:** confirmed (see AD3).

### AD5. Housekeeping done by fable + the queue only Jon can drain

Fixed this session (stale-doc smells from the rename): the
`boss_clusters.rs` module doc still claiming BossEncounter holds
health/liveness/hit-flash; the `boss_encounter/registry.rs` comment naming
the deleted `.health` field; `unified-actors.md`'s stale "separate
BossStatus" line (+ the actors|props taxonomy note flowed in per Jon's ask).

**Jon's queue (nobody else can do these):** feel-check the BLIND commits —
A2 knockback (`b4912001`) + stagger (E13: enemies flinch, duels read
launch→recover→re-engage), boss no-i-frame (E15, numerically a no-op today),
and the upcoming AS4b size flip, AS4c fold, and AD2 conversion when they
land.

---

## JON'S DESIGN DIRECTION (2026-07-04) — FOR FABLE TO ADJUDICATE

Triggered by the player-melee-fold fork (BULK REVIEW QUEUE) + a briefing on the
open decisions. Jon's direction, to be turned into a spec by fable:

### JD1. Abilities = parameterized effects (data) + arbitrary code (content plugin). Resolve the player-melee fork into this, don't pick A/B/C.

Jon's framing (verbatim intent): a FIXED verb vocabulary can't express
smash-like variety unless it's "one verb per possibility, which isn't
efficient." Attacks must be able to "run some code that responds to some tag —
the Bevy way," with a **static/prefab default**, escalating to **arbitrary
code** when needed. The arbitrary code "should be able to be passed PARAMETERS
via the character publishing," and the prefabs (`simple_melee`, `simple_u_tilt`,
`simple_ranged`, …) are **given those parameters** — so a later `sword_slash` is
"the `simple_melee` prefab with sword params (knockback, …) authored per
character, perhaps modified by items," no new code.

`[opus-4.8[1m]]` **the seam already substantially exists** (surfaced during the
briefing, for fable to build the spec on, not re-discover): a move's
`MoveEventKind::Effect{key}` (`ambition_entity_catalog/src/lib.rs:105`) bridges
to a `Special(key)` signal (`combat/moveset.rs:480`) that a **content-owned Bevy
system** keyed to the string runs — see the live ability library
`ambition_content/src/bosses/specials/*.rs` (`seismic_stomp`, `eye_beam`,
`overflow_flood`, `gradient_nova`, …), installed via `register_required_components`.
Core never matches the key; content adds an ability by dropping in a module. So
"abilities as arbitrary content code responding to a tag" is the ESTABLISHED
architecture; the pogo-"pollutes-the-runtime" worry in the player-melee fork
dissolves (pogo becomes such a Technique). **The gaps Jon's direction adds:**
1. **Params on the effect.** `Effect{key}` is a bare tag today (the technique
   reads its own params from content state). Jon wants params PASSED from the
   published character data → `Effect{key, params}`, so a parameterized prefab
   ("simple_melee" + {knockback, damage, reach}) needs no bespoke code, and items
   can modify the params at resolve time. Params likely an OPEN serialized value
   the effect deserializes into its own type (so a custom game's effect defines
   its own param shape — keeps core ignorant, preserves decomposability).
2. **A prefab (middle) tier.** Between static hit-volume DATA and arbitrary-code
   technique, a set of parameterized KIT prefabs (`simple_melee`/`u_tilt`/
   `simple_ranged`) authored purely by params. Answers the fork's real question.
3. **Input→move mapping in the PUBLISHED character data.** The sprite generator's
   emitted character data should declare which inputs its moves map to BY DEFAULT
   (directional intent → verb/move), perhaps overridable. This is where the
   directional-melee variants (up/down/air-*) live — as data, not a schema fork.
4. **Decomposability / compile-time is a first-class constraint.** A custom game's
   ability code lives in ITS content/plugin crate (downstream of core), registered
   at plugin build — so adding an ability recompiles content, not core. Fable's
   spec must keep the effect-registration seam content-side (it already is) and
   ensure params flow as data, not as core-known types.
**Open for fable — the params VALUE TYPE (Jon wants fable's read; "don't code
ourselves into a corner"):** all authored as RON, so bakeable at compile time via
`include_str!`+parse EITHER way — bakeability isn't the deciding factor. The corner
to AVOID is a closed typed enum core knows (the only non-decomposable option). Three
decomposable candidates:
- **(A) Opaque serde value + per-effect `Deserialize` struct** `[opus-4.8[1m]] lean`:
  the move carries params as a `ron::Value`/blob; each effect owns
  `#[derive(Deserialize)] SwordSlashParams{knockback,…}` and hydrates it at trigger.
  Typed AT the effect, core stays ignorant (decomposable), no registry ceremony,
  RON-bakeable. Mirrors how boss techniques already "read their own params."
- **(B) Bevy `Reflect`** (Jon: "the more professional option?"): each effect
  `#[derive(Reflect)]`s its params + registers the type; the move-data RON hydrates
  via the type registry. Buys the whole reflection ecosystem — INSPECTOR + scene
  integration + a future visual move/param EDITOR — at the cost of type-registration
  ceremony. Earns it IF a visual character/move editor is on the roadmap.
- **(C) `HashMap<String, f32>`** (Jon: "not opposed"): simplest, fully data-driven,
  but stringly-typed + no per-effect validation. Fine for numeric-only params;
  weakest for structured ones.
  `[opus-4.8[1m]]` recommend **(A) as the pragmatic default, (B) if a visual editor
  is planned** — but flagged for fable's wider-view call.
- **Item ↔ params (open, Jon "it depends"):** items either MERGE modifiers into the
  params blob at trigger-resolve (data-level: +knockback) or are COMPONENTS the effect
  reads (behavioral override). Likely both — numeric via merge, behavioral via
  component. fable to pin.
**Also open for fable:** dispatch shape (keep string-key `Special` MESSAGE, or move to
a marker-COMPONENT + observer per active window — more ECS-native); and the exact
published-character-data schema (moves + input map + per-move effect refs + params).
Jon: "we can keep discussing" + "I think fable should weigh in too" — direction, not
a final spec.

### JD2. C1 item catalog — BUILD IT as architecture prep (Jon overrules the defer).

Jon: preparing the install seam BEFORE a second game is the point ("I don't want
to build a second game against a moving target, but I want to prepare so when I do
it's easy"). "No consumer yet" is circular and not a real objection. The
roster-install pattern is proven (enemies/bosses/characters/specials), so an
`ItemCatalog` following it is low-risk prep; the `Item` enum's baked flavor text →
content is genuine "content out of core." Incremental is fine (C1 first, then the
held-registry C2 / projectile-spec C5 chain) — the "partial seam" objection raised
in the briefing was NOT real (Jon: "I still don't get the problem"). **Autonomous.**

### JD3. A1 boss driver fold — FINISH IT. Shape settled (Path B); the finish is nuanced, not a rubber-stamp.

Jon confirmed: "we have to finish that." The boss = actor archetype shape is
locked; remaining is execution: retire `BossStatus`/`BossAttackState` as
authorities, fold `update_ecs_bosses`/`tick_boss_brains` into the actor systems,
`BossAnim`→`CharacterAnim`. Real nuance (E53): retiring the `BossAttackState`
brain-write is NOT a dead-write removal — the trigger reads it as its intent
signal, so it needs an intent-component split. **Autonomous** (BLIND for the
render animator half). See A1/slice-3 below.

### JD4. D-front rooms/RoomSpec — Jon adjudicates WITH discussion. Map DONE (`[opus-4.8[1m]]`).

Jon: "I'll try to adjudicate, but we do need to discuss." Dependency map produced.
**Headline: the `RoomSpec`/`RoomSet` TYPES are already clean generic KIT — the leak
is NOT in the types.** `RoomSpec` (`world/rooms/room_graph.rs:10-48`) names no
specific room/boss; it's a pure LDtk-authored schema (world geometry, loading zones,
portals, gravity zones, the `Authored<T>` per-family entity lists). The sole
constructor is LDtk → `RoomSet::from_parts` (`world/rooms/graph.rs:27`) — no
hardcoded `RoomSpec {…}` literals, no static room table, no named-room fns. That half
is already the north-star shape.

**The actual leak is three-fold (all content EMBEDDED in core):**
1. **Core physically OWNS the world FILES + a hardcoded list.** The `.ldtk` files
   live in `gameplay_core/assets/ambition/worlds/{intro,cut_rope,hall,sandbox}.ldtk`;
   `world/ldtk_world/loading.rs:55-57` hardcodes `secondary_world_ids()` and
   `:222-250` `include_str!`-embeds those exact files. Content authors ZERO worlds.
2. **Core modules branch on specific room-id LITERALS to run bespoke content
   mechanics:** the whole `falling_sand.rs` module (gated `room.id == "falling_sand_room"`,
   ~11 systems early-return otherwise); `features/arena.rs` duel-arena staging
   (`"duel_arena"` → auto-spawns two named fighters, hooked into the GENERIC room-load
   path at `spawn/mod.rs:72` so every load calls a duel hook); `hall_of_characters`
   roster/dialogue/sfx tables; the cut-rope boss `placement_id`.
3. **`"central_hub_complex"`** is the hardcoded default start room (`conversion/mod.rs:51`).

**No content→core WORLD-registration seam exists** — content installs rosters/music/
quests/bosses via `AmbitionContentPlugin`, but there is NO `WorldRegistry`/`add_world()`;
core still owns world assembly end-to-end. `RoomSet::from_parts` is already
content-agnostic — it just needs an open feeder.

**`[opus-4.8[1m]]` proposed adjudication (for Jon to confirm/modify + fable to spec) —
mirrors actors|props one level up:**
- **KEEP in core:** `RoomSpec`/`RoomSet`/`RoomMetadata`/`LoadingZone`/`Authored<T>`
  types + the LDtk→runtime projection + validators. That's the world KIT.
- **MOVE content-side:** the `.ldtk` payloads, the hardcoded world list, and the
  room-id-keyed mechanic branches (falling-sand, duel-arena, hall, cut-rope).
- **BUILD:** an open world-registration seam (the `AmbitionContentPlugin` analogue for
  worlds) so content declares its own LDtk world ids + `.ldtk` payloads and hands
  assembled `RoomSpec`s to `from_parts`; per-room mechanics attach via a **room-id-keyed
  content hook registry** ("on load of room X, run my systems") instead of
  `if room.id == "…"` branches in core, OR via the generic `Authored<T>` lists where the
  mechanic is really just authored entities.
- **THE DISCUSSION POINT for Jon:** the per-room-mechanic shape. Duel-arena staging is a
  spawn-hook (fits a room-id hook registry cleanly); falling-sand is a whole simulation
  module (a content plugin gated on its room's presence); hall-of-characters is mostly
  authored NPCs + dialogue (could be pure `Authored<T>` data + content dialogue). Is one
  uniform "room-load content hook" seam right, or do these split by kind (data vs plugin
  vs hook)?

**[ADJUDICATED by Jon 2026-07-04]:**
- **LDtk is THE level-authoring path, permanent** ("that is always the case, I don't
  see that changing"). Legacy RON levels: VERIFIED GONE (`[opus-4.8[1m]]` — no `.ron`
  level files, no RON room loader in core; the only hits are unrelated comments).
- **BUILD the content world-registration seam** — "I do want ldtk in content. So the
  seam that lets content own the ldtk world is right." So: move the `.ldtk` payloads +
  the hardcoded world list OUT to `ambition_content`, and add the world-registration
  path (the `AmbitionContentPlugin` analogue for worlds) that feeds `from_parts`. Core
  keeps the `RoomSpec`/`RoomSet` KIT + the LDtk→runtime projection.
- **Per-room mechanics: SPLIT BY KIND** — Jon: "split by kind seems to make sense, less
  indirection." So each mechanic goes to the LIGHTEST seam: authored `Authored<T>` data
  where it's really just entities (hall NPCs); a self-gating content plugin for a heavy
  sim (falling-sand); an id-keyed load hook ONLY for the genuinely-imperative spawn-time
  ones (duel-arena staging). No single uniform hook.
- **fable to refine** the exact seam shape (Jon: "maybe fable can help here too"):
  what the world-registration API looks like, and the precise boundary of the room-id
  load-hook registry vs. `Authored<T>`. **Autonomous to EXECUTE once fable sizes it.**

---

## Synthesis — the top of the stack

If we could only do six things, in dependency order:

0. **Build the C4-symmetry harness at the body-tick level, then sweep the
   reaction-seam frame bugs** (B, esp. B1–B6). The movement core's frame discipline
   is genuinely strong; nearly every real physics bug found is an *epilogue* — a
   screen-frame fallback or cleanup after a frame-correct verb (post-blink clamp,
   slash recoil, moveset hitbox spawn, stale `surface_normal` consumers, the
   role-welded collision guards). A conformance rig at
   `update_body_with_tuning_clusters` (like the existing `step_kinematic` rig)
   driving attack/blink/knockback scenarios would trip five of them at once and
   guard every future fix. This is the "symmetry-under-gravity = strongest test"
   principle made mechanical.
1. **Delete the internal facade layer** (D1, ~0 risk, mechanical). Every dependency
   count inside `gameplay_core` is currently a lie: 271 internal refs name
   `crate::features::X` for symbols that live in `combat/`; 93 refs import
   `SfxMessage` through `crate::audio` when it lives in `ambition_sfx`; `crate::effects`
   and `crate::time` re-exports likewise. This is the prerequisite that de-risks every
   other extraction, and it is exactly the pre-release compat tax AGENTS.md says to delete.
2. **Unify the victim-side damage resolver** (A2+A3+A4+A5). Three consumers, three
   knockback models, i-frames checked at emit-time for players but consume-time for
   actors, hazards/contact/boss damage physically unable to hit non-players. This is
   the largest live violation of ONE BODY, ONE PATH and it blocks emergent play
   (lure a boss into lava). Small first steps exist (A5/A6 are S-sized).
3. **Dissolve the boss island** (A1). Bosses still carry a full parallel actor stack
   (`BossStatus`/`BossAttackState`/own integrator/own damage consumer/own animator
   rows). Everything needed to fold them onto the body vocabulary now exists and is
   proven (melee, movement limbs, relational damage).
4. **Item catalog + held-item registry → the roster install pattern** (C1+C2). The
   24-item `Item` enum with baked flavor text lives in machinery, and the full weapon
   table (`HELD_ITEMS`) is a hardcoded static in a *foundation* crate. The proven
   enemy/boss-roster pattern (generic schema + content-installed data) applies directly.
5. **Cut the `render → gameplay_core` edge via a sim-view crate** (D3, with D2 as
   its 300-LOC opener). The materialized read-model (`FeatureViewIndex`) already exists;
   moving it (plus `BodyHealth`/`BodyCombat`/`BodyWallet` down to `ambition_characters`)
   drops presentation out of the hot-edit rebuild path — the single biggest
   compile-time lever that doesn't touch the hard mechanics knot.

Recurring meta-finding: **the good seams already exist; the leaks are refusals to use
them.** `Special(String)` exists but presets can't reach it; `WorldView` exists but
brains side-load `BrainSnapshot`; `FrameEvents` exists but only the player's are
consumed; the roster-install pattern exists but items/worlds/catalogs don't use it.
Most fixes are "route the outlier through the existing seam and delete the fork,"
which is precisely the AGENTS.md unification directive.

---

## A. Actor unification — remaining forks (ranked)

### A1. The boss island — a full parallel actor stack (L)
`combat/boss_clusters.rs:47-71` (`BossStatus { health, alive, hit_flash, … }`),
`:201-224` (`BossMut::integrate_body`), `features/ecs/bosses/tick.rs` (whole file),
`features/ecs/bosses/sync.rs:20-40`.
Bosses duplicate nearly every unified body-fact: `BossStatus.health/alive/hit_flash`
vs `BodyHealth`/`BodyCombat` (and `sync.rs` *mirrors* BossStatus onto the body
read-models — a dual-authority copy the actor path just retired); `BossAttackState`
vs `BodyMelee`/`MeleeSwing`; `tick_boss_brains_system`+`update_ecs_bosses` vs
`tick_actor_brains`+`integrate_sim_bodies`; a separate victim consumer
(`damage/boss_hit.rs`) and a separate render animator
(`ambition_render/src/rendering/actors/boss.rs`, `BossAnim` vs `CharacterAnim`).
The boss integrator calls `step_floating_body` directly — it never enters the shared
ability-limb pipeline, so a boss can't dash/shield/blink via capability mask (I7
half-broken: player-robot-as-boss works, boss-rising-to-the-kit doesn't). Boss
possession (`tick.rs:124-188`) had to re-implement input→special mapping bespoke
because of this. `unified-actors.md` already names this "a parallel island, a later
slice" — it is the single largest remaining fork.
**Seam:** a boss is an actor archetype (capability mask + `BossPattern` brain +
phase-state component); delete `BossStatus`/`BossAttackState`/`update_ecs_bosses`.

### A2. Victim-side damage: three consumers, three knockback/death models (L)
`combat/damage.rs:338-471` (`apply_player_hit_events` → `handle_player_damage_events`),
`features/ecs/damage/actor_hit.rs:40-307`, `features/ecs/damage/boss_hit.rs`.
A hit on the player gets shield-block, difficulty scaling, feel-tuned frame-agnostic
knockback (`resolved_player_knockback_velocity`, damage.rs:243-274), hitstun +
recoil-lock + hitstop, and death→respawn. A hit on an actor (`actor_hit.rs:191-201`)
gets an inline `knock_x` plus a **hardcoded −90 vertical pop capped at −280** (not
frame-resolved, not feel-tuned) and **no hitstun/recoil/hitstop at all**. Death is
forked too (player → `death_respawn_player`; actor → inline drops/banner/timer).
Respawn destination and difficulty assist are genuine policy; knockback resolution,
hitstun, and shield consume are mechanics and should be one resolver
(`shield_blocks_hit` is already shared — proof the merge works).
**Seam:** one `apply_body_hit(body, event)` mutating `BodyHealth`/`BodyCombat` +
kinematics for every body, per-body death/respawn POLICY as data.

### A3. `apply_hitbox_damage`: three victim loops inside "one" system (M)
`combat/hitbox/mod.rs:57-337`. Actor victims resolve via `CenteredAabb` +
`damage_lands` with `knockback: None` (`:151-184`); a *separate* player loop rebuilds
a gravity-framed hurtbox from `BodyKinematics`, evaluates a 4-term vulnerability
predicate at emit-time, and inlines SFX/VFX/knockback (`:199-269`); player-faction
strikes take a third route (Volume broadcast, `:280-331`). i-frames are checked at
emit-time for player victims but consume-time for actor victims; knockback attaches
at emit for players, consume for actors.
**Seam:** one victim loop over "any body with a hurtbox + faction", vulnerability and
knockback resolved in ONE place (the consumer).

### A4. World damage only exists for players: hazards, body-contact, boss attacks (M)
`combat/hazards.rs:8-91` (hazard query is `With<PlayerEntity>` only),
`features/ecs/actors/update.rs:709-795` (`apply_actor_contact_damage` resolves
targets exclusively through `player_query`), `features/ecs/bosses/tick.rs:360-369,
455-499` (`update_ecs_bosses` damage targets only `PlayerEntity`).
An NPC can stand in lava unharmed; a boss's swing passes through an Npc duelist; an
enemy's body contact can never hurt the boss it feuds with. B1/B2 made
hitbox/projectile damage relational, but contact/hazard/boss emission still
hard-queries players. Guardrail 4 says hazards shouldn't be faction-*gated* — here
they're player-*scoped*, which is stronger and worse.
**Seam:** these emitters iterate "every vulnerable body whose faction the source can
damage", stamping `HitTarget` per victim. Mechanical but touches feel.

### A5. Player-vulnerability predicate copy-pasted at 5 emit sites (S)
`combat/hitbox/mod.rs:211-215`, `features/ecs/bosses/tick.rs:461-463`,
`features/ecs/actors/update.rs:763-766`, `combat/hazards.rs:60`,
`projectile/systems.rs:655` (drops the shield term — **already drifting**).
`!offense.invincible && !dodge_rolling && !shield.parrying() && combat.vulnerable()`
is re-derived per site. The player hurtbox is also built differently per site:
gravity-oriented `collision_aabb` in hitbox/mod.rs:199-210 vs raw `kin.aabb()` in
`apply_actor_contact_damage:762` and `update_ecs_bosses:460` — **under rotated
gravity these disagree** (also a relativity bug).
**Seam:** one `body_vulnerable()` + one hurtbox accessor; folds into A3/A4.

### A6. Hurtbox authority: actors publish `CenteredAabb`, the player doesn't (S/M)
`features/ecs/actors/update.rs:503-519` (actor publishes the frame-oriented
`CenteredAabb` billed as "the single source of truth") vs `combat/hitbox/mod.rs:113-125`
(owner resolution needs a two-way fallback because the player has none). Every
consumer that needs "a body's combat box" carries an actor path and a player path,
and they've already diverged (A5).
**Seam:** the player publishes the same `CenteredAabb` in `integrate_sim_bodies`.

### A7. Perception: `BrainSnapshot` is a second observation seam beside `WorldView` (M/L)
`features/ecs/actors/update.rs:341-402`, `:962-1021` (`build_enemy_brain_snapshot`).
Brains observe through TWO structs: the omniscient `BrainSnapshot` (target injected
from `ActorTarget` — no viewport, no line-of-sight, no memory) plus a terrain-only
`WorldView` whose peers/projectiles are hardcoded empty slices and whose faction is
hardcoded `ActorFaction::Enemy` (`:381`). S4/S5 is scaffolded, not done: the
world-out port exists but the real observation channel is the side-loaded snapshot.
A brain driving the player-robot body today gets an Enemy-flavored, target-omniscient view.
**Seam:** `WorldView`(+`WorldMemory`) becomes the ONLY world-out (peers/projectiles/
target wired in, faction from the body); `BrainSnapshot` shrinks to proprioception +
controller input.

### A8. Movement presentation: `FrameEvents` → SFX/VFX only for the player (S/M)
`player/movement_fx.rs:25-194` (`handle_player_events`, player tick only) vs
`features/ecs/actors/update.rs:492-502` (actor path consumes ONLY `move_events.blinks`
and drops every other op). An actor that jumps/dashes/dodge-rolls/wall-jumps/
ledge-grabs/shields produces **no dust/SFX**, while blink SFX got a hand-copied
second emit site (movement_fx.rs:168-182 AND update.rs:492-502 — the exact "parallel
emission site" AGENTS.md calls a bug).
**Seam:** one body-generic `FrameEvents`→facts system for every body.

### A9. `PlayerAnimState` presentation timers have no actor analogue (S)
`player/components/mod.rs:77-110`, `character_sprites/anim/mod.rs:667-716` vs
`:779-836`. The anim ladder is genuinely unified (one `pick_body_anim`); the overlays
fork: shoot/aim/wall-jump/interact/landing/dash-startup/blink-in poses are armed only
for the player; actors fire projectiles and wall-jump but can never show those rows.
Hit read differs too — `hitstun_timer` (player) vs `hit_flash` (actor), a consequence
of A2.
**Seam:** body-generic `BodyAnimFacts` armed by the shared events system (A8).

### A10. Projectile FIRE control: player charge-machine vs `try_fire_ranged`; parry is player-only (M)
`projectile/` (player pool: `PlayerProjectileState` charge/mana/cooldown machine) vs
`enemy_projectile/` (enemy pool + builder). In-flight stepping IS unified
(`step_projectiles`) and faction is owner-derived, but two pools/markers/spawn paths
remain, and the player's fire-rate/charge enforcement lives on the controller side —
an I3 violation. The spawner fold is *deliberately deferred* (feel-sensitive, per
unified-actors.md); actionable now: the parry asymmetry (`projectile/systems.rs:640-650`
reverses + re-owns + heals for players only — a shielding actor can never parry) and
the dual pool markers.

### A11. `SpecialActionSpec` residual closed variants + boss special dispatch bypass (S)
`ambition_characters/src/brain/action_set/mod.rs:486-508`. `Special(String)` exists
(good) but `BubbleShield` ("Player-only") and `BossSpotlight` ("Boss-only") remain
actor-kind-scoped variants, and multi-special bosses bypass `emit_brain_action_messages`
entirely — the boss tick writes `ActorActionMessage::Special` directly
(`bosses/tick.rs:219-240`, second bespoke copy in the possession arm `:145-187`).
Dissolves with A1. See also C8/C10 (the preset layer can't even reach `Special(String)`).

### A12. Interaction/affordance consumers are primary-player-gated (M — documented deferral)
`features/ecs/interact.rs:36-37`, `player/affordances/*`, `combat/pickups.rs:18,38`.
Intent seam is body-generic (`interact_pressed`); every consumer is player-gated.
Already recorded as the "NPC agency" deferred item (guardrail #6) — listed so it
isn't re-discovered.

### Player-branch classification (inside shared systems)
**Legitimate policy (keep):** slot-input sourcing for a possessed body; effective-
allegiance/`effective_faction`; `control_dt`; respawn destination; shrine heal+save;
the documented aim-resolving held abilities on `held_shot_aim_local`.
**Illegitimate (mechanics in a player branch):** the boss possession input→special
mapping (A11) and the player-scoped damage emitters (A4).

### Verified ALREADY UNIFIED (don't re-audit)
Movement (one engine seam, `integrate_sim_bodies` for actors AND player; bespoke
integrators deleted; flight/dash/blink/shield ride capability-masked limbs); melee
end-to-end (`start_body_melee`/`advance_body_melee` for EVERY body, one
`spawn_melee_strike`, one `emit_melee_slash`, one `BodyMelee` — a stale doc comment
at `combat/attack.rs:246-248` still claims the fork exists; fix the comment); anim
ladder (`pick_body_anim`); `shield_blocks_hit`; `BodyCombat`/`BodyHealth` single
authorities (bosses excepted); projectile stepping/attribution; relational
targeting + grudge; moveset runtime spawns the same `Hitbox` entities; ability
systems act on `ControlledSubject` with no player filters.

**Suggested attack order:** A5→A6 (tiny, unblocks) → A3+A4 (one relational
emit/consume shape) → A2 (victim resolver merge, behind the differential trace) →
A8+A9 (presentation) → A1 (boss island, using the proven body seams) → A7
(perception, S4/S5) → A10/A11 residuals.

---

## B. Physics / gravity / frame-of-reference (ranked, likelihood × impact)

**Meta-observation (the pattern behind all of these):** frame discipline is genuinely
strong at the *movement* layer; nearly every real finding is at a **reaction/effect
seam** — a verb correct in its main path with a screen-frame epilogue or fallback.
A cheap systemic guard: a C4-symmetry harness at the
`update_body_with_tuning_clusters` level (like the `step_kinematic` conformance rig)
driving attack/blink/knockback scenarios — B1, B3, B4, B5, B6, B9 would all trip it.

### B1. HIGH — Moveset hitboxes spawn in the SCREEN frame, not the owner's gravity frame
`combat/moveset.rs:138,143` builds the Active-window volume offset as
`Vec2::new(offset.0 * pb.facing, offset.1)` into `HitboxAnchor::FollowOwner`;
`Hitbox::world_volume` (`ambition_vfx/src/lib.rs:91`) adds it to `owner_pos`
**unrotated**. Under gravity=right, an authored above-the-head volume spawns
screen-up — into the effective ceiling. This is the Smash-model moveset runtime
meant for every actor, and it forks against the player melee path, which is correct
(gravity-aware manifest → `spawn_melee_strike`).
**Fix:** rotate the authored offset through `AccelerationFrame::to_world` at spawn —
the same seam `spawn_melee_strike` uses.

### B2. HIGH — `ActorSurfaceState::surface_normal` is a stale frame source for every non-surface-walker
Consumers derive the actor's frame as `-em.surface.surface_normal`: shield block
(`features/ecs/damage/actor_hit.rs:164-174`), slash-knockback (`:192-200`),
ranged-fire muzzle+direction (`features/ecs/brain_effects.rs:115-141`), sprite
rotation (`features/ecs/view_index.rs:213`). But `surface_normal` is written **only**
by the surface-walker path (`features/enemies/integration.rs:337-373`,
`fall_until_landed:445`) — regular actors keep their spawn constant `(0,-1)` forever
(`features/ecs/actor_clusters.rs:435,567`). Movement itself is correct
(`gravity.dir_at`, and `actors/update.rs:504-510` even knows to only trust the
normal for surface-walkers). Under gravity=up, an enemy on the ceiling blocks hits
from the wrong side, gets knockback popped INTO its floor, and fires projectiles in
the down-gravity frame — while its movement correctly obeys the flip. A pure
player/actor asymmetry (the player path uses live tuning gravity).
**Fix:** consumers use `gravity.dir_at(kin.pos)` unless
`tuning.surface_walker && on_ground`; reserve `surface_normal` for clung surfaces.

### B3. HIGH — Post-blink velocity damp/clamp is on world X/Y axes
`ambition_engine_core/src/movement/blink.rs:37-42` (`complete_blink_clusters`):
`vel.x *= damping; if vel.y > max_downward { clamp } else { damp }`. Under
gravity=left/right the actual fall axis is world X — damped but never clamped
(chained blinks inherit unbounded fall speed) while the harmless perpendicular axis
gets the clamp. Under gravity=up a true fall is never clamped and rising velocity
wrongly is.
**Fix:** `to_local` via `AccelerationFrame::new(tuning.gravity_dir)`, damp `.x`,
clamp `.y`, `to_world` back.

### B4. HIGH — Slash recoil kicks along world X instead of the local side axis
`ambition_engine_core/src/movement/control.rs:130`:
`kinematics.vel.x -= kinematics.facing * tuning.slash_recoil`. Under
gravity=left/right the side axis is world-vertical, so attacking shoves the body
along the gravity axis — a slash pushes you off/onto your wall-floor.
**Fix:** `vel -= frame.side * (facing * slash_recoil)`.

### B5. MED-HIGH — The spurious-graze guards in the player sweep are welded to world axes, not axis *roles*
`ambition_engine_core/src/movement/collision.rs`: `body_is_side_contact`
(`:111-114`) is written in Y top/bottom terms, gated `role == AxisRole::Gravity`
(`:279`); the X-sweep's counterpart protections (defer-to-other-axis, world-bounds,
the motion-continuation at `:201-210`) run only in the X=Side role. Under
gravity=left/right the roles swap and both guards vanish from the axes that need
them: the run axis (now Y) loses side-contact rejection + continuation → wall-running
stutters/stalls on a non-immediate graze; the gravity axis (now X) accepts exact-edge
side contacts → spurious landings (`on_ground` + feet snap + free jump refresh)
against surfaces the body merely slides past. The `is_contact_range_snap` bound
(post the 2026-06-25 sideways-hub OOB) caps this to stutter/false-ground, not OOB.
**Fix:** phrase both guards in role terms (`body_is_nested_along(axis)` whenever the
swept axis is the gravity axis; a generalized `resolve_side_penetration(axis)` with
defer/bounds/continuation whenever the swept axis is the side axis) so the pair
rotates with gravity.

### B6. MED — Wall-ability ordering differs between the two gravity-axis branches of the body tick
`ambition_engine_core/src/movement/integration.rs:176-217`: vertical gravity runs
sweep-side → `apply_wall_abilities` → reset `on_ground` → sweep-gravity; horizontal
gravity runs sweep-side → reset → sweep-gravity → stabilize → `apply_wall_abilities`.
Under sideways gravity the wall-slide clamp applies *after* gravity-axis motion
(steady-state slide ≈ `wall_slide_speed + gravity·dt`), climb response lags a frame,
and `on_ground` is read at different snapshots per branch. Not identical local traces
under C4 rotation — exactly what the conformance tests pin for `step_kinematic` but
not for this player path.
**Fix:** one branch: side-sweep → wall abilities → clear ground → gravity-sweep
(+ stabilize when gravity is on X), consistent `on_ground` snapshot.

### B7. MED — Body out-of-bounds reset only triggers past the world's *bottom* edge
`ambition_engine_core/src/movement/mod.rs:315-317`: `pos.y > world.size.y + 200.0`.
Under gravity=up/left/right a body exits through the top or a side and never trips
the reset — it falls forever (the exact symptom class the OOB flight recorder hunts).
**Fix:** gravity-relative exit test —
`(pos - world_aabb.clamp(pos)).dot(gravity_dir) > 200.0`.

### B8. MED — Portal-gun aim skips the acceleration-frame seam every other aimed ability uses *(verify against the portal agent's latest)*
`ambition_content/src/portal/input_adapter.rs:34-44` (`pick_aim`) returns the raw
stick and falls back to world-horizontal `(±1, 0)` on neutral input
(consumed by `fire_adapter.rs:47-51`). Grapple/blink/meteor/vortex/fireball all
resolve through `AccelerationFrame::to_world(resolve_aim_local(..))`
(`items/pickup/mod.rs:635-658`). Under sideways gravity a neutral-stick portal shot
fires world-horizontal — into or out of the wall-floor — and ignores the
body-relative-aim setting.
**Fix:** route through the shared `ability_aim_world`/`resolve_aim_local` seam.

### B9. MED — Blink zero-stick fallback and default aim offset are world-X
`ambition_engine_core/src/movement/blink.rs:53`, `control.rs:32,40,66-67,105`:
fallback/default aim = `Vec2::new(blink_distance * facing, 0.0)`. The stick paths
are correctly world-resolved; only the no-input fallback is raw. Under sideways
gravity a no-direction quick blink teleports along the gravity axis instead of
forward along facing. Same class as B8.
**Fix:** `frame.side * (blink_distance * facing)`.

### B10. MED (latent) — `Hitbox::world_volume` pins shaped volumes to screen-down
`ambition_vfx/src/lib.rs:95`: `shape.place_at(center, facing, Vec2::new(0.0, 1.0))`.
`VolumeShape::place_at` is fully gravity-capable; the caller hardcodes the frame.
Only orientation-invariant circles reach it today — the first authored OBB slash-arc
will be gravity-locked.
**Fix:** carry the owner's `gravity_dir` on the hitbox.

### B11. LOW-MED — Knockback side computed in screen-X at the source
`combat/hitbox/mod.rs:226` (`center().x >= owner_pos.x`) and
`projectile/systems.rs:659`. The consumer (`resolved_player_knockback_velocity`)
recomputes gravity-relatively and uses the stored `dir` only as a degenerate-case
fallback — but under sideways gravity attacker/victim separate along world-Y, which
is exactly when the projection is ~0 and the screen-frame fallback decides.
**Fix:** compute `dir` at the source as `sign((victim - owner)·frame.side)`.

### B12. LOW — Query-iteration-order dependence without stable keys
- Portal transit entry/rescue picks the **first** qualifying portal from a `Vec`
  collected off a `Query<&PlacedPortal>` (`ambition_portal/src/placement.rs:482-540`;
  same pattern `transit.rs:433-446`). Overlapping capture boxes (inside corner) →
  which pair you transit depends on archetype order. *(verify against latest)*
- Nearest-foe targeting tie-break (`combat/targeting.rs:266`) keeps the
  first-visited candidate on an exact distance tie.
**Fix:** deterministic tiebreak (deepest penetration / lowest channel id; stable id
per the query-order-determinism rule).

### B13. LOW — `FlipGravity` negates only `dir.y`, a no-op when ambient gravity is sideways
`encounter/systems.rs:277` (`base.dir.y = -base.dir.y`) + test twin
`gravity/lifecycle.rs:63`. After a Noether-Chamber `SetGravityLeft/Right`, the hub's
flip switch does nothing.
**Fix:** `base.dir = -base.dir`.

### Minor notes
- `player/body_integration.rs:179` — hard-fall screen-shake reads `vel.y`;
  presentation-only misfire under sideways gravity. Use `vel.dot(frame.down)`.
- `falling_sand.rs:816` — sand-stream VFX falls world-down with `Res<Time>` (not
  SimDt/GravityField); visibly wrong under a flip in that room.
- `platformer_primitives/src/gravity.rs` — `GravityField::vertical_sign` /
  `local_gravity_sign` have **zero consumers** and the module doc still claims the
  collision controllers use them: dead API + docs-describe-dead-things smell.
- **Mockingbird OOB: the memory/tooling note "still-unfixed" is STALE** — the
  2026-06-21 fix (`is_contact_range_snap` on every snap/push) is in place with a
  regression test. Residual OOB risk concentrates in B5's role-swapped guard holes —
  hunt there if it recurs under non-default gravity.

Known-open items from prior work (for cross-reference, `code_smells.md` 2026-06-15):
directional attack hitbox offset world-locked (`ambition_combat/src/lib.rs:446` —
same family as B1/B10); `ground_gap_below_feet` probes world-down
(`app/world_flow.rs:63`); thrown ground-item gravity world-locked
(`items/pickup/mod.rs:169`); player knockback untested under gravity flip (B11 is
the concrete mechanism). New from audit A: the player hurtbox emit-site divergence
under rotated gravity (A5).

### Areas verified CLEAN
`reference_frame.rs` (`AccelerationFrame`) — exemplary, pinned across all four
cardinals against frame-of-reference.md; `collision_semantics.rs` kernel
(gravity-relative, C4-tested, `supporting_block` ≤4px bound);
`platformer_primitives::kinematic::step_kinematic` (role-ordered sweeps, C4
trace-conformance tests); `integrate_normal_spine` + flight/climb/jump-buffer/
coyote/wall-jump/dodge/dash/jump-release (all frame-projected); ledge grab (fully
`_in_frame`); **portal core as read today** (momentum via `portal_map_vec`,
somersault-roll + `gravity_upright_angle`, normal-based eviction/pieces/exit-boost —
no hardcoded up anywhere in `ambition_portal`); projectile primitive (all-cardinal
tests, `ProjectileSeq`-sorted stepping); player combat (melee/knockback/shield/
meteor/gravity-grenade frame-agnostic and mostly gravity-tested); gravity zones /
per-body `gravity_dir_at` / `ActorRoll` righting.

---

## C. Engine/content separation — the "second game" oracle (ranked)

### Tier 1 — structural blockers

#### C1. `Item`: the 24-item named inventory catalog lives in machinery (L)
`items/mod.rs:69` — closed `#[repr(usize)]` enum (`PortalGun, Axe, …, GunSword,
PuppySlugGun, … DebugLens, ReservedSlot`) with compile-frozen
`ITEM_META: [ItemMeta; 24]` (`:118`) carrying display names and flavor text.
Discriminant == inventory grid slot. Consumed across menu IR, yarn `inventory_has`,
persistence, pickups, abilities. A second game cannot add or remove a single item
without editing core.
**Fix:** the proven roster pattern — machinery owns a generic `ItemCatalog` schema
(string id, category, grid slot, held_item_id, dialog_id) + installed holder; content
installs `items.ron`. `ItemCategory` (`:40`) is already the right generic vocabulary.

#### C2. `HELD_ITEMS`: the full weapon/ability roster is a hardcoded static in a foundation crate (M)
`ambition_characters/src/brain/action_set/mod.rs:~150-348` — a `LazyLock` table
hardcoding every held item (`"axe"`, `"javelin"`, `"gun_sword"`, `"puppy_slug_gun"`,
`"volley"`, `"beam"`, …), resolved via `held_item_by_id` (`:351`), with comments
binding entries to named content ("the smirking_behemoth eye-beam", "GNU-ton's
apple-rain"). Also `items/pickup/mod.rs:230,248` constructs `"axe"`/`"javelin"`
specs inline. The ability *systems* are legitimately generic; the closed binding
table is the leak.
**Fix:** installable `HeldItemSpec` registry (same `OnceLock` install seam as
`install_enemy_roster`); content authors the table as RON.

#### C3. Ambition's worlds and roster RON are embedded inside `gameplay_core` (M)
`assets/sandbox_assets/embedded.rs:254-271` (`include_bytes!` of `sandbox.ldtk`,
`intro.ldtk`, `you_have_to_cut_the_rope.ldtk`, `hall_of_characters.ldtk` + named
spritesheets `:121-169`); `world/ldtk_world/hot_reload.rs:17`
(`SANDBOX_LDTK_ASSET` wires the LDtk spine to one game's world file);
`character_roster.rs:21` (`include_str!` of `character_catalog.ron` — module doc
admits it "owns Ambition's actual roster DATA").
**Fix:** content-installed `WorldManifest` (entry world + secondary bundles +
embedded byte registrations) mirroring the boss-roster install; move
`character_catalog.ron` + lookups to `ambition_content`.

#### C4. The app is not thin assembly, and nothing enforces that it stay thin (L)
`app/plugins.rs` (1099 LOC) hand-wires ~30 plugins with explicit ordering — exactly
what ADR 0019 says subsystems should own — and names content inline
(`spawn_ldtk_world_root` `:496-561` hardcodes intro + cut-rope bundles;
cut-rope/gnu_ton/victory systems at `:267-268,427,795` and
`progression_schedule.rs:35/45/81`). `app/sim_systems.rs` (639 LOC) is content-free
*gameplay machinery* in the shell. `host/mobile_input/` (2.9k LOC, fully reusable
touch controls) belongs beside `ambition_input`. The `architecture_boundaries` suite
has **no test asserting app thinness** — this is the unguarded accumulation point.
**Fix:** machinery-owned `PlatformerEnginePlugin` group; content-owned hooks for
named systems/worlds; fold `sim_systems.rs` into owning gameplay plugins; extract
mobile input; add an app-thinness boundary test.

### Tier 2 — closed vocabulary a second game must edit

#### C5. `ProjectileKind` + `ProjectileVisualKind` closed in machinery (M)
`projectile/kind.rs:35` (`Fireball, Hadouken, HadoukenSuper`, per-kind stat `match`;
doc admits "This is named game content") and `projectile/visual_kind.rs:33`
(`Apple` = GNU-ton fruit, `Glider` = PCA shot, `Lasersword`). The generic seams
already exist (`ProjectileSpec`, `ProjectileArtSource`).
**Fix:** RON rows keyed by held-item/ability id lowering to `ProjectileSpec`;
visual kind → string key against a content-installed art registry.

#### C6. Named-boss residue despite the finished `Special(String)` seam (M)
`ambition_characters/src/brain/boss_pattern/mod.rs:243` (`BossAttackProfile` variants
commented "GNU-ton specific" / "Gradient Sentinel", geometry baked at
`boss_encounter/attack_geometry/mod.rs:582-603`); `boss_encounter/ids.rs:26`
(`MOCKINGBIRD_ENCOUNTER_ID` + chest sync, file documents its own generalization
plan); `features/bosses.rs:39-52` (`GNU_TON_*`, `GRADIENT_SENTINEL_*`); named
constructors `mockingbird()/gnu_ton()/trex_boss()` (`boss_encounter/behavior.rs:309-340`);
`MOCKINGBIRD_SHEET` (`boss_encounter/sprites/mod.rs:169,459,715`).
**Fix:** migrate the five named variants to `Special(String)` techniques; ship the
boss-death-reward table; per-boss sheet specs into the boss roster RON.

#### C7. Render has a bespoke code path for one boss and parses `" on Shark"` from display names (S/M)
`ambition_render/src/rendering/actors/boss.rs:105-135` (`is_gnu_ton` string match →
hardcoded body/hands split layers); `rendering/world.rs:611-615` +
`features/ecs/spawn_mounts.rs:95` (mount composition triggered by stripping the
literal `" on Shark"` suffix from the authored spawn *name*, in both sim and render).
**Fix:** multi-part layering as data in the boss sheet spec; mounts as an authored
spawn field (`mount: "shark"`), never display-name parsing.

#### C8. Catalog authoring presets are *more* closed than the runtime enums they mirror (S)
`ambition_characters/src/actor/character_catalog/entry.rs`: `SpecialPreset` (`:354`)
has only `BubbleShield, BossSpotlight` — it **omits** the `Special(String)` hatch its
resolution target already has (`resolver.rs:308-309`). `MeleePreset`/`RangedPreset`/
`MoveStylePreset`/`BrainPreset` (`:215-345`) re-freeze the action-spec enums. The
data authoring surface can't reach the engine's own open seam.
**Fix:** add `Special(String)` + string-keyed rows to the presets; resolver exists.

#### C9. `CharacterBrainTemplate`: closed AI-template enum incl. a named `Shark` variant (M)
`combat/components/mod.rs:344` (`StandStill, Wanderer, MeleeBrute, Skirmisher,
Sniper, Shark, Smash, Aerial`). Mostly legitimate vocabulary, but `Shark` is a named
creature's policy and the set is closed (a second game's custom AI = core edit);
`CharacterBrainSpec` carries seven `smash_*` kit fields inline.
**Fix:** near-term rename `Shark` → behavior name (`ChargeCrash`); longer-term a
string-keyed brain-constructor registry with the current templates as defaults.
(Dovetails with the logged "characters = capability kits, not archetypes" smell.)

#### C10. `SpecialActionSpec` residue + hardwired player special (S)
`action_set/mod.rs:486` (`BubbleShield`/`BossSpotlight` remain) +
`player/bundles.rs:196` hardwires the player's special slot to `BubbleShield` gated
on `abilities.shield`.
**Fix:** both become `Special("bubble_shield")`/`Special("spotlight")` techniques;
the player's special slot comes from equipped item/catalog data. (Same item as A11.)

#### C11. Named dialogue ids in machinery (S)
`dialog/content.rs:48-100` — production `KNOWN_DIALOGUE_IDS` naming
`"emmy_noether"`, `"perfect_cellular_automaton"`, `"pirate_admiral"`, etc.
**Fix:** derive the known-id set from the installed yarn project / content plugin.

#### C12. Minor closed VFX/SFX pairings (S)
`ambition_vfx/src/vfx.rs:31,104` — `ExplosionKind` (5 flavors, hardcoded
variant→`SfxId` map, no `Custom`); siblings `ParticleKind`/`SlashKind` milder.
`EntitySprite` / `FeatureVisualKind` are mostly genuine kit vocabulary — low priority.
**Fix:** id-carrying variant or data map for explosion→SFX.

### Already CLEAN (the templates to copy)
The roster install pattern (`features/enemies/mod.rs`: string-keyed
`CharacterRoster`, `OnceLock` install, production panics without content, embedded
data test-only; boss profiles/encounters identical); boss-special Techniques
(`ambition_content/src/bosses/specials/` via `register_required_components` +
`CombatSet::ContentSpecials` — the engine names no boss special);
`ambition_entity_catalog` (fully generic, string-keyed — the flagship of the target
shape); `ambition_combat` (`DamageKind::Custom`, genuine vocabulary);
`ambition_interaction` (`PickupKind::Custom(String)` etc. — exemplary); SFX
(string-hash `SfxId`); yarn commands extensible from content; smash brain generic;
`ambition_engine_core`/`ambition_platformer_primitives` clean; renderer's
`ProjectileArtSource` seam correct.

### ADR 0019 gap summary
The crate split succeeded (~36 subsystem `impl Plugin`s exist). Missing for "add a
content crate": (1) **no reusable engine bootstrap** — `add_simulation_plugins`/
`init_sandbox_resources`/`add_presentation_plugins` are ~30 hand-ordered installs a
second game must replicate, and `init_sandbox_resources` itself calls the *content*
boss install; (2) **content hooks bypass `AmbitionContentPlugin`** — named worlds and
cut-rope/gnu_ton systems wired inline in `app/plugins.rs`/`progression_schedule.rs`;
(3) **boundary tests don't guard the app layer**. Highest-leverage: C4 + C1; after
those, remaining leaks are mostly one-file data migrations along existing seams.

---

## D. Decomposition of `ambition_gameplay_core` (94.5k LOC)

### LOC map (top modules)
| Module | LOC | What it is |
|---|---|---|
| `features/` | 17,645 | actor ECS sim (`ecs/` 12.9k; `enemies/` 2.0k; `bosses.rs` 963) + a giant re-export facade in `mod.rs` |
| `world/` | 10,186 | LDtk load/convert/runtime (5.5k), rooms graph/spawn/transitions (2k), moving platforms, physics settings |
| `combat/` | 8,604 | targeting, attack, hitbox, damage, components, world_overlay, moveset, chests/breakables/hazards |
| `boss_encounter/` | 6,059 | encounter script/behavior/registry + `attack_geometry/` + `sprites/` (1.2k) |
| `player/` | 5,393 | systems, body_integration, bundles, `trail.rs` (1,045) |
| `persistence/` | 4,486 | save + settings model (~1.8k settings) |
| `character_sprites/` | 4,222 | sheet/anim registry, animator, sprite-metadata → attack-hitbox derivation |
| `abilities/` | 4,066 | blink/dive/possession/grapple + ranged kit |
| `projectile/`+`enemy_projectile/` | 4,285 | projectile engines |
| `assets/`+`asset_publish/` | 4,308 | asset profiles/loading + publish/hygiene classifier |
| `menu/` | 3,189 | settings IR + **Bevy-UI map panel in machinery** |
| `dev/` | 2,969 | trace detect/systems, dev_tools, profiling |
| smaller | | `encounter/` 2.5k, `items/` 2.4k, `dialog/` 2.3k, `audio/` 1.3k, `falling_sand.rs` 1.3k, `time/` 1.3k, `session/` 1.2k |

**Hot-edit surface (git, since May):** `features/`+`combat/`+`abilities/` = 1,084
file-touches vs 190 for `world/`+`persistence/`+`menu/`. The strategy: **move the
cold 40k out from around the hot 30k** and cut the render edge, rather than
attempting the verified-HARD mechanics extraction first.

### D1. Delete the internal facade layer (prerequisite, ~0 risk)
Dependency counts are a lie until this lands. Verified facades whose definitions
already live in foundation crates:
- `crate::audio::SfxMessage` — **93 of 94 inbound refs** are this one symbol
  (`pub use ambition_sfx::SfxMessage`, `audio/mod.rs:27`).
- `crate::effects` — entire module is `pub use ambition_vfx::*`.
- `crate::time::{world_time,clock_state}` — re-exports of `ambition_time`, kept "so
  historic paths keep resolving" — exactly the pre-release compat tax to delete.
- `crate::config::{world_to_bevy, WORLD_Z_*}` — re-export of engine_core; render
  imports it 28× through gameplay_core.
- `features/mod.rs` re-export hub — **271 internal refs** name `crate::features::X`
  for symbols living in `combat/` (`HitEvent`, `CenteredAabb`, `CollisionWorld`, …).
  The #1 navigability obscurer.
- `lib.rs` root: `pub use persistence::save_data as save` (2 users),
  `pub use items::shop` (4 users), `pub use crate::features::MeleeSwing`.

### D2. Re-home `BodyHealth`/`BodyCombat`/`BodyWallet` down to `ambition_characters` (tiny, keystone leverage)
`src/actor.rs` (299 LOC) is the top import of both render (52 refs) and app (100
refs) — but ~90% of it re-exports engine_core Body* clusters; only three real types
live there. Move them down and `crate::actor` becomes a pure facade → delete per D1.
This one file is why "everything imports gameplay_core for vocabulary."

### D3. Cut the `ambition_render → ambition_gameplay_core` edge (biggest compile-time win)
Hot edits in `features/ecs` currently rebuild gameplay_core (95k) → render (10k) →
portal_presentation → app. Render's imports are almost entirely read-model
vocabulary: `actor` (dissolved by D2), `config`/`time` (dissolved by D1), and the
`features` view accessors (`ActorSpriteData`, `FeatureViewIndex`,
`FeatureVisualKind`, `ecs_actor_anim_state`, …) + `rooms::RoomSet`.
**Missing abstraction:** a small `ambition_sim_view` crate (or grow
`ambition_characters`) holding the materialized read-model: `FeatureViewIndex`/
`FeatureView` (already rebuilt per-frame for presentation readers), `ActorSpriteData`,
anim-state enums, `CameraSnapshot2d` (459 LOC, already presentation vocabulary), and
the sim→presentation messages not already down (`DebrisBurstMessage`,
`GameplayBanner`). Hard part: the `ecs_*` accessors take live `Query`s; render must
switch fully to the materialized index; the few direct component reads
(`BodyCombat.hit_flash`, `BodyHealth` HUD) ride D2.
**Payoff:** render + portal_presentation drop out of the hot rebuild path and compile
in parallel with gameplay_core.
> `[opus-4.8[1m]]` **fable should re-check** (see E22 for the measured surface): the
> render→gameplay_core edge is wider than "read-model vocabulary" — it also carries
> `RoomGeometry`+rooms (world types, need D4) and **presentation systems render
> registers** (`portal::sync_*`, `abilities::traversal`, `dev_tools`, …). The
> sim-view crate is necessary but not sufficient; cutting the edge is multi-session.
> **[fable 2026-07-03: CONFIRMED — see AD4.]**

### D4. Extract `ambition_world` (10.2k — the narrowest big seam)
> `[opus-4.8[1m]]` **fable should re-check — outbound is NOT "mostly clean, 3
> inversions" (measured 2026-07-03; see E25).** `world/` OUTBOUND (what it imports
> from the rest of gameplay_core = the cycle surface a leaf crate must shed) spans
> **~15 modules**, concentrated in `ldtk_world/` (6.4k, 36 refs) — the LDtk
> **converter** maps LDtk entities → `portal`/`encounter`/`shrine`/`items`/
> `character_roster` domain specs, so making it a leaf needs a **content-registered
> converter** refactor (ADR 0009 pattern), not a move — and `rooms/` (2.4k, 21
> refs → `features`/`player`, entangled with the 18-param `load_room_geometry`).
> **The linchpin: `RoomGeometry` (`lib.rs:235`, a `Resource(ae::World)` newtype)** —
> `platforms`+`physics` are otherwise 0-outbound but BOTH read `Res<RoomGeometry>`,
> and render imports it ×27, so NOTHING in `world/` extracts until `RoomGeometry`
> has a foundation home. fable's "thin/3-inversions" reads the INBOUND surface;
> the OUTBOUND surface is the real cost. (I may be under-weighting a converter
> seam fable had in mind — flagging.)
> **[fable 2026-07-03: CONFIRMED — see AD4; converter extensibility is the crux and worth it.]**

Inbound surface is remarkably thin: `RoomSet` (22), `Authored<T>` (18),
`RoomSpec`/`RoomMetadata`, `MovingPlatformState`, `DebrisBurstMessage`,
`poll_ldtk_file_changes`. Outbound mostly clean (`DamageVolume` is a foundation
re-export). Three genuine inversions:
1. `rooms/systems.rs` queries `crate::features::FeatureName` — invert via a
   world-owned marker or move the label component down.
2. `rooms/load.rs` writes `PlayerBlinkCameraState`/`PlayerSafetyState`;
   `rooms/systems.rs` mutates `SlotInteractionState` — room transitions reach into
   player state. Elegant fix: emit `RoomTransitioned { spawn, reason }`; player/
   session systems react. (Also fixes the shared-scalar cooldown smell in
   `SandboxSimState.room_transition_cooldown`.)
3. `world/physics.rs` debris messages move to the sim-view crate (D3).
**Payoff:** −10k from the god crate; the LDtk machinery (+ `bevy_ecs_ldtk` dep)
becomes a leaf; the "second game" oracle needs exactly this crate to exist.

### D5. Unify the smeared menu/settings stack; evict wrong-layer UI
The menu system is in **four places**: `gameplay_core/menu` (3.2k, incl. a literal
Bevy-UI map panel inside machinery), `ambition_menu` (4.8k), `ambition_app/menu`
(**10k** — kaleidoscope/grid backends + model + parity tests: reusable machinery in
the app layer, 40% of the app crate), `persistence/settings` (1.8k model the IR
references 29×). Proposal: one menu crate stack (IR+model+backends) beside render,
importing a settings-schema crate; app keeps host wiring. Also evict:
`dev/dev_tools/editable.rs`+profiling toward app's dev split; `asset_publish/`
(890 LOC author-time tooling, no build.rs user) toward `ambition_asset_manager`/tools.

### D6. `character_sprites` down + `boss_encounter` dissolved
After D1/D2, `character_sprites` (4.2k) has no real gameplay_core deps and is
consumed by render/content/combat-geometry — it belongs beside
`ambition_sprite_sheet` as the one sprite-metadata pipeline (matches the
sprite-renderer refactor plan + actor-geometry unification). `boss_encounter/` then
splits along its grain: `attack_geometry/`+`sprites/` (~2.5k) join the metadata
pipeline; behavior/registry/script folds into `ambition_characters` (the next.md
"unified actor+brain crate" carve — bosses ARE actors, and this is the crate-level
face of A1); rewards stay with encounter/items. Stray: `character_sprites/assets.rs:487`
documents a nonexistent `crate::ambition_content::intro::plugin` path
(docs-describe-dead-things — log it).

### D7. Split `dialog/` runtime from bindings; move `falling_sand` out (easy wins)
`dialog/runtime.rs` (generic yarn runtime + lint) → reusable `ambition_dialog` crate;
`yarn_bindings.rs` (618 LOC binding save/shop/quest) stays up. `falling_sand.rs`
(1.3k, feature-gated desktop prototype) → its own optional crate; it currently drags
`bevy_falling_sand` into the 95k crate's feature matrix (deps: `config` facade,
`rooms` → needs D4, `features` ×6).

### The knot NOT to cut yet
`features/ecs`+`combat`+`abilities`+`projectile` (~30k, the hot mechanics core).
next.md verified ~15 dependency inversions needed; **D1–D4 ARE the pre-inversions.**
After them, the mechanics core's outward deps reduce to `persistence::settings`
(~13 tuning reads) and `character_sprites` (12, handled by D6) — at which point the
extraction stops being hard.

### Ordering/coupling smells (log-worthy)
- **WorldPrep mega-chain** (`features/mod.rs`): 20+ systems in 4 `add_systems` calls
  split only by Bevy's chain-length ceiling, ordering carried by `.before/.after` +
  comments. Would be crisper as explicit `SystemSet` phases inside `SandboxSet::WorldPrep`.
- **Read-model mirrors with documented one-tick lag:** `BodyCombat.alive` mirrors
  `BodyHealth` ("liveness-critical gameplay reads BodyHealth directly to avoid a tick
  of mirror lag") — the sim-view crate (D3) would formalize this.
- Room transition via shared scalar + direct player-state writes (see D4).
- `use super::*` is contained (max 4/file, mostly tests) — not a priority.

---

## Cross-audit intersections (highest-leverage compound moves)

- **A1 (boss island) × D6 (boss_encounter dissolution) × C6 (named-boss residue):**
  one arc — fold bosses onto the body vocabulary, move behavior into
  `ambition_characters`, migrate named variants to `Special(String)`, leave only RON
  in content. Three audits independently converged on this.
- **A2-A5 (damage unification) × B2/B11 (frame bugs):** the emit-site hurtbox
  divergence is both a fork and a relativity bug, and the actor knockback/shield
  frame bugs (B2) live exactly in the forked actor-victim consumer — one relational
  victim resolver built on `gravity.dir_at` fixes both classes at once.
- **B1 (moveset hitbox frame) × A (one strike seam):** the moveset runtime forked
  off `spawn_melee_strike`'s gravity resolution — routing it through the same seam
  is both the bug fix and the unification.
- **C1/C2 (item+held rosters) × A10 (projectile fire control):** item catalog → held
  registry → projectile specs is one data chain; converting it end-to-end retires
  `ProjectileKind` (C5) too.
- **D3 (sim-view) × ADR 0012:** the sim/presentation split's missing abstraction is
  the same crate the compile-time lever wants.

## Status
- [x] Audit A — actor unification forks
- [x] Audit B — physics/gravity frame bugs
- [x] Audit C — engine/content separation
- [x] Audit D — decomposition seams

---

# HANDOFF — start here if you are a fresh agent continuing this work

> **The big picture lives in `docs/planning/roadmap.md`** (rewritten 2026-07-03
> by fable): the full path to a Unity/Godot-class 2D platformer engine — phases
> P1–P5, the demo-game capability matrix, the MADE-decision register (M1–M12),
> the uncertainty watch-list (U1–U7), and JON'S OPEN QUESTIONS (Q1–Q12). This
> review's remaining work is phases P1+P2 of that roadmap. **If you hit a design
> fork: check the adjudications above, then the roadmap's M/U/Q lists. If your
> fork maps to a Q-item, it is Jon's call — log it and switch to parallel work;
> don't guess and don't stall.**

**State:** Sections A–D below are the ranked audit (file:line refs may have
drifted where the execution log says something landed — trust the log over the
audit). The execution log (**E1–E21**) records what is DONE; do not redo it.
Landed so far: the C4 harness + full §B gravity sweep, **§A2 COMPLETE** (one
`resolve_body_hit` + shared knockback/stagger for every body), A3–A6, **A1
slices 1 + 2a** (boss HP/damage on the shared body components + through the one
resolver), **4 of ~5 D1 facades** removed (config/effects/audio/time — only the
`features/mod.rs` hub remains), and **§D2 COMPLETE** (E20/E21:
`Body{Health,Combat,Wallet}` re-homed to `ambition_characters::actor::body`, all
~200 consumers redirected, the whole gameplay_core facade chain deleted), and
**§D3 IN PROGRESS** (E22–E24): D3.1 DONE (render names foundation crates directly
for body vocab — clean + independent). **D3.2a (sim_view crate) was tried and
REVERTED (E24)** — Jon flagged the read-model taxonomy (`FeatureVisualKind`) and
the premature tiny crate; **D3 is now BLOCKED on fable adjudicating the `actors`
vs `props` taxonomy** (see JON'S DESIGN FEEDBACK near the top) + a decision to
materialize the full read-model (what gives a sim-view crate real meat AND
enables the edge-cut). **§D4 STARTED** (E25/E26): scoped (bigger than audited — the LDtk converter is
the crux) and **D4.1 DONE** — `RoomGeometry` re-homed to `engine_core` (the
world-extraction linchpin; render shed its ×27 coupling). Remaining D4 is
multi-session (platforms/physics extract, converter extensibility, rooms
inversions). **§A1 slice 3 STARTED** (E27–E30): slice-3a landed (bosses are full
victim-side bodies — the vuln trio + `apply_hitbox_damage` `Option` dropped), the
motion+float **parity net** is in place (E28), the driver fold is precisely
re-scoped (E29), and the **brain half is DONE** (E30) — the boss brain ticks through
the universal `Brain::tick` (attack-state now a `BossPatternState` projection).
Remaining: attack-geometry→moveset (3b), the archetype swap + integrate fold (E29
blocker #1 — the big one), 3e/3f/3g. Other independent open items: the
**features/mod.rs hub**. All work is committed
linearly on main; the tree is green (counts in the verify block below).

**Verify before you start** (and after every change):
```bash
~/.cargo/bin/cargo test -p ambition_engine_core --lib      # 211, incl. the C4 harness
~/.cargo/bin/cargo test -p ambition_gameplay_core --lib    # 1091
~/.cargo/bin/cargo test -p ambition_characters --lib       # 250 (now hosts BodyHealth/BodyCombat/BodyWallet)
# Compile ALL test targets too — a word-boundary facade sed silently skips
# multi-line grouped `use x::{\n A, Moved, B\n}` imports (D2b bit us twice):
~/.cargo/bin/cargo check -p ambition_app -p ambition_render -p ambition_content --tests
# The ten app integration suites — plus plugin_minimal_app (the grouped-import canary):
~/.cargo/bin/cargo test -p ambition_app --test possession_end_to_end \
  --test unified_melee --test gravity_symmetry_room \
  --test player_robot_fights_player --test enemy_attacks_player --test duel_arena \
  --test boss_lifecycle --test boss_contact_iframes --test boss_possession_specials \
  --test plugin_minimal_app
# The §A1 slice-3 boss motion+float parity net (rl_sim; guards the driver fold):
~/.cargo/bin/cargo test -p ambition_app --test boss_motion_parity --features rl_sim  # 2
# (also green: content --lib 53, render --lib 24)
```

**Rules of engagement (Jon's, distilled):**
- Commit each completed, verified slice immediately; commit = checkpoint. Never
  leave a half-merged tree. Stage explicit paths (never `git add -A`).
- Behavior is NOT sacred pre-release, but feel-touching changes (knockback,
  hitstun, anything the player's hands notice) ship BLIND in their own
  `blind fix:`/clearly-marked commit for Jon to feel-check — with headless
  tests proving the mechanics, not the feel.
- Frame-agnostic always: any new reaction/effect code goes through
  `AccelerationFrame`; pin new frame fixes with a scenario in
  `crates/ambition_engine_core/src/movement/tests/c4_reaction_seams.rs`
  (author local-frame, assert all 4 gravity arms match — the pattern is in
  the file).
- ONE BODY ONE PATH: before adding anything keyed to player/actor/boss, check
  whether the other kind already does it and unify instead (AGENTS.md).
- Keep THIS document's execution log updated as you go — it is the handoff
  surface; Jon can only read, not ask.

**Work queue, in order** (details in "Next" at the end of the log):
1. ~~**A2**~~ — COMPLETE (E11–E13): `resolve_body_hit` + shared knockback +
   shared stagger for every body. Steps 6 (knockback, `b4912001`) and 7
   (stagger, see E13) are BLIND feel commits awaiting Jon's feel-check.
2. **A1** — boss island dissolution: slice 1 (authority flip, E14) and slice 2a
   (boss damage through the resolver, E15) are DONE; **slice 3 (the driver fold)
   is the only A1 work left** — full design in "Next" below, and it's a big
   multi-session fold (BossAttackState→BodyMelee, boss tick→actor driver needing
   the 18-cluster set + flight=SNAP equivalence, render BossAnim→CharacterAnim).
   Slice-2b (boss vuln clusters + drop the `apply_hitbox_damage` `Option`) folded
   into slice 3; grep `§A1` and `Without<BossConfig>` there to remove the victim
   special-cases.
3. ~~**D1 facade deletion**~~ — 4 of ~5 done (E16 config, E17 `crate::effects`,
   E18 `crate::audio::SfxMessage`, E19 `crate::time::*`). ONLY the `features/mod.rs`
   hub remains, and E21 reframed it: it's a 3-layer facade STACK (features →
   combat::components → crate::actor → foundation) entangled with the D2/D3 crate
   moves, so redirect it type-family-by-family as each family reaches its leaf
   home — NOT as one blind sed. **§D2 is the completed template** (E20/E21).
4. ~~**D2**~~ — COMPLETE (E20/E21): `Body{Health,Combat,Wallet}` →
   `ambition_characters::actor::body`, all consumers redirected, facade chain
   deleted. Next in this vein: **D3** (cut the render→gameplay_core edge — D2 was
   the keystone that lets render name `ambition_characters` directly; the
   remaining render imports are the `features` view-accessors + `rooms::RoomSet`),
   and **C1/C2** item catalog + `HELD_ITEMS` onto the roster-install pattern →
   **C3/C4** worlds/app-thinness → C5–C7, C9-registry, C12.

**Small loose ends** (sweep opportunistically):
- Verify portal findings B8 (portal aim skips the frame seam) and B12
  (first-portal-wins ordering) against the portal agent's final code before
  fixing.
- Blink PREVIEW divergence: `ambition_render/src/fx.rs` and
  `ambition_app/src/dev/debug_overlay/gizmos.rs` build quick-blink aim from
  raw device axes + world-X fallback instead of the resolved `blink_quick_dir`.
- Two pre-existing warnings, likely interrupt-window debris: unused `aim_dir`
  (`ambition_characters/src/brain/state_machine/mod.rs:742` — check whether a
  consumer was dropped, don't just underscore it) and an unused
  `hostile_brain_id_for_actor` import (`features/ecs/mod.rs:75`).
- `gravity_symmetry_room.rs`'s `allow_one_tick_landing_boundary` concession
  may be removable after the B5 sweep unification — check, don't force.
- Actors' `MAX_ENEMY_AIR_JUMPS` refresh + flying-never-grounded remain actor
  policy applied AROUND the shared tick (fine), but new actor policy goes in
  the same place, not inside the engine.

---

# EXECUTION LOG (live — session of 2026-07-02, post-portal-agent)

Jon's direction: start on the biggest, hardest items — the ones that unblock
weaker agents to "take us home." Keep this log current enough that a fresh agent
can resume from it cold. Working directly on main; commit = checkpoint.

## Done

### E1. C4 body-tick symmetry harness (synthesis item 0a) ✅
`crates/ambition_engine_core/src/movement/tests/c4_reaction_seams.rs` — a
local-frame scenario rig at the `update_player_with_tuning_clusters` level:
author blocks/spawn/input in the body's local frame, rotate through all 4
cardinal gravities, compare local-frame traces (pos/vel/on_ground/on_wall/
facing, tol 0.02). Runs in ms (no Bevy App). Scenarios: run+jump+land sanity,
slash recoil (B4), neutral quick blink (B9), post-blink fall clamp (B3), wall
slide steady-state (B5/B6), gravity-relative OOB reset (B7). All failed on
rotated arms before the fixes; all pass after. **Pattern for future agents:**
any new reaction-seam fix gets a scenario here first.

### E2. Engine-core reaction-seam fixes (B3, B4, B6, B9) ✅
- B4 `movement/control.rs` — slash recoil now `frame.side * facing`, not `vel.x`.
- B9 `movement/control.rs` + `movement/blink.rs` — every "forward along facing"
  blink default (quick-blink fallback, precision `aim_offset` seeds/resets) is
  `frame.side * facing`; `blink_destination_internal`'s own dead world-X
  fallback removed (callers own the fallback, documented).
- B3 `movement/blink.rs::complete_blink_clusters` — post-blink damp/clamp now
  decomposes into the local frame (damp side, clamp fall, damp rise).
- B6 `movement/integration.rs` — ONE sweep sequence for every gravity: sweep
  side axis → wall abilities (last-frame ground snapshot) → clear ground →
  sweep gravity axis. The horizontal-gravity branch (post-sweep wall abilities,
  `stabilize_on_support` patch) is gone.

### E3. B5 — role-parameterized collision sweep unification ✅ (the big one)
`movement/collision.rs`: `sweep_player_x_clusters` + `sweep_player_y_clusters`
merged into ONE `sweep_player_axis_clusters(axis, …)`, and the two repair
functions into ONE `resolve_axis_repair(axis, …)`. Every guard is now keyed by
AxisRole so it rotates with gravity: `body_is_side_contact` → axis-generic
`body_is_nested_along`; `resolve_x_penetration` → axis-generic
`resolve_side_penetration` (defer-to-gravity-pass / world-bounds / no-pushout /
grazing-continuation); gravity-axis feet-snap now sets `on_ground` on EITHER
axis (so `stabilize_on_support` + `grounded_against_gravity` are deleted);
side-contact normals now ALWAYS convert to the local frame via
`apply_side_contact`. **Real bug found by the harness en route: wall cling was
completely broken under UP gravity** — the X-path stored the raw world normal
sign into the local-frame `wall_normal_x`, so `pressing_into_wall` never
matched (caught by the wall-slide scenario's up arm, normx=+1 vs -1).
Down-gravity baseline preserved: all 211 engine-core lib tests green, zero
changes to existing test expectations.

### E4. B7 — gravity-relative OOB reset ✅
`movement/mod.rs` — "fell out of the world" is now distance past the world AABB
along `gravity_dir` (> 200px), replacing the bottom-edge-only `pos.y` check.
Pinned by `c4_out_of_bounds_reset_is_gravity_relative` (+ 100px grace case).

### E5. Gameplay-core frame-bug sweep (B1, B2, B10, B11, B13 + minors) ✅
Committed after E1-E4's checkpoint (`1c8c5589`):
- **B2** — fixed at the WRITER, not per-consumer: `ActorMut::update`
  (`features/enemies/integration.rs`) now keeps `surface_normal` LIVE for every
  body (anti-gravity at its position for non-surface-walkers; clung surface for
  surface-walkers). All four consumers (shield-block side, slash knockback,
  ranged muzzle, footprint publish) become frame-correct with zero edits; the
  footprint publish's conditional collapsed. Pinned by
  `a_normal_actor_surface_normal_tracks_live_gravity` (all 4 cardinals).
- **B1+B10** — `Hitbox` (ambition_vfx) gained `frame_down` (owner's gravity
  baked at spawn); `world_volume` places shaped volumes in that frame instead
  of hardcoded screen-down. The moveset runtime (`combat/moveset.rs`) rotates
  authored BODY-LOCAL offsets + extents through the owner's frame at spawn —
  the same resolution `spawn_melee_strike` performs (`local_offset`'s contract
  is now clearly "world offset baked at spawn"). `spawn_melee_hitbox/strike`
  take `frame_down`; world-anchored `DamageBox` hazards stay screen-down by
  design (world-authored arena geometry).
- **B11** — knockback side at both emit sites (`combat/hitbox/mod.rs` player
  loop, `projectile/systems.rs` enemy-shot hit) computed via
  `(victim - owner)·frame.side`, not screen-X (which degenerates exactly when
  sideways gravity separates the pair along world-Y).
- **B13** — `FlipGravity` now inverts the full gravity vector at BOTH sites
  (`encounter/systems.rs` switch action + `gravity/lifecycle.rs` walk-in
  switch); previously a no-op after a sideways SetGravity.
- **Minor** — hard-fall screen shake reads the along-gravity fall speed;
  `PlayerBodyFrameOutput.pre_sim_vy` renamed `pre_sim_fall_speed` (id matches
  meaning). NOT fixed (audit was wrong): `GravityField::vertical_sign` is NOT
  dead — `GravityCtx::sign_at` consumes it.
- gameplay_core lib 1080/1080 green (incl. 3 moveset tests updated: the test
  attacker now carries `BodyKinematics` like every real actor).

### E6. A5+A6 — ONE vulnerability rule + ONE published hurtbox ✅ (pending integration-test verify)
- **A5**: `combat::damage::body_vulnerable(offense, dodge, shield, combat)` is
  the one emit-side "can this body take a hit?" rule, replacing five
  copy-pasted predicates (hazards, enemy hitbox player loop, body-contact,
  boss volumes, enemy projectiles). The projectile site's missing parry term
  is now present (behavior-neutral: its parry-reflect branch runs first).
- **A6**: every player body now PUBLISHES the same gravity-oriented
  `CenteredAabb` footprint an actor does — added to `PlayerSimulationBundle`,
  the brain-driven clone, and registered as a required component of
  `PlayerEntity` (app plugins); `integrate_home_body` keeps it live (same
  publish as `integrate_actor_body`). All five consumers read `hurtbox.aabb()`
  instead of rebuilding per-site (two sites used raw `kin.aabb()`, which
  disagreed with the oriented box under rotated gravity — that divergence is
  gone by construction). Also fixed en route: the hazard knockback side was
  screen-X (an unlisted B11 instance) — now `frame.side`.
- **Safety check done**: broad `CenteredAabb` queries audited for accidental
  player inclusion — `actor_victims` in `apply_hitbox_damage` got
  `Without<PlayerEntity>` (else double-hit); targeting/pickups/interact are
  `With<FeatureSimEntity>`-scoped (safe); `tick_falling_hazards`' keyed lookup
  now RESOLVES for player targets (previously silently despawned the hazard —
  an improvement). The old owner-anchor kinematics fallback in
  `apply_hitbox_damage` is now nearly dead (player publishes the box; centers
  are identical because `SimpleActorGeometry::combat_offset == 0`).
- gameplay_core lib 1080/1080.

### E7. A4 — world damage is body-generic ✅ (committed by Jon as `c3fd6db7` after an interrupt)
- **Hazards** (`combat/hazards.rs`): a second victim pass over every
  `FeatureSimEntity` body with a published footprint — an NPC in the spikes
  takes a pre-resolved `HitTarget::Actor` hit (pinned by
  `a_non_player_body_touching_a_hazard_takes_the_hit_too`). Deliberately not
  faction-gated (unified-actors guardrail 4).
- **Body contact** (`apply_actor_contact_damage`): the attacker's tracked
  target may now be ANY body (a duel opponent, a grudge foe), not just a
  player. Restructured as a ParamSet two-pass (attacker-cluster snapshot via
  new `ActorMut::contact_attack()` → victim resolution via published
  hurtbox); `ContactAttack::hit_event` stamps Player/Actor by victim kind.
  The contact knockback side is now the attacker's live `frame.side`
  (another unlisted B11 instance, enabled by §B2's live `surface_normal`).
- **Boss volumes** (`update_ecs_bosses` + `boss_attack_damage`): the boss's
  tracked victim may be any body; `boss_attack_damage` takes the target stamp.
  A boss swing now lands on its duel opponent.

### E8. Delegated easy-end items (Codex/GPT agent, reviewed 2026-07-02) ✅
Jon had a second agent work the review's unblocked easy end during the
interrupt. Reviewed each diff — all five are correct, tested, and match the
review's fix shapes; none closed anything prematurely:
- **C8** (`42a819fc`): `SpecialPreset` gained the open `Special(String)` hatch
  + RON pin test.
- **C9, rename half** (`b95e7a49`): `CharacterBrainTemplate::Shark` →
  `ChargeCrash` (authoring surface + content RON). The L-term half — a
  string-keyed brain-constructor registry — remains open (see C9).
- **C10** (`ca9cc713`): `SpecialActionSpec::{BubbleShield,BossSpotlight}`
  DELETED (they were inert deferred seams); the player's special slot authors
  `Special("bubble_shield")`; `SpecialPreset` follows. C10 + A11's enum half
  are now closed; A11's boss-dispatch-bypass half still rides A1.
- **D6 stray** (`d5944051`): stale intro content path doc fixed.
- **C11** (`62864c3e` + `ca1739e6`): `KNOWN_DIALOGUE_IDS` derived from the
  installed yarn project titles instead of a hardcoded machinery const; the
  yarn source list gated to UI builds.

### E9. A3 — ONE victim loop in `apply_hitbox_damage` ✅
The aggressor branch's separate actor-victims and player-victims loops
collapsed into ONE loop over ONE victims query (every body with a published
footprint; `Option`-typed vulnerability clusters so a boss body still matches
pre-§A1). One relational rule for everyone — `damage_lands` (different-faction
|| personal grudge), which provably subsumes the player loop's old
`can_damage` gate since a player is never the aggressor's faction. Victim KIND
picks only policy: a player victim gets the emit-side vulnerability gate
(actor i-frames stay consume-time until §A2), the `HitKnockback` payload, and
the richer SFX/feedback; the `HitTarget` stamp routes to the right consumer.
Emit-time i-frame checking for players vs consume-time for actors is now the
LAST asymmetry in this system — it dissolves with A2's one victim resolver.
Verified: gameplay_core lib 1082/1082 + all six app integration suites
(possession, unified melee, gravity symmetry, robot duel, enemy-attacks,
duel arena).

### E10. A2 slice 1 — shield authority is the BODY's resolved guard ✅
`handle_player_damage_events` blocked off the RAW `input.shield_held` instead
of the body's resolved `BodyShieldState.active` — so a body with no shield
ability could block, and a guard held through a dash (the `resolve_shield`
rule gates both). Now reads `clusters.shield.active` — invariant I3 (the body
enforces, the controller attempts), and the same authority the actor victim
path already used. 1082/1082 + shield-adjacent integration suites green.

### E11. A2 steps 1–5 — ONE `resolve_body_hit` for player + actor victims ✅
`combat::damage::resolve_body_hit(combat, health, shield_active, facing, pos,
impact, gravity_dir, raw_damage, multiplier, never_dies, BodyHitFeel) ->
BodyHitResolution{Ignored|Blocked|Damaged{damage,died}}` — the one victim-side
mechanics core, called by BOTH consumers. It owns: the consume-time i-frame
gate (`combat.vulnerable()` + already-dead → `Ignored`, for EVERY body), the
directional shield block (arms the guard i-frame; the player's 0.12 floor and
the actor's full window are `BodyHitFeel` values), damage scaling
(player: difficulty × assist × setting; actor: 1.0; floor 1), `health.damage()`
+ died flag (`never_dies` pre-gates so a dummy's HP never moves; `health: None`
headless bodies are damaged-but-undying), and hit-flash + i-frame arming
(player 0.20/`knockback_invulnerability_time` — moved OUT of
`apply_player_knockback`, which now owns only launch + control-lock timers;
actor 0.16/`ACTOR_DAMAGE_IFRAME_S`, same values as before). What stays in the
consumers is genuine policy: player difficulty choice + SafeRespawn/death →
respawn + banner; actor peaceful-branch, barks (snapshotted pre-resolver so
the dedup-on-flash and pre-damage strike count are unchanged), cling-detach
pop, death → drops/respawn-timer/split/explode. The player's emit-side gate in
`apply_hitbox_damage` is GONE as an event-dropper — the event always flows and
i-frames resolve at consume time for every body (the last emit/consume
asymmetry); the emit-side `body_vulnerable` read remains ONLY to mute the
hit-landed feedback (sfx/burst/debris) for a hit the consumer will ignore.
Two consequences to know: an i-framed player now consumes a hitbox's
per-victim dedup slot exactly like an actor does, and `damaged_this_frame`
(safe-pos memory) is true while overlapping an attack even when ignored.
Also swept: the unused `PlayerInputFrame` in `apply_player_hit_events`'s query
(E10 debris) is removed. 5 new resolver unit tests (i-frame ignore, dead-body
ignore, faced-block vs back-hit, scaling/feel/floor, death + never_dies +
headless). Verified: engine-core 211, gameplay-core 1087, all six app suites.

### E12. A2 step 6 — actors ride the shared knockback resolution ✅ (BLIND — Jon feel-checks)
`resolved_player_knockback_velocity` renamed `resolved_body_knockback_velocity`
(it never was player-specific — pure side/rise resolution in the victim's
frame). The actor path's inline hardcoded `local.y - 90 max -280` slash pop is
DEAD; a struck actor's velocity is now SET by the same feel-tuned resolution
the player gets (side away from the source, `enemy/boss_knockback_x/y` ×
strength, rise against ITS gravity). Data flow: `apply_hitbox_damage` now
attaches `HitKnockback` for EVERY victim (aggressor swings launch actor
victims too — body-contact + hazards already attached it); a `PlayerSlash`
with no payload folds its `knock_x` into the same resolution (dir from sign,
standard strength); an event with neither leaves velocity alone.
`apply_feature_hit_events` gained an `Option<Res<SandboxFeelTuning>>`
(default in headless tests). Mechanics pinned by 2 new tests (launch matches
the shared resolution; slash-fold). **Feel notes for Jon:** enemies/NPCs now
get visibly LAUNCHED by slashes and by each other's swings (duels read much
more smash-like); the duel-arena canary tripped exactly as designed — knockback
separation makes committed-lunge blink-evades rarer, so its blink assertion is
now "the verb fires" (≥1) instead of ≥2. Verified: gameplay-core 1089, all six
app suites.

### E13. A2 step 7 — actors are STAGGERABLE ✅ (BLIND — Jon feel-checks) — **A2 COMPLETE**
The shared post-hit stagger, armed + consumed for every body:
- **Arming**: `combat::damage::apply_body_hit_reaction` is the ONE launch +
  stagger arming (knockback velocity SET + hitstun/recoil-lock/hitstop on
  `BodyCombat`), called by the player's `apply_player_knockback` (refactored
  onto it) and the actor consumer's knockback block. Player-tuned values
  everywhere (enemy 0.24s / boss 0.36s hitstun × strength, 0.12s recoil).
- **Consuming**: the two post-hit input gates extracted from the player bridge
  into `combat::attack::apply_post_hit_input_gates` (recoil = hard zero,
  hitstun = scaled axes, attack verb preserved); `ActorMut::integrate_body`
  applies it to the FINAL InputState (post flight-axis override) — timers
  threaded via `em.update(…, feel, (hitstun, recoil))`. Timers tick in
  `tick_actor_brains`; `sync_actor_components_from_cluster` carries them
  across the read-model rebuild (else the mirror wiped them each frame).
- **Two deliberate shape decisions** (both documented in code):
  (a) the FLY TOGGLE is exempt from both gates for every body — it's
  mode-switch INTENT, not movement authority (axes still stripped); eating the
  edge desynced open-loop brains (duel fighters got stuck airborne, melee
  11→0) and toggling flight to arrest a launch is legitimate recovery tech.
  (b) actor hitstop is ARMED but does NOT freeze the actor's own sim dt —
  tried it, per-victim freezes made AI-vs-AI duels degenerate; the
  player-involved beat stays the global-clock rule, per-body proper-time is
  the ADR 0011 seam.
- **Known limit → §A7**: brains can't PERCEIVE their own stagger, and the
  smash brain times blink-evades exactly around getting hit, so its one-frame
  blink tap can die inside hitstun with its own cooldown burnt. The duel
  abilities test now pins the wiring both ways instead of demanding a
  resolved blink; wire stagger into `WorldView`/`BrainSnapshot` when doing A7
  and restore the strict assertion.
- Tests: staggered-walker witness (recoil = no ground covered, hitstun =
  reduced authority, driven through the REAL `ActorMut::update`), knockback
  test extended to assert the stagger set arms. **Feel notes for Jon:**
  enemies now flinch — a landed hit steals their control for ~0.24s (recoil
  0.12s hard); duels read as launch → recover → re-engage.
Verified: gameplay-core 1090, engine-core 211, all six app suites.

### E14. A1 slice 1 — the boss authority flip ✅
`BossStatus.{health, alive, hit_flash}` are DELETED. A boss's HP authority is
the same `BodyHealth` every body carries (alive = `health.alive()` — no shadow
flag anywhere; scripted/environmental kills zero HP), its damage-blink is
`BodyCombat.hit_flash`, and `sync_boss_actor_components` no longer REBUILDS
health from boss state — it mirrors only presentation (attack timers), carrying
the authoritative reaction timers across the rebuild exactly like the actor
sync. `BossStatus` is now purely encounter state (phase mirror, sprite metrics,
entity-local phase machine). Mechanics of the flip: `BossClusterScratch` gained
the spawn-time `BodyHealth` (bundled by `into_components`);
`BossMut::reset_to_spawn(health, combat)`; `integrate_body(world, alive, …)`
takes liveness in; boss reaction-timer decay moved from `integrate_body` to
`update_ecs_bosses` (`&mut BodyCombat` — the actor tick still excludes bosses
until slice 3); `apply_entity_boss_damage(status, health, amount)` and
`apply_boss_hit(…, health, combat, …)` mutate the shared components. ~35 files
swept across gameplay_core (encounter systems/script/entity, save-sync, reset,
spawn, anim/target/predicate helpers), content (gnu_ton ladder gate, banter,
all seven specials), render (boss animator, health bars, hit-flash material,
overlays), app (debug gizmos + boss test suites). Verified: gameplay-core 1090,
engine-core 211, content 53, render 24, the six app suites, AND
boss_lifecycle (8) / boss_contact_iframes (4) / boss_possession_specials (1).

### E15. A1 slice 2a — boss damage flows through the ONE resolver ✅ (blind — no-i-frame decision surfaced)
`apply_entity_boss_damage` now routes its health/death mechanics through
`combat::damage::resolve_body_hit` — the boss is the FOURTH caller of the one
victim-side resolver (player, actor, boss victim, boss). The invulnerable-PHASE
gate (Intro/Transition/Dormant/Death swallow the hit) stays boss POLICY, checked
before the resolver. The boss's `BodyHitFeel` makes the tuning EXPLICIT and
one-field-tunable: `damage_invuln_time: 0.0` (NO post-hit i-frame — bosses never
had one; `hit_flash: 0.18` was only a bark debounce, so player DPS is unchanged),
no shield. This is the same per-body knob §A2 gave the player (0.75s) and actors
(0.2s). Behavior-preserving: the bark + overlap-flash + death-drops stay boss
policy in `apply_boss_hit`; the 4 contract tests still pass and a 5th pins the
no-i-frame invariant (back-to-back hits both land, `vulnerable()` stays true).
Blind because it's the last damage-mechanics touch on the boss feel surface,
even though it's a no-op numerically today. Verified: gameplay-core 1091,
the six app suites, boss_lifecycle/boss_contact_iframes/boss_possession_specials.
NOTE for slice 3: slice-2 part (b) (give bosses `BodyOffense`/`BodyDodgeState`/
`BodyShieldState` + delete the `Option`-typed vuln in `apply_hitbox_damage`)
was DEFERRED into slice 3 — the win is only removing an `Option` (the boss
victim path in `apply_hitbox_damage` stamps `HitTarget::Actor(boss)`, which
lands nowhere today since `apply_feature_hit_events`' actor loop is
`Without<BossConfig>` and its boss loop only runs when `actor_target.is_none()`),
so adding the clusters is behavior-neutral cleanliness best done WITH the
holistic boss→actor-archetype conversion + its query-aliasing audit, not before.

### E16. D1 — `crate::config` coordinate facade removed ✅ (first D1 slice)
The `pub use ambition_engine_core::config::{world_to_bevy, WORLD_Z_*, GRID_STEP,
WINDOW_*}` re-export is DELETED from `gameplay_core/src/config.rs`; all 39 refs
(27 in render/app/content, 12 internal) now name `ambition_engine_core::config`
directly — the foundation home of the coordinate transform + z-layer constants.
render/app/content no longer route a pure-geometry symbol through gameplay_core:
the ONLY remaining `gameplay_core::config` import anywhere is `render/fx.rs`'s
`rgba` (the one symbol that legitimately lives here — it needs `bevy::Color`).
Zero Cargo.toml changes (every crate already deps engine_core); pure
import-redirection, compiler-verified behavior-neutral. gameplay-core 1091,
all four crates build. **D1 remaining facades** (each its own commit): `crate::
audio::SfxMessage`→`ambition_sfx` (93 refs; needs `ambition_sfx` dep added to
app/content), `crate::effects`→`ambition_vfx::*` (needs `ambition_vfx` dep in
app/content), `crate::time::{world_time,clock_state}`→`ambition_time`, and the
big one — the `features/mod.rs` 271-internal-ref hub (all inside gameplay_core,
real homes in `combat/`, so NO Cargo changes; the #1 navigability win).

### E17. D1 — `crate::effects` facade DELETED ✅ (second D1 slice)
`effects/mod.rs` (a pure `pub use ambition_vfx::*` glob) is GONE, and `pub mod
effects` is removed from `lib.rs`. All 70 refs to `crate::effects::{Effect,
EffectRequest, DamageBox, DamageBoxEffect, SummonSpec, apply_effects,
spawn_damage_box}` (43 internal, 21 content, 6 app) now name `ambition_vfx::`
— the crate where all seven symbols actually live (verified). `ambition_vfx`
added as a direct dep of `ambition_content` + `ambition_app` (they were leaning
on gameplay_core to re-export the vfx vocabulary). Compiler-verified
behavior-neutral: gameplay-core 1091, content 53, all four crates build. The
substrate-bound executors (`apply_summon_effects`, `apply_projectile_effects`)
correctly STAY in the lib — they consume `ambition_vfx::Effect`, they aren't
facades.

### E18. D1 — `crate::audio::SfxMessage` facade removed ✅ (third D1 slice, the headline one)
The `pub use ambition_sfx::SfxMessage` re-export is DELETED from
`audio/mod.rs`; all 114 refs now name `ambition_sfx::SfxMessage` (95 internal,
10 app, 7 content, 1 render, 1 app-test). The audio module KEEPS its real
runtime code (`AudioLibrary`, `MusicChannel`, the Kira plugin, …) — only the
one re-exported type moved home. Its own audio-feature submodules
(`runtime.rs`, `tests.rs`) that reached `SfxMessage` via `use super::*` now
import it explicitly. `ambition_sfx` added as a direct dep of `ambition_render`
(its single ref in `fx.rs`); app/content already had it. Compiler-verified
behavior-neutral: gameplay-core 1091 (default + `--features audio`), all four
crates build, the scripted_gameplay app-test target compiles. This was the
audit's headline D1 item ("93 of 94 inbound refs are this one symbol").

### E19. D1 — `crate::time::{world_time,clock_state,time_control}` ambition_time re-exports removed ✅ (fourth D1 slice)
The generic time vocabulary (`WorldTime`, `ClockState`, `ClockDomain`,
`refresh_world_time`, `ProperTimeScale`) lives in `ambition_time`; gameplay_core
only re-exported it "so historic paths keep resolving." DELETED the pure
re-exports — `time/clock_state.rs` (whole module + its `pub mod`),
`pub use ambition_time::{refresh_world_time, ClockDomain, WorldTime}` in
`time/world_time.rs`, `pub use ambition_time::ProperTimeScale` in
`time/time_control/mod.rs`, and the three crate-root re-exports in lib.rs. All
~93 refs (69 internal + 24 in render/content/app) now name `ambition_time::`
directly (grouped-import audit first confirmed zero `use …::{…}` groups pulled
these symbols, so a word-boundary redirect was clean). The `time/` module KEEPS
its real sandbox code: `time_control` (the feel-tuned clock authority —
`ClockScaleRequest`/`RegimePolicy`/the dispatch systems), `camera_ease`, `feel`,
`move_toward`, and the `mirror_sim_dt_into_runtime` bridge (which now names
`ambition_time::WorldTime` for its own `Res` param). `ambition_time` added as a
direct dep of render + content (app already had it). Also fixed a
docs-describe-moved-thing: `platformer_primitives/src/time.rs` pointed at
`ambition_gameplay_core::WorldTime::sim_dt` (now `ambition_time::`).
Compiler-verified behavior-neutral: gameplay-core 1091, all four crates build,
the nine app integration suites green.

**D1 remaining** — only the big one now: the `features/mod.rs` re-export hub —
and see the E21 note below: the hub is a 3-layer facade STACK entangled with the
D2/D3 crate moves, so it can't be redirected cleanly in isolation (a naive
`features::X` → `combat::components::X` would just point at a middle facade). It
should be redirected type-family by type-family AS those families reach their
real leaf-crate home — exactly what D2 just did for `Body{Health,Combat,Wallet}`.
> `[opus-4.8[1m]]` **fable should re-check** — two reframings of fable's D1/ADR-0019
> read here: (a) the audit called the hub "271 internal refs"; I measured **445
> internal + 189 external = 634** (`grep -c` on `crate::features::X` /
> `gameplay_core::features::X`), and it's a **public**-surface change, not
> internal-only. (b) The ADR-0019 gap summary calls the residual leaks "mostly
> one-file data migrations along existing seams" — for `components::` symbols
> that's optimistic: they're a 3-layer facade STACK (features → combat::components
> → crate::actor → foundation), so the honest home is a *foundation crate*, not
> `combat::components`, and the redirect must ride the D2-style leaf move. Possible
> I'm undercounting a curated-prelude intent fable had in mind; flagging for review.
> **[fable 2026-07-03: CONFIRMED — see AD4; no curated-prelude intent survives the count. Family-by-family is binding.]**

### E20. D2a — re-home Body{Health,Combat,Wallet} DOWN to `ambition_characters::actor::body` ✅ (keystone)
`src/actor.rs` (300 LOC) was ~90% pure re-exports of foundation types
(`BodyKinematics`, the 18 engine_core `Body*` clusters, the entity markers) with
only THREE types actually DEFINED in the 95k game crate: `BodyWallet`,
`BodyHealth` (a thin wrapper over `ambition_characters::actor::Health`), and
`BodyCombat` (per-body combat/reaction status). All three are leaf body
vocabulary with no gameplay-shell deps → moved verbatim into a new
`ambition_characters::actor::body` module (retargeting the wrapped `Health` to
the sibling `super::Health`). `crate::actor` `pub use`d them back, so EVERY
existing path kept resolving with zero ref churn — the tiny, safe keystone move.
Feasibility first: characters deps bevy (Component derives) + engine_core, does
NOT dep gameplay_core (no cycle), and `Health` already lives there. Verified:
characters/gameplay_core/render/content/app build; gameplay_core 1091.

### E21. D2b — redirect ~200 consumers to the real home; delete the facade chain ✅
Every consumer now names `ambition_characters::actor::Body{Health,Combat,Wallet}`
directly, and the WHOLE re-export chain that surfaced them through gameplay_core
is deleted: the `crate::actor` `pub use`, the `combat::components::{BodyHealth,
BodyCombat}` re-exports (they only fed `features`), and the `features::{BodyHealth,
BodyCombat}` hub entries (`BodyMelee` stays — it genuinely lives in combat). Sweep
shape: word-boundary redirect of the dominant `*::actor::Body*` path (braces
auto-skip grouped `use`s), then ~12 grouped `use` sites split surgically (Body
types pulled out of groups keeping gameplay-owned neighbours like
`AncillaryMovementBundle` / `BodyKinematics` / the engine_core clusters), then the
facade deletions. The deletion exposed the glob-prelude reality: 13 internal
modules named the Body types BARE via a `super::*` / `features::*` glob — those
now import explicitly (`features/ecs/mod.rs` surfaces them to its `super::`-
referencing submodules; `combat/components/spawn.rs` + `projectile/systems.rs`
import directly; "explicit imports over globs"). **Payoff:** render/app/content
reach these three types without gameplay_core in the path (the D3 compile-time
lever), and this is the TEMPLATE for dissolving the rest of the features hub —
redirect a type family once it reaches its real leaf home, don't chase the middle
facade. **Grouped-import lesson (bit us twice):** a word-boundary sed silently
skips `use x::{\n  A, Moved, B\n}` multi-line groups; caught `plugin_minimal_app`
+ `spawn/tests` here (which ALSO carried the §D1-time `ClockState` grouped miss —
swept in the same pass). Always follow a facade-deletion sed with `cargo check
--tests` AND a multi-line-aware grep. Verified: gameplay_core 1091, characters
250, engine_core 211, render/content/app all build incl. every test target, the
ten app integration suites green.

### E22. D3 — render→gameplay_core edge: scoped the cut + landed the foundation-vocab slice ✅ (D3.1); plan below
Jon picked D3 (the compile-time lever). **Key finding: the payoff is binary** —
render's rebuild only drops out of the hot path when it FULLY stops depending on
`ambition_gameplay_core`; partial type-moves are prep, not payoff. And render
couples across ~30 distinct gameplay_core paths, so the full cut is multi-session.
Landed the safe prep slice and mapped the rest precisely.
> `[opus-4.8[1m]]` **fable should re-check** — the D3 audit says render's imports
> are "**almost entirely read-model vocabulary**." My enumeration
> (`grep -oE 'ambition_gameplay_core::\w+(::\w+)?' | sort | uniq -c`) shows render
> also imports **world/room types** (`RoomGeometry` ×27 — the single biggest) and
> a category the audit didn't call out: **presentation *systems* render registers**
> (`portal::sync_*`, `abilities::traversal`, `dev::dev_tools`, `physics::GravityCtx`,
> `schedule::SandboxSet`, …). So "move the read-model to a sim-view crate" is
> necessary but **not sufficient** to cut the edge — hence "payoff is binary /
> multi-session." Fable may have folded the systems into "presentation" deliberately;
> flagging so it can confirm the surface is bigger than the read-model.
> **[fable 2026-07-03: CONFIRMED — see AD4. Surface is bigger; slice order stands; D3 unblocked by AD1.]**

**D3.1 DONE (`111e8893`):** render's `gameplay_core::actor::Body*` imports were
all pure foundation re-exports → render now names `ambition_platformer_primitives`
(BodyKinematics + markers) and `ambition_engine_core` (the 18 clusters) directly.
~40 refs / 15 modules; `\b`-guarded so `PrimaryPlayerOnly` (a real gameplay_core
query alias) stays. render lib 24 green.

**The remaining render→gameplay_core surface, categorized (measured 2026-07-03):**
- **A. Foundation re-exports** — DONE for `actor::` (D3.1); `config`/`time`/`sfx`
  already done in §D1. Residual: `PrimaryPlayerOnly` (6, a query-filter alias —
  move to `platformer_primitives::markers` or inline).
- **B. Read-model (the sim-view crate — "the missing abstraction"):**
  `features::{ActorSpriteData(7), FeatureVisualKind, FeatureView, FeatureName,
  FeatureEcsWorldOverlay, ecs_actor_render_size, rider_hand_world_pos}`,
  `camera_snapshot::CameraSnapshot2d(2)`, `character_sprites::{CharacterAnim,
  baked_sheet_registry}`. **Entanglement audit:**
  · `FeatureView`+`FeatureVisualKind`+`BoundFeatureKind`+`FeatureCombatTuning`
    (combat/events.rs) are PURE DATA (`ae::Vec2` + primitives + each other) →
    the CLEAN core of `ambition_sim_view`. Footprint ~170 refs / 26 sites
    (FeatureVisualKind alone 114 — a mini-D2b sweep + grouped-import surgery).
  · `FeatureViewIndex` (view_index.rs, `use super::*`) is BUILT from live ECS
    queries (`rebuild_feature_view_index`) — the builder STAYS in gameplay_core;
    only the `FeatureView` value type + the index container move; render must
    read the materialized index, never the `ecs_*` query-taking accessors.
  · `CameraSnapshot2d` — `[opus-4.8[1m]]` **fable should re-check**: the audit lists
    it under the sim-view movers ("459 LOC, already presentation vocabulary"),
    implying a clean move, but its imports pull in
    `persistence::settings::{CameraFramingPreset, CameraAspectPolicy}` +
    `rooms::{CameraClampMode, CameraZoneSpec}` + `camera_ease::{CameraEaseState,
    Tuning}` — so it is NOT a clean mover today. (Fable may have intended those
    config types to move too; I read it as "move CameraSnapshot2d" in isolation.)
    Move it LAST (after settings/rooms/camera_ease are sorted) or invert
    those into a small camera-config type.
    **[fable 2026-07-03: CONFIRMED — see AD4.]**
  · `character_sprites` (4.2k) is its own carve (§D6) — move down beside
    `ambition_sprite_sheet`, then render names it there.
- **C. World/room vocab** — `RoomGeometry` (27, the single biggest render import!),
  `rooms::{Authored, RoomSet, RoomSpec, RoomMetadata, PortalSprite, CameraZoneSpec}`.
  This is **§D4 (extract `ambition_world`)**; render names the world crate.
- **D. Presentation SYSTEMS render registers** (not data — the subtle part):
  `portal::sync_*` (5 fns), `abilities::traversal` (7), `dev::dev_tools` (7),
  `shrine`, `session::{camera_layers, RespawnRoomVisualsRequested}`,
  `physics::{GravityCtx, gravity_aware_flip_x}`, `schedule::SandboxSet`,
  `presentation`, `platformer_runtime::lifecycle`. Each is a system/plugin render
  installs that reads sim state — they either move WITH their subsystem or invert
  through a registered-hook seam. Untangle case-by-case.
- **E. Misc**: `persistence::settings`(6), `dialog::DialogState`, `items::pickup`,
  `projectile::{ProjectileVisualKind, PlayerProjectileState}`,
  `boss_encounter::sprites`, `assets::{game_assets, sandbox_assets}`,
  `combat::BoundFeatureKind` (rides B), `SandboxDevState`, `RoomGeometry` (C).

**Recommended slice order for the cut:** (D3.2) create `ambition_sim_view`
{engine_core + bevy deps}, move the pure-data read-model core (FeatureView/
FeatureVisualKind/BoundFeatureKind/FeatureCombatTuning + ActorSpriteData + the
anim-state enums); gameplay_core's builder writes them, render reads them. →
(D3.3) §D4 `ambition_world` for RoomGeometry + rooms (biggest single reducer). →
(D3.4) §D6 `character_sprites` down. → (D3.5) settings/camera → move
CameraSnapshot2d. → (D3.6) untangle category-D systems. → (D3.7) drop the
`ambition_gameplay_core` dep from render's Cargo.toml — the lever fires. This is
the same "move a family to its leaf home, then redirect" template D2 proved.

### E23. D3.2a — `ambition_sim_view` crate created; pure-data read-model core moved ⟲ REVERTED (see E24)
Created the leaf crate (`crates/ambition_sim_view`, deps: `ambition_engine_core`
+ `bevy` ECS-derive only) and moved the pure-data read-model core out of
`combat/events.rs`: `FeatureVisualKind`, `FeatureView`, `BoundFeatureKind`,
`FeatureCombatTuning` (+ `DEFAULT_*_ATTACK_*` consts). D2-style a/b split:
**D3.2a** = gameplay_core `pub use`s them back from `combat/events.rs` (every
internal `crate::features::*`/`combat::events::*` path resolves with ZERO churn —
the 114 `FeatureVisualKind` internal refs untouched) while RENDER names
`ambition_sim_view::` directly (10 sites: features/world/primitives/actors/boss +
the `rendering::mod` re-export). render + gameplay_core both dep the new crate.
**Correction to the E22 plan `[opus-4.8[1m]]`:** `ActorSpriteData` and the
`ecs_*` anim accessors CANNOT move here — `ActorSpriteData` is a
`#[derive(QueryData)]` borrowing gameplay_core ECS components
(`actor_clusters::*`, `BodyMelee`, `crate::actor::Body*`), and `ActorAnimFrame`
holds `character_sprites::CharacterAnim` (§D6). Only genuinely transferable
*value* types belong in sim_view; live-query views stay in the sim crate and
render reads them through the accessors until the materialized-index switch.
**D3.2b remaining:** redirect the internal gameplay_core refs off the
`combat/events.rs` re-export (114-ref sweep, glob-prelude fixes like D2b) for
full honesty — deferred; behavior-neutral, no render-edge impact.
Verified: sim_view builds, gameplay_core 1091, render 24, content+app build incl
every test target, the ten app integration suites green.

### E24. D3.2a REVERTED (`4a36011b`) — premature; blocked on Jon's taxonomy feedback ⟲
Jon flagged the move (see **JON'S DESIGN FEEDBACK** near the top): (a) a closed
Ambition content taxonomy (`FeatureVisualKind`) does not belong in a crate billed
"reusable/content-free" — my labeling error; (b) the deeper `actors` vs `props`
rethink means `FeatureView.kind` itself is about to change shape, so extracting
the type that EMBEDS it was premature ("right shape first / reorganize don't
adapt"); (c) a ~120-line leaf crate that doesn't yet enable the edge-cut (render
still deps gameplay_core for the query-view read-model `ActorSpriteData` /
`FeatureViewIndex`) hasn't earned its keep. Honest read: the crate only gets
"meat" AND enables the cut once the FULL read-model is **materialized** (the
`ecs_*` query accessors → materialized per-actor snapshot data render reads) —
that's the real D3 work, and its shape depends on the taxonomy. So D3.2a is
`git revert`-ed; **D3.1 stays** (render→foundation redirect, independent + clean).
`[opus-4.8[1m]]` The sim-view abstraction is likely still right EVENTUALLY, but
gated on: (1) fable adjudicating `actors|props`, (2) committing to the read-model
materialization so the crate has real substance. Verified green after revert:
gameplay_core 1091, render 24, all crates+tests build.

### E25. D4 scoped — bigger than audited; `RoomGeometry` is the linchpin ⏸ (decision needed)
Jon picked D4. Measured the real extraction cost (contradiction tag on the D4
audit above). Findings: `world/` = `ldtk_world/` 6.4k (36 outbound refs, the
content-coupled LDtk converter), `rooms/` 2.4k (21 refs, entangled with the
18-param `load_room_geometry`), `platforms/` 951 + `physics.rs` 406 (0 *content*-
outbound but BOTH read `Res<RoomGeometry>`). **`RoomGeometry` (`lib.rs:235` —
`#[derive(Resource, Clone)] RoomGeometry(pub ae::World)`) is the linchpin:**
nothing in `world/` extracts until it has a foundation home, and it's ALSO
render's single biggest gameplay_core import (×27 → the biggest D3 reducer). It's
a trivial newtype over engine_core's `World`, and engine_core already carries
`bevy_ecs` (derives `Component` for the Body* clusters) — so `ambition_engine_core`
(next to `World`) is the obvious home, a clean D2-style value-type move.
**HELD for a decision (D3.2a lesson): don't relocate a type into a FUNDAMENTAL
crate without confirming the shape/naming.** Open Q for Jon/fable: is `RoomGeometry`
(a "Room"-named Resource) OK to live in reusable `engine_core`, or does the name/
placement need rethinking (like `FeatureVisualKind` did)? "Room" reads as a
generic platformer concept (a screen/area), not Ambition content — so I lean
engine_core — but confirming before a ~50-ref sweep it's only worth doing once.
Once the home is set: D4.1 re-home `RoomGeometry` (unblocks all of `world/` +
lands the biggest D3 render win), then platforms/physics extract cleanly, then the
converter-extensibility + rooms inversions are the multi-session remainder.

### E26. D4.1 — `RoomGeometry` re-homed to `ambition_engine_core` ✅ (`0eac4cfa`)
Jon confirmed the home (engine_core, as-is). Moved the `Resource(World)` newtype
next to `World` in `engine_core::world` (native `bevy_ecs::resource::Resource`
derive — engine_core already derives the Body* Components). All ~99 consumer refs
(gameplay_core 48, render 27, content 14, app 10) now name
`ambition_engine_core::RoomGeometry` directly; the gameplay_core crate-root facade
is DELETED. Word-boundary sweep + 9 grouped-import splits (incl. a multi-line
group in `combat/damage.rs` the sed skipped — the recurring lesson). Zero Cargo
changes (all consumers already dep engine_core). **Payoff banked:** render shed
its single biggest gameplay_core coupling (×27) toward the D3 edge-cut, and
`world/` extraction is unblocked (RoomGeometry no longer pins platforms/physics/
rooms to gameplay_core). Verified: engine_core 211, gameplay_core 1091, render 24,
content+app build incl every test target, ten app integration suites green.

**D4 remainder (multi-session, unchanged shape):** (D4.2) extract `platforms`+
`physics` — now free of the RoomGeometry pin, but still touch `world::rooms`
specs + the `MovingPlatformSet` crate-root Resource + `platformer_runtime`, so
they land WITH rooms or need those handled. (D4.3) the LDtk-**converter
extensibility** refactor (content-registered entity converters — the real crux,
ADR-0009-shaped). (D4.4) the rooms→player/features inversions (RoomTransitioned
message; decouple the 18-param `load_room_geometry`). These are the bulk; each is
its own slice.

### E27. A1 slice 3a (folded slice-2b) — bosses carry the vulnerability trio ✅ (`bed19ad3`)
The boss is a victim-side BODY like every other actor: it now carries
`BodyOffense`/`BodyDodgeState`/`BodyShieldState` (default-inert — bosses have no
dodge/shield/parry today), so EVERY `CenteredAabb`+`ActorFaction` body carries the
trio and `apply_hitbox_damage`'s victim tuple drops its `Option` fallback (which
only existed because the boss used to lack them). Both audits the slice-2b note
demanded came back clean: **(1)** no standalone `&mut BodyOffense/DodgeState/
ShieldState` query aliases the boss query (only the `Without<BossConfig>` actor
cluster views + the dev editable tool gated on `BodyMana`, which bosses lack);
**(2)** the ONLY `CenteredAabb`+`ActorFaction` entity lacking the trio was the boss
(actors get it via `AncillaryMovementBundle`, enemy projectiles spawn it
explicitly) — nothing is silently dropped from the victim query. Behavior-neutral:
the boss already matched as a victim (via the `Option`=None arm) and its
`HitTarget::Actor(boss)` event still lands nowhere until the driver fold flips the
actor loop off `Without<BossConfig>`. Verified: gameplay_core 1091; boss_lifecycle
8 / boss_contact_iframes 4 / boss_possession_specials 1.

### E28. A1 slice 3 — motion+float parity net for the driver fold ✅ (`a556281d`)
"Parity harness first, then port boldly." `crates/ambition_app/tests/
boss_motion_parity.rs` (rl_sim) pins the two invariants the integration fold most
threatens and NOTHING else covers: a boss FLOATS (never gravity-falls) and, once
woken, MOVES. The fold swaps the bespoke `step_floating_body` for the shared actor
flight limb; its two opposite failure modes — gravity leaking in (plummet) and the
pattern's `desired_vel` no longer reaching the body (freeze) — are both caught.
Asserted as RANGES not exact trajectories (behavior-not-sacred: the flight limb is
not bit-identical to the float). Baseline: the live boss drifts ~12px in 0.5s
(floats), never plunges >250px over 300 frames, covers real path length.

### E29. A1 slice 3 — the driver fold precisely re-scoped (NOT an adapter) ⏸
Mapping the actor tick against the boss driver turned the audit's vague "big
multi-session fold" into a concrete, blocker-aware plan — and surfaced a trap to
avoid. **The elegant end-state** (per Jon's actors-vs-props feedback + the
`reorganize-don't-adapt` rule): the boss is a real **flight-enabled actor
archetype** + a boss-**encounter** component; `integrate_sim_bodies` /
`tick_actor_brains` then integrate/tick it with NO boss-specific arm. **The trap:**
bolting a bespoke boss query-arm into `integrate_sim_bodies` (the way it already
holds player+actor arms) is tempting and would even pass the parity net — but it is
an *adapter toward a canonical form*, not canonicalization, so it's explicitly
ruled out. The real work is closing the archetype gap. Concrete blockers found:

1. **Archetype gap is large.** An actor carries `ActorStatus, ActorConfig,
   ActorMotionPath, ActorSurfaceState, BodyMelee, AncillaryMovementBundle (~15
   Body* clusters), CombatCapabilities`; the boss carries only `BodyKinematics,
   BossConfig, BossStatus, BodyHealth, BodyCombat, +vuln trio`. Making the boss an
   actor means reconciling `BossConfig/BossStatus` with `ActorConfig/ActorStatus`
   and giving it the movement clusters with **flight enabled** so `ActorMut::update`
   reproduces the float (the flight limb aerial enemies already fly through).
2. **Brain-context divergence.** ✅ RESOLVED (E30). The boss brain now ticks through
   the universal `Brain::tick` — `BrainSnapshot` carries the BossPattern fields and
   `BossAttackState` moved into `BossPatternState` as a projection (so the
   `(snapshot, out)` signature needs no separate attack-state out). Both "either/or"
   options in the original note were taken (snapshot fields AND state-owned
   attack-state), because both are the elegant shape.
3. **Attack-state authority (slice 3b).** `BossAttackState` (telegraph/active
   windows + profile) must merge into `BodyMelee`/moveset; it's read by
   `boss_attack_damage`, telegraph-volume rendering, sprite anim, AND the possession
   input→special map — all move together.
4. **Param ceiling.** `update_ecs_bosses` (integrate + publish `boss_attack_damage`)
   and `tick_boss_brains_system` are near Bevy's 16-param limit each; folding them
   into the already-full actor systems needs the tuple-bundling `tick_actor_brains`
   already uses.
5. **Reaction-timer decay is duplicated** (`update_ecs_bosses` lines ~430-434 vs
   `tick_actor_brains` ~288) — collapses for free once the boss is in the actor loop,
   but moving it standalone risks a one-frame i-frame-gate shift (boss_contact_iframes
   is sensitive), so do it WITH the fold, not before.

**Recommended bold sequencing next session:** (3b) `BossAttackState`→`BodyMelee`/
moveset first (decouples attack from the archetype); (3c/3d) then the archetype swap
+ tick/integrate fold as ONE bold commit gated on compile + the E28 parity net + the
13 boss suites; (3e) possession special-map dies with the unified path; (3f) render
`BossAnim`→`CharacterAnim`; (3g) `BossStatus`→`BossEncounter` component + `BossConfig`
→ pure archetype data. Each feel-touching commit ships BLIND (Jon feel-checks).

### E30. A1 slice 3c (brain half) — boss brain ticks through the universal `Brain::tick` ✅ (`5c4a2a9d`)
Killed the bespoke boss brain call site: the `BossPattern` brain now ticks through
the SAME `Brain::tick` → `tick_state_machine` path every other body uses.
`tick_boss_pattern_via_state_machine` was a neutral STUB (with a test pinning "it
stays neutral so the boss tick doesn't race it"); it's now real — it rebuilds the
`BossPatternContext` from the shared snapshot and calls `tick_boss_pattern`. Two
seams, both the elegant shape (NOT adapters): **(1)** `BrainSnapshot` gained the
three BossPattern-only inputs (`boss_encounter_phase` / `world_size` /
`front_wall_clearance`), added WITH their consumer per the snapshot's
"no-speculative-fields" rule; **(2)** `BossAttackState` moved INTO `BossPatternState`
as a projection of the pattern cursor (it always was one) — that's what lets the
universal `(snapshot, out)` signature carry no separate attack-state out, with the
ECS `BossAttackState` component now a read-model mirror the boss tick copies from
`state.attack_state`. The ECS boss tick builds the universal snapshot + calls
`brain.tick` + mirrors the projection; the `pattern_brain_mut` helper is deleted.
Possession + integration paths untouched. **Behavior-neutral + compiler+test-verified
(not blind):** a new parity test ticks a BossPattern brain via BOTH paths asserting
identical frame + attack-state, and the live sim confirms. brain 250 (incl. parity),
gameplay_core 1091, boss_lifecycle 8 / boss_contact_iframes 4 /
boss_possession_specials 1 / boss_motion_parity 2; render+content+app build.
**Remaining slice-3:** 3b (attack geometry→moveset), the archetype swap + integrate
fold (blocker #1 — still the big one), 3e/3f/3g.

### E31. A1 slice 3b scoped — a genuine capability gap, NOT a mechanical fold ⏸ (design fork)
> **[fable 2026-07-03] ADJUDICATED — see AD2.** Per-frame tracking is canonical;
> generalize the shared hitbox pipeline (frame-driven geometry in the combat
> layer), fold boss CONTACT damage onto `apply_actor_contact_damage` (not
> respawned-per-tick hitboxes), delete `boss_attack_damage` at the end.
Started 3b (`BossAttackState`→moveset). The **hurtbox** side is already
actor-unified (the `CombatGeometry` trait — player/enemy/boss share
`damageable_volumes`). The **attack** side is where the boss is genuinely special,
and it doesn't fit the moveset model as-is:
1. **Per-frame sprite-driven hitboxes.** `active_attack_volumes` re-reads
   `attack_state.active_elapsed` every tick to sample the sprite-authored
   per-animation hitbox, so a multi-part boss (GNU-ton) has an attack box that
   *tracks the drawn pose frame-by-frame*. The moveset `MoveSpec` and the shared
   `Hitbox` primitive only support STATIC body-local authored volumes
   (`FollowOwner{local_offset}` / `World`) — there is no sprite-frame-driven anchor.
   Converting naively LOSES per-frame tracking (a real feel/behavior change for
   multi-part bosses).
2. **Poll vs hitbox-entity.** `boss_attack_damage` is a per-tick POLL emitting
   `HitEvent` directly; everyone else spawns `Hitbox` entities resolved by
   `apply_hitbox_damage` (whose Boss-faction branch already exists — §A3). The dedup
   semantics differ: the poll re-emits each overlapping frame (gated by the victim's
   consume-time i-frames), a hitbox entity dedups per-lifetime via `HitboxHits`.
   Preserving the current contact/strike i-frame feel (pinned by
   `boss_contact_iframes`) through that switch is subtle.

**Recommended approach (behavior-preserving):** the boss tick keeps OWNING the
strike geometry (it already computes `active_attack_volumes`), but instead of
polling it MAINTAINS a Boss-faction `Hitbox` entity per active volume — spawned on
the telegraph→strike edge, its `half_extent`/`local_offset` UPDATED each tick from
the live sprite-driven volume (preserving per-frame tracking), despawned on
strike-end. Damage then flows through the shared `apply_hitbox_damage` Boss branch;
`boss_attack_damage`'s strike arm is deleted. The body-contact arm converts to a
persistent body-contact hitbox respawned per tick (to keep the i-frame-gated
continuous-overlap feel). This is FEEL-SENSITIVE (ships BLIND; `boss_contact_iframes`
+ `boss_motion_parity` are the mechanics net) and needs the per-tick hitbox-geometry
update on the primitive — a real change, not a rename. **This one is a design fork
worth a nod before building it blind** (per the same discipline that surfaced
actors-vs-props + the converter extensibility): the alternative is to accept static
strike hitboxes and drop GNU-ton's per-frame tracking, which is simpler but a
behavior change.

### E32. A1 slice 3 — archetype swap AS1/AS2/AS4a landed; the motion fold de-risked; the size flip is the gate ⏳
The driver fold executes as an **archetype swap** (the boss BODY becomes an aerial
actor; the ENCOUNTER wrapper — `BossConfig`/`BossEncounter`/`BossAttackState`/phase
machine/attack geometry — stays). Landed, each green + committed:
- **AS1** (`6dc9e6f5`) — `BossStatus` → `BossEncounter` (the body's HP/liveness
  already left it in §A1; what remains is genuinely encounter state).
- **AS2** (`e387c786`) — the boss carries the SAME aerial actor movement cluster
  every actor does (18 ancillary clusters + `ActorStatus`/`ActorConfig`(aerial,
  flight-enabled)/`ActorSurfaceState`/`BodyMelee`/`CombatCapabilities`), MINUS the
  `BodyKinematics`/`BodyHealth` it already owns. The `AncillaryMovementBundle` also
  supplies the slice-3a vulnerability trio (that standalone insert removed). INERT
  this slice — old driver still owns intent+integration, so `boss_motion_parity`
  stays byte-green. Archetype-collision audit: the only body-generic system a boss
  newly matches is `advance_body_melee`, which no-ops on `melee.swing == None`.
- **AS4a** (`d7325681`) — engine **direct-velocity flight mode** (`MovementTuning.
  flight_direct_velocity`, serde-default false). The shared flight limb smooths via
  accel/drag/deadzone; a boss commands an EXACT velocity/tick, so the smoothed limb
  would silently change boss feel. Direct mode takes `stick × terminal` verbatim →
  byte-identical to the old SNAP float (`step_floating_body`, `accel: None`).
  Default-off ⇒ every existing flyer + the engine replay canaries unchanged. This
  is the KEYSTONE that makes AS4c's motion fold provably zero-change (engine test
  `direct_velocity_flight_takes_the_commanded_velocity_verbatim`).

**Reframing vs the original AS-plan:** `BossRef`/`BossMut`/`BossClusterQueryData`
are NOT parallel-actor-stack bloat — they view the ENCOUNTER components
(`BossConfig.behavior`, `BossEncounter.sprite_metrics`) for `combat_size`/
`combat_offset`/`render_size`, which are genuine boss-encounter concerns, distinct
from the actor body cluster. So **AS5 (delete the views) is DROPPED as low-value /
high-churn** — the real convergence is the boss BODY integrating through the shared
seam, which is AS4b+AS4c. The one parallel-INTEGRATION to dissolve is
`update_ecs_bosses`' `step_floating_body` call + `BossMut::integrate_body`.

**AS4c (boss → shared flight limb) is GATED on AS4b (the size flip), and AS4b is a
blind cross-crate render untangling — the honest blocker.** The shared movement
seam (`update_body_with_tuning_clusters`) collides against `kin.size`; a boss
collides against `combat_size` (≠ `kin.size` — every boss has a distinct
`behavior.combat_size`, see `boss_profiles.ron`; `kin.size` is the LDtk spawn seed).
So AS4c needs `kin.size = combat_size`. But the boss RENDER
(`upgrade_boss_sprites` at `ambition_render/.../actors/boss.rs:76,157`) derives the
sprite quad as `boss_asset.spec.render_size(kin.size)` — flipping `kin.size` resizes
every boss sprite. The fix is to route render to an explicit render size
(`ActorRenderSize` = `sprite_metrics.sprite_render_size`, which
`derive_boss_sprite_metrics` already computes for hurtbox scaling) and set
`kin.size = derived_combat_size` there (after `sprite_render_size` is computed from
the seed). **Verifiability:** collision/hurtbox is covered by the boss suites
(`boss_contact_iframes`/`boss_lifecycle`/`damageable_volumes` tests) + a golden
geometry pin; the sprite quad is preserved-by-construction IFF
`sprite_metrics.sprite_render_size` equals today's
`boss_asset.spec.render_size(kin.size)` for every real boss — an invariant that
needs a **render-vs-gameplay spec-parity test** (the gameplay `sprites::*_SHEET`
constants that `sprite_render_size_for` picks by target vs the loaded
`boss_asset.spec` the render picks by `boss_key`). Build that pin FIRST; if it
holds, AS4b/AS4c land verified. If it diverges, that mismatch is a latent
render/hurtbox bug to fix regardless.

**AS4c mechanics (once AS4b holds):** `update_ecs_bosses` replaces
`feature.as_boss_mut().integrate_body(world, alive, control.0.velocity_target, dt)`
with `actor_cluster.as_actor_mut().update(world, target_pos, combat_tuning, None,
dt, false, control.0, gravity_dir, feel, stagger)` (the boss's `ActorConfig.tuning`
sets `flight_direct_velocity: true` + `chase_speed/max_run_speed = BOSS_FLIGHT_SPEED
= 1200`; add `flight_direct_velocity` to `ActorTuning` and thread it into the engine
tuning in `ActorMut::integrate_body`). The boss stays in `update_ecs_bosses` (keep
its presentation + `boss_attack_damage` publish); only the integration algorithm
swaps. Then delete `BossMut::integrate_body` + `step_floating_body` (last holdout).
Golden trajectory pin (capture current SNAP path, assert flight-limb path matches
within tight tolerance) makes it verified, not blind.

### E33. A1 slice 3 — archetype swap AS4b + AS4c LANDED: the boss body is an aerial actor ✅
Per fable AD3. The boss BODY now moves through the ONE shared movement seam.
- **AS4b** (`601496c2`) — `kin.size` IS the collision envelope (`combat_size`); the
  sprite RENDER-BASIS moved to `BossEncounter.render_size` (the LDtk seed the sheet
  scales the drawn quad from). The AD3 spec-parity pin REVEALED that the render draws
  from BAKED sheet dims while the const `render_size` uses const dims, and they
  DIVERGE for real bosses (gradient sentinel is really 256×253, not 128×128) — so a
  const-derived render size would resize sprites. Chosen fix: store the seed basis +
  let the render keep `spec.render_size(seed)` → byte-identical. (The pin is now a
  standing characterization guard; the render/hurtbox const-vs-baked gap is a latent
  bug to converge in a separate blind slice.) Byte-identical: gameplay_core 1092,
  boss geometry/hurtbox suites green.
- **AS4a** (`d7325681`) — engine `flight_direct_velocity` (default-off, canary-safe):
  the flight limb takes `stick × terminal` verbatim, byte-identical to SNAP.
- **AS4c** (`28bdf71d`) — new `integrate_boss_bodies` arm (boss sibling of the
  player's `integrate_home_body`; scheduled brain-tick → arm → presentation) routes
  the brain's `velocity_target` through `ActorMut::update` → the shared flight limb in
  direct-velocity mode, self-heals `kin.size` to the collision envelope, publishes the
  render-basis `CenteredAabb`. `update_ecs_bosses` is now presentation + attack-damage
  publish only. **boss_motion_parity green** — the boss floats + moves correctly
  through the flight limb. Wall-collision sweep now goes through the shared pipeline
  (was `step_kinematic`) — a deliberate convergence, velocity byte-identical, in Jon's
  feel-check queue (AD5).

**AS4c cleanup — DONE** (`c0b3f591`): the bespoke boss float
(`BossMut::integrate_body` + `step_floating_body` + the orphaned
`combat::util::approach`) is DELETED (~70 LOC). The wall-collision test was migrated
to drive the boss pattern through the PRODUCTION path (aerial `ActorClusterSeed` →
`ActorMut::update` → flight limb), which VERIFIES the flight-limb sweep stops the
boss at a wall — resolving the AS4c blind-wall-sweep concern with a real test. A boss
IS just an aerial actor; no parallel float remains.

**Follow-ups (net-LOC-down + AD-driven):**
- **3b per AD2** — generalize a `FrameDrivenHitbox` in the combat layer; fold boss
  contact onto `apply_actor_contact_damage` (flip the boss cluster's
  `body_contact_damage` false→true from `behavior.body_damage` in the same commit);
  delete `boss_attack_damage`. Ships blind + new frame-tracking test.
- Converge render + hurtbox on ONE true (baked) render size — fixes the latent
  const-vs-baked gap the AS4b pin documents (AD3 "fix regardless"; blind, tiny).
- **AD1-T1** — collapse `FeatureVisualKind` actor variants to one `Actor`; the boss
  render can then read `ActorRenderSize` on the unified actor sprite-upgrade path.

### E34. A1 slice 3b (AD2) LANDED — the boss ATTACK PATH is unified ✅
The boss's offense now flows through the SAME systems every actor uses; the bespoke
`boss_attack_damage` poll is DELETED.
- **3b-strike** (`8408d3ae`) — `sync_boss_strike_hitboxes` maintains one Boss-faction
  frame-driven `Hitbox` per `active_attack_volumes` part while a strike is live
  (geometry re-derived each tick so GNU-ton's multi-part hands track the drawn pose;
  despawn on strike-end). Damage resolves through the shared `apply_hitbox_damage`
  Boss branch, deduped per-strike via `HitboxHits`. Removed the `boss_attack_damage`
  strike arm. Mechanism-pinned: `boss_strike_spawns_a_frame_driven_boss_hitbox`.
- **3b-contact** (`054a32fc`) — boss body-contact folds onto the shared
  `apply_actor_contact_damage` (dropped its `Without<BossConfig>` carve-out; the boss's
  contact tuning is driven from `behavior.body_damage` at spawn). `boss_attack_damage`
  DELETED entirely; `update_ecs_bosses` collapses to pure presentation. **Verified by
  `boss_contact_iframes`** — i-frame-gated boss contact damage lands correctly over a
  multi-second run (the fold WORKS, not just compiles).

Ships blind on FEEL only (Jon's AD5 queue) — the mechanisms are test-verified. Follow:
the frame-driven hitbox is now generic enough for actor melee / the moveset
clip-by-phase seam to opt in later (retiring freeze-at-entry), per AD2(b).

### E35. AD1-T1 LANDED — the four actor `FeatureVisualKind` variants collapse to ONE `Actor` ✅ (`8cef2245`)
`actors|props` is now the render taxonomy in code. `FeatureVisualKind` becomes
`{ Actor, Hazard, Breakable, Chest, Pickup, Switch }`; the five prop kinds stay
closed (real interaction-kit families), the four actor kinds are ONE. Net **-52 LOC**
— convergence, not just behavior.
- **Merge:** `upgrade_enemy_sprites` + `upgrade_npc_sprites` → one name-first
  `upgrade_actor_sprites` (override label → actor name → registry, then a STATE-keyed
  fallback: sandbag→sandbag sheet, fighting→generic enemy sheet, peaceful-unregistered
  → keep terminal placeholder). The app-side separate npc registration is gone; it runs
  in the render `PresentationVisualSync` slot the enemy upgrade held.
- **Deletes:** the two duplicate `visual_kind` derivation helpers (`enemy_visual_kind`,
  `EnemyIntegration::visual_kind`), `CharacterSpriteAssets::enemy_asset(kind)`,
  `is_boss_kind`, `entity_sprite_for_kind`'s actor arms. Their surviving logic is the
  ONE sandbag fallback pick: `enemy_spawn_is_sandbag` (spawn) + `ecs_actor_is_sandbag`
  (runtime). `ecs_npc_name`/`ecs_enemy_name` were byte-identical → one `ecs_actor_name`.
- **State, not type:** `FeatureView` gains `fighting: bool` (a fact about the actor —
  NOT "hostile to the player"; relativity principle), stamped at the rebuild site from
  the disposition signal (INTERIM — the `FightingAble` capability component + its state
  transitions are the later disposition-reshape slice, deferred per AD1 so T1 doesn't
  balloon). Placeholder tint modulates on it, so enemy-red / npc-blue survive as STATE.
- **Boss:** an actor like any other; `upgrade_boss_sprites`' gate is now its own query
  family (the `ecs_bosses` find_map), not a `Boss` variant.
- **Blind visual deltas (Jon's feel pass):** sandbag placeholder tan→blue, boss
  placeholder purple→red, NPC placeholder z −1 layer (now == actor); decorative props
  borrow the Actor placeholder kind (pre-existing conflation, marked SMELL in
  `spawn_room_prop`). Sprite RESOLUTION is name-first and unchanged → sheeted actors
  look identical. SMELL to sweep later: `assets/sprites/**/entity_manifest.yaml` still
  labels `category: FeatureVisualKind::Npc/::Boss` (tooling metadata, not parsed by
  Rust; regen or relabel to `Actor`).

Verified: gameplay_core 1089 + render 24 green; full workspace compiles. **D3 is now
UNBLOCKED** — T2 (materialize the read-model, then re-create `ambition_sim_view`) is next.

### E36. T2 IN PROGRESS — the ACTOR read-model is materialized; render's actor path holds NO live sim borrow ✅ (`177c182d`, `ceed1fd8`)
AD1-T2's core: render must read a MATERIALIZED read-model, never the sim's live
ECS (the condition E24 set for re-creating `ambition_sim_view`). Render's entire
actor-cluster borrow was exactly two systems; both are now cut.
- **Slice A — identity** (`177c182d`): `upgrade_actor_sprites` took
  `Query<ActorSpriteData>` (all 18 clusters) + `Query<ActorRenderSize>` only to
  read four STATIC facts (name / sprite-override / sandbag / render size). Those
  materialize into `ActorRenderIndex` (`rebuild_actor_render_index`, in the sim's
  `FeatureViewSync` set beside the geometry index). Identity is static, so the
  mark-and-sweep rebuild re-clones only on a genuine change — no per-`String`
  churn as the sim steps. Deleted the 4 orphaned accessors (`ecs_actor_name`,
  `ecs_actor_is_sandbag`, `ecs_enemy_sprite_override`, `ecs_actor_render_size`).
  Pins: `actor_render_index_snapshots_identity_sweeps_and_refreshes`.
- **Slice B — pose** (`ceed1fd8`): `animate_characters` took the same 18-cluster
  query for the per-frame pose. That materializes into `ActorAnimIndex`
  (`rebuild_actor_anim_index`, the SAME `pick_actor_anim`) — but because the pose
  is presentation-ONLY, its rebuild is registered in the RENDER presentation
  plugin, chained just before `animate_characters`, so a headless/RL build never
  computes poses it won't draw (RL-step sensitivity respected). `ActorAnimFrame`
  is `Copy` → overwrite in place. Deleted `ecs_actor_anim_state`.
- **Net:** render's actor systems consume `ActorRenderIndex` + `ActorAnimIndex`
  and name `ActorSpriteData` NOWHERE. Per AD1-T2 this keeps `FeatureView` `Copy`:
  the `String`/pose identity lives in side indices keyed by id, not on the shared
  geometry view. Verified gameplay_core 1090 + render 24 green; workspace compiles.
- **Remaining T2 before the edge narrows:** (C) boss render path (`ecs_boss_name`,
  `ecs_boss_anim_state*`, the boss's own `upgrade_boss_sprites` cluster query) →
  materialize like the actor path; (D) prop state (`ecs_chest_opened`,
  `ecs_breakable_state`) — small; then the value types (`FeatureView`/kind,
  `ActorAnimFrame`+`CharacterAnim` §D6) move to a re-created `ambition_sim_view`,
  then §D4 world + category-D systems, then drop render's gameplay_core dep (D3.7,
  the lever). Payoff stays binary/multi-session — these are prep, but each lands green.

### E37. T2 slice C + the CLEAN-READ-MODEL BOUNDARY — boss binding materialized; boss POSE is a bidirectional coupling ⏹ (`af45bc78`)
- **Slice C (`af45bc78`):** `upgrade_boss_sprites` (boss sheet BINDING) now reads
  `FeatureViewIndex` (the boss's render `size`, already built by the boss loop) +
  `BossRenderIndex` (name + behavior id, materialized by `rebuild_boss_render_index`)
  — it names `BossClusterRef` nowhere. Deleted `ecs_boss_name`. Byte-identical
  key derivation; behavior-preserving.
- **BOUNDARY FINDING — `animate_bosses` (boss POSE) is NOT a clean read-model gap.**
  Unlike `animate_characters` (slice B, a pure read), `animate_bosses` WRITES a
  `BossAnimationFrameSample` component BACK onto the boss entity, keyed off the
  render animator's *current frame* (`animator.frame`) — the drawn pose drives the
  boss's per-frame ATTACK-HITBOX geometry (AD2/3b: GNU-ton's hands track the drawn
  frame). That is a genuine render→sim WRITE, a bidirectional coupling, not "render
  borrows sim state to draw." Materializing it into an index is the wrong shape;
  the right fix is to move `BossAnimator` (frame→geometry) SIM-SIDE so the sim owns
  the sample — a much larger slice (the boss analogue of pulling the animator into
  gameplay_core), NOT a T2 materialization. **So the clean T2 read-model slices are
  DONE**: every PURE render→gameplay_core actor/boss live-query borrow that was
  "render reads sim state" is now materialized (`ActorRenderIndex`/`ActorAnimIndex`/
  `BossRenderIndex` + `FeatureViewIndex`). What remains is NOT more of the same:
  boss-pose is bidirectional (own slice), prop-state is low-value FeatureView bloat,
  and the value-type move to `ambition_sim_view` stays PREMATURE (E24 objection 2)
  until the edge actually narrows — which needs §D4 (RoomGeometry done; `rooms`
  next) + category-D systems, the multi-session remainder. Verified gameplay_core
  1090 + render 24 green.

### E38. D3-edge FACADE REDIRECTS — render names the foundation directly for 3 families (D3.1 template) ✅ (`7e221004`, `ffe88faa`, `f59207ab`)
Harvested every render→gameplay_core import that was really a FOUNDATION type
behind a thin gameplay_core facade (the D3.1 `actor::Body*` pattern). Three landed,
zero behavior change, render 24 green each:
- **gravity** (`7e221004`): `crate::physics` is `pub use ambition_platformer_primitives::gravity::*`;
  render's `GravityCtx`/`gravity_aware_flip_x`/`GravityField`/`GravityZone` (7 sites)
  now name the foundation directly.
- **PrimaryPlayerOnly** (`ffe88faa`): the alias `(With<PlayerEntity>, With<PrimaryPlayer>)`
  is a pure composition of two foundation markers → MOVED into
  `platformer_primitives::markers` (gameplay_core re-exports it); render's 6 refs
  name the foundation.
- **lifecycle + gravity_upright_angle** (`f59207ab`): `crate::platformer_runtime::{gravity,lifecycle}`
  are pure re-exports of `platformer_primitives::{gravity,lifecycle}`; render's
  `PlayerVisual`/`RoomVisual`/`SceneEntities`/`gravity_upright_angle` (incl. render's
  own `primitives.rs` re-export) name the foundation. `orientation::ActorRoll`
  (gameplay_core's OWN module) correctly stays.

**Finding: the clean facade redirects are now EXHAUSTED.** The remaining
render→gameplay_core surface (re-measured) is all GENUINE gameplay_core, none a
foundation-in-disguise: `features` (44, the read-model — needs the `sim_view` move,
still premature until the edge narrows), `rooms` (15, the entangled crux —
`RoomSpec`/`RoomSet` pull `characters`+`interaction`+`combat`+`portal`+`platforms`+
`debug_label`; breaking that coupling is a design call for Jon, NOT a safe
autonomous move — see the rooms-extraction map), `portal` (12, category-D: the
sandbox portal-integration SYSTEMS are DEFINED in gameplay_core's `portal/mod.rs`,
not the foundation `ambition_portal`/`_presentation` crates, so they need the
case-by-case system untangle, not a redirect), `assets` (12), `character_sprites`
(9, §D6 carve), + the dev/abilities/session/persistence tail. So D3 is now blocked
on the three genuinely-hard, non-autonomous fronts: the rooms crux, the value-type
`sim_view` move (premature), and the category-D system untangles.

### E39. VERIFICATION + TEST-HARDENING pass — a full `cargo test --workspace` surfaced rotted leaf-crate tests ✅ (`e085d2a2`, `a1a4c04c`, `6782aa0b`)
With the clean architecture work done, ran the FULL workspace suite (24 crates)
end-to-end — the first time this session — as a correctness net. It surfaced a
real INFRA pattern + fixed what was safely fixable:
- **INFRA FINDING — leaf-crate unit tests silently ROT.** The common dev flow
  (`cargo test -p ambition_app`) builds the leaf crates' LIBs but NOT their own
  `#[cfg(test)]` targets, so a crate's unit tests can break for many commits with
  no signal. Three were broken this way, all pre-existing (none from this session):
  the two `architecture_boundaries` tests (stale file-structure assertions —
  `e085d2a2`), and `ambition_vfx`'s `hitbox_shape_tests` (missing the `frame_down`
  field added to `Hitbox` in e56cd830 — failed to COMPILE since; `a1a4c04c`). Same
  root as the logged content-portal-skip smell. **Recommend CI run
  `cargo test --workspace` (not just `-p ambition_app`)** so leaf tests can't rot
  unseen. 24 crates, 17 carry their own unit tests — a broad rot surface.
- **Swept the two documented loose-end warnings** (`6782aa0b`): dead `aim_dir`
  (tick_shark steers by `orbit_dir`), the `hostile_brain_id_for_actor` re-export
  (gated `#[cfg(test)]` — only a test uses that path), plus the `With` import my
  PrimaryPlayerOnly move orphaned.
- **PRE-EXISTING FAILURE flagged for Jon (NOT this session's — confirmed by
  building+running at the session-start commit 7c0872a7 in an isolated worktree):**
  `unified_body_movement::home_body_and_actor_body_move_through_the_same_integration_phase`
  (rl_sim) FAILS — the `cellular_automaton_fighter` enemy doesn't chase: at
  7c0872a7 it moved the WRONG way (x 1110→1134.8, +25px right, away from the
  left-side player); at HEAD it moves the right way but far too little (1110→1109.4,
  −0.6px vs the >5px the test wants). The ~25px swing between two builds of a
  fixed-60hz deterministic sim smells of QUERY-ORDER non-determinism in the chase
  pipeline (per the query-order-determinism rule: sort order-sensitive systems by a
  stable id, not `Entity`). A real chase-AI / determinism bug (possibly tied to the
  PAUSED PCA encounter work), NOT a stale test — out of the architecture-grind
  scope; needs a focused gameplay-debugging slice. Logged to code_smells.

### E40. Full-workspace verification caught a SELF-INTRODUCED regression — arc now green across all feature configs ✅ (`c63646b7`, `f14eae7e`)
A `cargo check --workspace --all-targets` gate (the E39 recommendation, run on
myself) caught that the warning-cleanup commit `889c859d` had BROKEN the render
build: it dropped `mut` from `camera.rs`'s `snapshot` on an "unused mut" lint that
only fires WITHOUT `portal_render` — but a `#[cfg(feature = "portal_render")]` block
reassigns `snapshot.center_world`/`.rotation_radians`, so `mut` is required under
that feature (which render's default + the workspace build enable). Per-crate default
checks + content's `--all-features` dep-build (portal_render OFF) both missed it.
Fixed with `mut` restored + `#[cfg_attr(not(feature="portal_render"), allow(unused_mut))]`
(`c63646b7`); also dropped a pre-existing unused module-level `BossEncounter` import
(`f14eae7e`). Now: `cargo check --workspace --all-targets` = EXIT 0. Remaining 3
warnings are PRE-EXISTING in files this session never touched (portal_presentation
gun_visuals, dev prims DebugLabel/height, smash/arena) — left alone precisely because
the camera.rs episode proved unused-mut/dead-code lints can be feature-gated and are
dangerous to blind-fix. Lesson recorded in dev/journals/lessons_learned.md
(2026-07-03): warnings are config-relative; verify fixes with `--workspace
--all-targets`; prefer scoped `cfg_attr(allow)` over deleting near `#[cfg(feature)]`.
This episode VALIDATED the E39 `--workspace` CI recommendation — it caught the
regression, the rotted leaf-crate tests, AND is the only gate that sees all configs.

### E41. VERIFICATION SWEEP against code — §B is ~COMPLETE, several §C done; the doc's TASK sections were badly stale ✅ (`afd91842`)
Resuming on Jon's "continue the review" ask, re-verified the open TASK-section
items against current code (the task descriptions are the ORIGINAL audit; many
got fixed via E-entries / delegated agents without striking the description).
Findings — **most of the audited work is already done; the doc misrepresented it:**
- **§B frame-of-reference is ~COMPLETE.** Verified fixed in code (frame-projected,
  most annotated `§Bn`): **B3** (`movement/blink.rs:35-49` — `AccelerationFrame`
  `to_local`/damp-x/clamp-y/`to_world`), **B4** (`control.rs:139` — recoil on
  `frame.side`), **B6** (`integration.rs:150-231` is now ONE role-based branch:
  side-sweep → wall abilities → clear ground → gravity-sweep, `side_axis`/
  `gravity_axis` derived from `gravity_dir` — the two X/Y branches the audit
  describes are gone), **B9** (`control.rs:31/72/111` + `blink.rs:60` — fallback
  aim on `frame.side`). B5/B7 done (E3/E4); B1/B2/B10/B11/B13 done (E5). Minor
  notes: hard-fall shake now `vel.dot(gravity_dir)` (`body_integration.rs`);
  `gravity.rs::vertical_sign`/`local_gravity_sign` now HAVE consumers (`:306-307`)
  — not dead. **Jon's bug-list "attack hitbox direction bugged in non-down
  gravity" is FIXED** — `combat/attack.rs:182,201` builds the owner's
  `AccelerationFrame::new(gravity_dir)` and applies `spec.into_world_frame(frame)`
  (rotates hitbox/impulse/knockback). **Genuinely-open §B: B8** (portal-gun aim —
  still caveated "verify vs portal agent"), **B12** (query-order tiebreak —
  `targeting.rs:266` keeps first-on-tie; portal first-qualifying) — both LOW.
- **§C already done (doc stale): C8** (`SpecialPreset` IS `Special(String)` at
  `entry.rs:357`), **C10/A11** (`SpecialActionSpec` collapsed to `Special(String)`
  at `action_set/mod.rs:483`; the per-boss variants are gone; player special is
  `Special("bubble_shield")` in `player/bundles.rs`).
- **LANDED this pass — C7 (partial):** the composite rider-name suffix hardcode
  (`.strip_suffix(" on Shark")` duplicated in sim `spawn_mounts.rs` + render
  `world.rs`) → authored `CompositeVisualSpec::rider_name_suffix: Option<String>`
  routed through ONE shared `composite_rider_name` helper. A second game's mount
  authors its own suffix; the engine names no `" on Shark"`. `afd91842`.

### E42. A8 — ONE body-generic movement-fx emitter for player + actors ✅ (`219188c7`, BLIND)
Actors that jump/dash/dodge/wall-jump/ledge/shield emitted NO dust/SFX — the actor
tick consumed only `move_events.blinks` and dropped every other op, and that blink
SFX was a hand-copied second emit site (`Play{PLAYER_BLINK}` at the destination),
the "parallel emission site" bug. Extracted the player's `FrameEvents`→SFX/VFX
emission verbatim into body-generic `player::emit_movement_fx(events, pos, facing,
size, on_ground, was_grounded)`; `handle_player_events` now wraps it (keeping the
player-only wall-jump pose + blink-camera lerp + hit-flash), and the actor tick
(`features/ecs/actors/update.rs`) routes through it. AI fighters now emit the same
jump/dash/dodge dust + SFX + landing dust as the player; the blink SFX unifies to
`SfxMessage::Blink` at the origin. Player SFX/VFX byte-identical by construction;
emitter contract test added. BLIND: the audio/feel of actor movement SFX is Jon's
to verify. (A9 anim-overlay fork stays open — the emitter deliberately omits
body-specific anim state.)

### E43. A9 — actors show movement overlay poses via body-generic BodyAnimFacts ✅ (`9aedb222`, `0122eddf`, slice 2 BLIND)
The presentation overlay-timer component was player-only in name AND effect.
Slice 1 (`9aedb222`, safe compiler-gated, 47 refs, zero behavior change): rename
`PlayerAnimState` → body-generic `BodyAnimFacts`. Slice 2 (`0122eddf`, BLIND):
extract the player's edge-arming into body-generic `advance_body_anim_overlays`
(the player tick calls it; the dead `update_anim_signal_timers` in
`app/sim_systems` is gone); actors carry `BodyAnimFacts` (`EnemyActorBundle`),
advanced by a presentation-scheduled `advance_actor_anim_overlays` (excludes the
home player → no double-tick; possessed bodies ARE advanced; `sim_dt` so poses
pause/slow with the sim); `ActorAnimState` + `pick_actor_anim` gain the overlay
reads (wall_jump / dash_startup / landing / shoot), fed from the actor's
`BodyAnimFacts` (`Option`, base-ladder-safe) by `rebuild_actor_anim_index`. An AI
fighter now shows the same wall-jump / dash-startup / landing poses the player
does (whatever rows its sheet owns). Player poses unchanged by construction; pick
test added. **Follow-up:** actor wall-jump/shoot ARMING is op/projectile-driven
(the pick already reads them; they show once armed) — arm wall_jump on the actor
WallJump op in the actor update, shoot on the enemy-projectile fire path. The
`landing` hard/soft grade still reads screen-Y `vel.y` (a §B-family assumption
shared with the player). BLIND: on-screen feel is Jon's to verify.

### E44. §A-LINE convergence push — every SAFE actor-unification slice landed; the two remainders scoped ✅
On Jon's "push the A-line to completion, it's the biggest win surface" ask, re-mapped
§A against code (the doc's task list is stale — trust the E-log) and closed every
convergence slice that is safe to land autonomously (verifiable, non-blind,
non-speculative). Five commits, each workspace `--all-targets` green:
- **A9 follow-up** (`ac0ad68c`, BLIND) — actors now ARM the wall-jump + shoot overlay
  poses, not just read them. New body-generic `arm_movement_anim_overlays` (WallJump
  op → pose) called by BOTH the player tick and the actor integrator (retires the
  player's inline arming loop); shoot armed on the frame a body accepts a ranged shot
  in `spawn_enemy_projectiles_from_brain_actions` (autonomous AND possessed). A9 fully
  closed.
- **A10 parry** (`4cb688dd`) — extract body-generic `reflect_parried_shot`; a shielding
  ACTOR (possessed body / mixed-faction duelist) now reflects a shot through the SAME
  re-own mechanic the player uses — parry was player-only. Heal stays player reward
  policy. Dual-pool markers ASSESSED: sim is already unified (`LiveProjectile` +
  owner-faction routing) and render reads `ProjectileVisualKind`, so
  `PlayerProjectile`/`EnemyProjectile` are now only pool-scoped bookkeeping tags, NOT
  a behavior fork — A10 substantively closed (the spawner fold stays deliberately
  deferred, feel-sensitive).
- **A1-3e / A11** (`7aa3dbf1`) — ONE `dispatch_boss_special`; the boss's possession arm
  and autonomous arm no longer carry duplicate `ActorActionMessage::Special` writes
  (behavior-identical: `is_special()` ⟺ `special_key().is_some()`).
- **A7 self-view** (`650a7b6a`) — the enemy `WorldView` build's three hardcoded lies
  are fixed: faction → real EFFECTIVE faction (possession-aware), `can_fire` → real
  ranged capability, relations → the LIVE `FactionRelations`. Latent by construction
  (only Smash reads `WorldView`, via faction-independent LOF) so AI behavior is
  unchanged; the self-view is now honest.
- **A1 shared decay** (`ac4feea8`) — `BodyCombat::decay_reaction_timers(dt)`; the actor
  tick and the boss tick stop hand-copying the same five-timer decay. Byte-identical.

**THE TWO REMAINDERS (both genuinely large; NOT safe autonomous single-session slices):**
1. **A1 boss driver fold — the `BossAttackState` → `BodyMelee`/moveset conversion.** The
   boss brain already ticks the universal `Brain::tick` (3c) and moves through the
   shared flight limb (AS4c); what remains is the boss's ATTACK-STATE authority. Today
   `BossAttackState` (telegraph/active windows + `active_profile`) is a boss-only
   component with **127 references** across volumes/anim/damage/overlay/content-specials,
   and multi-special bosses set `ActionSet.special = None` because the unified single
   special slot can't hold their repertoire. Dissolving it means the boss carries the
   actor cluster + a **moveset** ([[project_moveset_system]]: `MoveSpec` phases) instead
   of `BossAttackState`, and `tick_boss_brains`/`update_ecs_bosses`/`sync_boss_actor_components`
   fold onto the actor systems. **This needs a DESIGN decision from Jon: how a
   `BossPattern` step-sequence (Telegraph/Strike/Rest + a named special repertoire)
   maps onto `MoveSpec`** — it defines how every future boss is authored, a foundational
   fork like AD1/AD2. Attempting it blind risks a 127-ref refactor in the wrong shape.
2. **A1-3f render `BossAnim` → `CharacterAnim`** — a wholly separate boss animator stack
   (`BossAnim`/`BossAnimator`/`pick_boss_anim`, `animate_bosses`, GNU-ton's two-layer
   path) PLUS a genuine render→sim WRITE-BACK (`animate_bosses` inserts
   `BossAnimationFrameSample` consumed next frame by `sync_boss_strike_hitboxes`). E37
   already flagged this as needing the animator moved SIM-SIDE — a large, presentation-
   UNVERIFIABLE (blind) slice, not a T2 read-model gap.
3. **A7 brain migration** — making `WorldView`(+`WorldMemory`) the ONLY world-out: wire
   the surrounding-world channel (peers incl. the player, projectiles, portals — today
   empty slices) AND migrate every brain arm off the side-loaded `BrainSnapshot.target_pos`
   onto the view, add per-body `WorldMemory` storage + an update system, thread a
   `WorldView` through the boss/player/clone tick paths. The primitives are built +
   tested; the work is broad and BEHAVIOR-CHANGING (touches AI targeting for every
   brain — and E39 already shows the chase pipeline is determinism-fragile). Wiring
   peers/projectiles WITHOUT the brain migration is speculative scaffolding (no
   consumer), so this is one deliberate slice, not two. A12 (interaction/affordance
   NPC-agency) stays the documented deferral.

### E45. A1 boss fold — DESIGN adjudicated: NOT a boss-first port; PROVE the moveset on a real actor first (Path B)
Designed the `BossAttackState`→moveset fold before touching code (`docs/reviews/boss-moveset-fold-design.md`,
commit `8f0044b7`). **PUSHBACK (recorded, see the design doc for the full case):** the
moveset (`MoveSpec`/`MovePlayback`) is UNPROVEN SCAFFOLDING — no production `MovePlayback`
is ever created, `MoveEventMessage` has no consumer, and its static-offset hitbox model
is a DOWNGRADE from the boss's per-tick multi-part pose-tracking geometry. The boss attack
model, conversely, is already well-factored (capability/policy split, `Special(key)`→Technique
seam, separable phase machine); its ~126 `BossAttackState` refs are mostly legitimate
consumers of a clean projection, not duplication. A boss-first port would ADD a runtime
layer + keep `BossAttackState` as a projection = more code + a geometry downgrade, failing
the "convergence = less code" test.
**JON'S CALL: Path B — but with the full data-driven vision retained.** Prove the moveset
on a REAL actor first (not a throwaway), then extend + fold the boss. The proving ground
must be an ELEGANT real target that demonstrates smash-like expressivity data-driven — Jon:
give a NORMAL actor (the PCA, or the player-robot, or an item) a boss-like sophisticated
move (e.g. a tracking projectile), so "design smash-like characters with full expressivity,
data-driven" becomes natural — the Godot/Unity-for-2D-platformers vision
([[project_engine_for_other_games]]). Chosen Phase-0 target: **the PCA gains a data-driven
signature move via the moveset** — a normal actor with boss-like expressivity proves the
generalization (bosses ARE actors), additively (no touch to the player's proven combat).
Phased plan in the design doc §6.

### E46. Phase 0 LANDED — the moveset goes LIVE + the PCA carries a data-driven signature move ✅ (`0d236091`, `d0bb2935`)
Path B Phase 0 (prove the moveset on a real actor) is done + green.
- **0a — the moveset system goes live** (`0d236091`): the Smash-model runtime
  (`MoveSpec`/`MovePlayback`/`advance_move_playback`) was fully built + tested but
  DEAD in the shipping game (nothing ever inserted a `MovePlayback`, nothing consumed
  `MoveEventMessage`). Built the two missing production seams: `ActorMoveset(MovesetContract)`
  (the Bevy carrier of a body's repertoire); `trigger_moveset_moves` (a control-frame
  verb edge — `special`/`melee` — inserts the matching `MovePlayback`; `Without<MovePlayback>`
  gates re-trigger so the move's duration is the fire-rate floor); `dispatch_move_events`
  (`Sfx{cue}`→positioned sound; `Effect{key}`→BRIDGE to the SAME
  `ActorActionMessage::Special{Special(key)}` the brain special path emits, so a move
  fires a content technique with zero new plumbing — the exact seam the boss
  `Special(key)` profiles reuse in Phase 2). Registered in the combat schedule. Pins:
  a production-trigger→real-hitbox-damage test + the dispatch bridge test.
- **0b — the PCA authors a data-driven signature move** (`d0bb2935`):
  `CharacterArchetypeSpec` gains `signature_move: Option<MovesetContract>`; the PCA
  (`cellular_automaton_fighter`) authors "Cellular Pulse" (telegraph→Active hit
  volume→recovery, proper-time) entirely in `character_archetypes.ron`. The spawn plan
  attaches `ActorMoveset` from the archetype in `hostile` (covers every hostile path).
  A NORMAL actor now carries a boss-grade move AS DATA — the vision proof. Pin:
  `pca_fighter_authors_a_data_driven_signature_move`.

**PHASE 1 FORK (flagged for Jon before I pick a shape) — autonomous firing / the
brain↔moveset capability seam.** The PCA move fires today only from an injected/possessed
`special_pressed`; the autonomous Smash brain never fires special (its `SpecificAction::Special`
path is DEAD — `choose_action` never returns it). Making it fire cleanly touches how the
brain SEES its moveset capability: the brain gates actions on `&ActionSet`, but the move
lives in the separate `ActorMoveset`. Options: (a) derive `ActionSet.special = Special(move_id)`
from the moveset as the brain's capability signal + teach `choose_action` to fire it —
simplest, but `emit_brain_action_messages` then also emits a redundant (harmless, no
matching technique) `Special` message; (b) add a dedicated `ActionSet` capability flag the
brain reads (no redundant message, small schema add); (c) the deeper unification where the
moveset SUBSUMES `ActionSet.special`.
**JON CHOSE (c) — subsumption.** The moveset becomes the SOLE special executor; the flat
`ActionSet.special → ActorActionMessage::Special` resolution (`action_set/mod.rs:780`)
retires; the brain reads its move repertoire. **Blast radius (sized):** only THREE bodies
set `ActionSet.special` today — the player (`Special("bubble_shield")`,
`player/bundles.rs:238`), a catalog character (`Special("eye_beam")`,
`character_catalog/resolver.rs:320`), and (once wired) the PCA. Every one must gain a
MOVESET move for its key: a hitbox move (Cellular Pulse) OR an Effect-bridge move (a
`MoveEvent{Effect{key}}` that fires the existing content technique — bubble_shield /
eye_beam — through `dispatch_move_events`, zero new plumbing). **Plan:** (S1) source the
PCA's `ActionSet.special` from its moveset + teach the Smash brain `choose_action` to fire
special (dead today) on a range/cooldown gate [BLIND feel]; guard `emit_brain_action_messages`
to skip the flat emission when the body has a moveset for the verb → no redundant message,
moveset-bodies already off the flat path. (S2) migrate the player's bubble_shield + the
catalog eye_beam to Effect-bridge moveset moves [player = BLIND feel]. (S3) delete the flat
`ActionSet.special` resolution + retire the field to a capability marker. This is
feel-sensitive (touches the player's shield + AI cadence) — a focused slice, not the tail
of the §A-line run. Deferred here at a clean checkpoint (Phase 0 complete + green).
**PHASE 1 (geometry):** extend `HitVolume` for the boss's per-tick multi-part
frame-tracking strikes (the moveset's static-offset model is the one real downgrade).
**PHASE 2:** fold the boss (`BossPattern`→move-sequencer; `BossAttackState` as a
projection written from `MovePlayback`; retire `sync_boss_strike_hitboxes`).

### E47. SUBSUMPTION LANDED — the moveset is the sole special executor ✅ (`04ddb532`)
Jon chose subsumption; executed + green. Verified the flat `ActionSet.special →
ActionRequest::Special` arm broke NOTHING live before deleting it: bosses dispatch via
`dispatch_boss_special` (never the flat arm); the player's `Special("bubble_shield")` was
VESTIGIAL (the shield is `BodyShieldState`; the bubble render reads shield-held, not a
message); eye_beam-via-flat-arm was test-only. Landed: **retire** the flat arm in
`ActionSet::resolve` (`ActionSet.special` is now a pure capability marker); **source** the
PCA's `ActionSet.special` from its archetype `signature_move`; **Smash brain** fires its
(previously dead) `SpecificAction::Special` while its melee recharges → the PCA uses
Cellular Pulse through the moveset. Tests: the two `resolve` request-count pins now assert
special emits NO flat request + a new `engage_on_cooldown_fires_the_signature_special`.
Brain (96) + moveset/PCA + workspace `--all-targets` green. The AI cadence ships BLIND
(Jon tunes against the landed system, not before it). **Next: Phase 2 — fold the boss onto
the moveset** (the geometry frame-tracking downgrade is a feel detail Jon sweeps, so the
fold uses multi-volume static windows + keeps the per-frame path only where a strike needs
it).

### E48. Moveset expressivity COMPLETE + boss SPECIAL fold landed ✅ (`29a4d47a`, `ce1deac0`, `0bcd811c`)
Kept building (Jon: don't pause/checkpoint, land the arch — [[feedback_dont_pause_for_feel_land_architecture]]).
The moveset now has EVERY primitive smash-like expressivity needs, all headless-tested:
- **Multi-hit combos** (`29a4d47a`): a move with two Active windows lands two distinct
  hits; the `player_robot` archetype authors "Theorem Chain" (2-hit) — generalizes the
  moveset beyond the PCA (invariant I7, the protagonist's body as a data-driven fighter).
- **Held/sustained effects** (`ce1deac0`): `MoveWindow.sustain_effect: Option<String>`
  emits `Effect{key}` EVERY active frame — the primitive for a CONTINUOUS special (a
  lingering beam, a rain), the concrete blocker for folding the boss's `apple_rain`-style
  specials (which fire per-frame + time their own cadence).
- **BOSS SPECIAL FOLD** (`0bcd811c`): the boss's content-technique specials run through
  the SHARED moveset. `boss_special_moveset(cap)` generates a sustain-move per `Special(key)`;
  `trigger_boss_special_moves` starts it while that profile is `active_profile` (matched
  duration keeps the sustain in lockstep with the strike). Both the autonomous pattern AND
  possession route here (set `active_profile`); **`dispatch_boss_special` is DELETED**. The
  boss's special path == the actor's.
**BOSS FOLD — what remains** = the boss's GEOMETRY strikes (`sync_boss_strike_hitboxes`,
world-space + frame-tracking) + the `BossAttackState` pattern-cursor TIMING → a
move-sequencer with `BossAttackState` as a projection. This is the boss analogue of the
ACTOR-MELEE subsumption (actor melee is still `BodyMelee`/`spawn_melee_strike`, not the
moveset) — so the boss geometry staying off the moveset is CONSISTENT with the actor melee,
not a boss-only fork. Do them together as the **melee subsumption** (a large, feel-sensitive
slice touching every actor + the player + the boss geometry). The special fold is the clean,
complete boss-fold slice; the melee/geometry fold is the next big focused effort.

### E49. MELEE SUBSUMPTION — actor melee is now a data-driven moveset `"attack"` move ✅ (`2bc4bbae`)
The headline of the 2026-07-03 handoff. An actor's authored `ActionSet.melee` is folded
into its `ActorMoveset` as a `"attack"`-verb `MoveSpec` (`attack_move_from_melee`:
windup/active/recover → Startup/Active(one forward hit volume)/Recovery on proper time).
The swing triggers on `melee_pressed` through the SAME `trigger_moveset_moves` →
`advance_move_playback` runtime as the specials and lands through the real hitbox path —
so there is no longer a `BodyMelee` melee driver AND a moveset; **there is one executor.**
- The flat swing is retired for these bodies via a `MovesetMelee` marker:
  `start_body_melee` / `advance_body_melee` skip the swing logic (cooldown floors still
  tick → ranged fire-rate unaffected), and the `BodyMelee` read-model every consumer
  reads (actor anim index, telegraph/view index, HUD, the melee integration tests) is
  PROJECTED from the live `MovePlayback` (`project_moveset_melee_to_body_melee`) — the
  read-model keystone the handoff §3a named. All melee tests stayed green through the
  projection (`enemy_attacks_player`, `unified_melee`, `possession_end_to_end`,
  `player_robot_fights_player`).
- **What remains of the melee fold** = the PLAYER's melee (directional up/down/air
  variants + pogo + sprite-manifest hitbox). Unlike the actor swing, these are real
  mechanics the moveset does not yet express, so the player stays on the flat path and
  this is a RECORDED next slice (needs a directional-variant + pogo-behavior schema
  extension). Doing it is the feel-heaviest change left; it wants the moveset to (a)
  select a move by `attack_axis`, (b) carry a pogo-bounce behavior on a down volume, (c)
  optionally sample the manifest box. See the BULK REVIEW QUEUE for the deferred-tuning
  list the actor fold already shipped.
- The boss GEOMETRY fold (`sync_boss_strike_hitboxes` → move-sequencer + `BossAttackState`
  projection) is the boss analogue and stays a separate focused effort (E48 note above).

### E50. Melee subsumption COMPLETE for every non-boss actor + C9 ✅ (`ee23c074`, `f9843202`)
Continued straight on (Jon: don't pause — [[feedback_dont_pause_for_feel_land_architecture]]).
- **Uniform fold** (`ee23c074`): the peaceful-NPC spawn path folds melee → moveset too, so
  a possessed / provoked peaceful NPC's swing runs through the SAME moveset runtime. Every
  non-boss actor's melee is now a data-driven `"attack"` move; the flat `BodyMelee` swing
  survives only as the projected read-model + the player's (still-flat) directional/pogo
  path. **Bosses (geometry strikes) are the ONLY actor melee left off the moveset** — the
  boss-geometry fold is the clearly-scoped next big effort (E48 note; Path B Phase 1/2, now
  unblocked because the moveset is proven on actor melee).
- **C9** (`f9843202`): the `Shark` state-machine internals → `ChargeCrash` (behavior, not
  creature); the data-facing `CharacterBrainTemplate::ChargeCrash` was already done.
- Two remaining convergence items are RECORDED GENUINE FORKS (BULK REVIEW QUEUE): the
  player-melee fold (directional + pogo schema — pogo would pollute the content-free move
  runtime with player physics, so it wants a separate player-physics reader; Jon's call) and
  the ranged subsumption (dynamic-aim vs facing-lock). Full `cargo test --workspace`: **92
  suites green, 1 red = the pre-existing rl_sim determinism bug (§5, not this work).**

### E51. BOSS GEOMETRY FOLD — every boss strike now runs through the moveset ✅ (`7ecae45a`)
The handoff §3a headline. Specials + non-boss actor melee were already on the moveset
(E47–E50); this folds the LAST actor-melee piece — the boss's GEOMETRY strikes.
- `boss_special_moveset` → **`boss_attack_moveset`**: ONE move per authored strike
  profile. A geometry profile (FloorSlam / SideSweep / HazardColumn / GNU-ton hands / …)
  becomes a move whose Active window carries the profile's static hit volumes as
  BODY-LOCAL `HitVolume`s — derived from `volumes_for_profile` at a ZERO origin (the
  world-space math cancels the boss position, leaving a constant body-local offset, so a
  spawn-time-built move is position-correct). A `Special(key)` still becomes a
  sustain-`Effect` move. Keyed by the new `BossAttackProfile::move_id`.
- `trigger_boss_special_moves` → **`trigger_boss_attack_moves`**: ONE trigger for every
  boss strike (geometry AND special) — reads `active_profile`, inserts the matching
  `MovePlayback`; `advance_move_playback` spawns/despawns the Boss-faction strike hitbox
  through the shared `apply_hitbox_damage` path. Possessed-boss geometry stays suppressed
  (parity with the retired sync); its specials still fire.
- **DELETED `sync_boss_strike_hitboxes` + `FrameDrivenBossStrike` (~130 lines).** Boss
  strike damage no longer has a bespoke per-tick geometry poll. `active_attack_volumes` /
  `volumes_for_profile` survive for the DEBUG overlay + hurtbox-pose selection only.
- **What's the geometry cost?** The static `volumes_for_profile` fallback replaces the
  sprite-frame-tracked multi-part geometry for GAMEPLAY hitboxes (bulk-review: "Boss
  GEOMETRY strikes folded onto the moveset"). Blessed by the handoff as a parameterizable
  fidelity detail.
- **What's still off the full target shape?** `BossAttackState` still OWNS strike timing
  (the pattern cursor writes it; the move is slaved to `active_profile`). The full shape —
  pattern → pure move-SEQUENCER, `BossAttackState` → PROJECTION from the live
  `MovePlayback` (mirror of `project_moveset_melee_to_body_melee`) — is the recorded NEXT
  slice: it converts the Telegraph/Strike/Rest steps into Startup/Active/Recovery windows
  and migrates the ~37 `BossAttackState` consumers off the cursor. The load-bearing win
  (retire the bespoke poll, unify the damage path) is banked; the timing-authority flip is
  a follow-up. Full `cargo test --workspace`: all green except the pre-existing E39 red.

### E52. C7-render — split-layer boss render is now asset-convention, not a `gnu_ton` string match ✅ (`323c2107`, BLIND)
The render half of C7. `upgrade_boss_sprites`'s `is_gnu_ton` bespoke path (a behavior-id
check + 4 name spellings hardcoding `gnu_ton_body`/`gnu_ton_hands`) is replaced by a
generic CONVENTION: a boss whose art ships `{boss_key}_body` + `{boss_key}_hands` sheets
renders split across two layers (body behind one-way platforms, overlay in front of the
player). Any future giant gets the look by shipping the two sheets — no engine edit. The
GNU-ton-named components/systems/consts (`GnuTon*Layer`, `apply_gnu_ton_body_z`,
`sync_gnu_ton_hands`, `GNU_TON_*_Z`) are renamed generic (`Boss*Layer`,
`apply_boss_split_body_z`, `sync_boss_split_overlay`, `BOSS_SPLIT_*_Z`). GNU-ton is
unchanged (its sheets ARE those keys). Per-boss z overrides + a truly generic overlay
suffix (`_hands` is still the convention) are parameterizable details (bulk-review).
- **C7-residual REMAINING (blocked):** the mount/rider-name half — mount composition is
  still triggered by stripping the literal `" on Shark"` suffix from the spawn NAME
  (`rendering/world.rs`, `spawn_mounts.rs`). The full fix authors a `mount:` spawn field,
  which needs `ambition_ldtk_tools` (per [[feedback_ldtk_tools_only]]) — not autonomously
  unblocked.

### E53. Boss `BossAttackState` → PROJECTION — LANDED ✅ (`a3c69655`, `2dadea94`, `ba924163`)
The handoff §3a headline: flip the boss from `BossAttackState`-owns-timing to
`BossAttackState`-is-PROJECTED-from-the-live-`MovePlayback`. The load-bearing win (one
damage path) was banked in E51; this is the timing-authority flip. Re-verifying the code
surfaced the exact sub-cases + a clean design; recorded here so it lands in confident
green slices instead of one risky all-at-once commit.

**The enabling facts:** (1) a `Telegraph{p,tel}` step is ALWAYS immediately followed by
`Strike{p,strike}` for the SAME profile (pinned by
`gradient_sentinel_telegraph_steps_are_paired_with_matching_strike`), so a telegraph→strike
pair IS one move: window `[tel, tel+strike]` with the strike volumes, duration `tel+strike`.
(2) The projection recovers WHICH profile a live move is via `BossAttackProfile::from_move_id`
(landed `a3c69655`). (3) The move's own clock `t` reproduces `BossAttackState` EXACTLY:
telegraph_elapsed=`t`, telegraph_remaining=`tel-t` while `t<tel`; active_elapsed=`t`,
active_remaining=`tel+strike-t` while `tel≤t<tel+strike` — because the brain's
`active_elapsed` already folds in the preceding telegraph (`prev_tel+strike_elapsed`), which
equals `t`. Lockstep holds because both accumulate `dt` continuously off one playback.

**The possession trap (checked `boss_possession_specials.rs:155`):** possession asserts the
strike is `active_profile=Some(primary)` the SAME frame Attack is pressed — no telegraph
delay. A telegraph-spanning move started at `t=0` would be in its Startup, so
`active_profile` reads `None` → test fails. **Resolution: a `MovePlayback` START OFFSET
`t0`.** Scripted telegraph edge starts the move at `t0=0` (plays the telegraph); possession
(and any `tel=0` path) starts it at `t0=tel` (skips straight to the strike). The trigger
distinguishes cleanly: `telegraph_profile` set → `t0=0`; only `active_profile` set → `t0=tel`.

**Slice plan (each a GREEN, behavior-preserving-until-the-flip commit):**
- **Slice A ✅ `a3c69655`** — `BossAttackProfile::from_move_id` + round-trip test.
- **Slice B+C ✅** — behavior-preserving FOUNDATION: boss moves carry the telegraph
  offset (window `[tel, tel+strike]`, duration `tel+strike`), `MovePlayback::new_at(t0)`,
  `trigger_boss_attack_moves` starts at `t0=tel` (strike edge, UNCHANGED hitbox timing →
  damage + possession identical), and a new `project_boss_attack_state_from_move` re-derives
  the ACTIVE half of `BossAttackState` from the live move. Adds
  `BossPatternCfg::telegraph_windows()` + spawn wiring. Lands the projection machinery +
  telegraph-carrying moves; the brain-write stays as the seed. One sub-frame read-model wart
  deferred (see BULK REVIEW QUEUE) — damage is byte-identical, Slice D removes the wart.
- **Slice D ✅** — the fuller flip: scripted trigger at the telegraph edge (`t0=0`, playing
  the telegraph THROUGH the move so a bound anim clip can later slave to it), the projection
  now derives the TELEGRAPH half too (`telegraph_profile`/remaining/elapsed while `t<tel`),
  and `trigger_boss_attack_moves` ABORTS a move still in its windup when the pattern
  abandons the intent (phase change / suppress / rest / switched profile) so an interrupted
  telegraph never strikes — parity with the old strike-edge trigger. Possession keeps its
  instant strike via the `t0 = tel` branch (only `active_profile` set → skip the windup).
  Pinned by `telegraph_edge_trigger_projects_windup_then_strike` +
  `interrupted_windup_is_aborted_before_the_strike`. Full workspace test green except the
  pre-existing E39 red. REMAINDER (optional, NOT a trivial dead-write removal): making the
  projection the SOLE writer means something must CLEAR the component when no move plays.
  Today the brain's per-tick write does that (rest → `clear()`), and it is load-bearing for
  two no-move cases a blind clear-when-no-move would clobber: (a) a possessed boss's GEOMETRY
  strike (has `ActorMoveset` but its move is suppressed, so `active_profile` is set with NO
  `MovePlayback`), and (b) moveless test fixtures. So retirement needs a careful carve-out,
  not a one-liner. Left as-is: the two-writer form is behavior-correct (projection wins while
  a move plays; the brain write covers rest/fixtures/possession-geometry).

### E56. A7 BRAIN MIGRATION — every non-boss brain now TARGETS through the world-out port ✅
The behavior-shifting slice fable teed up: brains no longer read the omniscient
`BrainSnapshot.target_pos` (injected from `ActorTarget` by `select_actor_targets`) — a non-boss
actor's foe is now the nearest hostile IN its viewport (`WorldView.nearest_hostile`), or, when
none is visible, the most-confident foe it REMEMBERS (`WorldMemory.last_known_hostile`, pursuit
of one that left view). Migrated at the ONE seam every backend funnels through
(`tick_actor_brains` fills `snapshot.target_pos`/`target_alive` from perception), so Smash + all
state-machine arms (Patrol/MeleeBrute/Skirmisher/Sniper/ChargeCrash/Aerial) move together — the
Explore of the brain code confirmed they all read `target_pos` (directly or via
`to_character_ai_snapshot`/`target_delta_local`). Bosses keep the omniscient path (separate
`brain.tick`, §A1); the player brain ignores the target.
- **Two real gaps closed to make it faithful:**
  - **Grudge-aware perception.** `nearest_hostile` resolved hostility by `FactionRelations`
    ALONE — it would have MISSED a same-faction grudge-duel opponent (the two-NPC duel mechanism),
    the exact set `select_actor_targets` also honors. Added `PerceptionPeer.entity` +
    `PerceptionBody.grudge`; `build_world_view` now flags `hostile_to_self = relations.is_hostile
    || grudge == peer`. Pinned by `a_grudge_makes_a_same_faction_peer_hostile`. (Also collapsed
    `PerceptionPeers(Vec<(Entity, Peer)>)` → `Vec<Peer>` since the peer now carries its entity.)
  - **WorldMemory storage.** It was built + unit-tested but wired into NO sim body. Added a
    `PerceptionMemory(WorldMemory)` component (`ensure_perception_memory` attaches it to every
    non-boss brained actor — a component, so it dies with the body, no manual prune), updated each
    tick from the fresh view before the target is read.
- **Behavior-neutral where it matters:** with a foe IN view (the common case — all aggro radii
  80–320 < the 480px viewport, duel fighters spawn 150px apart), the perceived target EQUALS the
  omniscient one (verified frame-by-frame: `perceived == omni` every tick in the melee scenario).
  The migration only changes behavior at the viewport/memory edges (a foe >480px away and never
  seen is no longer omnisciently known → the actor idles until it comes into view; a foe that
  just left view is pursued from belief). The `perception_peers.is_some()` gate keeps the
  omniscient fallback for perception-less brain unit fixtures, so all 1121 gameplay_core tests
  are unchanged.
- **Canaries:** `duel_arena` (4), `actor_phase_split`, `possession_end_to_end`,
  `enemy_attacks_player`, `player_robot_fights_player`, all boss tests — GREEN. `unified_body_movement::
  home_body_and_actor_body_move_through_the_same_integration_phase` was loosened to its SPIRIT
  (the actor body integrates its intent through the shared phase — a MATERIAL horizontal
  displacement, gravity being vertical), dropping the leftward-SIGN over-specification: it was
  **already red at HEAD** (noted at the E53 entry as "pre-existing E39 red"; the duelist's neutral
  game nets a small reposition either way when the player charges INTO it). Now green.
- **⚠ PRE-EXISTING failure surfaced, NOT fixed (out of A7 scope, feel-reserved):**
  `unified_melee::a_hostile_actor_enters_the_same_body_melee_lifecycle` (`#![cfg(feature =
  "rl_sim")]`) fails at HEAD, independent of A7 (confirmed by stashing the A7 diff — identical
  failure). Root cause diagnosed: its `owns_a_strike` asserts a legacy `Hitbox` owned by the
  actor, but the PCA's melee is a moveset `"attack"` move (E49/E50) — and against a lone PASSIVE
  target the move ARMS the melee cooldown (so `engaged` via `cooldown>0`) but never enters its
  Active window, so NO `Hitbox` ever spawns (`any_hitbox==0` across 240 frames). This is a
  moveset-fold cadence/behavior gap in the feel-reserved duel-tuning territory (the BULK REVIEW
  QUEUE's "duel-fighter hop/lunge/blink re-weighting"), left for Jon's feel pass. The test also
  caches the enemy `Entity` once (a staging→live handle can go stale — the sibling
  `enemy_attacks_player`, identical scenario, re-queries by id and PASSES its swing assertion).

### E59. C4 app-thinness — 5 of 7 `sim_systems.rs` systems folded into owning gameplay_core plugins ✅
The app binary held real gameplay-sim logic in `ambition_app::app::sim_systems` (7 systems). Moved the
LOGIC down to its owning `ambition_gameplay_core` module; the host schedule
(`register_player_input_systems` / `register_presentation_sync_systems`) keeps owning the ordering +
`run_if` gates and now references the moved `pub fn`s by their library path.
- **Moved (render-free, app-only-free):** `apply_suspended_time_scale_system` →
  `gameplay_core::time::time_control` (+ its two suspended-time regression tests, now in
  `time_control::tests`); `sync_live_player_dev_edits_system` → `gameplay_core::dev` (beside the dev
  STATE it reads); `input_timer_system` + `interaction_input_system` + `cleanup_timers_system` → a new
  `gameplay_core::player::input_systems` (+ the interaction-suppression tests). Mechanical, behavior-
  preserving (`ambition_gameplay_core::` → `crate::` path rewrite only).
- **LEFT in the app (genuine host/reset concerns, cannot move):** `apply_player_reset_input_system` +
  `apply_cut_rope_room_replay_request_system` — both call the app-only `world_flow::reset_sandbox` AND
  write render `ambition_render::fx::VfxMessage`, and gameplay_core has NO render dep. The cut-rope one
  is NAMED content; moving it content-side needs the rooms world-hook seam (JD4, fable-reserved), so it
  stays for now. `sim_systems.rs` is now a documented 2-system host-bound file.
- **The `Res<Time>` gameplay lint** (`gameplay_systems_must_not_read_res_time_directly`, scans
  gameplay_core) now sees the moved timer systems; added `player/input_systems.rs` to its allowlist with
  the same ADR 0011 player-clock follow-up justification the app-tick path carries (the move preserves
  their raw-frame-dt behavior — presentation flash decays even while paused, by design; NOT a time-domain
  change snuck in).
- **Pinned** by a new `architecture_boundaries_input_timer_systems_moved_to_gameplay_core` (mirrors the
  touch-input guard): `sim_systems.rs` no longer DEFINES the 5 moved systems, `plugins.rs` references
  them via `ambition_gameplay_core::` paths, and the 2 host-bound ones DO stay defined in the app.
- **Green:** gameplay_core --lib 1128; `architecture_boundaries`, `plugin_minimal_app`,
  `possession_end_to_end` all pass; app compiles `--all-targets`. **REMAINING C4:** the
  `PlatformerEnginePlugin` group (collect the ~30 engine plugins) — sprawly, deferred as noted in the
  handoff (do only if mechanical).

### E60. §B frame-bug residue — VERIFIED complete in code; the one missing symmetry canary (B1) added ✅
The handoff re-listed B1/B3/B4/B5/B6 as work, but a fresh code read confirms **every one already
landed** (E2/E3/E5) with a §B comment marking the fix:
- **B1** ✅ `combat/moveset.rs:364-392` rotates the authored volume offset through
  `body_frame.to_world` (and `to_world_half` for the half-extent) at spawn — the unification with the
  player-melee path.
- **B3** ✅ `movement/blink.rs:35-49` (`complete_blink_clusters`) does `to_local` → damp `.x` / clamp
  `.y` → `to_world`.
- **B4** ✅ `movement/control.rs:139-140` — `vel -= frame.side * (facing * slash_recoil)`.
- **B5** ✅ `movement/collision.rs` — `body_is_side_contact` delegates to `body_is_nested_along(_, _, Axis::Y)`
  and `resolve_side_penetration(axis)` runs in axis-ROLE terms.
- **B6** ✅ `movement/integration.rs:142-195` — ONE branch parameterized by `(side_axis, gravity_axis)`
  from `gravity_on_x`: side-sweep → `apply_wall_abilities` → clear `on_ground` → gravity-sweep; no
  per-orientation ordering divergence.
- **B9** ✅ `movement/control.rs:31,72` fallback aim = `frame.side * ...`.
The engine-core `c4_reaction_seams` harness pins B3/B4/B9 + the full jump/land + wall-slide traces under
all four gravities. **The single gap:** B1 lives in gameplay_core (not the engine-core harness) and had
NO dedicated gravity-symmetry test. **Added `moveset_hitboxes_spawn_in_the_owner_gravity_frame`** — spawns
the same move under all four cardinal gravities and asserts the spawned `FollowOwner` hitbox's body-local
offset is gravity-INVARIANT (and the half-extent rotates), a real canary (an unrotated spawn trips it under
sideways/inverted gravity). **B2** (`surface_normal` stale-frame for non-surface-walkers) is the only §B
item still open; it overlaps the A3/A4 shield/knockback/muzzle consumers and lands with that work (item 3).
Green: moveset 11, c4 harness 6. No production change — §B was already done; this closes its test coverage.

### E61. §A3/§A4 victim-side damage — VERIFIED complete (incl B2); the B2 live-frame test gap closed ✅
The handoff re-listed A3/A4 as work, but a fresh code read confirms the whole victim-side damage stack is
already unified (E6/E7/E9/E11–E13) — the doc lagged the code:
- **A3** ✅ `combat/hitbox/mod.rs` `apply_hitbox_damage` is ONE `victims` query over every body with a
  published `CenteredAabb` + faction + the vuln trio (bosses too) — the three drifted victim loops collapsed.
- **A4** ✅ the world emitters iterate every vulnerable body, NOT just players: `combat/hazards.rs` has an
  `actor_victims` loop ("an NPC in lava takes the hit, a boss lured into spikes" — pinned by
  `a_non_player_body_touching_a_hazard_takes_the_hit_too`); `apply_actor_contact_damage` resolves player +
  `Without<PlayerEntity>` victims off the published `CenteredAabb`; `update_ecs_bosses` emits NO damage —
  strike damage flows through `apply_hitbox_damage`, body-contact through `apply_actor_contact_damage`.
- **A5** ✅ ONE `combat::damage::body_vulnerable()` at all 6 emit sites; **A6** ✅ the player publishes the
  same frame-oriented `CenteredAabb`, so every hurtbox read is the one gravity-oriented accessor.
- **B2** ✅ `features/enemies/integration.rs:156-168` keeps `surface_normal` LIVE (`= -gravity_dir` at the
  body's position) for every non-surface-walker, so the shield-block / slash-knockback / ranged-muzzle
  consumers (`actor_hit.rs:129`, `brain_effects.rs:~117`) read the CORRECT live frame. Surface-walkers keep
  the clung-surface normal. **Closed the test gap:** added
  `a_non_surface_walker_keeps_its_frame_normal_live_under_gravity` (drives the real integrator under all four
  cardinal gravities, asserts `surface_normal == -gravity`; a real canary — dropping the §B2 write trips it).
No production change to the damage paths — item 3 was already done; this banks the B2 regression canary.
Green: gameplay_core --lib 1130 (new B1 + B2 canaries).

### E62. C6 — boss sheet-specs are RON-authorable (content out of core) ✅
The hardcoded `pub const BossSheetSpec`s (`BOSS_SHEET`/`MOCKINGBIRD_SHEET`/`GNU_TON_SHEET`/…, with
`rows: &'static [(BossAnim, AnimRow)]`) are now serde-round-trippable owned specs, so a content boss
authors its sheet LAYOUT as data — the same "out of core" as E58's `StrikeRect` override.
- **Type surgery:** `BossAnim` + `AnimRow` + `BossSheetSpec` gained `serde` + `PartialEq`; `rows` →
  owned `Vec`; `BossSheetSpec` dropped `Copy` (now `Clone`). The 6 `pub const`s became `LazyLock`
  statics (the byte-identical built-in DEFAULTS). Consumers updated to clone/borrow (`BossAnimator::new`,
  `dedicated_boss_sheets`, `load_boss_sprite_in`, `sprite_render_size_for` via `&*`).
- **The seam (mirrors `BossProfileRegistry`):** a `BossSheetRegistry` (`HashMap<boss_key, BossSheetSpec>`)
  + `install_boss_sheets` (`OnceLock`, ADDITIVE — the engine ships its own demo sheets) +
  `boss_sheet_override(key)`. `dedicated_boss_sheets` / `load_boss_sprite_in` resolve the override first,
  else the built-in (E58's "empty default = built-in").
- **Content owns the data:** authored `ambition_content/assets/data/boss_sheets.ron` (all 8 built-in keys,
  byte-identical, GENERATED by serializing the built-in statics so no transcription drift) + wired
  `install_boss_sheets` into `install_boss_roster` alongside `install_boss_profiles`. Shipped bosses render
  UNCHANGED; re-authoring a sheet is a RON row edit, no Rust.
- **Pinned:** `boss_sheets_ron_matches_builtin_defaults` (the fixture-vs-const test — the content RON
  deserializes byte-identical to every built-in, incl. the `2.0/576.0` f32 round-trip) +
  `an_authored_sheet_overrides_the_built_in_layout`. Green: gameplay_core --lib 1132, content 53, render 24,
  boss_lifecycle 8.
- **NOT attempted (separate, larger — noted for fable):** the 11-variant `BossAttackProfile` enum collapse
  (72 refs across 8 files, also keys anim rows/overlays/behavior) stays; that's a distinct semantic change,
  not sheet-layout data-fication.

### E63. C1 — item catalog is an installable content override (JD2: BUILD IT as prep) ✅
The 24-item `Item` enum's baked flavor/wiring (`ITEM_META`: display_name / description / category /
held_item_id / dialog_id) is now content-authorable data, mirroring the C6 boss-sheet pattern exactly.
- **Type surgery:** `ItemMeta` fields → owned (`String`) + `serde` + `PartialEq` (+ `serde` on
  `ItemCategory`); `ITEM_META` const → a `LazyLock<[ItemMeta; 24]>` built-in default. The five accessors
  (`display_name`/`description`/`category`/`held_item_id`/`dialog_id`) keep their `&'static str` /
  `Option<&'static str>` signatures — they resolve through `item_meta(index)` and borrow from the
  process-global (so still `'static`). No caller changed.
- **The seam:** `ItemCatalog { rows: Vec<ItemMeta> }` + `install_item_catalog` (`OnceLock`) +
  `item_meta(index)` resolving the installed override row first (per grid slot), else the built-in default —
  the E58/C6 "empty default = built-in" pattern. ADDITIVE, so gameplay_core tests + a content-less build
  keep the built-ins with NO install-ordering panic (unlike the sole-source `BossProfileRegistry`).
- **Content owns the data:** authored `ambition_content/assets/data/items.ron` (all 24 rows in grid order,
  byte-identical, GENERATED by serializing the built-in table) + wired `install_item_catalog` into
  `AmbitionContentPlugin::build`. Shipped items read unchanged; re-authoring is a RON row edit, no Rust.
- **Pinned:** `items_ron_matches_builtin_defaults` (fixture-vs-const, all 24 rows) +
  `an_authored_item_row_overrides_the_built_in_slot`. Green: gameplay_core --lib 1134, content 53, app
  compiles. **The `Item` ENUM stays** structural KIT (OoT 24-grid, save-index keyed) — relocating the item
  SET itself is a bigger change (saves/menu/OwnedItems key off the index); the C2 held-item registry + C5
  projectile-spec chain are the natural follow-ons (JD2 "incremental is fine").

### E64. C7 rider-name half — NOTED-AND-SKIPPED: the premise is stale + it hides a mount-authoring FORK ⏸ (for fable)
Verified against code: **mount COMPOSITION is already brain-driven, NOT name-parsed.** An `EnemySpawn`
carries `brain: "pirate_on_shark"` / `"pirate_heavy_on_shark"` (a FUSED rider+mount archetype whose
`composite_visual` fans it into mount + rider) — the engine already de-hardcoded `" on Shark"` from the
composition path (the earlier §C7 slice). The ONLY residual name-parse is the rider DISPLAY-name strip
(`composite_rider_name(spawn_name, rider_name_suffix=" on Shark", fallback)` in
`features/enemies/mod.rs:109`, called by both the sim `spawn_mounts.rs` and render `world.rs:622`).
- **The handoff's `mount:` field IS a design fork.** Building a `mount: "shark"` spawn FIELD that DRIVES
  composition means splitting the two fused `*_on_shark` archetypes into a plain rider brain
  (`pirate` / `pirate_heavy`) + a mount reference, and teaching the LDtk loader to compose
  `[rider brain] + [mount archetype]` from the field. That decision — **is the mount authored as a field,
  or does the brain-key encoding stay?** — is the mount-authoring model, which fable should own (it also
  decides whether the 7 shipped spawns' `mount` value keys an archetype id vs a mount-registry id).
- **Why not just land the display-name half:** re-authoring the 7 spawn names (6 `pirate_on_shark` +
  1 `pirate_heavy_on_shark`, across `sandbox.ldtk`) to drop `" on Shark"` + deleting `rider_name_suffix`
  is unambiguous, BUT it is ENTANGLED with the field decision (the same re-author touches the same 7
  spawns fable's `mount:` field would), it's a purely cosmetic display-name change with ONE visible
  behavior (the rider nameplate) that can't be headless-verified, and doing it now would pre-empt +
  likely re-do fable's mount-field authoring choice. So it waits with the fork.
- **The ldtk-tools capability already EXISTS** (`def update-entity` to add the field def, `entity set-field`
  to set per-spawn values) — building the seam is loader + archetype work, not a missing subcommand.
- **Concrete plan once fable picks the model:** (1) `def update-entity EnemySpawn` → add `mount: String`
  (optional); (2) loader: `mount` set → compose rider-brain + mount-archetype (retire the fused
  `*_on_shark` brains, or keep them as constructors); (3) `entity set-field` the 7 spawns: `name` → the
  rider's real name, `brain` → plain rider, `mount` → "shark"; (4) delete `rider_name_suffix` +
  `composite_rider_name`'s strip (rider name = spawn name); (5) `roundtrip` + `validate` the .ldtk.

### E65. A1 slice — the boss INTENT/projection split LANDED (JD3 prerequisite) ✅
JD3's named nuance: retiring the `BossAttackState` brain-WRITE is NOT a dead-write removal because
`trigger_boss_attack_moves` READS it as its intent signal. Did the split first (the sanctioned prerequisite):
- **New `BossAttackIntent` component** (`ambition_characters::brain`, `{telegraph_profile, active_profile}`) —
  the per-frame fire INTENT the driver writes and the trigger reads. Added at both boss spawn sites
  (`spawn_actors.rs`, `encounter_script.rs`).
- **`tick_boss_brains_system`** now publishes the intent via a `mirror_intent(&attack_state, &mut intent)`
  helper at each arm exit (dead / possession / non-pattern / BossPattern) — an EXACT mirror of the profiles
  it writes to `BossAttackState`, so the seam is behavior-identical. **`trigger_boss_attack_moves`** reads
  `BossAttackIntent` instead of `BossAttackState`.
- **Why behavior-identical:** the schedule runs tick_boss_brains → trigger → advance_move → projection, so
  the trigger always read the brain/possession-written `BossAttackState` BEFORE the projection overwrote it;
  the intent mirror captures exactly that value. Trigger + possession + telegraph/interrupt tests re-pointed
  at the intent; all green.
- **What this unblocks (NEXT slice, has a BLIND part):** `BossAttackState` can now become a pure PROJECTION
  of the live `MovePlayback` — retire the brain-tick's `*attack_state = bps.attack_state.clone()` write and
  make `project_boss_attack_state_from_move` the SOLE writer (clearing when no move plays). Two carve-outs to
  handle then: (a) a possessed boss's GEOMETRY strike (intent set, move suppressed → no projection source; its
  strike POSE would clear — a BLIND presentation change, Jon feel-checks) and (b) the projection query must
  iterate move-less bosses to clear them (add `Option<&MovePlayback>`). This is the convergence (LESS code);
  slice 1a is the safe seam it rides on.
- Green: gameplay_core --lib 1134; boss suites `boss_lifecycle` (8), `boss_contact_iframes` (4),
  `boss_possession_specials` (1), `boss_motion_parity` (2, rl_sim).
- **A1 REMAINING after this:** the projection-sole-authority slice (above); the full driver fold
  (`update_ecs_bosses` — now a thin presentation system — + `tick_boss_brains` into `tick_actor_brains` /
  `integrate_sim_bodies` with boss as an actor archetype); `BossAnim`→`CharacterAnim` (BLIND, retires the
  `animate_bosses` render→sim write-back). This is the multi-session tail the handoff flagged; the 171-ref
  `BossAttackState` surface + 4 boss suites make each a its-own-verified-slice job.

### E66. A1 slice 1b — `BossAttackState` is now a PURE PROJECTION (the brain-write RETIRED) ✅
The convergence E65 unblocked. `project_boss_attack_state_from_move` is now the SOLE writer of the boss
telegraph/strike read-model; `tick_boss_brains_system` no longer borrows or writes `BossAttackState` at all —
it publishes ONLY the `BossAttackIntent`, and the projection derives the read-model from whatever move that
intent starts (and CLEARS it when no move plays). One path, less code.
- **Projection is sole writer:** its query became `Option<&MovePlayback>`; no move → `attack_state.clear()`
  (a resting boss, a no-moveset fixture, or a suppressed possessed-geometry strike); the windup/strike
  branches are unchanged; the spent-tail (t ≥ end) now also clears. `tick_boss_brains_system` drops
  `&mut BossAttackState` from its query; the BossPattern arm mirrors intent straight from the brain's internal
  `BossPatternState.attack_state`, the possession arm writes intent directly from the input mapping.
- **Why the DAMAGE path is behavior-identical** (verified against the schedule, not the doc): the boss HURTBOX
  publisher `refresh_boss_damageable_volumes` reads `BossAttackState` at features/mod.rs:222 — BEFORE both the
  brain tick and the projection — so it consumes the END-OF-PREVIOUS-FRAME value in BOTH old and new code
  (projection was already the frame's last writer during any attack). `apply_feature_hit_events` reads it live
  AFTER the projection (combat_schedule.rs:172 > :158). For an IDLE boss the brain wrote a CLEARED state and
  the old projection LEFT it → the new projection CLEARS it → identical. Every attacking boss carries an
  `ActorMoveset` (geometry AND special profiles both build an Active-window move, bosses.rs:209), so the
  projection always has a move to derive an active strike from.
- **BLIND (Jon feel-checks)** — carve-out (a) from E65: a possessed boss's GEOMETRY strike is suppressed by
  `trigger_boss_attack_moves` (no move → the retired `sync_boss_strike_hitboxes` never struck for a
  player-controlled boss), so with the projection sole-writing it no longer shows a strike POSE (its damage was
  already suppressed). `boss_possession_specials` updated: the possessed geometry-primary Attack now asserts
  `active_profile == None` (was `Some(HandSlam)`); the SPECIAL path (apple_rain — runs through the moveset)
  still projects its pose AND fires the technique. Restoring the geometry strike as a REAL strike, routed
  through the moveset with the possessor's EFFECTIVE faction (so it hits the boss's former allies, not its
  possessor), is the effective-faction follow-up.
- Green: gameplay_core boss module 33; app suites `boss_lifecycle` 8, `boss_contact_iframes` 4,
  `boss_possession_specials` 1, `boss_motion_parity` 2 (rl_sim), `possession_end_to_end` 3 (rl_sim).
- **A1 REMAINING after this — the multi-session tail, each piece with a NAMED blocker (surveyed this run):**
  1. **integrate fold** (`integrate_boss_bodies` → `integrate_sim_bodies`): NOT a mechanical fold — it hides a
     design nuance. `integrate_actor_body` publishes `CenteredAabb` from `em.kin.size` (the COLLISION box);
     `integrate_boss_bodies` publishes it from `render_size` (the boss's GROSS RENDER/composite envelope —
     GNU-ton's coarse AABB that `refresh_boss_damageable_volumes` treats as the whole-creature bound). Boss
     also passes a deliberate `(0,0)` stagger gate and self-heals `kin.size` each frame. Folding the boss
     THROUGH the shared integrator needs either an adapter flag (a special-case branch the codebase avoids per
     [[feedback_reorganize_not_adapt]]) OR splitting the render-envelope from the collision footprint into its
     own component (the elegant answer, but it touches the renderer + `damageable_volumes`). A DECISION, tee it
     up — don't bulldoze. (A cheap intermediate: add a third sibling boss-arm query to `integrate_sim_bodies`
     so all body movement is ONE scheduled phase, boss arm unchanged — low value, mild.)
  2. **brain fold** (`tick_boss_brains` → `tick_actor_brains`): `tick_actor_brains` is at Bevy's 16-param
     ceiling and explicitly `Without<BossConfig>` to avoid a double brain-tick; the boss snapshot carries
     boss-only fields (encounter phase, front-wall clearance, possession→`BossCapability` mapping). Needs the
     snapshot to absorb those first.
  3. **`BossAnim`→`CharacterAnim`** render rows (BLIND, retires the `animate_bosses` render→sim write-back).
  The 171-ref `BossAttackState` surface + 4 boss suites keep each an its-own-verified-slice job.

## Next (in order) — **the MOVESET UNIFICATION is COMPLETE (E47–E55): melee, specials, ranged, AND boss strikes all run through the ONE moveset runtime.** The audit's TASK sections are stale; trust E-entries + a code re-check before working an item.

---

### ⇢ AUTONOMOUS-EXECUTION RUN RESULT (2026-07-04, HANDOFF list items 1–7) — read this first

The 7-item autonomous handoff (`HANDOFF-2026-07-04-autonomous-execution.md`) ran to completion.
Outcome per item (details in E59–E65):
- **C4 (app-thinness)** ✅ LANDED (E59) — 5 of 7 `sim_systems.rs` systems folded into owning gameplay_core
  plugins; the 2 host/reset-bound stay (render dep); boundary test pins it.
- **§B (gravity frame-bugs)** ✅ VERIFIED COMPLETE + canary (E60) — the doc lagged: B1/B3/B4/B5/B6/B9 all
  already landed; added the missing B1 moveset-hitbox gravity-symmetry test.
- **A3/A4 (victim damage)** ✅ VERIFIED COMPLETE + canary (E61) — A3/A4/A5/A6 + **B2** all already landed
  (one relational victim loop; NPC-in-lava; live `surface_normal`); added the B2 live-frame gravity test.
- **C6 (boss sheet-specs → RON)** ✅ LANDED (E62) — serde-owned `BossSheetSpec` + `BossSheetRegistry` +
  content `boss_sheets.ron`, byte-identical; the 11-variant `BossAttackProfile` collapse noted separate.
- **C1 (item catalog)** ✅ LANDED (E63) — serde-owned `ItemMeta` + `ItemCatalog` override + content
  `items.ron`, byte-identical; the `Item` ENUM stays structural KIT.
- **C7 (rider-name half)** ⏸ NOTED-AND-SKIPPED (E64) — hides a design FORK for **fable**: composition is
  already brain-driven (the "name-parsing drives composition" premise is stale); the `mount:` field means
  restructuring the fused `*_on_shark` archetypes → a mount-authoring-model decision. Concrete plan in E64.
- **A1 (boss driver fold, JD3)** ▸ ADVANCED, slices 1a+1b LANDED (E65, E66) — the **intent/projection split**
  then **projection-sole-authority**: `BossAttackState` is now a PURE PROJECTION of the live `MovePlayback`;
  the brain-write is retired (one path). One BLIND part shipped: a possessed geometry strike loses its pose
  (E66). **REMAINING (the multi-session tail):** the driver fold (`update_ecs_bosses`+`tick_boss_brains` → the
  actor systems), `BossAnim`→`CharacterAnim` (BLIND). See E66 for the precise next slices.

**Verification (this run):** engine_core 212, gameplay_core 1134, characters 253, content 53, render 24;
app suites `boss_lifecycle` 8, `boss_contact_iframes` 4, `boss_possession_specials` 1, `boss_motion_parity`
2 (rl_sim), `possession_end_to_end` 3, `gravity_symmetry_room` 5, `duel_arena` 4, `enemy_attacks_player` 1,
`player_robot_fights_player` 1, `plugin_minimal_app` 8, `architecture_boundaries` 32 — all GREEN. Only the
**known pre-existing** `unified_melee::a_hostile_actor_enters_the_same_body_melee_lifecycle` (rl_sim) is red
(confirmed unchanged, NOT from this run). **No BLIND commits this run** — every landed slice is
behavior-identical or verified; the BLIND parts (A1 1b + BossAnim) are the documented remainder.

### ⇢ STATE FOR THE NEXT AGENT (2026-07-04) — read this first

**Is the doc ready for a next agent?** YES for the autonomous work below. **Is everything
down to Jon's decisions?** NO — autonomous levers remain (C4/C6/C1). But the genuinely
shape-defining forks ARE Jon's, and they're called out here so nothing silently stalls.

**A7 BRAIN MIGRATION — DONE ✅ (E56).** Every non-boss brain now targets through the world-out
port (`WorldView.nearest_hostile` + `WorldMemory` pursuit), off the omniscient `ActorTarget`.
Grudge-aware perception + `PerceptionMemory` component landed to make it faithful;
behavior-neutral where a foe is in view (verified `perceived == omni`). See E56 for the full
account, including a **pre-existing** `rl_sim` red (`unified_melee::a_hostile_actor`) surfaced +
diagnosed but left for Jon's feel pass (moveset-fold cadence gap, NOT A7).

**Remaining autonomous items:** C4 (app thinness — sim_systems fold in progress), C6 remainder
(boss sheet-specs → RON; the 11-variant enum collapse), C1 (item catalog — now GREEN-LIT as prep,
see JD2), A1 boss driver fold (JD3), A2–A5 damage unification, B frame-bug residue.

**⚠ JON DECISIONS — UPDATED 2026-07-04 (see the JON'S DESIGN DIRECTION section, JD1–JD4):**
1. **Player-melee fold — REFRAMED (JD1).** NOT an A/B/C schema pick. Jon's direction: abilities =
   parameterized-prefab effects (DATA) + arbitrary content code (a Bevy Technique system keyed by
   `Effect{key}`), with params passed from the published character data and input→move mappings in
   that data too. The pogo-pollution worry dissolves (pogo = a Technique). **fable to spec** the
   params value type, the dispatch shape (message vs component/observer), and the published-
   character-data schema. Discussion ongoing — not a final spec.
2. **D-front `rooms`/`RoomSpec` (JD4) — ADJUDICATED.** Types stay core KIT; move the `.ldtk`
   payloads + world list to content + build a world-registration seam (LDtk is the permanent
   level path, legacy RON verified gone); per-room mechanics SPLIT BY KIND (data / self-gating
   plugin / id-hook). fable to size the seam, then autonomous to execute.
3. **C1 item catalog — RESOLVED: BUILD IT (JD2).** Jon overrules the defer — it's architecture
   prep (proven roster pattern, low risk); incremental is fine. Autonomous.
4. **A1 boss driver fold — RESOLVED: FINISH IT (JD3).** Shape settled (Path B); nuanced finish
   (the `BossAttackState` brain-write needs an intent-component split, not a dead-write removal).
   Autonomous (render animator half is BLIND).
5. **DEFERRED-TUNING sweep** (not blocking, but yours) — the whole BULK REVIEW QUEUE at the top
   is feel/value tuning you asked to defer: boss/ranged/melee cadence, the duel-fighter
   hop/lunge/blink re-weighting, fire-move windups, the E53 sub-frame read-model wart. All
   headless-landed with sensible defaults; sweep when you do the feel pass.

---

**DONE 2026-07-03/04 (this + the prior run):** **E49/E50** actor melee → moveset `"attack"` move (every non-boss actor); **C9** `Shark` → `ChargeCrash`; **E51** BOSS GEOMETRY FOLD (`7ecae45a`); **E52** C7-render (`323c2107`); **E53** BOSS `BossAttackState` → PROJECTION (`a3c69655`/`2dadea94`/`ba924163`); **E54** RANGED SUBSUMPTION (`536d5ac1`/`9075e8b7` — the last flat combat path); **E55/E55b** A7 peers + projectiles perception channels wired (`0a9293b5` + follow-up); **E56** A7 BRAIN MIGRATION — every non-boss brain targets through the world-out port (grudge-aware perception + `PerceptionMemory`), off the omniscient `ActorTarget`; **E57** C4 mobile/touch input EXTRACTED to the `ambition_touch_input` sibling crate (`414a1e58`); **E58** C6 boss strike geometry data-fied into a `StrikeRect` table + RON-authorable override (`fe41aee9`/`7d646188`).

**RECORDED GENUINE FORKS:** ranged subsumption — RESOLVED + IMPLEMENTED (E54, option A: a
content-free `MoveEventKind::Ranged` that samples live aim; enemy/NPC/boss ranged is now a
moveset `"fire"` move — the LAST flat combat path folded, so melee+specials+ranged+boss all
run through the ONE moveset runtime). STILL OPEN: player-melee fold (directional-variant +
pogo schema — Jon's call).

**Verified stale/already-done or intentionally deferred:** C8 (`SpecialPreset` already carries the open `Special(String)` hatch); C6-mockingbird reward-table (deferred in-code — "one example isn't a pattern"); C2 (`HELD_ITEMS` — a bare install seam is speculative scaffolding; defer until a second game / per-character loadout).

**Genuinely-open, autonomous-friendly remaining (biggest-lever first):**
- **Boss `BossAttackState` → projection — LANDED ✅ (E53 A–D, `a3c69655`/`2dadea94`/`ba924163`).**
  The pattern's Telegraph/Strike windows are folded onto ONE move per profile spanning
  `[tel, tel+strike]`; `BossAttackState` is now DERIVED from the live `MovePlayback`
  (`project_boss_attack_state_from_move`, both halves), triggered at the telegraph edge with
  a windup-abort for interruptions, possession preserved via a `t0=tel` skip. **No autonomous
  remainder** — the earlier "retire the brain component write" idea is a FALSE lead (the
  trigger READS that component for its intent signal, so retiring it needs a full intent-
  component split, not a dead-write removal; the two-writer form is behavior-correct — see the
  E53 E-log entry). The one deferred nuance is a sub-frame read-model wart inherent to
  cursor→move time (BULK REVIEW QUEUE; damage is byte-identical).
- **A7 perception** (L) — **MIGRATION DONE ✅ (E56).** Channels wired (E55/E55b) AND every
  non-boss brain now TARGETS through `WorldView.nearest_hostile` (+ `WorldMemory` pursuit), off
  the omniscient `ActorTarget`. Grudge-aware perception + a `PerceptionMemory` component landed
  to make it faithful. See the E56 E-log entry. **REMAINING (small, autonomous):** (1) the boss
  (`§A1`) + player brains still read the omniscient target — fold when A1 lands / if a player-AI
  needs it; only THEN can `BrainSnapshot.target_pos` be deleted (bosses/fixtures still fill it).
  (2) Per-body viewport override — a duelist authored with arena-wide aggro (1100 > the 480px
  default viewport) is now perception-limited to 480px + 3s memory; add a `viewport_half` knob if
  a character wants keener senses (documented future in `DEFAULT_VIEWPORT_HALF`). (3) Portals
  channel is still `&[]` — wire like peers when an S5 routing brain needs it. (4) The
  disposition/aggro vocabulary that seeds hostility is still frame-tainted
  (`disposition.is_hostile()`) — AD1's follow-up smell, reshape onto the fighting-state model.
- **C1** (L) — 24-item `Item` enum → installable `ItemCatalog` (consumed across menu IR / yarn / persistence). **⚠ see JON DECISIONS #3 above — may be premature (C2-class scaffolding).**
- **C4** (L) — app-thinness. **PROGRESS (E57): `host/mobile_input/` (2.9k LOC) EXTRACTED ✅** to
  the sibling `ambition_touch_input` crate (the touch adapter is reusable engine input, not host
  glue — the "second game" oracle). Clean move (the module had ZERO app-only `use crate::` refs);
  the app now WIRES `ambition_touch_input::TouchControlsPlugin` and forwards its `mobile_touch`
  feature. Guarded by `architecture_boundaries_touch_input_crate_is_extracted`. **REMAINING
  (each its own slice, ranked by tractability):** (1) fold `app/sim_systems.rs` (579 LOC, 7
  systems) into owning library plugins — ENTANGLED (interleaved into the app-owned
  `SandboxSet::PlayerInput` `.chain()` with `run_if` gates; two systems call the app-only
  `world_flow::reset_sandbox`, which must move down first; one is NAMED content = cut-rope boss →
  `ambition_content`). (2) A `PlatformerEnginePlugin` group collecting the ~30 engine plugins
  hand-added in `plugins.rs` — sprawling design task. Neither is a clean one-pass; the review's
  "boundary test" piece is subsumed by the guard test just added.
- **C6** (M) — named-boss geometry. **PROGRESS (E58): the strike geometry is DATA now ✅.**
  `volumes_for_profile`'s 11 hardcoded magic-number `match` arms became a declarative,
  `serde`-ready `StrikeRect` table (`{offset_factor·size + offset_const, half_factor·size +
  half_const}`) resolved by ONE generic `to_aabb`; `strike_geometry(profile) -> &[StrikeRect]` is
  the single source both the gameplay path (`boss_attack_moveset` → `HitVolume`s) and the
  debug/pose fallback read. Byte-identical (pinned by
  `strike_geometry_is_byte_identical_to_the_old_hardcoded_match` across origins/sizes).
  **"OUT OF CORE" CAPSTONE DONE ✅ (E58):** `BossBehaviorProfile` (RON) now carries a
  `strike_geometry: HashMap<move_id, Vec<StrikeRect>>` OVERRIDE — a boss authors its OWN strike
  rects in `boss_profiles.ron` and they REPLACE the built-in table for that move, NO edit to core.
  `volumes_for_profile` consults it (feeding BOTH the gameplay `HitVolume`s via `boss_attack_moveset`
  AND the debug/pose path from its single source); empty default = built-in, unchanged. Pinned by
  `an_authored_override_replaces_the_built_in_geometry_for_that_move`. **REMAINING (larger,
  separate):** the 11 named variants stay — they also key anim rows / overlays / behavior (72 refs
  across 8 files), so collapsing the ENUM itself is a distinct semantic change, not part of the
  geometry data-fication.
- **C7 rider-name half** — BLOCKED on `ambition_ldtk_tools` (mount composition still parses `" on Shark"` from the spawn NAME; the fix authors a `mount:` spawn field).

**D-front** (`rooms`/`RoomSpec` content-coupling) — Jon's call, unchanged below.

### Superseded (the prior D-focused Next; still accurate for the D-front) — **T2 clean read-model + D3 facade redirects DONE (E36/E37/E38).** D3's remaining reducers are all non-autonomous (need Jon's design input or are risky/unverifiable): the `rooms` extraction crux (RoomSpec content-coupling — Jon's call), the value-type→`ambition_sim_view` move (premature until the edge narrows), the boss-pose SIM-SIDE animator move (retires the `animate_bosses` write-back; presentation-unverifiable), and the category-D portal/dev/session system untangles. Recommend Jon adjudicate the `rooms`/`RoomSpec` content-coupling direction next (as he did the actors|props taxonomy). Deferred to Jon's feel pass: render/hurtbox baked-size convergence (~1.2% gap); the T1 placeholder color/z blind deltas (E35).

**§A2 is COMPLETE** (E10–E13). The victim-side damage path is ONE resolver +
ONE reaction for every body; per-body policy is the only fork left.

*POLICY (stays in each consumer around the resolver — landed this way in E11):*
- Player: difficulty/assist multiplier, `HitMode::SafeRespawn`, death →
  `death_respawn_player`, safe-position memory, banner text.
- Actor: peaceful-branch (strikes/barks/provoke stimulus — NOT damage), death
  → drops/banner/respawn-timer/split/explode, cling-detach pop.
- Boss: untouched until A1.

**A1 — boss island dissolution** (slices 1 + 2a DONE — E14, E15; slice 3
remains; slice-2b folded into slice 3):

*Slice 2a — boss damage through the resolver — DONE (E15).*

*Slice 2b (folded into slice 3) — give bosses `BodyOffense`/`BodyDodgeState`/
`BodyShieldState` (default-inert) so `apply_hitbox_damage`'s victim tuple drops
its `Option`, and grep `§A1` + `Without<BossConfig>` victim carve-outs.* Deferred
because it's behavior-neutral cleanliness whose only payoff arrives WITH the
boss→actor conversion (see E15 note on the dead `HitTarget::Actor(boss)` route),
and adding components to bosses needs the query-aliasing audit slice 3 does
anyway. Audit already started: no standalone `Query<&mut BodyOffense/…>` exists
(only the composite `BodyClusterQueryData` views + the movement-pipeline fn
params), so adding the clusters won't newly-alias a mutable query — but confirm
no `ActorFaction`-carrying non-body (enemy projectile?) would be dropped from the
victims query when the tuple goes non-`Option`.

*Slice 3 — driver fold (the big one).* `BossAttackState` → `BodyMelee`/moveset;
`update_ecs_bosses` + `tick_boss_brains` fold into `tick_actor_brains` +
`integrate_sim_bodies` with the boss as an actor archetype (capability mask +
`BossPattern` brain via the existing `Brain::StateMachine` seam; floating =
`fly_enabled` body, the flight limb replaces `step_floating_body`). Boss
possession's bespoke input→special mapping then dies. Render: `BossAnim` →
`CharacterAnim` rows. This slice is where `BossStatus` (by then only
encounter_phase/sprite_metrics/encounter) renames to a boss-encounter
component and `BossConfig` becomes pure archetype data.

## Notes for a resuming agent
- The C4 harness is the safety net — extend it per fix; a scenario that fails
  only on rotated arms is a frame bug, not a rig bug.
- Engine-core movement input (`InputState.axis_*`) is ALREADY body-local;
  `blink_quick_dir`/`blink_aim_step` are world-space (resolved at the input
  bridge). Don't re-resolve.
- Blink PREVIEW divergence found (not yet fixed): `ambition_render/src/fx.rs:897`
  and `ambition_app/src/dev/debug_overlay/gizmos.rs:477` build quick-blink aim
  from RAW device axes + world-X facing fallback instead of the resolved
  `blink_quick_dir` — the preview can disagree with the actual blink under
  rotated gravity / non-default frame modes. Log/fix when touching those files.
- `movement/tests/wall_collision.rs` has a pre-existing `unused_mut` warning
  (line ~162) — not from this work, left alone.
