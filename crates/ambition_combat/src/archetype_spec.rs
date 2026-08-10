//! The enemy-ARCHETYPE authoring vocabulary — the type `character_archetypes.ron`
//! is deserialized into.
//!
//! ⛔ **it lived in `ambition_platformer2d_actor_monolith::features::enemies` and
//! was `pub(crate)`, which blocked the content compiler from owning the family.**
//! A schema must be registered by the crate owning its type and the validator
//! has to link that crate; the actor crate is 708 crates and a renderer against
//! the validator's 242.
//!
//! It belongs here: every field is combat/movement tuning, and the two types it
//! could not have named from anywhere lower — `BodyMovementPatch` and
//! `BodyMovementTuning` — are defined in THIS crate. The roster's assembly logic
//! (inheritance folding, provider-local parents, transactional publication)
//! stayed in the actor crate, because that is genuinely coupled to it.
//!
//! ⚠ `CharacterBrainTemplate` moved to `ambition_characters::brain` in the same
//! change, for the same reason and with the same justification already written
//! in its own doc comment: *"the brain module is the universal-actor abstraction
//! and shouldn't know named enemies"*.

use ambition_platformer2d_core as ae;

/// Vec2 deserialization shim: `bevy_math::Vec2` does not implement `Deserialize`
/// under the features this crate compiles with, so authored pairs route through
/// a tuple.
mod vec2_option {
    use ambition_platformer2d_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<Option<ae::Vec2>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Option<(f32, f32)> = Option::deserialize(de)?;
        Ok(raw.map(|(x, y)| ae::Vec2::new(x, y)))
    }
}

/// Serde default for [`ArchetypeSpec::attack_cooldown_mult`]: the
/// multiplicative identity (most archetypes use the shared cooldown).
pub fn default_attack_cooldown_mult() -> f32 {
    1.0
}

fn default_mass() -> f32 {
    1.0
}

/// Serde default for the `bool` spec fields that are true for the common
/// case (`attacks_player`, `body_contact_damage`).
pub fn default_true() -> bool {
    true
}

fn default_turns_at_walls() -> bool {
    true
}

/// Serde default for [`ArchetypeSpec::weight`] (CM1): the reference
/// body, so knockback growth divides by 1.0 for every un-authored archetype.
pub fn default_weight() -> f32 {
    1.0
}

