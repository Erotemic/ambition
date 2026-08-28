//! The per-actor POSE index + the per-boss FRAME index (E4 slices 3, 7,
//! 19): id-keyed read-models rebuilt once per sim tick; presentation
//! animates from these snapshots and never borrows the live clusters.

use bevy::prelude::{Query, ResMut, Resource};

use ambition_boss_encounter::anim::boss_anim_state_for;
use ambition_characters::actor::ai::ActorStatus;
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::BodyMelee;
use ambition_combat::components::FeatureId;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_core::BodyKinematics;
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

/// Read-only query of the unified actor cluster every actor (was-NPC, was-enemy,
/// encounter mob, mount/rider) carries — the SAME `Body*` movement/ability
/// clusters the player reads, plus the actor's identity/status/config. Systems
/// declare `Query<ActorSpriteData>`; the helpers take `&Query<ActorSpriteData>`.
///
/// All fields are required (not `Option`): every spawned actor carries the full
/// [`ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle`] (the same bundle the player nests)
/// plus `ActorStatus` / `ActorConfig` / `BodyMelee`, so an entity that is missing
/// any of them — a boss (its own cluster + anim path) or a prop — correctly does
/// not match and is skipped, instead of half-resolving from a sparse read. This
/// is what lets [`ecs_actor_anim_state`] build the player's FULL `BodyAnimView`
/// from an actor's real clusters, so any ability a brain drives animates.
#[derive(bevy::ecs::query::QueryData)]
pub struct ActorSpriteData {
    pub feature_id: &'static FeatureId,
    pub kin: &'static BodyKinematics,
    pub status: &'static ActorStatus,
    pub health: &'static ambition_characters::actor::BodyHealth,
    pub combat: &'static ambition_characters::actor::BodyCombat,
    pub config: &'static ActorConfig,
    pub attack: &'static BodyMelee,
    pub ground: &'static ambition_platformer2d_core::BodyGroundState,
    /// The published semantic movement facts (ADR 0024) — maneuver reads
    /// (dash/blink/wall/ledge/dodge/glide) come from here, never from policy
    /// internals.
    pub motion_facts: &'static ae::BodyMotionFacts,
    pub flight: &'static ambition_platformer2d_core::BodyFlightState,
    pub body_mode: &'static ambition_platformer2d_core::BodyModeState,
    pub env_contact: &'static ambition_platformer2d_core::BodyEnvironmentContact,
    pub abilities: &'static ambition_platformer2d_core::BodyAbilities,
    pub shield: &'static ambition_platformer2d_core::BodyShieldState,
    /// Movement-driven presentation overlays (wall-jump / dash-startup / landing /
    /// shoot poses), shared with the player. `Option` so an actor spawned without
    /// the component (a legacy / bespoke path) still animates its base ladder —
    /// it just shows no overlays (fable review §A9).
    pub anim: Option<&'static ambition_characters::actor::BodyAnimFacts>,
    /// The body's own resolved reference basis, so the locomotion metric is
    /// measured along ITS run axis rather than world-x.
    ///
    /// not the global `GravityField`: that is a per-tick mirror of the PRIMARY body's frame, so
    /// every NPC in a localized-gravity zone would be animated against the player's gravity.
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    /// Content-driven pose PIN. When present it wins over the picked pose — a
    /// content state machine (e.g. a shelled enemy's withdraw cycle) uses it to
    /// show a pose the disposition-agnostic picker can't infer. `Option`, so an
    /// ordinary actor is picked exactly as before.
    pub anim_override: Option<&'static ActorAnimOverride>,
    /// The move this body is playing, so the drawn row can be the one the
    /// move names. `None` for a body that is not mid-move, which is most of them
    /// most of the time. See [`ActorAnimFrame::clip`].
    pub playback: Option<&'static ambition_combat::moveset::MovePlayback>,
}

