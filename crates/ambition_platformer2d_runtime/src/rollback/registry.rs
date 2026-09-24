//! Backend-neutral rollback schema metadata.
//!
//! Gameplay domains declare typed rollback obligations through
//! `ambition_platformer2d_core::snapshot::RollbackRegistrar`. This module records the exact
//! managed schema those declarations describe; concrete rollback hosts install storage/checksum
//! machinery separately.

use std::collections::BTreeMap;
use std::fmt;

use bevy::prelude::*;

use crate::content_identity::SnapshotSchemaFingerprint;

/// Managed same-build version of the rollback schema contract.
///
/// Bump when the registered state set, wire type identity, encoded payload, or
/// checksum projection changes incompatibly. Peers with different versions must
/// not treat their snapshots as compatible.
/// ⛔ v131: `actor.move_playback` gained its AIM LATCH — the direction the
/// player asked for while the move was coming out. An aimed teleport resolves
/// its destination from it, so two peers that disagree about it put the same
/// fighter in two places: one on the ledge, one under it.
/// ⛔ v130: the same projection gained `ChargeSustain` — whether the freeze is
/// held by the button or by the move. Two peers agreeing on the elapsed hold and
/// disagreeing about what ends it resume the move on different ticks.
/// ⛔ v129: `actor.move_playback`'s charge projection gained the policy's two
/// BOOLEANS. `stores` was never encoded and `roots` is new; both decide what a
/// hold buys — whether reaching maximum fires, and whether the body may steer
/// while it holds — so two peers agreeing on the elapsed hold could disagree
/// about the move it produces.
/// ⛔ v127: `BodyMode` gained `Submerged = 6`. The bytes an OLDER peer writes are
/// unchanged — every existing mode keeps its discriminant, which is why the
/// variant went on the end — but a peer on 126 cannot decode a `6`, and a
/// fighter under the stage is exactly the state a desync would hide: invisible
/// and intangible on one machine, standing in the open on the other.
/// ⛔ v132: `resource.sandbox_save` and `resource.quest_registry` gained CHECKSUM
/// PROJECTIONS. The snapshot bytes are unchanged — both are still clone
/// snapshots — but a peer that checksums them and one that does not compute
/// different checksums over identical state, so the two cannot agree. They were
/// registered with `rollback_resource_clone`, whose probe is PRESENCE-ONLY: a
/// rewind that lost a visited-room flag or a `RoomEntered` push moved no
/// checksum at all, which is exactly the divergence the sync test exists to
/// report, and exactly what the systems pairing a non-rewinding `Local`
/// edge-detector with these resources would produce.
/// ⛔ v133: `resource.switch_activation_queue` gained a CHECKSUM PROJECTION, for
/// the reason its own type doc already gives — "a rewind keeps predicted
/// activations and resimulation pushes them again, double-applying an encounter
/// reset". The clone registration PREVENTS that (it restores); the presence-only
/// probe could not SEE it, and presence cannot tell one queued entry from five.
/// Bytes unchanged; a peer that checksums it and one that does not disagree.
/// ⛔ v136: `marker.posed_body` — the opt-in that publishes a body's pose read
/// model (which row, which clip, which frame). A construction-time marker on a
/// simulated body, so a rewind that dropped it would stop publishing mid-match.
/// ⛔ v138: `item.released_as` — the throw/drop decision made durable, so the
/// bomb and grenade fuses stop inferring "somebody threw this" from a nonzero
/// velocity. It decides whether an object in the world is going to explode and
/// lives on a `GroundItem`, which is already an anchor.
/// ⛔⛔ v137: THE ANCHOR IS A SEPARATE FACT FROM THE CODEC, and five dynamic
/// archetypes were missing one or the other. `portal.shot` and
/// `encounter.falling_hazard` had a codec and NO anchor — the registry listed
/// them, the census counted them accounted, and nothing restored them, because
/// nothing put the ENTITY in the envelope. `ability.sentry`,
/// `ability.vortex_well`, `gravity.temporary_zone` and `gravity.zone` were not
/// in the vocabulary at all. All five are spawned MID-MATCH by an ability, a
/// fuse or an encounter beat, so a one-shot census of a booted world could
/// never see them. Reported by a GPT re-review 2026-08-30.
/// ⛔ v135: eight EVENT-CREATED components joined the schema —
/// `ability.player_mark`, `ability.bomb_fuse`, `ability.gravity_grenade_fuse`,
/// `ability.puppy_slug_ally`, `feature.falling_chest`,
/// `encounter.commanded_move`, `encounter.falling_hazard` (+ its entity
/// mapping) and `item.held_projectile`. None existed in a boot world, which is why the coverage census —
/// which sweeps the INITIAL world — never asked whether they rewind.
/// ⛔ v134: `content.cut_rope_heavy_object_cycle` gained a checksum projection.
/// One index decides which prop the arena rebuilds, and a presence-only probe
/// cannot see WHICH — while `reset_cut_rope_boss_arena_on_room_reset` advances
/// it on the sim schedule, so a resimulation can move it.
/// ⛔⛔ v140: `PendingLifecycleCommit`'s ENCODING changed and the readable dump
/// could not see it. Four `LifecycleIntent` variants were deleted
/// (`DeathReset`, `ManualReset`, `Replay`, `FullReset`), so tags 0, 1, 2 and 4
/// no longer decode — a snapshot from a v139 build carrying one of them now
/// refuses. Nothing about the registry LIST changed: same stable name, same
/// encoder type, same projection, so `schema_dump()` was byte-identical and
/// every ledger stayed green. That is the class this constant exists for. A
/// codec body is part of the wire format even when the registry row is not.
/// ⛔ v141: `InputStreamRecorder` LEFT the snapshot. It was
/// `rollback_resource_clone`, so every GGRS save cloned the whole recorded
/// input history and saving frame N cost N. It was only registered because
/// `InputStream::push` was append-only and a resimulated tick recorded itself
/// twice; `push` is tick-addressed now, so a rewind rewrites its own tail and
/// the recorder reproduces its state instead of being restored to it. A peer
/// that snapshots the recorder and one that does not cannot agree about a
/// snapshot, so this is a wire change even though it only REMOVES bytes.
/// ⛔ v142: `CheckpointResumeProgress` ENTERED the snapshot. It was two `Local`s
/// on `restore_checkpoint_on_session_start`, which runs in `PlayerSimulation` —
/// so the once-per-session resume memory did not rewind with the world it is
/// about. Unreachable today because a confirmed transition rebases GGRS onto a
/// new frame zero; registered anyway, because a correctness that holds only
/// because some other layer rebases moves when the rebase does.
/// ⛔ v143: `ActorControl`'s strength hint became a BYTE. It was a bool that
/// could only ever ADD a smash, so a right-stick tilt mode's full deflection
/// armed a flick and came out Smash anyway — a full deflection could not be a
/// tilt. `AttackStrengthHint` is `Auto`/`Tilt`/`Smash` and encodes as one byte.
/// A peer reading the old bool would round `Tilt` onto whichever value it
/// aliased, so the two cannot share a stream.
/// ⛔ v144: `MovePlayback` gained TWO CONTACT FACTS in its checksum projection.
/// `landed_hit` means OVERLAP — the hitbox sweep's channel — so an ordinary held
/// guard set it and the move's `OnHit` cancel confirmed on a blocked strike.
/// `connected_hit` and `blocked_hit` are the damage road's verdict, and they are
/// state: two peers whose in-flight move disagrees about whether it connected or
/// was blocked take DIFFERENT cancels out of the same recovery. A message
/// (`BlockedBodyHit`) also entered the cleared-on-rollback set.
/// ⛔ v145: `MovePlayback` carries an INSTANCE. A self-cancel replaces a
/// playback with a fresh one of the same move in the same update, so two peers
/// agreeing on the move id and its clock could still disagree about whether this
/// is the first jab or the second — which is a different move to staling, to
/// hit-once memory, and to anything reading the instance. Seeded from the
/// playback it replaces, so it needs no counter of its own.
/// ⛔ v146: `LifecycleIntent` gained a SECOND VARIANT, `ReconstituteRoom` — a
/// room rebuild with nobody in it, which a crossing cannot describe. It encodes
/// under tag 5 rather than 4: tags 0-4 all belonged to the four reset variants
/// deleted in v140, so reusing 4 would decode an old `FullReset` as a
/// reconstitution instead of refusing it. A peer on v145 meets tag 5, finds no
/// arm, and refuses the snapshot — which is correct, because it has no executor
/// for the operation.
/// ⛔ v147: A brain DECLARES the perception it needs (ADR 0034), and a brain
/// that needs none stops maintaining `actor.perception_memory`. The wire FORMAT
/// is unchanged — this is a VALUE change, and that is exactly why it needs a
/// version. Two peers on either side of it compute different remembered sets
/// from the same inputs: the old build tracks every peer a `StandStill` body can
/// see, the new one leaves that body's store empty because nothing reads it.
/// Deterministic on both sides (the gate reads `StateMachineCfg`, which is
/// rollback state), and irreconcilable ACROSS them, which is what a schema
/// version is for. Saves and replays taken before it decode to a different
/// world than ones taken after.
/// ⛔ v148: TWINTRACK LEFT THE LAUNCHER, and took 27 registrations with it. The
/// relativity components and the two relativity message buffers were registered
/// by `TwinTrackExperiencePlugin`, so removing it from the shell's provider list
/// removes `relativity.*` from the schema entirely. The crate is still compiled
/// — `ambition_platformer2d`'s `all_capabilities` names the `relativity` feature
/// — but nothing registers its state any more. A peer that still lists the demo
/// has 27 entries this one does not, which is a different snapshot layout.
/// ⛔ v149: THE QUEST ROOM-ENTRY MEMORY IS ROLLBACK STATE. `push_room_entered_
/// quest_events` remembered the previous room in a system `Local`, which a
/// rewind does not touch, so a resimulation across a room change could skip the
/// `RoomEntered` push (S2 in the determinism plan; the same defect
/// `LastCutsceneRoom` closed for cutscenes). It is now `LastQuestRoom`,
/// registered and checksummed beside `QuestRegistry` — one more entry in the
/// snapshot layout, so a peer without it disagrees about every snapshot.
/// ⛔ v150: THE PARALLEL HELD-SHOT SIMULATION IS GONE (K2). A hand-fired
/// gun-sword bolt or fireball is now an `ActionRequest::Ranged` on the one
/// projectile road, so `item.held_projectile` leaves the layout, and
/// `ProjectileGameplay` gains `splash_half_extent` (the fireball's burst,
/// formerly a `HeldProjectile` flag) — one more f32 in every encoded
/// projectile. Layout AND value change: a peer on v149 has an entry this one
/// lacks and decodes a projectile four bytes short.
/// ⛔ v151: `portal.owned_gun_pair` JOINS THE SNAPSHOT LAYOUT. `OwnedPortalGunPair`
/// became rollback-registered at `cf3ee3953`, so the layout gained an entry: a
/// peer without it saves and restores a world one component short, and a load
/// puts the two hosts in different states even though every checksum they
/// compare still agrees.
/// ⚠ NOT a checksum change, and the distinction matters — `rollback_component_clone_probed`
/// records `RollbackEntryKind::ComponentClone`, the same kind as a bare clone,
/// and its own note says *"value-probed for localization, not in the session
/// checksum"*. The probe is a DESYNC-LOCALIZATION aid; what obliges this bump is
/// the entry, not the projection.
/// ⚠ The entry landed at `bf8cbadb4` with the txt baseline updated and this
/// constant left at 150, and the guard that says so —
/// `rollback-wire-format-changes-are-declared` — could not run at all, because a
/// stale sentinel `Cargo.lock` was crashing the checker before it reported.
/// ⛔ v152: `message.parried_body_hit` JOINS THE CLEARED-ON-ROLLBACK SET. A
/// successful parry is now published as a fact naming its attacker, and a
/// counter stance answers it — so the message is not decoration, it drives a
/// move. Two peers that disagree about whether a rewound frame's parry survives
/// disagree about whether a counter fires: one replays the retaliation, the
/// other does not, and the divergence appears a tick later as a checksum
/// mismatch with no obvious cause. ⇒ The buffer is cleared on rollback beside
/// `blocked_body_hit` and `resolved_body_hit`, and a peer without that clear
/// cannot share this stream.
/// ⛔ v153: `MovePlayback` CARRIES A FLOW CURSOR. A move may now author a
/// `TechniqueFlow` — what happens next, based on what happened before — and the
/// node it is on plus that node's wait clock are per-occurrence state. This is
/// v145's reasoning one rung up: two peers agreeing on the move id AND its
/// clock can still disagree about which BRANCH the move took, and from there
/// they resimulate different moves under one name. ⓘ The component is a CLONE
/// snapshot, so the cursor is restored either way; what the projection buys is
/// that the divergence is caught at the checksum rather than when the world
/// shows it.
/// ⛔ v154: `BodyCombat` CARRIES A SLEEP. A move may now put a body to sleep —
/// a fifth named cause in the control lock's `max()`, beside the recoil and
/// landing locks it sits with and the guard-break dizzy and shieldstun the
/// shield owes. It is state for the same reason every lock beside it is: two
/// peers disagreeing about how long a fighter stays helpless resimulate
/// different matches from that moment. ⓘ Cleared by `reset()` (a fighter who
/// respawns still asleep is helpless on arrival with nothing explaining why) and
/// by a real hit (`hit_reaction`), which is the move's whole counterplay.
/// ⛔ v155: `BodyShieldState` CARRIES THE PARRY'S MODE. A stance may now absorb a
/// caught projectile instead of returning it, and `absorb_window_timer` is a
/// MODE on the existing parry window rather than a second window — `parrying()`
/// still decides whether a shot is caught. It is state because two peers
/// disagreeing about which response a stance is running send the same shot two
/// different ways: one has a bolt flying back at the firer and the other has no
/// bolt at all. ⓘ Decayed beside `parry_window_timer` in the movement kernel, so
/// a stance that stops re-arming stops absorbing on the same tick it stops
/// parrying.
/// ⛔ v156: `SmashHoldState` CARRIES WHETHER THE HOLD IS A CARGO CARRY. A carry
/// is an ordinary hold with two terms changed — where the captive rides, and
/// whether its captor may walk — so `carrying` rides the RULESET's half of the
/// hold rather than `CapturedBy`, which is the generic relation and has no
/// opinion about locomotion. It is state despite being decided once and constant
/// after, and that is exactly the reason: a restore that put the hold back
/// without it hands the resimulated captor a hold they can no longer walk with,
/// so the two peers' captors stand in different places from that moment.
/// ⓘ Also v156: `smash.placed_mine`, a clone-snapshotted arming clock, and
/// `message.capture_carry_requested`, a same-frame transient beside the three
/// capture requests it joins.
/// ⛔ v158: `SteeredBolt`, a bolt the caster flies with the stick. Position,
/// velocity, clock AND the latch that says it has cleared its caster all rewind
/// — more than the mine's clock-only row, because a bolt is STEERED: two peers
/// can agree on its lifetime and disagree about its HEADING, and the heading is
/// what decides whether it comes home and throws a fighter across the stage.
/// ⛔ v159: `PlacedSpring`, a plate on the floor that throws whoever steps on it.
/// THREE clocks and a use count all rewind — the lifetime, the per-body re-arm,
/// and the arming delay that stops it launching the fighter who dropped it. ⇒ A
/// restore that lost any of them hands the resimulated timeline a launch the
/// confirmed one had already spent, and a launch is a fighter standing
/// somewhere else.
/// ⛔ v160: `HomingDash`, a fighter being carried at whoever they were pointing
/// at. Its clock and its COMMITTED direction both rewind: the direction is
/// remembered at the press rather than re-read, so a restore that lost it would
/// let the resimulated dash re-aim from a facing the confirmed one never used.
/// ⛔ v161: `AxisManeuverState` CARRIES A TIMED GRAVITY MULTIPLIER — the pair
/// `gravity_modifier_scale` + `gravity_modifier_timer`, appended to the motion
/// codec beside `blink_grace_timer`. A move asks for a locomotion regime (a
/// parasol, a float, a slow-fall) and the MOVEMENT domain owns the clock, so
/// the state is where every other maneuver timer already lives. ⇒ It rewinds
/// because gravity is integrated every tick: two peers disagreeing about how
/// much longer a float lasts do not disagree about a flag, they disagree about
/// where the body IS, and the gap widens for as long as the modifier runs.
/// ⓘ The TIMER is the activation switch and the scale is read only while it is
/// positive, so a restore that lands on a zeroed pair means "no modifier"
/// rather than "zero gravity" — the dangerous state is unrepresentable rather
/// than merely avoided.
/// ⛔ v162: `TimeDilated`, the clock a body was put on and the one it gets back.
/// `ProperTimeScale` was already canonical as `actor.proper_time_scale`; what was
/// missing is the REMAINDER — how much longer the victim's moves, hurtbox
/// resolution and animation run slow — and the PRIOR scale to restore. ⇒ Two
/// peers disagreeing about the remainder do not disagree about a flag; from that
/// tick on they resimulate different swings, because move playback advances on
/// `WorldTime::entity_dt`. And a restore that lost `prior` would put the body
/// back on the wrong clock permanently, since nothing else owes it a reset.
/// ⓘ The scale itself needs no new row: this row is the smash ruleset's clock
/// over the engine's existing one.
/// ⛔ v163: `MatchScoped`, which MATCH an object was created by.
/// A player found a mine laid in one match still standing in the next: the smash
/// ruleset spawns at five sites (bomb, bolt, mine, portal, spring) and every one
/// ended only by its own rule — a fuse, a trigger, a lifetime — so a match ending
/// was not among them. ⇒ The marker is stamped at spawn and swept by whoever owns
/// the match, which puts that lifetime in ONE place instead of five that must
/// each remember.
/// ⭐ IT IS REGISTERED BECAUSE BOTH SIDES OF THE COMPARISON MUST REWIND. The
/// sweep asks "is this object's match the active one", and `ActiveMatch` already
/// rewinds; an unregistered marker would be LOST on any restore, leaving the
/// objects it marks permanently unsweepable — cleanup that silently stops working
/// after the first rollback. ⓘ `component-clone` like the four objects it marks:
/// a stable identity copied at spawn has nothing for a codec to normalise.
/// ⛔ v164: `TetherReel`, the ledge a tether latched and the clock reeling to it.
/// The aerial half of the Projectile Polygon's tether: a line thrown at a ledge,
/// which then pulls her to the exact point the ledge authority wants a hanging
/// body to occupy. ⇒ The reel is several frames of authored motion, so its clock
/// decides where a fighter IS — the same argument `HomingDash` carries.
/// ⭐ THE ANCHOR IS THE STRONGER HALF, AND IT IS WHY THIS CANNOT BE RECOMPUTED
/// ON RESTORE. It is not merely a destination: it is a point on the stage she
/// committed to on ONE frame, chosen by probing the solids at that frame. A peer
/// that restored the reel without it would re-probe against its own view and
/// could latch a DIFFERENT ledge — a divergence that grows with every tick
/// instead of correcting itself. ⓘ `component-clone`: two floats and a point,
/// with nothing for a codec to normalise.
/// ⭐ 164 -> 165 (2026-09-06): `actor.control_claims` joined the schema. It is
/// the CLAIM COLLECTION behind `actor.temporary_control` — prerequisite B's
/// arbiter — and the projection alone is not enough to restore, because a rewind
/// that dropped a shadowed claim would resume with a possession or a ride the
/// simulation had forgotten. New state on the wire, so peers must agree on it.
/// ⭐ 165 -> 166 (2026-09-07): `smash.body_mark` grew two fields and its probe
/// changed. The mark now carries the ATTACKER'S SEAT (the blast's credit) and
/// the authored fuse (the telegraph's denominator), and the probe folds the
/// seat in — two peers agreeing on WHEN a mark goes off and disagreeing on WHO
/// is credited is a kill on different fighters on the two screens.
/// ⭐ 166 -> 167 (2026-09-07): `smash.seat_credit` + `smash.seat_credit_stand_in`
/// joined -- the credit of a mark whose attacker was eliminated inside the fuse
/// rides a stand-in entity for the blast's lifetime, and a rewind across the
/// detonation has to restore it.
/// ⭐ 167 -> 168 (2026-09-08): `resource.outstanding_checkpoint_request` +
/// `resource.admitted_checkpoint_restore` joined — A1c made checkpoint
/// restoration go through one ADMITTED operation instead of three domains each
/// reading the raw `ResetToCheckpoint`. The request now outlives the frame it
/// arrived on (a reset asked for while another lifecycle intent owns the slot is
/// remembered, not lost), so a rewind past that frame must take it back or one
/// timeline restores a checkpoint the other never asked for. The admitted token
/// is same-frame TODAY and registered anyway: when application moves to the
/// confirmed commit boundary its lifetime changes, and the registration must not
/// be the thing anyone remembers to add.
/// ⭐ 168 -> 169 (2026-09-08): `resource.accepted_checkpoint_restore` joined.
/// A1c/3 gave the accepted checkpoint operation a lifetime longer than the frame
/// that admitted it: room preparation reads the population it was accepted with,
/// several frames later, instead of deriving one from the live ledger that the
/// restore had overwritten in order to be read. A rewind that crossed the
/// admission must take the selection back, or the load prepares a room from a
/// population the resimulated timeline never chose.
/// ⭐ 169 -> 170 (2026-09-08): no key moved and no encoding changed — the
/// CHECKSUM PROJECTION behind `resource.accepted_checkpoint_restore` did. Its
/// first version covered the frame, the pinned ledger and the intent's target
/// room, so two accepted restores agreed while differing in their pinned mint
/// recipes, and two crossings agreed while differing in subject, arrival, edge
/// or door cue. A projection that covers part of a value reports agreement
/// between peers holding different operations, which is the desync it exists to
/// catch. Peers compare these numbers, so a corrected projection is a peer
/// compatibility change even though the registered set is unmoved.
/// ⭐ 170 -> 171 (2026-09-08): `resource.admitted_checkpoint_restore` LEFT the
/// schema. A1c/1-2 introduced it as a one-frame token every checkpoint reducer
/// had to remember to read; A1c/3b moved those reducers into the commit
/// executor's own schedule, which only an authorized commit runs, so the
/// guarantee is now made out of WHEN they run and no reducer can forget it. A
/// token with no consumer is state two peers must agree about for nothing.
/// ⭐ 171 -> 172 (2026-09-08): `resource.session_checkpoint_operations` joined,
/// and `resource.accepted_checkpoint_restore`'s projection grew the operation
/// key. A restore operation now has an IDENTITY — the session's ownership stamp
/// plus a sequence that advances only on admission — because intent equality
/// could not tell two crossings to one room with one subject apart, and a frame
/// number cannot either: a room rebase restarts the rollback timeline at zero.
/// The counter rewinds so a resimulated admission mints the same key; it is
/// deliberately not reset at a rebase, because recycling a live identifier is
/// how a stale host-side load gets authorized on a matching integer.
/// ⭐ 172 -> 173 (2026-09-08): the checkpoint restore gained a terminal answer
/// and startup stopped keeping its own. `resource.session_checkpoint_outcomes`
/// and `resource.session_startup_resume` joined;
/// `resource.checkpoint_resume_progress` left. ⛔ THE KEY MOVED WITH THE MEANING
/// rather than being kept across a rename: A1b deliberately preserved that name
/// through a pure module move, but this is the opposite case — the value changed
/// from two per-generation latches to a state machine naming an admitted
/// operation, so keeping the name would let two peers agree on a key whose
/// contents mean different things.
/// ⭐ 173 -> 174 (2026-09-08): no key moved; three PROJECTIONS changed together.
/// `CheckpointOperationKey` gained one canonical tagged encoding and the two
/// resources that had spelled it as `scope.0 | 1 << 63` now reuse it — a bit-or
/// that silently collided a scope with its top bit set against the absent case.
/// And `resource.session_checkpoint_outcomes` stopped hashing only
/// committed-versus-failed: a failure now carries a closed `RestoreFailure`
/// rather than a free-form string, so two peers that agreed on "failed" while
/// blocking gameplay for different reasons no longer agree. Peers compare these
/// numbers, so a corrected projection is a compatibility change.
/// ⭐ 174 -> 175 (2026-09-08): `RestoreFailure::Population` joined the terminal
/// outcome's closed set, which is a new WIRE CODE — an old peer cannot produce
/// discriminant 6, and the outcome is snapshot state two peers compare. The
/// primitive COUNT is unchanged (one `put_u8` of a discriminant, as before),
/// which is exactly the shape this file's history records as the least
/// wire-looking wire there is.
/// ⭐ 175 -> 176 (2026-09-08): `CheckpointRestoreOutcome::Cancelled` joined the
/// terminal-outcome set — a new variant AND a new reason code, both wire codes an
/// old peer cannot produce. It exists because retirement-without-an-outcome was
/// a second completion mechanism beside the terminal outcome this packet
/// established, and because a restore that ended BEFORE destructive application
/// (world whole, candidate discarded) is a different report from one that ended
/// after (gameplay blocked, no claim about the old world).
/// ⭐ 176 -> 177 (2026-09-09): `HitTarget::Feature(_)` joined the hit-event
/// target set with its own wire tag (5). A projectile's direct contact names the
/// boss or breakable it selected instead of broadcasting an `UnresolvedFeatures`
/// volume the applier re-scans — which damaged every breakable the volume
/// overlapped and let query order decide which part of a multi-part boss was
/// credited. A peer on the old schema cannot produce the new tag, and a targeted
/// feature hit must not compare equal to a broadcast remainder.
/// ⛔ v178: `smash.seat_credit` LEAVES THE SNAPSHOT LAYOUT. `SeatCredit` labelled
/// the stand-in entity a mark's blast uses when its author has left the match,
/// and MEASURED 2026-09-10 nothing ever read it — attribution runs on `Entity`
/// throughout, `BodyKnockedOut` carries a `cause` and no attacker, and no
/// per-seat KO tally exists anywhere in the tree. A layout entry for a fact no
/// system reads, so a peer on v177 has an entry this one lacks. ⭐ The stand-in
/// ENTITY stays: what the blast needs is a valid non-victim owner carrying no
/// `MatchSeat`, and both are properties of the entity rather than of the label.
/// ⭐ 181 -> 182 (2026-09-12): `BodyCombat`'s armor stopped being a `bool` and
/// became `ArmorPolicy`, so the component's canonical encoding changed shape —
/// one authored tag byte, plus the threshold for the variant that carries one.
/// A `bool` peer and a policy peer cannot agree about a restore: the old wire
/// has no way to say *"armored against hits under 10"*, and reading its single
/// byte as a tag would decode `true` as `Super` by luck and `false` as `None` by
/// luck while every threshold policy became unreadable. ⛔ THE VARIANT CODES ARE
/// AUTHORED (`None`=0, `Super`=1, `Damage`=2, and 3 RESERVED for the knockback
/// threshold that `ArmorPolicy` explains it does not have yet) so that adding
/// the reserved one later does not renumber anything already written.
/// ⭐ 183 -> 184: `SessionScopedEntity` moved from `component-canonical` to a
/// probed clone snapshot. Its value is a host-local activation count, so
/// including it in the peer checksum made two peers with different session
/// histories disagree about a mechanically identical world. The value is still
/// snapshotted — `construction`'s scope gather reads it to filter another
/// session's entities — it is only out of the checksum.
/// ⭐ 184 -> 185: `ActiveMatch` moved to a clone snapshot with a peer-stable
/// checksum projection. Its `session` is a per-App activation count and its
/// `seat_topology` is a local device-topology generation that moves when a host
/// re-captures an identical set of seats; both still snapshot, neither is
/// compared. `ActiveMatch::peer_stable_checksum` owns the split.
/// The same bump splits the KIND: a canonical snapshot whose checksum is a
/// projection now reports `resource-canonical-custom-checksum`, not
/// `resource-canonical`. The two behaved differently and shared one label, so no
/// guard could tell "every field is compared" from "these fields are" — which is
/// the whole of the peer-agreement question. `ActiveMatch` moves with them.
/// ⭐ 185 -> 186: the four `MatchInstance`-stamped resources — `ActiveMatch`,
/// `StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks` — took
/// peer-stable checksum projections and now compare MECHANICAL FACTS ONLY: the
/// agreed seat count, the verdict, whether sudden death is latched, and the
/// micros elapsed since the match's own start. All four still snapshot whole, so
/// a rewind restores every local field.
/// ⛔⛤ THE FIRST VERSION OF THIS BUMP KEPT THE ACTIVATION TICK AND CALLED IT
/// PEER-STABLE. It is not: `SimTick` has one writer, sits unconditionally at the
/// head of the sim schedule and is never rebased, so it counts every step this
/// App has run INCLUDING MENU FRAMES. Two peers who reached one lobby by
/// different routes would have disagreed from the first compared frame. Found by
/// the GPT architecture review of 2026-09-15.
/// The same bump splits the KIND: a canonical snapshot whose checksum is a
/// projection now reports `resource-canonical-custom-checksum`, not
/// `resource-canonical`. The two behaved differently and shared one label, so no
/// guard could tell "every field is compared" from "these fields are" — which is
/// the whole of the peer-agreement question.
/// ⭐⭐ 186 -> 187: `ActiveMatch` gained an ORDINAL — which match of this session
/// it is — and it is the first activation term two peers actually agree on. It
/// starts at zero for everyone who joins a session together and counts up as
/// they activate matches together, so it is insensitive to how long either App
/// has run, how many menus it sat in, and how many sessions it played before.
/// A new `SessionMatchOrdinal` resource mints it and rewinds with everything
/// else, restarting at zero when the session changes.
/// ⛔ TWO CONSUMERS MOVED ONTO IT, both of which were peer-divergent: the match
/// item draw context (which read the absolute activation tick, so two peers drew
/// different items) and `SimId::match_spawn`, which embedded that tick in a
/// `component-canonical` identity string — so two peers minted DIFFERENT
/// canonical identities for the same spawned item.
/// ⭐⭐ 187 -> 188: `CheckpointOperationKey` stopped writing the raw
/// `SessionScopeId` into the checksum three resources compare —
/// `SessionStartupResume`, `AcceptedCheckpointRestore`,
/// `SessionCheckpointOutcomes`. A scope id counts THIS App's session
/// activations, so two peers in one agreed session hold different ones and would
/// have disagreed from the first compared frame. The peer projection is now the
/// ADMISSION SEQUENCE plus whether a scope owns the operation at all; admission
/// is simulated, so two timelines that admitted the same operations are on the
/// same number, and the presence tag is a composition fact both peers share.
/// ⚠ THE SCOPE IS NOT GONE — it still decides stale-operation rejection and
/// still round-trips, because all three are `rollback_resource_clone_checksum`:
/// the SNAPSHOT is a `Clone` of the whole struct and never goes through the
/// projection. This is the peer/local split, not a deletion.
/// ⛔⛤ 188 -> 189: `SessionMatchOrdinal` was registered
/// `rollback_resource_canonical` — a WHOLE-VALUE checksum — in the very commit
/// that introduced it, while the comment beside the registration claimed its
/// `session` half "is compared only against ITSELF". The sentence described
/// `take`'s reset rule; the registrar decided the checksum. A per-App session
/// activation count was in the peer comparison, which is the exact defect the
/// ordinal exists to remove. It now projects the count of matches this session
/// has activated and nothing else. Found by the GPT architecture review of
/// 2026-09-15.
/// ⚠ ONE WINDOW SURVIVES: the mint resets lazily inside `take`, so between
/// joining a session and activating that session's first match it still holds
/// the previous session's count.
/// `two_peers_who_played_different_prior_matches_disagree_before_the_first_activation`
/// holds it, and closing it means making the mint session-OWNED state rather
/// than an App-global resource with an owner tag.
/// ⛔⛤ 189 -> 190: the four `MatchInstance`-stamped projections were
/// FALSE-NEGATIVE. Excluding the local stamp from a peer checksum was right and
/// it left nothing saying WHICH match the value described, so a verdict, a
/// sudden-death latch or a clock belonging to the PREVIOUS match checksummed
/// identically to one belonging to the live match — while `settled(active)` and
/// `entered(active)` answered differently, because those compare the local
/// instance. Two peers could hold identical checksums over state that simulates
/// differently, which is worse than a checksum that disagrees: it HIDES a
/// desync instead of reporting one.
/// ⭐ `MatchInstance` now carries both halves. The LOCAL half (`session`,
/// `activated_on`) still decides staleness and `belongs_to`; the PEER half is
/// the session-relative ordinal, and `peer_match_digest` is the only part of the
/// instance any projection may read. All four projections now hash it:
/// seats + match, verdict + match, latch + match, elapsed + match.
/// ⚠ The wire format grew with it — the ordinal travels in every codec that
/// encodes a `MatchInstance`, so a rewind restores which match a value is for.
/// Found by the GPT architecture review of 2026-09-15, which named this a
/// false-NEGATIVE checksum bug beside the false-POSITIVE one at 188 -> 189.
/// ⭐⭐ 190 -> 191: the six hand-rolled peer-checksum projections collapsed onto
/// one owner, `ambition_platformer2d_core::snapshot::PeerDigest`. No projection
/// changed WHICH fields it compares; all of them changed how those fields are
/// encoded, so the values differ and peers on two versions would disagree.
/// ⛔ WHAT WAS THERE: a raw field returned unhashed, a `Vec` plus
/// `extend_from_slice`, a wrapping-multiply by a constant, a byte-writer, and two
/// variants of a local `peer_stable_digest` — three hashing strategies for one
/// job, and only ONE of the six carrying a domain tag. Every projection defect
/// this campaign found was a projection encoding something almost the same way as
/// its neighbour, `CheckpointOperationKey`'s three spellings worst of all.
/// ⚠ Two rules are now structural instead of remembered: a domain is required,
/// and an optional writes a presence tag so absent never equals zero. The second
/// one caught a live defect in the same pass — `LiveMatchTicks` folded an absent
/// match through `unwrap_or(0)`, so a clock belonging to NO match agreed with one
/// belonging to a match whose digest happened to be zero.
/// ⭐⭐ 191 -> 192: `ContentBinding::canonical_summary` now renders WHICH CONTENT
/// alongside which activation of it, so `TransactionId` — `component-canonical`,
/// whole string compared — carries a peer-stable term for the first time. It was
/// `{epoch}\t{room}\t{session}` with two of three terms per-App counts, and a
/// projection excluding them would have left `{room}` alone, handing every entity
/// in one room the same identity.
/// ⚠ ONLY PRODUCTION STRINGS MOVE. The `|content:` segment is rendered only when
/// a content identity is STATED, and it is ABSENT rather than zero-filled when it
/// is not — so a binding built outside a prepared session renders `epoch:N`
/// exactly as before, and every fixture's identity is byte-identical.
/// ⛔ THE PROJECTION ITSELF IS NOT LANDED. `TransactionId` is a bare `String`
/// whose `from_raw` is the codec's decode half, so a checksum over "content and
/// room but not epoch and session" needs it restructured into parts — a 61-use
/// change, and the next reviewable step. `ContentBinding::peer_stable_summary`
/// is the term it will project.
/// ⭐⭐ 192 -> 193: `TransactionId` — the campaign's ORIGINAL finding — stopped
/// comparing its whole string. It renders as `{binding}\t{room}\t{session}` and
/// two of those three terms are per-App counts: the binding's content epoch and
/// the owning session's activation stamp. The projection keeps the CONTENT
/// IDENTITY and the ROOM and drops both.
/// ⚠ THE STRING IS UNCHANGED AND MUST BE. It carries local ownership that the
/// construction scope's gather filter and A10's candidate-vs-live separation both
/// read; two sessions committing one room at one epoch once minted the same token
/// and each classified the other's roots as its own. This is the split the
/// campaign exists for, not a substitution — the snapshot is still the whole
/// value, only the COMPARISON narrows.
/// ⭐ It is the first COMPONENT to state a projection, which is why
/// `rollback_component_canonical_checksum` had to exist first.
/// ⛔⛤ 193 -> 194: NOTHING MECHANICAL CHANGED, AND THAT IS THE POINT OF THIS
/// ENTRY. No type entered or left the layout, no projection changed, no value is
/// encoded differently. What changed is a SENTENCE: the `detail` on 99
/// `component-clone`/`resource-clone` rows stopped reading *"state checksum
/// supplied by another authoritative projection"* — a claim about some other
/// registration that `rollback_component_clone` (bound only by `T: Clone`) has no
/// way to establish — and now reads *"not in the session checksum"*, which is
/// `RollbackEntryKind::feeds_peer_checksum() == false` stated plainly.
/// ⚠ A BUMP IS OWED ANYWAY, because `compute_schema_fingerprint` hashes the whole
/// `schema_dump()`, `detail` column included. That is the defect, not the
/// protocol: English wording is inside a peer-visible identity, so removing a
/// false claim costs a version. Asked as `Q122`; measured by poison — pluralising
/// one word in `detail::MESSAGE_CLEAR` moves 83 rows.
/// ⛔⛤ 194 -> 195: `AuthoredOccurrences` STOPPED CLAIMING TO BE DERIVED. It was
/// `declare_rollback_derived_resource` justified as *"republished from live state
/// while its room is loaded"*, and that assertion was false on a shipped road:
/// `process_new_game_reset_request` calls `forget_everything()` from INSIDE the
/// rewinding schedule, and a `Placed` row for a room that is not resident has no
/// live producer to republish it. Measured: ONE `adopt_rows` call inside the
/// schedule desyncs the sync test at frames 3, 4 and 5 for a write at tick 5 —
/// no save file, no load, no New Game.
/// ⭐ THE TYPE ASKED FOR THIS ITSELF. `AuthoredOccurrences::rewind_argument` said
/// *"if a non-rederived whereabouts state gains a producer, this ledger must
/// become registered value state with a value-sensitive probe"* — and
/// `adopt_rows`, 100 lines above it in the same file, was already that producer.
/// The contract named its own trigger and missed the one it had.
/// ⚠ THIS ONE IS REAL MECHANICAL GROWTH, unlike 193 -> 194. A row enters the peer
/// checksum: `resource.placement_continuity`, `resource-clone-custom-checksum`,
/// projected by `peer_stable_checksum` (a domain-separated fold over
/// `(SimId, whereabouts)`; the `BTreeMap` order is what makes it deterministic).
/// The `derived.placement_continuity` row leaves. Entity-free, so a clone
/// snapshot is the whole story and no `MapEntities` is owed.
/// ⛔⛤ 195 -> 196: TWO PEER PROJECTIONS OF OCCURRENCE ROWS BECAME ONE ENCODING
/// UNDER TWO DOMAINS, AND THE BYTES MOVED FOR BOTH. 195's projection was a bare
/// `StateHasher` fold with no domain and no length prefixes, which
/// `PeerDigest`'s own doc calls the one way NOT to build a peer checksum. It was
/// structurally ambiguous, not merely untidy: two `InCustody` rows `"a"` and
/// `"b"` wrote `a 01 b 01`, and one row with id `"a\x01b"` wrote the same
/// stream. `AuthoredOccurrences::encode_rows` is now the single byte encoding,
/// folded by `PeerDigest::in_domain` as `lifecycle.authored_occurrences` and, for
/// the baseline that copies it, `lifecycle.occurrence_baseline`.
/// ⭐ THE TWO DOMAINS ARE LOAD-BEARING AND A SINGLE SHARED VALUE WOULD HAVE BEEN
/// A NEW DEFECT. `bevy_ggrs` combines `ChecksumPart`s by XOR and warns in its own
/// source that a value appearing an even number of times cancels; `OccurrenceBaseline::adopt`
/// copies the ledger, so EQUAL CONTENTS IS THE STEADY STATE. Collapsing both to
/// one digest would have removed both entries from the frame checksum for as long
/// as they agree. Held by `the_baseline_and_the_ledger_do_not_cancel_each_other_out`
/// and `a_row_boundary_cannot_be_re_read_as_part_of_an_id`, the second of which
/// rebuilds 195's fold and asserts it collides.
/// ⚠ `OccurrenceBaseline::checksum` ALSO MOVES, though nothing about the baseline
/// itself changed: it gained a domain. A projection's bytes are peer-visible
/// whether or not the value behind them did anything.
/// ⛔⛤ 196 -> 197: FIVE MESSAGE TYPES CARRIED TWO ROWS EACH, AND THE SECOND ROW
/// WAS ALSO A SECOND SYSTEM. `ClearPortals`, `DropPortalGun`, `FirePortalGun`,
/// `PickUpPortalGun` and `TogglePortalGun` were each registered under a canonical
/// name and a historical alias (`message.portal_clear`, `message.portal_gun_drop`
/// …), retained by a carve *"so the full compatibility registration keeps the
/// existing rollback schema byte-for-byte"*. Nothing outside the definition and
/// the baseline ever read the alias.
/// ⚠ IT WAS NOT ONLY A DUPLICATED ROW. `should_install_backend` dedupes on the
/// registration's stable NAME, not on its type, so each alias also installed a
/// second `clear_message_channel::<T>` into `LoadWorld::Mapping`: ten systems
/// doing five jobs. This is the one bump where the dump SHRINKS, by exactly the
/// five alias rows, and no mechanical state enters or leaves the snapshot.
/// ⇒ Held by `no_two_schema_rows_describe_the_same_type_the_same_way`, so an
/// alias cannot come back as a compatibility kindness a second time.
/// ⛔⛤ 198 -> 199: A PEER PROJECTION WAS READING A RENDERED STRING AND STOPPED
/// ONE FIELD TOO EARLY. `TransactionId::peer_stable_checksum` folded the content
/// identity and the room and then quit; `ConstructionLane::transaction` appends
/// a FOURTH field, `lane:{name}`, to the three `ConstructionScope::transaction`
/// renders, and that field never reached the digest. So two roots in one room,
/// at one content, on the gravity lane and on the portal-gun lane contributed
/// the SAME value — measured at `4192778953073586539` for both before the fix.
/// ⭐ THE LANE IS PEER-STABLE, unlike the session and the binding's epoch beside
/// it: `ConstructionLane::named` takes an authored domain constant (a capability
/// name such as `PORTAL_GUN_CONSTRUCTION_DOMAIN`), not a per-App counter. Two
/// peers running one content render one lane name or they disagree about
/// something real, which is exactly the thing a peer checksum is for.
/// ⚠ NO MECHANICAL STATE ENTERS OR LEAVES THE SNAPSHOT. `TransactionId` still
/// snapshots WHOLE; only the comparison widens, and the row's description moves
/// with it. The bytes move for every transaction stamp, including primary-lane
/// ones, because the absent lane now folds in as the literal `primary`.
/// ⇒ Held by `two_construction_lanes_do_not_share_one_peer_projection` (primary
/// vs named AND named vs named) and `the_primary_lane_projects_as_the_name_no_
/// named_lane_may_take`, which pins the absent-lane default to the one name
/// `ConstructionLane::named` refuses.
/// ⛔⛤ 199 -> 200: A COMPONENT NO COMPOSITION EVER BUILT WAS INSIDE THE
/// FINGERPRINT TWO PEERS COMPARE. `GravityFlipSwitch` held two rows —
/// `entity:gravity_flip_switch` (required-rollback) and `gravity.flip_switch`
/// (component-clone) — for an overlap pressure plate whose only system
/// registration in the workspace was inside its own `#[cfg(test)]` module and
/// which nothing authored or spawned. `Q137` ruled on 2026-09-19 that gravity
/// SWITCHING stays (the LDtk-authored `FlipGravity`/`SetGravity` switches and
/// the developer controls are the product) and that the parallel plate goes.
/// ⚠ NO MECHANICAL STATE LEAVES THE SNAPSHOT, because none was ever in it: the
/// component had no production insert site, so both rows were always empty.
/// This is the second bump where the dump SHRINKS, by exactly those two rows.
/// ⛔⛤ 200 -> 201: THE `Fade` SNAPSHOT CODEC GREW A FIELD AND THE REGISTRY
/// LIST DID NOT MOVE. `CutsceneBeat::Fade` now encodes `from_alpha` ahead of
/// `to_alpha` (`Q143`: both ends of a fade are authored), so a beat inside
/// `ActiveCutscene`'s snapshot writes one more `f32`. Nothing about the row
/// changed — same stable name, same encoder type, same projection — and all
/// four baseline arms passed at v200 without a bump. ⇒ **That is the v140
/// class exactly, and the commit that landed the codec argued the opposite:**
/// it said no bump was needed BECAUSE the encoding lives inside the resource's
/// own snapshot bytes. Those bytes are the wire contract this constant
/// represents. A codec body is part of the wire format even when the registry
/// row is not.
/// ⛔⛤ 201 -> 202: A LIVE ROOM GOT AN IDENTITY THE ROOM DEFINITION DOES NOT
/// HAVE. One row enters: `root.live_room_instance`, `component-canonical`,
/// a `u32` ordinal minted by the one road that seats a session in a published
/// room. Real mechanical growth, not a wording change — the value is
/// snapshotted and it feeds the session checksum.
/// ⭐ WHY IT IS NOT DERIVABLE FROM WHAT WAS ALREADY THERE. `root.room_set`'s
/// checksum folds the active index, the start index and the active room's id,
/// and all three read identically after a session leaves a room and comes
/// back — so the state before and after a round trip is one value to every
/// existing row. `RoomConstructionPlanId` cannot fill the gap either: it is a
/// CONTENT hash whose own doc excludes `SessionSpawnScope` and `TransactionId`
/// on purpose, so two constructions of one room share it.
/// ⚠ It is OW1's precondition in
/// `docs/planning/engine/open-world-runtime-and-residency.md` and not OW1: one
/// index still selects one live room, and this identity lives on the session
/// root because that is where the one live room lives.
/// ⛔⛤ 202 -> 203: A BODY'S RESOURCES BECAME THEIR OWN ROW. One row enters:
/// `body.resources`, `component-canonical`, `ActorResources` — the per-actor
/// bank a Smash seat's Limit now lives in (`composable-actor-resources.md`,
/// step 1). Real mechanical growth: the codec writes each slot's authored
/// name, capacity, start and level, so a restore rebuilds the layout the
/// values are read through, and the bytes feed the session checksum. A body
/// that declared no resource carries no bank and writes nothing.
/// ⚠ AND ONE ROW LEFT THE PEER CHECKSUM WITHOUT ITS OWN BUMP: `resource.sim_dt`
/// became a DECLARED-DERIVED mirror of the clock in `77aee7ae4` (the clock
/// heads the tick; `SimDt` is rebuilt from it), so it stopped feeding the
/// checksum at v202. This bump is the first to cover that change.
/// ⛔⛤ 203 -> 204: MANA JOINED THE BANK AND ITS OWN ROW LEFT. `body.mana`
/// (`BodyMana`, a `ResourceMeter` every body carried) is gone: Mana is a
/// declared resource in `body.resources` now, held only by a body whose
/// experience declared the pool (`composable-actor-resources.md`, step 2).
/// One row fewer, and a body that holds no Mana writes no Mana bytes.
/// ⛔⛤ 204 -> 205: `message.npc_provocation_changed` is cleared on load. A
/// talkable NPC's durable provocation is announced by the transitions that own
/// it (the flip, the release) and recorded in the same tick, so a resimulated
/// tick re-announces it rather than replaying an abandoned branch's.
/// ⚠ 205 ALSO COVERS A CODEC CHANGE THAT SHIPPED UNBUMPED: `290296f2e` made
/// `ActorAggression` encode a one-byte grudge-faction tag (`Grudge::Faction`
/// survives a rewind; a body grudge is carried by the entity map), which moved
/// the bytes under v204. v205 is the first version that names that encoding.
/// ⛔⛤ 205 -> 206: FOUR DEAD PROJECTIONS LEFT THE WIRE. `boss.pattern_timer`
/// (`BossPatternTimer`, a copy of the boss brain's own timer) is gone;
/// `ActorStatus` no longer encodes `ai_mode` (a mode nothing read);
/// `ActorControl`'s fire request no longer encodes a launch speed (the resolved
/// `RangedActionSpec` owns it); and `CombatTuning` (clone, unhashed) lost
/// `attack_cooldown_mult`.
/// ⛔⛤ 206 -> 207: `actor.pose` LEFT. `ActorPose` was a checksummed copy of the
/// body's box and facing, synced by two systems on different schedules and
/// seeded from the placement rect before the first sync corrected it; its one
/// reader (the brain action origin) reads `BodyKinematics::pos` now.
/// ⛔⛤ 207 -> 208: `body.lifetime` is `body.restart_latch`. The restart
/// announcement latch is the only body-lifetime fact the sim owns; the
/// `time_alive` / `resets` diagnostics left the wire (`BodyLifeStats`,
/// unregistered) and `max_speed`, read by nothing, is deleted.
/// ⛔⛤ 208 -> 209: `actor.scripted_control` LEFT. The marker was present
/// exactly when `ControlHolds` was; presence of the hold set is the fact now.
/// `actor.temporary_control` LEFT with it: `TemporaryControl` was a per-tick
/// projection of `ControlClaims`, which readers now ask directly.
/// ⛔⛤ 209 -> 210: `actor.combo_trace` LEFT. The HUD's movement readout is
/// app presentation state fed from `FrameEvents` after the kernel ran; no
/// simulation system read it.
/// ⛔⛤ 210 -> 211: `actor.ranged_refire` ENTERED and `BodyMelee` no longer
/// encodes `ranged_cooldown`. The ranged fire-rate floor (invariant I3) is its
/// own component, `RangedRefire`, not melee state.
/// ⛔⛤ 212 -> 213: `BodyCombat` encodes `struck_recently`, the gameplay
/// recent-strike window bark dedup and chatter suppression read instead of the
/// presentation flash; and `BossAnimFrame` lost its `Hit` drive phase (the sim
/// cursor never enters the hit row, so boss geometry does not follow a flash),
/// which renumbers the `Death` phase byte.
pub const GGRS_ROLLBACK_SCHEMA_VERSION: u32 = 213;

