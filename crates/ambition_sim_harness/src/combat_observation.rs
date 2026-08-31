//! ONE serialization of what combat did on a tick, shared by every recorder.
//!
//! ⭐⭐ THE SAME REASON `move_exercise` EXISTS, one layer along. Two tools drive
//! a move — `moveset_takes` records every tick of it, `moveset_render`
//! photographs some of them — and for a while each also DESCRIBED what it saw in
//! its own words. The recorder queried `Hitbox` and called `world_volume`
//! itself; the renderer described nothing at all. So the browser could be shown
//! two pictures of one move whose geometry came from two implementations, and
//! the only thing keeping them honest was that one of them had no geometry.
//!
//! ⛔⛔ THE GEOMETRY COMES FROM [`CombatGeometryView`] AND FROM NOWHERE ELSE.
//! That read model resolves strike volumes with the same `Hitbox::world_volume`
//! the resolver uses and applies the runtime's three-way damageable rule; a
//! second copy of either in a tool is a second answer to a question the engine
//! already answers. This module may join what it sees against IDENTITY —
//! `SimId`, `MatchSeat`, ownership — because an artifact needs stable names, and
//! identity is not geometry.
//!
//! ⛔ AND A ROLE IS RECORDED, NOT INFERRED. A reader must never have to work out
//! which fighter is the subject from a seat number, a colour, or a character id
//! the scenario deliberately reuses.

use bevy::prelude::{Entity, World};

use ambition_platformer2d::observation as obs;
use ambition_platformer2d::observation::CombatGeometryView;

/// The schema these rows are written in. Bumped when a field's MEANING changes;
/// a reader that finds an unknown version must say so rather than guess.
pub const OBSERVATION_SCHEMA: &str = "ambition.combat_observation.v1";

/// What one entity IS in the scenario being inspected.
///
/// ⭐⭐ THE DISTINCTION THE WHOLE OBSERVATORY RESTS ON. An inspection scenario
/// seats two fighters and may well seat the same CHARACTER twice, so neither the
/// character id nor a seat index names the thing under inspection. This does,
/// it is written into the artifact, and every consumer — canvas, SVG, report —
/// reads the word rather than re-deriving it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioRole {
    /// The fighter whose move is being inspected.
    Subject,
    /// What the move is being performed against.
    Target,
    /// A projectile, summon or strike the SUBJECT owns.
    SubjectOwned,
    /// The same, belonging to the target.
    TargetOwned,
    /// Everything else on the stage: scenery, a hazard, an unowned shot.
    Other,
}

impl ScenarioRole {
    /// The stable word an artifact writes down.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Target => "target",
            Self::SubjectOwned => "subject_owned",
            Self::TargetOwned => "target_owned",
            Self::Other => "other",
        }
    }

    /// Does this role belong to the subject's side?
    ///
    /// The one question the take's own statistics ask: a move is credited with
    /// what the SUBJECT produced, never with what the stage did over the same
    /// frames.
    pub fn is_subjects(self) -> bool {
        matches!(self, Self::Subject | Self::SubjectOwned)
    }
}

/// The two identities a scenario is ABOUT.
///
/// Fixed for the whole take: a re-seat spawns new bodies, so these are resolved
/// once after staging and never again. What they own is a different question —
/// see [`ScenarioRoles::resolve`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ScenarioRoles {
    subject: Option<Entity>,
    target: Option<Entity>,
}

impl ScenarioRoles {
    /// Resolve the scenario's two fighters from the seats a match staged.
    ///
    /// ⛔ A SEAT THAT IS NOT THERE IS `None`, NOT A GUESS. A scenario whose
    /// target never seated is a scenario with no target, and an artifact that
    /// says so is worth more than one that promotes a bystander.
    pub fn from_seats(world: &mut World, subject_seat: usize, target_seat: usize) -> Self {
        let mut seats = world.query::<(Entity, &ambition_platformer2d::actor::MatchSeat)>();
        let mut roles = Self::default();
        for (entity, seat) in seats.iter(world) {
            if seat.0 == subject_seat {
                roles.subject = Some(entity);
            } else if seat.0 == target_seat {
                roles.target = Some(entity);
            }
        }
        roles
    }

