//! **Snakes on a plane: the flying-swarm enemy archetypes.**
//!
//! Two characters, not two skins — the Cartesian one is a maths joke and the
//! paper one is an aviation joke — so both get an archetype and Jon picks which
//! one 1-2 places. Their catalog rows, pedestals and conversations landed
//! 2026-08-05; this is the behaviour half the ledger said was missing.
//!
//! ⭐ **the engine already flies, and it is DATA.** The ledger row claimed a
//! flying swarm needed a motion authority that is neither `step_kinematic` nor a
//! projectile. It needed neither: `CharacterBrainTemplate::Aerial` and
//! `MoveStyleSpec::Float` have existed the whole time (the comment beside
//! `Float` names *"aerial bosses, sharks"*), and the catalog rows already say
//! `body_kind: Floating`, which is the chain
//! `Floating -> is_aerial -> gravity_scale: 0.0` plus the fly ability from
//! spawn.
//!
//! ⚠ **so this file is a table, not a system**, and that is the finding rather
//! than an accident of scope. A new enemy SHAPE in this engine is an archetype
//! row unless it needs a verb the movement kernel does not have.

/// The two flying-swarm archetype ROWS (no outer braces), folded into Mary-O's
/// single roster fragment the same way `SNAKE_ROSTER_ROWS` is — assembly rejects
/// a second fragment from one provider.
///
/// ⚠ **`aggro_radius: 0.0` and `attack_range: 0.0` are deliberate, and copied
/// from the snake for the same reason**: the `Aerial` template ignores both, and
/// giving them numbers would state a rule nothing reads. Offense is body contact
/// only, which is what a thing you jump on top of should be.
///
/// ⭐ **the two differ in SPEED and HEALTH, not in kind.** A paper plane is
/// light and quick and dies to anything; a Cartesian plane is a grid and moves
/// like one — slower, steadier, and it takes two hits. That is the whole
/// difference a player feels, and putting it here rather than in code is what
/// makes the pair cheap to tune.
pub(crate) const SNAKES_ON_A_PLANE_ROSTER_ROWS: &str = r#"
    "mary_o_snakes_on_a_paper_plane": (
        max_health: 1,
        run_speed: 58.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Aerial,
        move_style: Float,
        is_aerial: true,
        respawn: OnRoomReenter,
    ),
    "mary_o_snakes_on_a_cartesian_plane": (
        max_health: 2,
        run_speed: 38.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Aerial,
        move_style: Float,
        is_aerial: true,
        respawn: OnRoomReenter,
    ),
"#;

/// ⛔ **`is_aerial: true` is on the ARCHETYPE and it is not redundant.** The
/// catalog rows say `body_kind: Floating`, which is what makes the CHARACTER
/// fly — but a placed enemy resolves its archetype too, `ArchetypeSpec::is_aerial`
/// defaults to `false`, and the two are separate authorities on the same
/// question. A row that stated it in one place would fly or not depending on
/// which road the spawn took, which is precisely the class of bug that had both
/// catalog rows landing as `Standard` an hour after they were written.
///
/// The brain key an LDtk `EnemySpawn` names to place a paper-plane swarm.
pub const PAPER_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_paper_plane";
/// The brain key an LDtk `EnemySpawn` names to place a Cartesian-plane swarm.
pub const CARTESIAN_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_cartesian_plane";

/// The catalog character each brain wears, for the `EnemySpawn.character_id` a
/// level authors beside the brain key.
pub const PAPER_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_CHARACTER_ID`].
pub const CARTESIAN_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_cartesian_plane";