//: ⭐ MOVED to `ambition_platformer2d_core::rollback_kind` 2026-09-16 and
//: re-exported here. It had to sit beside the `RollbackRegistrar` TRAIT, which
//: `core::snapshot` declares -- a default trait body cannot name a type living
//: in a crate above it, and naming each method's kind at its declaration is
//: ROLLBACK-KIND-SPELLING's acceptance.
pub use ambition_platformer2d_core::rollback_kind::RollbackEntryKind;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RollbackRegistrationDescriptor {
    pub name: String,
    pub owner: String,
    pub kind: RollbackEntryKind,
    pub type_name: String,
    pub detail: String,
}

#[derive(Resource, Debug, Default)]
pub struct RollbackRegistry {
    entries: BTreeMap<String, RollbackRegistrationDescriptor>,
    /// Memoised [`Self::schema_fingerprint`]. See that method for why.
    ///
    /// ⭐ SOUND BECAUSE `entries` HAS EXACTLY ONE MUTATION SITE — the `insert` in
    /// `try_register` — which clears this. A second mutation path would have to
    /// clear it too, which is why `entries` stays private.
    fingerprint: std::sync::OnceLock<SnapshotSchemaFingerprint>,
}

// ⛔ HAND-WRITTEN because `OnceLock` is not `Clone`. A clone starts with an EMPTY
// memo rather than copying it: same value on next demand, and no risk of a clone
// carrying a fingerprint its own entries no longer justify.
impl Clone for RollbackRegistry {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            fingerprint: std::sync::OnceLock::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationError {
    EmptyName,
    EmptyOwner,
    Conflict {
        name: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
    /// Two DIFFERENT Rust types reduce to the same [`wire_type_identity`].
    ///
    /// This is what keeps v20's narrower identity sound. The fingerprint hashes
    /// the type's final segment so that relocating a type is not a wire-format
    /// change — and that is only truthful while final segments are unique. Two
    /// crates each registering a `Cooldown` would hash equal, and a peer that
    /// had them the other way round would be declared compatible.
    ///
    ///  registering ONE type under several stable names is not this. The whole
    /// point of a stable name is that it identifies the registration; 39 of the
    /// live rows do exactly that, and they carry identical type names.
    TypeIdentityCollision {
        identity: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
}

impl fmt::Display for RollbackRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "rollback registration name must not be empty"),
            Self::EmptyOwner => write!(f, "rollback registration owner must not be empty"),
            Self::Conflict {
                name,
                existing,
                incoming,
            } => write!(
                f,
                "conflicting rollback registration '{name}': existing {existing:?}, incoming {incoming:?}"
            ),
            Self::TypeIdentityCollision {
                identity,
                existing,
                incoming,
            } => write!(
                f,
                "two different types share the rollback wire identity '{identity}', which the \
                 schema fingerprint cannot tell apart: existing {existing:?}, incoming {incoming:?}. \
                 Since v20 the fingerprint hashes a type's FINAL SEGMENT so that moving a type \
                 between crates or modules is not a wire-format change, and that stays sound only \
                 while final segments are unique. Rename one of the two types."
            ),
        }
    }
}