    /// A scenario named directly, for a fixture or a caller that already knows
    /// its two bodies.
    pub fn of(subject: Option<Entity>, target: Option<Entity>) -> Self {
        Self { subject, target }
    }

    /// The subject's body, when the scenario seated one.
    pub fn subject(&self) -> Option<Entity> {
        self.subject
    }

    /// The target's body, when the scenario seated one.
    pub fn target(&self) -> Option<Entity> {
        self.target
    }

    /// Classify everything on the stage AS IT IS RIGHT NOW.
    ///
    /// ⛔⛔ RESOLVED EVERY TICK, BECAUSE OWNERSHIP IS NOT A FACT OF THE
    /// SCENARIO. The pirate's shark does not exist when the take is set up — it
    /// is summoned by the move being inspected, twenty ticks in — so ownership
    /// resolved once at staging classifies the one body everybody opens this
    /// view to watch as a piece of scenery. The two FIGHTERS are fixed; what
    /// they own is discovered.
    pub fn resolve(&self, world: &mut World) -> ResolvedRoles {
        let mut owned: std::collections::HashMap<Entity, ScenarioRole> = Default::default();
        let side = |owner: Entity| -> Option<ScenarioRole> {
            if Some(owner) == self.subject {
                Some(ScenarioRole::SubjectOwned)
            } else if Some(owner) == self.target {
                Some(ScenarioRole::TargetOwned)
            } else {
                None
            }
        };

        // ⛔⛔ A SUMMON IS NOT SCENERY, and it carries no seat and no worn
        // character — so an observer that classified by seat alone filed the
        // subject's own mount under "other" and drew it like stage furniture.
        let ridden: Vec<(Entity, Entity)> = {
            let mut riders = world.query::<(Entity, &ambition_platformer2d::actor::RidingOn)>();
            riders
                .iter(world)
                .map(|(rider, riding)| (rider, riding.mount))
                .collect()
        };
        for (rider, mount) in ridden {
            if let Some(role) = side(rider) {
                owned.insert(mount, role);
            }
        }

        let shots: Vec<(Entity, Entity)> = {
            let mut projectiles =
                world.query::<(Entity, &ambition_platformer2d::actor::ProjectileOwner)>();
            projectiles
                .iter(world)
                .map(|(shot, owner)| (shot, owner.0))
                .collect()
        };
        for (shot, owner) in shots {
            if let Some(role) = side(owner) {
                owned.insert(shot, role);
            }
        }

        ResolvedRoles {
            scenario: *self,
            owned,
        }
    }
}

/// Who is who on ONE tick: the scenario's fighters, plus everything they own
/// right now.
#[derive(Clone, Debug, Default)]
pub struct ResolvedRoles {
    scenario: ScenarioRoles,
    owned: std::collections::HashMap<Entity, ScenarioRole>,
}

impl ResolvedRoles {
    /// The subject's body, when the scenario seated one.
    pub fn subject(&self) -> Option<Entity> {
        self.scenario.subject
    }

    /// The target's body, when the scenario seated one.
    pub fn target(&self) -> Option<Entity> {
        self.scenario.target
    }

    /// What this entity is in the scenario.
    pub fn role_of(&self, entity: Entity) -> ScenarioRole {
        if Some(entity) == self.scenario.subject {
            ScenarioRole::Subject
        } else if Some(entity) == self.scenario.target {
            ScenarioRole::Target
        } else {
            self.owned
                .get(&entity)
                .copied()
                .unwrap_or(ScenarioRole::Other)
        }
    }

    /// What something this entity OWNS is — a strike, a shot, a summon.
    ///
    /// A strike belongs to the side of the body that threw it: the subject's own
    /// swing is `subject_owned`, never `subject`.
    pub fn owned_role_of(&self, owner: Entity) -> ScenarioRole {
        match self.role_of(owner) {
            ScenarioRole::Subject | ScenarioRole::SubjectOwned => ScenarioRole::SubjectOwned,
            ScenarioRole::Target | ScenarioRole::TargetOwned => ScenarioRole::TargetOwned,
            ScenarioRole::Other => ScenarioRole::Other,
        }
    }
}