/// One actor's resolved animation frame for the renderer: the chosen anim plus
/// the bits the per-frame apply needs that aren't in the anim itself — world
/// position (for localized-gravity facing) and facing sign.
#[derive(Clone, Debug)]
pub struct ActorAnimFrame {
    pub anim: CharacterAnim,
    pub pos: ae::Vec2,
    pub facing: f32,
    /// The authored clip the body's ACTIVE MOVE asks to be drawn as, with
    /// its fallbacks, or `None` when no move is playing.
    ///
    /// sprite redirect P0. `anim` is a [`CharacterAnim`] — 56 semantic body
    /// states — and the new fighter sheets carry rows it has no variant for
    /// (`smash_forward`, `air_dodge`, `tumble`, `tech_roll`). `MoveSpec` already
    /// names its clip and its fallback chain and says its timeline is
    /// authoritative for gameplay AND presentation, so the read-model carries
    /// the request and the renderer resolves it against the sheet it is about
    /// to draw.
    ///
    /// it is a REQUEST, not a row. Whether `smash_forward` exists is a
    /// question about one sheet, so it is answered at the draw
    /// (`CharacterAnimator::request_clip`) and never here — that is the same
    /// rule `AnimRow` binding follows everywhere else.
    ///
    /// and `anim` stays populated, because it is what a sheet with none of
    /// the chain draws. A body playing a move is still semantically in a pose.
    pub clip: Option<ClipRequest>,
    /// This body's SMASH CHARGE, `None` when it is not charging.
    ///
    /// Normalized `0..=1`: it appears when the hold latches, rises to `1.0` at
    /// maximum, and goes back to `None` the instant the move releases — so
    /// latched / building / loaded / released are all readable from this one
    /// value. Resolved by simulation (`MovePlayback::smash_charge_fraction`).
    ///
    /// ⛔ presentation must not re-derive it from move names or Startup
    /// progress: a tapped smash and a fully held one share both.
    pub smash_charge: Option<f32>,
}

/// The clip + fallbacks one active move asks for. See [`ActorAnimFrame::clip`].
///
/// owned strings rather than borrows: this is a materialized read-model that
/// outlives the frame's queries, and a move's chain is three or four short names
/// resolved once per drawn actor.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipRequest {
    pub clip: String,
    pub fallbacks: Vec<String>,
}

impl ClipRequest {
    /// Build a request from a static preference chain — the shape
    /// `ambition_character_sprites::body_state_clip` answers in.
    pub fn from_chain(chain: &[&str]) -> Option<Self> {
        let (clip, fallbacks) = chain.split_first()?;
        Some(Self {
            clip: (*clip).to_string(),
            fallbacks: fallbacks.iter().map(|f| (*f).to_string()).collect(),
        })
    }

    /// The chain in preference order — the exact clip, then the author's
    /// fallbacks. Feed straight to `CharacterAnimator::request_clip`.
    pub fn chain(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.clip.as_str()).chain(self.fallbacks.iter().map(String::as_str))
    }

    /// The same request with `clip` tried FIRST and everything this request
    /// already asked for kept behind it.
    ///
    /// A body state that outranks the move's own row says so this way rather
    /// than by replacing the chain: a sheet that does not author the new row
    /// falls straight through to what the move wanted, which is the graceful
    /// degradation `CharacterAnimator::clip_slot` already provides for free.
    pub fn ahead_of(self, clip: &str) -> Self {
        let mut fallbacks = Vec::with_capacity(self.fallbacks.len() + 1);
        fallbacks.push(self.clip);
        fallbacks.extend(self.fallbacks);
        Self {
            clip: clip.to_string(),
            fallbacks,
        }
    }

    /// A request for `clip` alone — the head of a chain a body state names when
    /// no move is naming one.
    pub fn only(clip: &str) -> Self {
        Self {
            clip: clip.to_string(),
            fallbacks: Vec::new(),
        }
    }
}

/// The sheet row a body holding a smash charge asks for. Authored on the
/// shipped fighter sheets; a sheet without it falls through the chain.
pub const SMASH_CHARGE_CLIP: &str = "smash_charge";