impl std::error::Error for RollbackRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationOutcome {
    /// The descriptor was inserted and the active GGRS host should install its
    /// runtime snapshot/checksum machinery.
    Inserted,
    /// The exact descriptor was already present.
    Idempotent,
    /// The descriptor was inserted for schema/content identity, but this host
    /// does not run GGRS and therefore must not install rollback machinery.
    RecordedOnly,
}

/// The part of a type's name that a CARVE leaves alone.
///
///  the final segment, and not the module path below the crate, which is what the answer
/// was until the diff it cited was read.
///
/// Every path INSIDE the name is shortened, not only the outermost one, so a
/// generic keeps its constructor: `Vec<foo::Bar>` is `Vec<Bar>` and not `Bar>`.
/// No registration is generic today; taking a single `rsplit` would quietly give
/// `Vec<X>` and `VecDeque<X>` one identity, and this is a hash whose entire job
/// is telling wire formats apart.
fn wire_type_identity(type_name: &str) -> String {
    let mut out = String::with_capacity(type_name.len());
    let mut path = String::new();
    for ch in type_name.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' {
            path.push(ch);
        } else {
            out.push_str(final_segment(&path));
            path.clear();
            out.push(ch);
        }
    }
    out.push_str(final_segment(&path));
    out
}

fn final_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