/// One body's combat facts, and the identity they were joined to.
pub struct ObservedBody {
    /// The simulation entity, so a caller can merge these onto its own row.
    pub entity: Entity,
    /// The engine's stable identity, when this body carries one.
    pub sim_id: Option<String>,
    pub role: ScenarioRole,
    /// The combat half of the row: geometry, move state, the tuning readout.
    pub facts: serde_json::Value,
}

/// Everything combat published on one tick, in artifact form.
pub struct CombatObservation {
    pub bodies: Vec<ObservedBody>,
    /// Complete strike rows — geometry, damage, owner, role, identity.
    pub strikes: Vec<serde_json::Value>,
    /// Which strike has connected with which body, as the RUNTIME says.
    ///
    /// ⛔⛔ MEASURED OVERLAP AND A RESOLVED HIT ARE TWO FACTS. An observer may
    /// measure the first from the geometry in this observation; it must never
    /// conclude the second from it. A volume can pass through a body that is
    /// intangible, on the same team, shielded, or already struck by this same
    /// strike — and the picture is identical in all four cases.
    pub contacts: Vec<serde_json::Value>,
}

impl CombatObservation {
    /// Read the tick. Every value comes from [`CombatGeometryView`]; identity is
    /// joined beside it.
    ///
    /// ⛔ NOTHING HERE MUTATES. An observation that could change the run is not
    /// an observation of it.
    pub fn capture(world: &mut World, roles: &ResolvedRoles) -> Self {
        let Some(view) = world.get_resource::<CombatGeometryView>().cloned() else {
            return Self {
                bodies: Vec::new(),
                strikes: Vec::new(),
                contacts: Vec::new(),
            };
        };

        let bodies = view
            .bodies
            .iter()
            .map(|body| ObservedBody {
                entity: body.body,
                sim_id: sim_id_of(world, body.body),
                role: roles.role_of(body.body),
                facts: serde_json::json!({
                    // The coarse envelope, beside the volumes that actually
                    // decide a hit. Both, because "the attack landed inside the
                    // body box but outside every hurtbox" is a real answer.
                    "collision": aabb_json(body.collision),
                    "hurtboxes": body
                        .hurtboxes
                        .iter()
                        .map(volume_json)
                        .collect::<Vec<_>>(),
                    // ⭐ WHY THERE IS OR IS NOT A HURTBOX. An empty list is
                    // either a deliberate intangible window or a body nothing
                    // published for, and a reader cannot tell those apart from
                    // the geometry.
                    "hurtbox_source": body.hurtbox_source.as_str(),
                    "damage_taken": body.damage_taken,
                    "facing": body.facing,
                    "velocity": [body.velocity.x, body.velocity.y],
                    "grounded": body.grounded,
                    "on_wall": body.on_wall,
                    "wall_normal_x": body.wall_normal_x,
                    "hitstun_s": body.hitstun_s,
                    "hitlag_s": body.hitlag_s,
                    "landing_lag_s": body.landing_lag_s,
                    "jump_squat_s": body.jump_squat_s,
                    // ⭐⭐ THE MOVE CLOCK, WHICH IS WHAT MAKES A BOX READABLE.
                    // "a red box appeared" is not frame data; "tick 7 of 41,
                    // inside the authored Active window" is.
                    "move_state": body.move_state.as_ref().map(|state| serde_json::json!({
                        "id": state.id,
                        "phase": state.phase.as_ref().map(|tag| format!("{tag:?}")),
                        "elapsed_s": state.elapsed_s,
                        "duration_s": state.duration_s,
                        // The orientation the move COMMITTED to, which is not
                        // necessarily the body's live facing — seeing the two
                        // disagree explains a strike on the far side.
                        "attack_facing": state.attack_facing,
                        "landed_hit": state.landed_hit,
                    })),
                }),
            })
            .collect();

        // The contact pairs, from the resolver's own hit-once memory. Emitted
        // as their own list as well as on the strike row, because "what
        // connected this tick" is the question, and answering it should not
        // require walking every volume.
        let mut contacts = Vec::new();
        for strike in &view.strikes {
            let owner_id = sim_id_of(world, strike.owner);
            let owner_role = roles.owned_role_of(strike.owner);
            let strike_id = sim_id_of(world, strike.strike);
            for victim in &strike.hit {
                contacts.push(serde_json::json!({
                    "strike": strike_id,
                    "owner_id": owner_id,
                    "owner_role": owner_role.as_str(),
                    "victim": sim_id_of(world, *victim),
                    "victim_role": roles.role_of(*victim).as_str(),
                }));
            }
        }

        let strikes = view
            .strikes
            .iter()
            .map(|strike| {
                let role = roles.owned_role_of(strike.owner);
                let owner_id = sim_id_of(world, strike.owner);
                let mut row = volume_json(&strike.volume);
                let object = row.as_object_mut().expect("a volume serializes as an object");
                object.insert(
                    // ⭐ THE VOLUME'S OWN IDENTITY where it has one, and an
                    // OWNER-QUALIFIED fallback where it does not: a bare strike
                    // index names one volume of every unidentified owner, and an
                    // id that identifies two things is worse than an absent one.
                    "id".to_string(),
                    serde_json::json!(sim_id_of(world, strike.strike)),
                );
                object.insert("owner_id".to_string(), serde_json::json!(owner_id));
                object.insert("damage".to_string(), serde_json::json!(strike.damage));
                object.insert("role".to_string(), serde_json::json!(role.as_str()));
                // Kept beside the role for readers written before roles existed.
                // The role is the authority; this is its projection.
                object.insert(
                    "subject_owned".to_string(),
                    serde_json::json!(role.is_subjects()),
                );
                object.insert(
                    // Only a body-tracking strike stands in for somebody's
                    // swing; a world-anchored one is a place, not a limb.
                    "anchored_to_body".to_string(),
                    serde_json::json!(strike.anchored_to_body),
                );
                object.insert(
                    // Whom this strike has already connected with. Its FIRST
                    // appearance across consecutive ticks is the contact tick,
                    // exactly, with no threshold to tune.
                    "hit".to_string(),
                    serde_json::json!(strike
                        .hit
                        .iter()
                        .map(|victim| sim_id_of(world, *victim))
                        .collect::<Vec<_>>()),
                );
                row
            })
            .collect();

        let mut this = Self {
            bodies,
            strikes,
            contacts,
        };
        this.canonicalize();
        this
    }