// The per-actor identity accessors (`ecs_actor_name`, `ecs_actor_is_sandbag`,
// `ecs_enemy_sprite_override`, `ecs_actor_render_size`) are GONE: those static
// facts are now materialized once into `ActorRenderIndex` (see
// `rebuild_actor_render_index`), which the renderer reads by id — so
// presentation no longer live-queries the actor clusters to bind a sprite. The
// per-frame ANIM frame below stays a live read until slice B materializes it.

/// Materialized per-frame animation pose for every actor, keyed by
/// [`FeatureId`] — the MOVING half of the actor read-model. `ActorAnimFrame`
/// stopped being `Copy` when it gained the active move's clip request
/// the chain is owned strings, because a materialized read-model
/// outlives the queries that built it. Presentation reads the pose by id and never borrows the
/// actor clusters to animate. Because this pose is presentation-ONLY, its
/// rebuild is registered in the render presentation plugin — NOT the sim
/// schedule — so a headless / RL build never pays for poses it won't draw.
#[derive(Resource, Default, Clone, Debug)]
pub struct ActorAnimIndex {
    frames: std::collections::HashMap<String, (ActorAnimFrame, u64)>,
    generation: u64,
}

impl ActorAnimIndex {
    /// borrowed rather than copied since the frame gained the active
    /// move's clip chain — the caller draws from it in place and never needs to
    /// own it, so nothing clones the strings per actor per frame.
    pub fn get(&self, id: &str) -> Option<&ActorAnimFrame> {
        self.frames.get(id).map(|(frame, _)| frame)
    }

    /// Every `(id, frame)` row. A presentation pass that acts on "whichever
    /// actors are doing X right now" walks this instead of asking the sim.
    ///
    /// AMBITION_REVIEW(determinism): hash-order iteration is safe here for the
    /// reason `FeatureViewIndex::iter` gives — this index is DERIVED state,
    /// rebuilt from the sim every frame, excluded from snapshots and from the
    /// state hash. Every consumer is presentation, so its order can never enter
    /// a trajectory.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ActorAnimFrame)> {
        self.frames
            .iter()
            .map(|(id, (frame, _))| (id.as_str(), frame))
    }

    /// Build a fixture index directly — the same escape hatch
    /// `FeatureViewIndex` / `ActorRenderIndex` / `BossRenderIndex` offer, for
    /// the same reason: a consumer's test needs the read-model it consumes, not
    /// the whole sim that publishes it.
    ///
    /// Constructs a NEW index rather than mutating an existing one, so it
    /// cannot be misused to edit the live one mid-frame. The parameter is
    /// `entries` to match the siblings.
    pub fn from_rows(entries: impl IntoIterator<Item = (String, ActorAnimFrame)>) -> Self {
        let mut index = Self::default();
        for (id, frame) in entries {
            index.frames.insert(id, (frame, index.generation));
        }
        index
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.frames.retain(|_, (_, g)| *g == gen);
    }

    fn insert(&mut self, id: &str, frame: ActorAnimFrame) {
        let gen = self.generation;
        if let Some(slot) = self.frames.get_mut(id) {
            slot.0 = frame;
            slot.1 = gen;
        } else {
            self.frames.insert(id.to_string(), (frame, gen));
        }
    }
}