impl RollbackRegistry {
    pub fn try_register(
        &mut self,
        descriptor: RollbackRegistrationDescriptor,
    ) -> Result<RollbackRegistrationOutcome, RollbackRegistrationError> {
        // The protocol every canonical registry answers, read from
        // `ambition_registry_core` (2026-09-03): blank identity refused by
        // name, a second registration classified as idempotent or conflict,
        // never silently replaced. The wire-identity collision below is this
        // registry's own extra rule on top of it.
        if let Err(empty) = ambition_registry_core::require_non_empty(&[
            ("name", &descriptor.name),
            ("owner", &descriptor.owner),
        ]) {
            return Err(match empty.field {
                "name" => RollbackRegistrationError::EmptyName,
                _ => RollbackRegistrationError::EmptyOwner,
            });
        }
        match ambition_registry_core::classify(self.entries.get(&descriptor.name), &descriptor) {
            ambition_registry_core::Classification::Idempotent => {
                return Ok(RollbackRegistrationOutcome::Idempotent);
            }
            ambition_registry_core::Classification::Conflict { existing } => {
                return Err(RollbackRegistrationError::Conflict {
                    name: descriptor.name.clone(),
                    existing: existing.clone(),
                    incoming: descriptor,
                });
            }
            ambition_registry_core::Classification::New => {}
        }
        // What keeps v20's narrower identity sound. The fingerprint hashes
        // [`wire_type_identity`] so that relocating a type is not a wire-format
        // change; two crates each registering a `Cooldown` would then hash equal,
        // and a peer that had the two the other way round would be declared
        // compatible with this one. The duplicate-NAME refusal above does not
        // reach it — these arrive under different stable names, which is exactly
        // the case that looks legitimate.
        let identity = wire_type_identity(&descriptor.type_name);
        let collision = self
            .entries
            .values()
            .find(|existing| {
                existing.type_name != descriptor.type_name
                    && wire_type_identity(&existing.type_name) == identity
            })
            .cloned();
        if let Some(existing) = collision {
            return Err(RollbackRegistrationError::TypeIdentityCollision {
                identity,
                existing,
                incoming: descriptor,
            });
        }
        self.entries.insert(descriptor.name.clone(), descriptor);
        // The memo describes the entry set that just changed.
        self.fingerprint = std::sync::OnceLock::new();
        Ok(RollbackRegistrationOutcome::Inserted)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &RollbackRegistrationDescriptor> {
        self.entries.values()
    }

    /// Stable human-readable representation; byte-identical under equivalent
    /// plugin/registration insertion orders.
    pub fn deterministic_dump(&self) -> String {
        let rows: Vec<String> = self
            .entries
            .values()
            .map(|entry| {
                ambition_registry_core::canonical_row(&[
                    &entry.name,
                    &entry.owner,
                    entry.kind.canonical_name(),
                    &entry.type_name,
                    &entry.detail,
                ])
            })
            .collect();
        ambition_registry_core::canonical_section(
            Some(&format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}")),
            rows.iter().map(String::as_str),
        )
    }

    /// What the schema actually IS, with every organisational label removed.
    ///
    /// [`Self::deterministic_dump`] carries `owner` and the type's full path because a human
    /// reading a conflict wants to know which module registered a thing and where the type
    /// lives.
    ///
    /// Both moves require the schema fingerprint to stay unchanged — which was impossible while
    /// the fingerprint hashed who registered a row. `owner` left in v5; [`wire_type_identity`]
    /// is the second half of that decision, in v20.
    pub fn schema_dump(&self) -> String {
        let rows: Vec<String> = self
            .entries
            .values()
            // ⛔⛤ THE ONLY FILTER IN THIS DUMP, AND IT IS THE PEER QUESTION.
            // `deterministic_dump` keeps every entry because it describes what
            // this build registered; this one describes what a peer can observe,
            // and a registration outside the schema identity is not part of
            // that. Measured 2026-09-16: without this, `--features causal` moved
            // the fingerprint (494 -> 497 rows) for a simulation that is
            // mechanically identical, so two such peers would refuse each other
            // for no mechanical reason.
            .filter(|entry| entry.kind.in_peer_schema_identity())
            .map(|entry| {
                ambition_registry_core::canonical_row(&[
                    &entry.name,
                    entry.kind.canonical_name(),
                    &wire_type_identity(&entry.type_name),
                    &entry.detail,
                ])
            })
            .collect();
        ambition_registry_core::canonical_section(
            Some(&format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}")),
            rows.iter().map(String::as_str),
        )
    }

    /// Which of these requirements is NOT installed.
    ///
    /// A capability offers its rollback state and the composition installs it,
    /// which keeps the capability's dependency closure to foundations. The hole
    /// that leaves is that nothing forces the composition to accept the offer —
    /// and a skipped registration is a DESYNC, not a missing feature.
    ///
    /// This closes it the way the content compiler closes the same shape: the
    /// obligation is declared next to the thing that has it
    /// ([`ambition_platformer2d_core::snapshot::RequiredRollbackState`]) and the
    /// assembler can refuse when it is unmet.
    ///
    ///  it checks the OWNER too. A name registered by somebody else is not
    /// this capability's state — two capabilities may reasonably both want a
    /// `cooldown`, and only the owner distinguishes them.
    pub fn missing_required_state<'a>(
        &self,
        required: &'a [ambition_platformer2d_core::snapshot::RequiredRollbackState],
    ) -> Vec<&'a ambition_platformer2d_core::snapshot::RequiredRollbackState> {
        required
            .iter()
            .filter(|req| {
                !self
                    .entries
                    .values()
                    .any(|entry| entry.name == req.name && entry.owner == req.owner)
            })
            .collect()
    }

    /// ⛔⛔ MEMOISED SINCE 2026-08-29, BECAUSE IT WAS COSTING 292us OF EVERY FRAME.
    /// `enforce_session_contract` calls this once per frame to notice a schema
    /// that changed under a live session. Uncached that meant building the entire
    /// schema DUMP as a ~40KB `String` — the same table
    /// `rollback_schema_baseline.txt` records — and blake3ing it, 60+ times a
    /// second, to detect a change that can only happen when `entries` is mutated:
    /// **8.29s of a four-minute hardware run, the second largest recurring zone
    /// in the trace.**
    ///
    /// ⭐ The VALUE is unchanged, which is the point — this is a pure memo, so
    /// every existing schema/baseline test still pins the same fingerprint and
    /// no new correctness surface appears. `try_register` clears it.
    pub fn schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        if let Some(memo) = self.fingerprint.get() {
            return memo.clone();
        }
        let computed = self.compute_schema_fingerprint();
        // A racing caller may have filled it first; either value is identical.
        let _ = self.fingerprint.set(computed.clone());
        computed
    }

    /// HAS THIS REGISTRY ALREADY COMPUTED ITS FINGERPRINT?
    ///
    /// ⭐ THE INSTRUMENT THAT CATCHES A DEFEATED MEMO. The memo is per-instance
    /// and a clone starts empty, so a hot-path caller that reads the fingerprint
    /// off a CLONE recomputes it every single time while every value-based test
    /// still passes. Asking the world's own registry whether it is memoised is
    /// the one question that separates "the cache exists" from "the cache is the
    /// thing being read". Callers outside a test have no reason for it.
    pub fn fingerprint_is_memoised(&self) -> bool {
        self.fingerprint.get().is_some()
    }

    fn compute_schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ambition.ggrs-rollback-schema\0");
        hasher.update(&GGRS_ROLLBACK_SCHEMA_VERSION.to_le_bytes());
        let dump = self.schema_dump();
        hasher.update(&(dump.len() as u64).to_le_bytes());
        hasher.update(dump.as_bytes());
        SnapshotSchemaFingerprint::from_bytes(*hasher.finalize().as_bytes())
    }
}