    /// Put both lists in stable identity order.
    ///
    /// ⛔⛔ SORTED HERE, WHERE IT CANNOT BE FORGOTTEN. The order rows arrive in
    /// is Bevy query order, which is archetype order, which changes when
    /// anything about component composition changes — so an unsorted recording
    /// is one two runs cannot compare byte for byte, and a recording nothing can
    /// diff is not evidence. Every consumer of this type gets the canonical
    /// order because the constructor establishes it, not because each consumer
    /// remembered to.
    fn canonicalize(&mut self) {
        self.strikes.sort_by_key(canonical_key);
        self.contacts.sort_by_key(|row| {
            (
                row["strike"].as_str().unwrap_or_default().to_string(),
                row["victim"].as_str().unwrap_or_default().to_string(),
            )
        });
        // An unidentified body would tie here — which is exactly why a recorder
        // refuses to write a take containing one.
        self.bodies.sort_by(|a, b| {
            (a.sim_id.as_deref().unwrap_or_default(), a.entity.index())
                .cmp(&(b.sim_id.as_deref().unwrap_or_default(), b.entity.index()))
        });
    }

    /// The whole observation as one document, in the canonical order
    /// [`Self::capture`] established.
    pub fn to_json(&self) -> serde_json::Value {
        let bodies: Vec<serde_json::Value> = self
            .bodies
            .iter()
            .map(|body| {
                let mut row = body.facts.clone();
                let object = row.as_object_mut().expect("a body serializes as an object");
                object.insert("id".to_string(), serde_json::json!(body.sim_id));
                object.insert("role".to_string(), serde_json::json!(body.role.as_str()));
                row
            })
            .collect();
        serde_json::json!({
            "schema": OBSERVATION_SCHEMA,
            "bodies": bodies,
            "strikes": self.strikes.clone(),
            "contacts": self.contacts.clone(),
        })
    }
}