/// Resolve EVERY brain-driven actor's animation frame from its REAL ECS clusters
/// — the SAME `Body*` movement/ability clusters, and the SAME picker, the player
/// uses ([`ambition_character_sprites::pick_actor_anim`] → `body_view_from_body`).
/// One path, disposition-agnostic: an enemy and an NPC animate from identical
/// reads. Whatever a brain (or an LLM) drives the actor's clusters into — a dash,
/// a blink, flight, a shield, a ladder climb, a wall-grab, a dodge-roll, a
/// crouch/slide, an in-flight swing — animates with no per-archetype branch; the
/// sheet's anim set decides how richly each pose reads. The picked poses land in
/// [`ActorAnimIndex`] for the renderer to consume by id.
pub fn rebuild_actor_anim_index(mut index: ResMut<ActorAnimIndex>, actors: Query<ActorSpriteData>) {
    index.begin_rebuild();
    for a in &actors {
        let anim = ambition_character_sprites::pick_actor_anim(
            a.kin,
            a.ground,
            a.motion_facts,
            a.flight,
            a.body_mode,
            a.env_contact,
            a.abilities,
            a.shield,
            a.attack.swing.as_ref(),
            ambition_character_sprites::ActorAnimState {
                alive: a.health.alive(),
                hit_flash: a.combat.hit_flash > 0.0,
                // Gravity-free FLIGHT archetype (parrot / shark): the locomotion
                // tail reads Fly/Idle and the airborne gate is suppressed.
                aerial: a.config.tuning.is_aerial,
                // Movement overlays from the shared BodyAnimFacts (None → all off).
                wall_jump: a.anim.is_some_and(|f| f.wall_jump_anim_timer > 0.0),
                dash_startup: a.anim.is_some_and(|f| f.dash_startup_timer > 0.0),
                landing: a
                    .anim
                    .filter(|f| f.land_anim_timer > 0.0)
                    .map(|f| f.land_anim_hard),
                shooting: a.anim.is_some_and(|f| f.shoot_anim_timer > 0.0),
                rolling: a.anim.is_some_and(|f| f.rolling),
                held: a.anim.is_some_and(|f| f.held),
            },
            a.frame.map_or_else(
                || ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR),
                |f| f.basis(),
            ),
        );
        // A content pose PIN wins over the picked pose (e.g. a shelled enemy's
        // withdraw cycle, which the disposition-agnostic picker cannot infer).
        let anim = a.anim_override.map(|o| o.0).unwrap_or(anim);
        let charge = a
            .playback
            .and_then(ambition_combat::moveset::MovePlayback::smash_charge_fraction);
        index.insert(
            a.feature_id.as_str(),
            ActorAnimFrame {
                anim,
                pos: a.kin.pos,
                facing: a.kin.facing,
                // what the ACTIVE MOVE asks to be drawn as. The move's own
                // timeline is authoritative for presentation as well as
                // gameplay, so this is the move speaking, not a guess about it.
                // a MOVE names its row; failing that, a fighter STATE does
                // (sprite redirect P2 — air dodge, tumble, knockdown, getup).
                // the move wins: a body that is mid-swing while tumbling is
                // drawn as its swing, which is what its timeline says it is.
                smash_charge: charge,
                // A HELD CHARGE outranks the move's own row, and only while it
                // is held: the whole point of the beat is that a fighter
                // winding up looks different from one swinging. It goes AHEAD
                // of the move's chain rather than replacing it, so a sheet
                // that authors no charge row draws exactly what it drew before.
                clip: a
                    .playback
                    .map(|playback| ClipRequest {
                        clip: playback.spec.clip.clip.clone(),
                        fallbacks: playback.spec.clip.fallbacks.clone(),
                    })
                    .or_else(|| {
                        ClipRequest::from_chain(ambition_character_sprites::body_state_clip(
                            a.motion_facts,
                            ambition_character_sprites::FighterClipFacts {
                                held: a.anim.is_some_and(|f| f.held),
                                holding: a.anim.is_some_and(|f| f.holding),
                                guard_break: a
                                    .shield
                                    .break_phase()
                                    .map(ambition_character_sprites::GuardBreakBeat::from_phase),
                                parrying: a.shield.parrying(),
                                guard_stunned: a.shield.stun_timer > 0.0,
                            },
                        )?)
                    })
                    .map(|chain| match charge {
                        Some(_) => chain.ahead_of(SMASH_CHARGE_CLIP),
                        None => chain,
                    })
                    .or_else(|| charge.map(|_| ClipRequest::only(SMASH_CHARGE_CLIP))),
            },
        );
    }
    index.end_rebuild();
}

#[derive(Clone, Copy, Debug)]
pub struct HazardLaneFact {
    /// `true` during the strike window (red solid); `false` during
    /// telegraph (yellow pulsing).
    pub striking: bool,
    pub center: ae::Vec2,
    pub size: ae::Vec2,
}

