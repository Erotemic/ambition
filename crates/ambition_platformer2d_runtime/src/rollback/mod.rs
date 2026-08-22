//! Backend-neutral rollback schema composition.
//!
//! Gameplay domains own their concrete declarations through
//! [`RollbackRegistrar`].  This module composes those declarations into stable
//! schema metadata for every simulation host. A concrete rollback backend may
//! call the same composition function with its own registrar to install storage,
//! checksums, mapping, and load behavior without making the generic runtime
//! depend on that backend.

use ambition_platformer2d_core::snapshot::{checksum_bytes, RollbackRegistrar};
use bevy::prelude::*;

pub mod registrar;
pub mod registry;

pub use registrar::SchemaRollbackRegistrar;
pub use registry::*;

/// Install the host-independent typed rollback schema used by prepared-content
/// identity. This plugin records metadata only; rollback hosts install their
/// backend machinery through the same declarations in their owning crate.
pub struct AmbitionRollbackSchemaPlugin;

impl Plugin for AmbitionRollbackSchemaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RollbackRegistry>();
        let mut registrar = SchemaRollbackRegistrar::new(app);
        register_engine_rollback_state(&mut registrar);
    }
}

const ENGINE: &str = "ambition_platformer2d_runtime";

pub fn register_engine_rollback_state(registrar: &mut impl RollbackRegistrar) {
    use ambition_platformer2d_core::body_clusters as bc;

    // **DOMAIN-OWNED ROLLBACK DECLARATIONS.** The composition supplies one backend-neutral
    // registrar; each capability names its own concrete types and projections.
    // This is composition, not a type census: adding state to an existing domain
    // edits only that domain, and the runtime contains no gameplay type paths.
    ambition_encounter::register_rollback_state(registrar);
    ambition_combat::register_rollback_state(registrar);
    ambition_platformer2d_actor_monolith::register_rollback_state(registrar);
    ambition_characters::register_rollback_state(registrar);
    ambition_time::register_rollback_state(registrar);
    ambition_boss_encounter::register_rollback_state(registrar);
    ambition_conversation::register_rollback_state(registrar);
    ambition_sprite_sheet::register_rollback_state(registrar);
    ambition_platformer2d_shared_tangle::register_rollback_state(registrar);
    ambition_vfx::register_rollback_state(registrar);
    ambition_items::register_rollback_state(registrar);
    // Portal rollback state exists only when the portal capability is enabled.
    #[cfg(feature = "portal")]
    ambition_portal2d::register_rollback_state(registrar);
    ambition_cutscene::register_rollback_state(registrar);
    ambition_projectiles::register_rollback_state(registrar);
    ambition_sim_view::register_rollback_state(registrar);
    ambition_platformer2d_world::rooms::register_gate_portal_rollback_state(registrar);

    // Rollback participation. These anchors cover the canonical session root,
    // every simulated body, projectile-only entities, encounter authorities,
    // and any semantic-identity entity that does not fit those families.
    //
    // ⚠ actor-owned anchors now live in
    // `ambition_platformer2d_actor_monolith::register_rollback_state`; only
    // foundation/runtime-owned rows remain below. In-flight strike volumes (moveset melee
    // windows, DamageBox effects, world AOEs).

    // Canonical live-session root. Authored definitions are immutable and bound
    // by PreparedContentIdentity; only mutable selection/cursor state rewinds.
    //
    // ⚠ actor-owned members moved to
    // `ambition_platformer2d_actor_monolith::register_rollback_state`; the
    // geometry is `ambition_platformer2d_core`'s and stays.
    registrar.rollback_component_clone::<ambition_platformer2d_core::RoomGeometry>(
        ENGINE,
        "root.geometry",
    );

    // Global authoritative resources.
    registrar.rollback_resource_canonical::<ambition_time::SimTick>(ENGINE, "resource.sim_tick")
        // **The match activation latch.** (AA2 / AC2)
        //
        // Published from inside the sim schedule on the tick the last seat is
        // filled, and it GATES two behaviours: seating returns early while it
        // exists, and the countdown treats it as proof the match is live. Left
        // unregistered, a rewind across activation restored the fighters — or
        // un-spawned them — and left the latch pointing at a future in which
        // they existed, so seating refused to rebuild the roster it had just
        // lost while the countdown carried on.
        //
        // Correct because `bevy_ggrs` restores ABSENCE as well as value: a
        // rewind to before activation REMOVES this, seating sees no match, and
        // the roster is reconstructed from the same authored inputs.
        //
        // ⚠ **this registration is load-bearing, and there is now a fixture that
        // says so.** Remove it and
        // `rollback_match_activation::a_rewind_across_the_activation_frame_reconstructs_the_same_match`
        // fails on "the restored world was not pre-activation after all", while
        // the two older tests in that file stay green — they cannot reach a
        // pre-activation frame, and their docstring admits it.
        //
        // AA2's lifecycle half is CLOSED as of the seating transaction: seats
        // are resolved and validated before any is built, then constructed in
        // one command flush with this latch, so there is no "between two seats"
        // state a rewind can land in.
        .rollback_resource_canonical::<ambition_time::WorldTime>(ENGINE, "resource.world_time")
        .rollback_resource_canonical::<ambition_platformer2d_world::collision::MovingPlatformSet>(
            ENGINE,
            "resource.moving_platform_set",
        )
        // **The gate portals' live phase** is registered too — but NOT here, and
        // not by this crate. `ambition_platformer2d_world` owns both halves of it
        // now: `GatePortalPhases` documents why an integrator whose input rewinds
        // must rewind with it, and `register_gate_portal_rollback_state` (called
        // with the domain-owned declarations at the top of this function) performs the
        // registration through the floor's `RollbackRegistrar` vocabulary. ⛔ it
        // carries a VALUE projection, not a presence probe — see that function.
        .rollback_resource_clone::<crate::InputStreamRecorder>(
            ENGINE,
            "resource.input_stream_recorder",
        )
        .rollback_resource_canonical::<ambition_time::ClockState>(ENGINE, "resource.clock_state")
        .rollback_resource_clone::<ambition_persistence::save::AmbitionGameSave>(
            ENGINE,
            "resource.sandbox_save",
        )
        .rollback_resource_clone::<ambition_persistence::quest::registry::QuestRegistry>(
            ENGINE,
            "resource.quest_registry",
        )
        // G2b: id → live encounter entity, remapped on every load. A presence probe over a
        // singleton resource sees "still present"; this sees an id pointing at the wrong
        // encounter. Folded in the map's own (sorted) key order, so a permutation between two
        // ids is a difference. G2b: probed through the possessed/home pair's stable identities.
        // A presence probe over a singleton resource sees "still present" and nothing else —
        // and a restore that exchanged the possessed body for the home avatar would invert the
        // whole possession while folding the same census, which is why the ORDER of the pair is
        // folded in. Cross-frame FIFO: produced in `GameplayEffects`, drained in
        // `EncounterSimulation` — which is ordered EARLIER, so the queue is non-empty across a
        // save boundary and a rewind would otherwise replay switch activations the confirmed
        // timeline already applied . Latent until something mutates them in-session, but a
        // rewind that keeps a predicted faction flip would be a silent desync — registered
        // ahead of the first mutating feature (Phase 5 resource-coverage pass). Cross-frame
        // FIFO: victim-side hits staged in `Combat`, drained by `apply_player_hit_events` in
        // the NEXT frame's `PlayerSimulation` — same shape as `SwitchActivationQueue` above.
;

    // Core body state.
    registrar
        .rollback_component_canonical::<bc::BodyAbilities>(ENGINE, "body.abilities")
        .rollback_component_canonical::<bc::BodyGroundState>(ENGINE, "body.ground")
        .rollback_component_canonical::<bc::BodyWallState>(ENGINE, "body.wall")
        .rollback_component_canonical::<bc::BodyJumpState>(ENGINE, "body.jump")
        .rollback_component_canonical::<bc::BodyDashState>(ENGINE, "body.dash")
        .rollback_component_canonical::<bc::BodyFlightState>(ENGINE, "body.flight")
        .rollback_component_canonical::<bc::BodyBlinkState>(ENGINE, "body.blink")
        .rollback_component_canonical::<bc::BodyDodgeState>(ENGINE, "body.dodge")
        .rollback_component_canonical::<bc::BodyShieldState>(ENGINE, "body.shield")
        .rollback_component_canonical::<bc::BodyOffense>(ENGINE, "body.offense")
        .rollback_component_canonical::<bc::BodyLifetime>(ENGINE, "body.lifetime")
        .rollback_component_canonical::<bc::BodyActionBuffer>(ENGINE, "body.action_buffer")
        .rollback_component_canonical::<bc::BodyBaseSize>(ENGINE, "body.base_size")
        .rollback_component_canonical::<bc::SweepSample>(ENGINE, "body.sweep_sample")
        .rollback_component_canonical::<bc::BodyMana>(ENGINE, "body.mana");
    // In-flight strike volumes — the components on the `entity:hitbox` family (see the
    // require_rollback anchor above). G2b: probed through the OWNER's stable identity, paired with
    // the hitbox's own — the same treatment `ProjectileOwner` has. A strike volume remapped onto
    // the wrong body damages the wrong faction's targets, and a presence count could not tell that
    // from a correct restore. The victims this strike has ALREADY hit. Losing one from the set is a
    // sustained overlap re-hitting a body it already hit, which is exactly the kind of one-frame
    // difference the aggregate reports as a desync with no name attached. G2b: probed through the
    // fired victims' stable identities. A presence count sees the component and nothing of WHO is
    // in the set, so a remap redirecting one victim to the wrong body changes no census — and the
    // visible consequence is a sustained overlap re-firing an on-hit at a body it has already fired
    // at.

    // A live match's per-body state. Registered together because they are one decision — match
    // activation — landing on a body, and a rewind that kept some and dropped others would
    // produce a fighter that is half in the match. S4 — the stocks loop's own state. A stock
    // count that is NOT rollback state un-spends itself on a rewind: the body comes back and
    // the count does not, so a fighter loses the same stock twice or never loses it at all.
    // Elimination is the same fact one step later, and a rewind that restores a fighter while
    // leaving it eliminated is a body standing in a match nothing will ever let it play. The
    // "already announced" latch for a stocks match's outcome.
    registrar
        .rollback_component_canonical::<ambition_platformer2d_core::geometry::CenteredAabb>(
            ENGINE,
            "actor.centered_aabb",
        )
        .rollback_component_canonical::<bc::BodyModeState>(ENGINE, "actor.body_mode")
        .rollback_component_canonical::<bc::BodyLedgeState>(ENGINE, "actor.ledge")
        .rollback_component_canonical::<ambition_platformer2d_core::MotionModel>(
            ENGINE,
            "actor.motion_model",
        )
        .rollback_component_canonical::<bc::BodyComboTrace>(ENGINE, "actor.combo_trace")
        .rollback_component_canonical::<ambition_time::ProperTimeScale>(
            ENGINE,
            "actor.proper_time_scale",
        );

    // Complete rollback entity shapes. The old custom restore engine only patched a narrow
    // state subset and left the remaining components stale. GGRS recreates entities, so every
    // marker, authored/config component, and mutable controller that a recreated actor needs is
    // explicitly stored. The transformation beat's VALUE, not just its participation. The
    // anchor declaration above only carries the participation marker; without this the beat's
    // `remaining` and — worse — the `was_invulnerable` it borrowed never restore, so a rewind
    // into the middle of a transformation can leave a body permanently untouchable. The REQUEST
    // is state for the same reason. The pose pin the beat and the snake shell both write.
    // Snapshotting the slot itself is owner-agnostic and therefore correct for both: a restore
    // reinstates whatever pin was actually in force. Deriving it from beat state instead would
    // fight the shell for a component it does not own. G2b: a rig IS its slot→limb map, and the
    // map is remapped on every load. A presence count sees "one rig, still here" while the left
    // hand hangs off the right shoulder.
    //
    // The first repair projected `limbs.values()` into the entity-SET census and
    // claimed the slot order came with it. It did not: that census folds targets
    // with a commutative sum, so the two hands trading slots is the same multiset
    // and the same digest — the probe was blind to the one failure the comment
    // named. The MAP census folds each slot's discriminant
    // against its limb's identity, which is what makes an exchange visible.
    // G2b: which HOST this limb belongs to. Remapped onto the wrong body, the
    // limb station-keeps around a stranger and strikes where that stranger is.
    // G2b: who is riding. A remap that seats the wrong rider locks a body to a
    // mount it never boarded, and the count of occupied slots is unchanged.
    // The UN-GRANTED baseline the live `ActionSet` / `ActorMoveset` are a pure
    // function of (`identity + worn equipment`). Registering the two derived
    // halves and not their base is the `WornEquipment` oversight again: a rewind
    // restored the live kit but left the baseline at whatever an abandoned future
    // derived, so the next `reconcile_equipment_grants` — fired by any armor
    // spend or pickup — recomputed the live kit from the WRONG base and the
    // resimulation stopped matching. That is precisely what
    // `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`
    // caught: it went red when the protagonist's re-rig changed which kit the
    // overlay derives, and stayed red because nothing rewound the base.
    // The MEMO that says the identity baseline above is current for this body.
    //
    // It looks like a cache and is not safe to treat as one. `apply_worn_character_gameplay`
    // re-derives a persona when this record disagrees with the body's worn id or
    // the cast generation — so a rewind that restores an EARLIER `WornCharacter`
    // while leaving this at the abandoned future's id makes the record say
    // "already applied" about a kit the body no longer wears. The derive skips,
    // and the resimulation runs a fighter with somebody else's moves.
    //
    // Exactly the `IdentityKit` oversight one entry up, one level further out:
    // registering a derived value and not the record of what derived it. Found by
    // `every_component_in_a_boss_arena_is_registered_derived_or_waived` within
    // minutes of the component existing.
    // PROBED over both fields, because a desync here is silent by construction:
    // the wrong baseline does not corrupt a number, it makes the persona derive
    // SKIP — and a presence-only probe would see the component and nothing about
    // which cast it claims. The id is hashed rather than counted so two bodies
    // that swapped identities during a rewound frame do not read as identical.
    // **THE PROJECTION'S OWN MEMO**, and the third time this exact shape has had
    // to be registered rather than assumed derived — after `IdentityKit` and
    // `PersonaBaseline` directly above.
    //
    // `project_prepared_character_definitions` early-exits when this record
    // agrees with the body's worn id and the cast generation, and it also
    // records what it GRANTED so it can retract exactly that. Leave it out of
    // rollback and a rewind restores an earlier `WornCharacter` while the memo
    // still claims the abandoned future's id: the projection skips, and the body
    // resimulates wearing a kit — hurtboxes, movement tuning, sprite-posed body
    // — that its identity no longer asks for. `granted` makes it worse than a
    // stale read, because retraction is driven from it: the wrong record retracts
    // the wrong facts.
    //
    // ⚠ it was ALWAYS unregistered; nothing caught it because no tested room had
    // a body carrying one. Registering the protagonist's own incarnations as
    // characters put it on the PLAYER, so it appeared in every room
    // at once and three coverage gates went red together. The component did not
    // become dangerous that day — it became visible.
    //
    // PROBED over the id, the generation AND the grant set, for the reason
    // `PersonaBaseline` is: a desync here is silent by construction. It does not
    // corrupt a number, it makes a derive SKIP, and a presence-only probe would
    // see the component and nothing about what it claims.
    // The quad's placement travels with its size: both are re-derived per pose
    // from the sheet, so restoring one without the other would leave a body
    // drawn at the right scale in the wrong place until the next pose change.
    // The pose→geometry binding itself. Constant per body, but a body the
    // rewind RE-CREATES must come back still bound to its sheet — otherwise it
    // silently reverts to whatever box it was spawned with and never recovers.
    // A body's pose clock ACCUMULATES, and its elapsed value selects which hurtbox
    // keyframe is live -- so a rewind that lost it would resolve a body's damageable
    // silhouette from a different instant than the confirmed timeline did.
    // Authored and immutable at runtime, but bevy_ggrs DESTROYS AND RECREATES
    // rollback entities: unregistered, the doc is simply absent afterwards and the
    // body silently reverts to its sprite-derived compatibility box forever. Same
    // reasoning as `SwitchFeature`.
    // World features that MUTATE during play.
    // Without these a brick broken in an abandoned future stays broken through
    // the rewind, and the crumble/respawn countdowns resume from predicted
    // values instead of confirmed ones.
    // A chest's PAYLOAD AND STATE, and the marker that says it was opened.
    //
    // Found by A19's unswept-population sweep; no room the sweep visited had ever
    // contained a chest.
    // Switch liveness. The `SwitchActivated` MESSAGE is cleared on rollback, but
    // the state that message produced was not rewound — so a switch flipped in an
    // abandoned future stayed on.
    // The switch's authored payload. Immutable at runtime, but bevy_ggrs
    // DESTROYS AND RECREATES rollback entities — anything not registered is
    // simply absent on the recreated entity, so an unregistered authored
    // component silently strips the switch of its identity after a rewind.
    // Same reasoning for the room-visual lifecycle tag: its siblings
    // (`RoomScopedEntity`, `SessionScopedEntity`) are registered, and losing the
    // tag on recreation would leak the entity past its room's teardown.
    //
    // Same reasoning once more, for the tag on the player's body. The portal host asks
    // `With<PlayerVisual>, Without<PortalSceneBody>` to decide what to tag as a portal scene
    // body, so a recreated player that came back without the tag would stop being seen by
    // portal staging entirely. The explicit world-pogo contributor marker, beside the body
    // policy and pogo volumes that were already registered. Same reasoning as `PlayerVisual`:
    // bevy_ggrs recreates the entity, and a stand-to-crumble surface that loses
    // `PogoTargetContributor` after rewind silently stops being a world rebound surface. Body
    // pogo eligibility itself is data (`PogoPolicy` + volumes), not a second marker.
    //
    // Their registered siblings sat two lines away this whole time. Same recreated-entity
    // reasoning as `SwitchFeature` above. The collected latch. Unregistered, a rewind past a
    // collection could not REMOVE it: the resimulated pickup started already-collected, the
    // magnet skipped it (`Without<Collected>`), and its registered `CenteredAabb` froze while
    // the first pass had it moving — the exit oracle's first checksum divergence
    // (combat_calibration_lab, frames 10–12). The mid-toss collection lock (a scattered ring's
    // uncollectible window), registered for the SAME reason `Collected` is: a rewind past the
    // lock's removal must restore it, or the resimulated ring would be collectible a frame
    // early — the magnet/collect guards read it, so it is authoritative. Which sheet a pickup
    // is drawn with.
    registrar
        .rollback_component_clone::<ambition_platformer2d_core::body_clusters::AbilityBase>(
            ENGINE,
            "body.ability_base",
        )
        // Runtime-staged actors need this marker after restore so presentation can
        // rediscover them. `SfxSource` must also survive for projectiles because it
        // is stamped at spawn and may outlive the firing body; probe it by value.
        .rollback_component_clone_probed::<ambition_sfx::BodyPresentationSource>(
            ENGINE,
            "presentation.body_source",
            |source| checksum_bytes(source.id().as_str().as_bytes()),
        )
        // The marker that says the per-tick derivation OWNS that source and may retract
        // it. Losing it across a restore would strand a body's source: the derivation
        // stops maintaining what it can no longer recognise as its own.
        .rollback_component_clone::<ambition_sfx::DerivedPresentationSource>(
            ENGINE,
            "presentation.body_source_derived",
        )
        // Portal-gun cooldown, in-flight shot, and pickup arm timers affect future
        // transit/grab decisions and therefore must rewind with portal state.
        .rollback_component_clone::<bevy::prelude::Name>(ENGINE, "entity.name")
        .rollback_component_clone::<bevy::prelude::Transform>(ENGINE, "entity.transform");

    // These values are guaranteed to be republished before any downstream
    // consumer in each GGRS frame, so storing them would duplicate authority.
    // A per-tick MIRROR of the item's own body, not a second authority:
    // `sync_ground_items_to_transitable` overwrites pos/vel/half_extent from the
    // authoritative `GroundItem` (registered state) before portal core reads it, and
    // `sync_transitable_to_ground_items` mirrors the possibly-teleported result
    // straight back. Snapshotting it would give one body two restorable positions.
    //
    // ⚠ this DECLARED-DERIVED group lost its actor-owned head to
    // `ambition_platformer2d_actor_monolith::register_rollback_state`; the rest belongs to
    // `ambition_characters`.
    registrar.declare_rollback_derived_component::<bevy::prelude::GlobalTransform>(
        ENGINE,
        "derived.global_transform",
        "Bevy transform propagation rebuilds it from Transform and hierarchy",
    )
    // AE6. Derived, not state: `project_combat_rules` rebuilds it in WorldPrep
    // every tick from the match's declaration folded over the world's baseline,
    // both of which outlive any rollback window — the declaration is route
    // lifecycle (`Update`, outside the sim) and the baseline is authored tuning.
    // Registering it as STATE would be the borrow again: a rewind would restore
    // a rules value independently of the declaration that produced it, and the
    // two could then disagree for a frame.
;
    // Scope, projectile, and encounter state. Derived state: one maintenance path, never
    // restore-only repair code. `ProjectileOwner(Entity)` is now the single firing-occurrence
    // reference, restored/remapped as entity-bearing rollback state rather than re-derived from
    // presentation or configuration identity. This was DECLARED DERIVED, on the promise that
    // `heal_projectile_owners` re-resolves it from `SpawnOrigin::Dynamic { parent }`. The promise
    // is not kept: that system's query requires `&SpawnOrigin`, and enemy projectiles carry NONE —
    // measured, `has_origin=false` for every live projectile in the oracle route. So after
    // bevy_ggrs recreated the entity the component was simply gone, the shot's `HitEvent` was
    // emitted with `attacker: None`, and the firer's `ranged` move never learned it connected. That
    // is the equipment oracle's divergence: `MovePlayback.landed_hit` true on three passes and
    // false on the fourth.
    //
    // It is now ordinary rollback state with entity remapping — the same pairing
    // `MovePlayback` uses for its own `live_boxes` handles. A derived declaration
    // is only as good as the system that honours it, and this one names a
    // component the system cannot even see.
    // The boss's SIM-OWNED animation cursor, and the hurtbox sample derived from
    // it. Neither was rollback state, and the coverage sweep never visited a room
    // with a boss in it, so nothing said so. See `rollback_coverage`'s boss-arena
    // sweep, added with this.

    // G2: probed through the OWNER's stable `SimId`, not by counting carriers.
    registrar
        .declare_rollback_derived_component::<ambition_platformer2d_core::body_clusters::BodyEnvironmentContact>(
            ENGINE,
            "derived.body_environment_contact",
            "rewritten every movement step from body geometry and the live world",
        )
        .declare_rollback_derived_component::<ambition_platformer2d_core::BodyMotionFacts>(
            ENGINE,
            "derived.body_motion_facts",
            "republished from MotionModel every movement step",
        )
        .declare_rollback_derived_component::<ambition_sim_view::BodyPoseView>(
            ENGINE,
            "derived.body_pose_view",
            "SimView projection rebuilt every tick",
        )
        .declare_rollback_derived_component::<ambition_sim_view::ProjectileView>(
            ENGINE,
            "derived.projectile_view",
            "SimView projection rebuilt every tick",
        )
        // Frame-derived RESOURCES (Phase 5 resource-coverage pass): each is
        // republished by its ordinary maintenance system before anything reads it,
        // so a rewind that keeps a stale value is overwritten before it matters.
        .declare_rollback_derived_resource::<ambition_platformer2d_core::control_frame::ControlFrame>(
            ENGINE,
            "derived.control_frame",
            "per-tick input frame regenerated from the synchronized input stream",
        );

    // Abandoned-future transient ingress must be empty after LoadWorld. Replayed inputs and
    // deterministic systems regenerate the correct messages. S4 — the stocks loop's two
    // messages.
    registrar.clear_message_on_rollback::<ambition_platformer2d_world::rooms::RoomLoaded>(
        ENGINE,
        "message.room_loaded",
    )
    // A same-tick handshake: the reset processor announces it, and the teardown
    // systems chained after it read it. A cursor GGRS did not rewind would let
    // that teardown fire for a reset the resimulation never committed to — the
    // held items and portals of a session that was, on this timeline, never
    // reset.
    .clear_message_on_rollback::<ambition_persistence::quest::QuestAdvanceRequested>(
        ENGINE,
        "message.quest_advance_requested",
    )
    .clear_message_on_rollback::<ambition_sfx::OwnedSfxMessage>(ENGINE, "message.owned_sfx")
    .clear_message_on_rollback::<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>(
        ENGINE,
        "message.respawn_room_visuals",
    )
    // Phase 5 resource-coverage pass: the remaining sim-facing buffers the
    // computed audit surfaced. Same policy as every entry above — empty after
    // LoadWorld, regenerated by replayed inputs.
    .clear_message_on_rollback::<ambition_persistence::quest::QuestAdvanceRequested>(
        ENGINE,
        "message.quest_advance_requested",
    );
}