/// The sort key that makes a recording diffable: identity first, geometry only
/// to break ties among rows that carry none.
pub fn canonical_key(row: &serde_json::Value) -> (String, String, u64, u64) {
    (
        row["id"].as_str().unwrap_or_default().to_string(),
        row["owner_id"].as_str().unwrap_or_default().to_string(),
        row["pos"][0].as_f64().unwrap_or_default().to_bits(),
        row["pos"][1].as_f64().unwrap_or_default().to_bits(),
    )
}

/// The engine's stable identity for an entity, when it has one.
///
/// ⛔ A RAW ENTITY ID IS NOT AN IDENTITY: an entity index depends on every spawn
/// and despawn the app made first, so two runs of one binary label the same
/// shark differently and a byte-diff reports physics that did not change.
pub fn sim_id_of(world: &World, entity: Entity) -> Option<String> {
    world
        .get::<ambition_platformer2d::observation::SimId>(entity)
        .map(|id| id.as_str().to_string())
}

/// An AABB as `{pos, half}` — the shape every consumer can draw without knowing
/// about shapes.
fn aabb_json(aabb: obs::Aabb) -> serde_json::Value {
    use obs::AabbExt;
    let center = aabb.center();
    let half = aabb.half_size();
    serde_json::json!({ "pos": [center.x, center.y], "half": [half.x, half.y] })
}