pub fn descriptor<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: &'static str,
) -> RollbackRegistrationDescriptor {
    descriptor_owned::<T>(owner, name, kind, detail.to_string())
}

/// [`descriptor`] for a detail this crate COMPOSES rather than quotes.
///
/// The recorded `detail` is two halves: how the value is stored (the backend's
/// half — "bevy_ggrs clone snapshot") and what the checksum sees (the domain's
/// half). A domain that registers its own state through
/// [`ambition_platformer2d_core::snapshot::RollbackRegistrar`] supplies only the
/// second half, precisely so a crate with no `bevy_ggrs` dependency never has to
/// write the word; this joins them back into the exact string the schema baseline
/// records.
pub fn descriptor_owned<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: String,
) -> RollbackRegistrationDescriptor {
    RollbackRegistrationDescriptor {
        name: name.to_string(),
        owner: owner.to_string(),
        kind,
        type_name: std::any::type_name::<T>().to_string(),
        detail,
    }
}

/// Record one schema descriptor on an app, independent of the active rollback backend.
///
/// Backend installation deliberately has a separate idempotence authority: a row may
/// already have been recorded by a capability plugin before a concrete rollback host
/// installs its typed snapshot machinery. Therefore callers must not interpret an
/// `Idempotent` schema row as evidence that a backend registration already exists.
pub fn record_descriptor(
    app: &mut App,
    descriptor: RollbackRegistrationDescriptor,
) -> RollbackRegistrationOutcome {
    app.init_resource::<RollbackRegistry>();
    app.world_mut()
        .resource_mut::<RollbackRegistry>()
        .try_register(descriptor)
        .unwrap_or_else(|error| panic!("{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ THE GUARD ON THE FINGERPRINT MEMO, AND IT IS THE WHOLE REASON THE MEMO
    /// IS SAFE. `schema_fingerprint` caches into a `OnceLock`; if `try_register`
    /// ever stops clearing it, this registry would keep reporting a fingerprint
    /// its own entries no longer justify — and a stale schema fingerprint is
    /// exactly what `enforce_session_contract` exists to catch, so the failure
    /// would be silent AND load-bearing.
    #[test]
    fn registering_after_reading_the_fingerprint_changes_it() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("a", "owner", "detail"))
            .expect("first registration");
        // Read it FIRST, so the memo is populated before the mutation.
        let before = registry.schema_fingerprint();
        // ...and reading twice must agree, or the memo is not a memo.
        assert_eq!(
            before,
            registry.schema_fingerprint(),
            "two reads with no mutation between them disagreed"
        );

        registry
            .try_register(entry("b", "owner", "detail"))
            .expect("second registration");
        assert_ne!(
            before,
            registry.schema_fingerprint(),
            "a registration after the fingerprint was memoised did not change it — \
             the memo is stale, and a stale schema fingerprint is what \
             enforce_session_contract cannot afford to be wrong about"
        );
    }

    /// A CLONE must not inherit a memo, because a clone that is then mutated
    /// would otherwise answer for entries it no longer has.
    #[test]
    fn a_clone_agrees_with_its_source_and_still_notices_its_own_changes() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("a", "owner", "detail"))
            .expect("registration");
        let _ = registry.schema_fingerprint();
        let mut copy = registry.clone();
        assert_eq!(
            registry.schema_fingerprint(),
            copy.schema_fingerprint(),
            "a clone of the same entries must fingerprint the same"
        );
        copy.try_register(entry("b", "owner", "detail"))
            .expect("registration on the clone");
        assert_ne!(
            registry.schema_fingerprint(),
            copy.schema_fingerprint(),
            "the clone changed and still reported the source's fingerprint"
        );
    }

    fn entry(name: &str, owner: &str, detail: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: owner.to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: "test::Type".to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn schema_is_insertion_order_independent() {
        let mut a = RollbackRegistry::default();
        a.try_register(entry("z", "provider-b", "second")).unwrap();
        a.try_register(entry("a", "provider-a", "first")).unwrap();

        let mut b = RollbackRegistry::default();
        b.try_register(entry("a", "provider-a", "first")).unwrap();
        b.try_register(entry("z", "provider-b", "second")).unwrap();

        assert_eq!(a.deterministic_dump(), b.deterministic_dump());
        assert_eq!(a.schema_fingerprint(), b.schema_fingerprint());
    }

    #[test]
    fn identical_registration_is_idempotent() {
        let descriptor = entry("same", "provider", "same");
        let mut registry = RollbackRegistry::default();
        assert_eq!(
            registry.try_register(descriptor.clone()).unwrap(),
            RollbackRegistrationOutcome::Inserted
        );
        assert_eq!(
            registry.try_register(descriptor).unwrap(),
            RollbackRegistrationOutcome::Idempotent
        );
        assert_eq!(registry.descriptors().count(), 1);
    }

    fn typed_entry(name: &str, type_name: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: "test-owner".to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: type_name.to_owned(),
            detail: "test-only descriptor".to_owned(),
        }
    }

    fn registry_of(rows: &[(&str, &str)]) -> RollbackRegistry {
        let mut registry = RollbackRegistry::default();
        for (name, type_name) in rows {
            registry.try_register(typed_entry(name, type_name)).unwrap();
        }
        registry
    }

    /// Where a type LIVES is not part of the wire format (v20).
    ///
    /// Only the final segment survived either move.
    #[test]
    fn relocating_a_type_leaves_the_fingerprint_alone() {
        let before = registry_of(&[
            (
                "actor.anim_override",
                "ambition_platformer2d_actor_monolith::features::ecs::actor_clusters::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_actor_monolith::avatar::components::PlayerBlinkCameraState",
            ),
        ]);
        let after = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState",
            ),
        ]);
        assert_eq!(
            before.schema_fingerprint(),
            after.schema_fingerprint(),
            "moving a rollback-registered type to another crate and another \
             module moved the schema fingerprint. Nothing a peer can observe \
             changed, so two peers running byte-identical snapshot logic would \
             refuse to agree — which makes every carve in the decomposition \
             campaign a netplay compatibility break."
        );

        // POISON. Without it this test is equally green for a fingerprint that
        // hashes nothing about the type at all, and dropping `type_name` from
        // the dump entirely was a real alternative — it costs the last signal
        // that a DIFFERENT Rust type got registered under an existing name.
        let renamed = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkEaseState",
            ),
        ]);
        assert_ne!(
            after.schema_fingerprint(),
            renamed.schema_fingerprint(),
            "a stable name that changed which TYPE it registers left the \
             fingerprint alone, so the dump is no longer hashing the type in \
             any form."
        );
    }

    /// What makes the narrower identity sound.
    ///
    /// Two `Cooldown`s in two crates hash equal once the final segment is the
    /// identity, so a peer holding them the other way round would be declared
    /// compatible. The second half asserts the guard is not merely strict: one
    /// type registered under two stable names is the ordinary case, and 39 of
    /// the live rows are it.
    #[test]
    fn two_types_sharing_a_final_segment_are_rejected_and_one_type_twice_is_not() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(typed_entry(
                "ability.cooldown",
                "ambition_combat::ability::Cooldown",
            ))
            .unwrap();

        let error = registry
            .try_register(typed_entry(
                "weapon.cooldown",
                "ambition_projectiles::weapon::Cooldown",
            ))
            .unwrap_err();
        assert!(
            matches!(
                error,
                RollbackRegistrationError::TypeIdentityCollision { .. }
            ),
            "two different types whose names end in `Cooldown` were accepted, \
             and the fingerprint cannot tell them apart: {error}"
        );

        registry
            .try_register(typed_entry(
                "ability.cooldown_mirror",
                "ambition_combat::ability::Cooldown",
            ))
            .expect(
                "registering ONE type under a second stable name is not a \
                 collision — the stable name is what identifies a registration, \
                 and refusing this would reject 39 of the live rows",
            );
    }

    #[test]
    fn conflicting_registration_is_transactional() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("same", "provider-a", "old"))
            .unwrap();
        let before = registry.deterministic_dump();
        let error = registry
            .try_register(entry("same", "provider-b", "new"))
            .unwrap_err();
        assert!(matches!(error, RollbackRegistrationError::Conflict { .. }));
        assert_eq!(registry.deterministic_dump(), before);
    }
}