/// Materialized per-frame boss presentation facts, keyed by [`FeatureId`]: the resolved
/// [`BossAnimState`] (facing / tint / row-selection facts), the boss's collision AABB, and the
/// hazard-column lane when one is live. The MOVING half of the boss read-model — `BossRenderIndex`
/// carries the static identity.
#[derive(Resource, Default, Clone, Debug)]
pub struct BossFrameIndex {
    frames: std::collections::HashMap<String, (BossFrameView, u64)>,
    generation: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct BossFrameView {
    pub anim: ambition_boss_encounter::sprites::BossAnimState,
    /// The SIM-owned draw cursor (`BossAnimFrame`), published by id so the
    /// render's draw-only [`BossAnimator`] can mirror the advancing frame WITHOUT
    /// borrowing the sim entity's component. The render's `FeatureVisual` entity
    /// is a separate by-id mirror (it never carries the sim `BossAnimFrame`), so
    /// the frame — like every other sim→render fact — has to cross the boundary
    /// through this read-model. `drive_boss_animators` advances the cursor earlier
    /// in the sim tick; this captures its current value.
    pub cursor_anim: ambition_boss_encounter::sprites::BossAnim,
    pub cursor_frame: usize,
    /// The boss's combat AABB (debug health bars anchor here).
    pub aabb: ae::Aabb,
    pub hazard_lane: Option<HazardLaneFact>,
}

impl BossFrameIndex {
    pub fn get(&self, id: &str) -> Option<BossFrameView> {
        self.frames.get(id).map(|(frame, _)| *frame)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &BossFrameView)> {
        self.frames
            .iter()
            .map(|(id, (frame, _))| (id.as_str(), frame))
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.frames.retain(|_, (_, g)| *g == gen);
    }

    fn insert(&mut self, id: &str, frame: BossFrameView) {
        let gen = self.generation;
        if let Some(slot) = self.frames.get_mut(id) {
            slot.0 = frame;
            slot.1 = gen;
        } else {
            self.frames.insert(id.to_string(), (frame, gen));
        }
    }
}

pub fn rebuild_boss_frame_index(
    mut index: ResMut<BossFrameIndex>,
    bosses: Query<(
        &FeatureId,
        ambition_boss_encounter::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
        // The SIM-owned draw cursor. `Option` so a boss fixture spawned without the anim cursor
        // still lands in the index (it just draws Rest frame 0).
        Option<&ambition_boss_encounter::sprites::BossAnimFrame>,
    )>,
) {
    use ambition_boss_encounter::sprites::BossAnim;
    use ambition_characters::brain::BossAttackProfile;
    index.begin_rebuild();
    for (id, feature, health, combat, attack_state, brain, anim_frame) in &bosses {
        let boss = feature.as_boss_ref();
        let anim = boss_anim_state_for(boss, health.alive(), combat.hit_flash, attack_state, brain);
        let (cursor_anim, cursor_frame) = anim_frame
            .map(|f| (f.current, f.frame))
            .unwrap_or((BossAnim::Rest, 0));
        // Hazard-column lane: live only while an ALIVE boss telegraphs or
        // strikes `hazard_column`; the rect reuses the damage volume math.
        let in_telegraph = matches!(
            &attack_state.telegraph_profile,
            Some(p) if p.move_id() == "hazard_column"
        );
        let in_strike = matches!(
            &attack_state.active_profile,
            Some(p) if p.move_id() == "hazard_column"
        );
        let hazard_lane = if health.alive() && (in_telegraph || in_strike) {
            let boss = feature.as_boss_ref();
            ambition_boss_encounter::attack_geometry::volumes_for_profile(
                &BossAttackProfile::Strike("hazard_column".to_string()),
                boss.kin.pos,
                boss.combat_size(),
                &boss.config.behavior,
            )
            .pop()
            .map(|volume| HazardLaneFact {
                striking: in_strike,
                center: volume.center(),
                size: volume.half_size() * 2.0,
            })
        } else {
            None
        };
        let boss = feature.as_boss_ref();
        index.insert(
            id.as_str(),
            BossFrameView {
                anim,
                cursor_anim,
                cursor_frame,
                aabb: boss.aabb(),
                hazard_lane,
            },
        );
    }
    index.end_rebuild();
}