/// One combat volume, exactly.
///
/// ⛔⛔ THE REAL SHAPE, AND THE BOX AROUND IT. A rotated box, a disc and a convex
/// arc are all *contained* by an AABB, and for a sweeping arc that box is a
/// great deal larger than the thing that can actually hit you — the difference
/// between a diagram and a decoration. ⭐ The AABB stays beside it because it is
/// the broad phase the engine itself uses and a consumer can draw it knowing
/// nothing about shapes.
pub fn volume_json(volume: &obs::CombatVolume) -> serde_json::Value {
    use obs::AabbExt;
    let bounds = volume.bounds();
    let center = bounds.center();
    let half = bounds.half_size();
    let shape = match volume {
        obs::CombatVolume::Aabb(_) => serde_json::json!({ "kind": "aabb" }),
        obs::CombatVolume::Obb {
            center,
            half,
            rotation,
        } => serde_json::json!({
            "kind": "obb",
            "center": [center.x, center.y],
            "half": [half.x, half.y],
            "rotation": rotation,
        }),
        obs::CombatVolume::Circle { center, radius } => serde_json::json!({
            "kind": "circle",
            "center": [center.x, center.y],
            "radius": radius,
        }),
        obs::CombatVolume::Convex { points, .. } => serde_json::json!({
            "kind": "convex",
            "points": points.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
        }),
    };
    serde_json::json!({
        "pos": [center.x, center.y],
        "half": [half.x, half.y],
        "shape": shape,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strike_belongs_to_its_owners_side_not_to_the_owner() {
        let subject = Entity::from_raw_u32(1).unwrap();
        let target = Entity::from_raw_u32(2).unwrap();
        let roles = ResolvedRoles {
            scenario: ScenarioRoles::of(Some(subject), Some(target)),
            owned: Default::default(),
        };

        assert_eq!(roles.role_of(subject), ScenarioRole::Subject);
        assert_eq!(roles.owned_role_of(subject), ScenarioRole::SubjectOwned);
        assert_eq!(roles.owned_role_of(target), ScenarioRole::TargetOwned);
        // A hazard nobody owns is nobody's, and in particular not the
        // subject's — the classification that let a stage's output be counted
        // as a move's.
        let hazard = Entity::from_raw_u32(3).unwrap();
        assert_eq!(roles.owned_role_of(hazard), ScenarioRole::Other);
        assert!(!ScenarioRole::Other.is_subjects());
        assert!(!ScenarioRole::TargetOwned.is_subjects());
        assert!(ScenarioRole::SubjectOwned.is_subjects());
    }

    /// ⛔ AN ABSENT TARGET IS NOT A ROLE SOMEBODY ELSE INHERITS.
    #[test]
    fn a_scenario_with_no_target_promotes_nobody() {
        let roles = ResolvedRoles {
            scenario: ScenarioRoles::of(Some(Entity::from_raw_u32(1).unwrap()), None),
            owned: Default::default(),
        };
        assert_eq!(roles.target(), None);
        assert_eq!(
            roles.role_of(Entity::from_raw_u32(9).unwrap()),
            ScenarioRole::Other
        );
    }

    /// ⛔⛔ THE SUMMON DOES NOT EXIST WHEN THE SCENARIO IS SET UP. It is spawned
    /// by the move under inspection, so ownership resolved once at staging
    /// files the subject's own mount under `other` — and the shark is the body
    /// the whole view exists to show.
    #[test]
    fn a_summon_that_appears_mid_take_belongs_to_whoever_summoned_it() {
        use bevy::prelude::*;
        let mut app = App::new();
        let subject = app.world_mut().spawn_empty().id();
        let roles = ScenarioRoles::of(Some(subject), None);

        // Before the move: nothing is owned, and the future mount is nobody's.
        let staged = roles.resolve(app.world_mut());
        let mount = app.world_mut().spawn_empty().id();
        assert_eq!(staged.role_of(mount), ScenarioRole::Other);

        // The move summons it and the subject boards.
        app.world_mut()
            .entity_mut(subject)
            .insert(ambition_platformer2d::actor::RidingOn { mount });
        let riding = roles.resolve(app.world_mut());
        assert_eq!(
            riding.role_of(mount),
            ScenarioRole::SubjectOwned,
            "a mount the subject is riding is the subject's"
        );
        // And a strike the MOUNT throws is still the subject's side.
        assert_eq!(riding.owned_role_of(mount), ScenarioRole::SubjectOwned);
    }

    #[test]
    fn every_volume_shape_survives_serialization() {
        use obs::AabbExt;
        let circle = obs::CombatVolume::Circle {
            center: obs::Vec2::new(3.0, 4.0),
            radius: 5.0,
        };
        let row = volume_json(&circle);
        assert_eq!(row["shape"]["kind"], "circle");
        assert_eq!(row["shape"]["radius"], 5.0);
        // ⛔ AND THE BOX AROUND IT IS STILL THERE, so a consumer that knows no
        // shapes still draws something true.
        assert_eq!(row["pos"][0], 3.0);
        assert_eq!(row["half"][0], 5.0);

        let obb = obs::CombatVolume::Obb {
            center: obs::Vec2::new(1.0, 2.0),
            half: obs::Vec2::new(6.0, 2.0),
            rotation: 0.5,
        };
        assert_eq!(volume_json(&obb)["shape"]["kind"], "obb");
        assert_eq!(volume_json(&obb)["shape"]["rotation"], 0.5);

        let convex = obs::CombatVolume::convex(vec![
            obs::Vec2::new(0.0, 0.0),
            obs::Vec2::new(10.0, 0.0),
            obs::Vec2::new(0.0, 8.0),
        ]);
        let row = volume_json(&convex);
        assert_eq!(row["shape"]["kind"], "convex");
        assert_eq!(row["shape"]["points"].as_array().map(Vec::len), Some(3));

        let aabb = obs::CombatVolume::aabb(obs::Aabb::new(
            obs::Vec2::new(2.0, 2.0),
            obs::Vec2::new(4.0, 4.0),
        ));
        assert_eq!(volume_json(&aabb)["shape"]["kind"], "aabb");
        assert_eq!(aabb.bounds().center(), obs::Vec2::new(2.0, 2.0));
    }

    /// The canonical order is IDENTITY first. Two strikes of one move can share
    /// a position and a damage — a multihit's mirrored pair does — and a tie
    /// then falls back to query order, which is the thing being canonicalised.
    #[test]
    fn canonical_order_leads_with_identity() {
        let a = serde_json::json!({ "id": "b", "owner_id": "x", "pos": [0.0, 0.0] });
        let b = serde_json::json!({ "id": "a", "owner_id": "x", "pos": [9.0, 9.0] });
        let mut rows = vec![a, b];
        rows.sort_by_key(canonical_key);
        assert_eq!(rows[0]["id"], "a");
    }
}