// ⛔ **`deny_unknown_fields` is the CONTRACT, not a nicety.**
// `ContentSchemaHandler::check`'s own doc: *"a handler MUST report an authored
// field it does not consume … rolling your own field walk and forgetting is how
// a typo becomes a mechanic that silently never fires."* This type had no such
// guard, and an audit measured the consequence by authoring
// `favourite_snack: "worms"` into a real file: the pack compiled CLEAN, and the
// field reached neither the runtime nor the fingerprint. A misspelled tuning
// value is exactly that shape, and it looks identical to authoring nothing.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeSpec {
    /// Optional parent archetype id to inherit movement tuning from. The resolver
    /// folds `BASELINE ← parent (resolved) ← this row's `movement` patch`, so an
    /// archetype can extend another and override only what differs. `None` =
    /// inherit straight from the generic baseline.
    #[serde(default)]
    pub inherits: Option<String>,
    /// Authored movement overrides (a partial patch; every knob optional). Layered
    /// onto the resolved parent/baseline at roster-build time.
    #[serde(default)]
    pub movement: crate::BodyMovementPatch,
    /// Resolved movement physics — filled by the roster's inheritance pass, NOT
    /// authored. Defaults to the baseline so a spec used outside the roster still
    /// has sane physics.
    #[serde(skip)]
    pub movement_resolved: crate::BodyMovementTuning,
    pub max_health: i32,
    /// **The BODY's ground-run capability (px/s)** — the fastest this archetype
    /// can locomote, and the only absolute speed it authors.
    ///
    /// §4.7: locomotion crosses the brain→body seam as normalized effort, never
    /// world-space speed. This is the body half of that sentence. It used to be
    /// implicit — `max(patrol_speed, chase_speed)` — which meant a body's top
    /// speed was a side effect of how hard its brain happened to chase, and an
    /// archetype reused on a different body kept the first body's absolute
    /// numbers (queue C1).
    pub run_speed: f32,
    /// Idle-pace exertion, `0.0..=1.0` of [`Self::run_speed`].
    ///
    /// A heavy at `0.9` and a light at `0.35` sometimes reaching the same
    /// absolute speed is NOT wrong: effort is relative exertion, not a
    /// cross-character ranking.
    pub patrol_effort: f32,
    /// Aggro/engage exertion, `0.0..=1.0` of [`Self::run_speed`].
    pub chase_effort: f32,
    pub aggro_radius: f32,
    pub attack_range: f32,
    pub contact_strength: f32,
    pub damage_amount: i32,
    /// Multiplier on the shared attack cooldown (fast skirmishers
    /// < 1.0, lumbering heavies > 1.0).
    #[serde(default = "default_attack_cooldown_mult")]
    pub attack_cooldown_mult: f32,
    /// Physical mass, used to weight the mount+rider center of gravity (a heavy
    /// shark vs a light rider) so the pair rotates as a unit around the COG under
    /// a gravity flip. Defaults to 1.0 so existing archetypes need no RON change;
    /// heavy mounts author a larger value.
    #[serde(default = "default_mass")]
    pub mass: f32,
    /// Walks surfaces hugging the surface normal (wall/ceiling
    /// crawler with ledge-aware patrol).
    #[serde(default)]
    pub surface_walker: bool,
    /// Autonomous simple-walker steering: turn away from a semantic side
    /// contact. This is control policy consumed by Patrol/Wanderer brains, not
    /// movement/collision policy. Defaults TRUE; other brain families ignore it.
    #[serde(default = "default_turns_at_walls")]
    pub turns_at_walls: bool,
    /// Surface-walker only: a hit knocks the actor off its surface — it
    /// loses cling and falls with gravity for a moment before re-attaching.
    /// Authored `false` for crawlers that hold on when struck.
    #[serde(default)]
    pub cling_breaks_on_hit: bool,
    /// **Whether this archetype flies — or is SILENT about it.**
    ///
    /// ⛔ `None` is not `Some(false)`, and that distinction is the whole point.
    /// Two spawn paths decide aerial-ness: `new_peaceful_npc_in` reads the
    /// catalog's `body_kind: Floating`, the hostile `EnemySpawn` path reads this.
    /// While this was a bare `bool` with `#[serde(default)]`, an archetype that
    /// authored `false` and one that authored nothing were the same value — so a
    /// disagreement between the two paths could not even be stated, let alone
    /// resolved. The Perfect Cellular Automaton is the live case: `Floating` in
    /// its catalog row, played grounded by the shipped duel.
    ///
    /// ⭐ this is the same defect `deny_unknown_fields` was added for, one field
    /// away — there a misspelled key "looks identical to authoring nothing", here
    /// a deliberate `false` did.
    ///
    /// ⚠ **readers resolve absence with `unwrap_or(false)`**, which is exactly
    /// what the bare bool did, so making the question expressible changed no
    /// behaviour. Whether ANY of them should instead defer to the catalog is the
    /// open product question (`review-gpt56-through-32eb27a.md` P5).
    #[serde(default)]
    pub is_aerial: Option<bool>,
    #[serde(default)]
    pub is_sandbag: bool,
    /// Detonates at the corpse on death (see `CombatCapabilities`).
    #[serde(default)]
    pub explodes_on_death: bool,
    /// Splits into offspring on death.
    #[serde(default)]
    pub divides_on_death: bool,
    /// A fast charge stopped dead by a wall destroys this actor.
    #[serde(default)]
    pub charge_crash_explodes: bool,
    /// Damage never kills (infinite training dummy).
    #[serde(default)]
    pub never_dies: bool,
    /// When this defeated actor reappears (ADR 0022). DEFAULT =
    /// `DeadStaysDead` — respawning is an authored opt-in: trash mobs
    /// author `OnRoomReenter`, mini-boss presences `OnRest`, training
    /// sandbags `InPlace(secs)`.
    #[serde(default)]
    pub respawn: ambition_entity_catalog::placements::RespawnPolicy,
    /// Knockback weight (CM1): heavier bodies launch less under the growth term.
    /// Default `1.0` (the reference body) keeps every un-authored archetype at
    /// today's flat knockback.
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// Damage-meter death policy (CM1). DEFAULT `HpDepleted` (dies at pool max)
    /// leaves Ambition unchanged; a smash-style fighter authors `Unbounded`
    /// (death from the blast-zone, not the meter).
    #[serde(default)]
    pub death_policy: crate::DeathPolicy,
    /// Deep-dream visual jitter seed (psychedelic shader pass);
    /// `None` = the archetype doesn't participate.
    #[serde(default)]
    pub dream_seed: Option<f32>,
    /// This archetype can be ridden (ADR 0020): the content-defined mount
    /// class a rider must be allowed to pilot. `None` = not a mount.
    #[serde(default)]
    pub mount_class: Option<String>,
    /// Mount classes a *rider* of this archetype may pilot (ADR 0020).
    /// Empty = this archetype cannot mount anything. A shark-rider carries
    /// `["shark"]`; it cannot board a `"mech"`-class mount.
    #[serde(default)]
    pub pilotable_mount_classes: Vec<String>,
    /// Damage this *mount* splashes onto its rider when it dies (ADR 0020).
    /// `None` = the rider drops unharmed (a `MountDeathImpact::Dismount`);
    /// `Some(n)` = the rider takes `n` damage (a mech exploding).
    #[serde(default)]
    pub mount_death_splash: Option<i32>,
    #[serde(default, with = "vec2_option")]
    pub default_size: Option<ae::Vec2>,
    /// Brain template the spawn site instantiates for this archetype.
    /// MeleeBrute reads the archetype's tunings (chase_speed,
    /// aggro_radius, attack_range) for its cfg; Wanderer + StandStill
    /// ignore them.
    pub brain_template: ambition_characters::brain::CharacterBrainTemplate,
    /// Which rung of the fighter ladder a `Fighter`-template archetype plays at.
    /// `None` is the middle rung; every other template ignores it.
    #[serde(default)]
    pub fighter_level: Option<u8>,
    /// Concrete melee action this archetype's `ActionSet` carries.
    /// `None` = no melee capability (peaceful patrollers, ranged-only
    /// actors).
    #[serde(default)]
    pub melee: Option<ambition_characters::brain::MeleeActionSpec>,
    /// Concrete ranged action this archetype's `ActionSet` carries.
    /// `None` = no ranged capability.
    #[serde(default)]
    pub ranged: Option<ambition_characters::brain::RangedActionSpec>,
    /// Optional held-item id, resolved against the held-item registry
    /// (`ambition_characters::brain::held_item_by_id`). The item's abilities overlay the
    /// archetype action set at spawn / state transitions so weapons, not
    /// ad-hoc Rust branches, own whether an actor can melee or fire.
    #[serde(default)]
    pub held_item: Option<String>,
    /// Smash-brain melee hit band (the `attack_range`/engage sizing the
    /// `Smash` template uses, distinct from the CharacterAI stop-distance
    /// `attack_range` above). `None` for non-Smash archetypes; the Smash
    /// config builder falls back to a 36px default. Moving this out of the
    /// `smash_cfg_for_archetype` match arms (CharacterAI migration, #194)
    /// so a new Smash enemy is a data row, not a code edit.
    #[serde(default)]
    pub smash_hit_band: Option<f32>,
    /// Smash-template heavy base: longer reach + slower chase
    /// (`SmashCfg::BRUTE_DEFAULT`) vs the lighter striker default. Inert
    /// unless `brain_template` is `Smash`.
    #[serde(default)]
    pub smash_heavy: bool,
    /// Smash-template dash-to-close: a richer action set that dashes to
    /// close a large gap (goblins). Inert unless `brain_template` is `Smash`.
    #[serde(default)]
    pub smash_dash_to_close: bool,
    /// Smash-template **duelist neutral game**: footsies (weave in/out of poke
    /// range), neutral hops, and a real spacing/retreat rhythm instead of the
    /// grunt's close-and-camp (`SmashCfg::DUELIST_DEFAULT` base). Set for the
    /// "platform fighter" archetypes (the PCA, the player-robot) so they MOVE
    /// and space rather than mash at point-blank. Inert unless `brain_template`
    /// is `Smash`; the per-flag kit (blink/shield/dash/fly) still layers on top.
    #[serde(default)]
    pub smash_duelist: bool,
    // --- Movement kit ---
    //
    // The verbs THIS body has, independent of which brain drives it — the
    // character IS its movement kit. Each authored verb feeds ONE authored
    // [`ae::AbilitySet`] (`movement_kit`) that both ports read: the body
    // unions it into its `AbilitySet` at spawn (`ActorBody::from_kit`, enforce)
    // and the Smash brain reads the same verbs to decide when to attempt them
    // (`brain_spec`, attempt). No `smash_` prefix: these are body capabilities,
    // not Smash-template tuning (cf. `smash_heavy`/`smash_duelist`, which ARE).
    /// Movement kit: this body can **blink** (short-range teleport).
    #[serde(default)]
    pub can_blink: bool,
    /// Movement kit: grounded-base **hybrid flyer** — prefers to fight grounded
    /// but takes to the air to cover a long traversal gap. (`is_aerial` bodies
    /// fly unconditionally; this is the grounded-base opt-in.)
    #[serde(default)]
    pub can_fly: bool,
    /// Movement kit: this body can **reactive-block** — raise a shield to guard a
    /// perceived lunge it won't blink away from.
    #[serde(default)]
    pub can_shield: bool,
    /// Movement kit: this body can **dash** — a short burst above walk speed
    /// (see `smash_dash_to_close` for the Smash brain's *decision* to dash; the
    /// brain always attempts a dash via its Dash action, this lets the body
    /// turn it into a real burst).
    #[serde(default)]
    pub can_dash: bool,
    /// When provoked from peaceful, force an aggressive MeleeBrute brain
    /// with at least this aggro radius (cove PirateHeavy crew). `None` =
    /// use the template's default aggressive brain.
    #[serde(default)]
    pub provoke_forced_brute_min_aggro: Option<f32>,
    /// Hostile by default: actively tracks the player and publishes contact
    /// damage. Peaceful patrollers (cove crew, ambient wildlife) set false
    /// and stay dormant until a system explicitly provokes them.
    #[serde(default = "default_true")]
    pub attacks_player: bool,
    /// Body touch hurts the player. Training dummies and the composite shark
    /// (whose rider is the threat) opt out; the peaceful cove crew also
    /// stay non-damaging until provoked.
    #[serde(default = "default_true")]
    pub body_contact_damage: bool,
    /// Open visual id of this archetype's ranged projectile. Authored so the
    /// render layer resolves shot art by id through the content-owned catalog
    /// (e.g. `"glider"` for the Perfect Cell-ular Automaton) instead of sniffing
    /// the owner-id string. Defaults to the empty string (the generic orange
    /// shot); archetypes with a distinct projectile look name it explicitly.
    #[serde(default)]
    pub ranged_visual: String,
    /// Data-driven signature MOVE repertoire — the Smash-model moveset this
    /// character carries (windows / hit volumes / timed effects, authored on the
    /// owner's proper-time clock). Attached at spawn as an `ActorMoveset`; a control
    /// verb edge (`special`/`attack`) triggers the matching move through the shared
    /// moveset runtime (`combat::moveset`). This is how a character's expressive,
    /// boss-grade moves are designed AS DATA (the engine-for-2D-platformers vision) —
    /// the PCA is the first consumer (fable review 2026-07-02 §A1, Path B). `None`
    /// for characters whose combat is only the flat `melee`/`ranged` `ActionSet`.
    #[serde(default)]
    pub signature_move: Option<ambition_entity_catalog::MovesetContract>,
    /// Locomotion style for the actor's `ActionSet.move_style`.
    pub move_style: ambition_characters::brain::MoveStyleSpec,
}
