//! The actor and boss animation views.
//! Actor pose clocks advance each tick. Clip requests use the final pose for the frame.

use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::{Query, Res, ResMut, Resource, Time, With};

use ambition_boss_encounter::anim::boss_anim_state_for;
use ambition_characters::actor::ai::ActorStatus;
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::{ActorIdentity, FeatureId};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_core::BodyKinematics;
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

/// Required body components for the actor pose and appearance indexes.
pub type ActorSpriteBody = (
    With<ActorIdentity>,
    With<ActorStatus>,
    With<ambition_characters::actor::BodyHealth>,
    With<BodyKinematics>,
    With<ae::BodyGroundState>,
    With<ae::BodyMotionFacts>,
    With<ae::BodyFlightState>,
    With<ae::BodyModeState>,
    With<ae::BodyEnvironmentContact>,
    With<ae::BodyAbilities>,
    With<ae::BodyShieldState>,
);

/// Movement and action facts used to select an actor's pose.
#[derive(bevy::ecs::query::QueryData)]
pub(crate) struct ActorPoseData {
    pub feature_id: &'static FeatureId,
    pub kin: &'static BodyKinematics,
    pub health: &'static ambition_characters::actor::BodyHealth,
    pub combat: &'static ambition_characters::actor::BodyCombat,
    pub config: &'static ActorConfig,
    /// The body's own moveset. With `playback` it gives the swing
    /// (`melee_swing_of`).
    pub moveset: Option<&'static ambition_combat::moveset::ActorMoveset>,
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
    /// The movement policy, for the body's own top speed (the sprint row).
    pub model: Option<&'static ae::MotionModel>,
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
    /// A rider sits its mount (`crate::pose_view::seated`).
    pub riding: bevy::prelude::Has<ambition_mount::RidingOn>,
    pub playback: Option<&'static ambition_combat::moveset::MovePlayback>,
    pub unmirrored: bevy::prelude::Has<ae::Unmirrored>,
}

#[derive(bevy::ecs::query::QueryData)]
pub(crate) struct ActorClipData {
    feature_id: &'static FeatureId,
    kin: &'static BodyKinematics,
    combat: &'static ambition_characters::actor::BodyCombat,
    motion_facts: &'static ae::BodyMotionFacts,
    frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    ground: &'static ae::BodyGroundState,
    shield: &'static ae::BodyShieldState,
    anim: Option<&'static ambition_characters::actor::BodyAnimFacts>,
    control_holds: Option<&'static ambition_characters::control::ControlHolds>,
    playback: Option<&'static ambition_combat::moveset::MovePlayback>,
    anim_override: Option<&'static ActorAnimOverride>,
    pinned: Option<&'static ambition_sprite_sheet::character::PinnedRow>,
    death_beat: Option<&'static ambition_combat::components::DeathBeat>,
    target: Option<&'static ambition_combat::components::ActorTarget>,
}

/// One actor's resolved animation frame for the renderer: the chosen anim plus
/// the bits the per-frame apply needs that aren't in the anim itself — world
/// position (for localized-gravity facing) and facing sign.
#[derive(Clone, Debug)]
pub struct ActorAnimFrame {
    pub anim: CharacterAnim,
    pub opacity: f32,
    pub conversation_held: bool,
    pub pos: ae::Vec2,
    /// The side the body is DRAWN toward ([`ae::mirror_side`]), not its facing:
    /// an [`ae::Unmirrored`] body reads `+1` while it faces left.
    pub facing: f32,
    /// The body's own toward-feet direction, from its resolved frame (the
    /// default down for a body with none). The facing flip reads it, so the
    /// render does not resolve gravity a second time.
    pub gravity_dir: ae::Vec2,
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
    pub move_charge: Option<f32>,
    /// How the body is drawn while it goes through a door or a gate: the twin
    /// of the `PassageLookView` beside a player body's `BodyPoseView`, here
    /// because a possessed body and an NPC take this road. `None` outside a
    /// passage. Set by `rebuild_passage_views`, with the passage's row.
    pub passage_look: Option<crate::PassageLookView>,
    /// The body's last stop on the ground: the player road's rule
    /// ([`crate::StopClock`]), so a fighter stops as the player does.
    pub stop: crate::StopClock,
    /// The body's last turn on the ground: the player road's rule
    /// ([`crate::TurnClock`]), so a fighter turns as the player does.
    pub turn: crate::TurnClock,
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
    /// The clock supplied by the requester. None uses the sheet's clock.
    pub time: Option<ClipTime>,
    /// A content clock for these rows. A fallback keeps the base clock.
    pub row_clock: Option<(Vec<String>, ClipTime)>,
    /// The row is the picture of the body above the hips, and this is the
    /// pose of its legs: what the body does under the move (it walks, runs,
    /// jumps or falls). `None`: the row is the whole body. See
    /// `WindowTag::UpperBody`.
    pub legs: Option<ambition_sprite_sheet::character::CharacterAnim>,
}

/// The position of a clip on its requester's clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClipTime {
    Phase(f32),
    Seconds { elapsed: f32, looping: bool },
    /// The sheet's own clock, round and round: a stance held for as long as
    /// the body is in it (a guard), not a beat with an end.
    Looping,
}

impl ClipRequest {
    /// Put a content pin ahead of the body's request and retain its clock.
    ///
    /// A `crouched` body is drawn crouched: each pinned row gives way to its
    /// crouched variant ([`ambition_sprite_sheet::character::stance_row`]), and a sheet with none
    /// draws the body's own request (its crouch). A pinned standing row would
    /// be drawn feet-first on the crouched box, at full height. The variants
    /// use the pin's clock. A missing variant keeps the base request's clock,
    /// so a fallback crouch is not looped by the content pin.
    ///
    /// A pin made [`PinnedRow::pin_over_legs`](ambition_sprite_sheet::character::PinnedRow::pin_over_legs)
    /// draws only the upper body: over a body that walks, runs or falls
    /// (`legs`, when [`legs_go_on_under`]) the legs keep that pose.
    pub fn with_pin(
        base: Option<Self>,
        pin: Option<&ambition_sprite_sheet::character::PinnedRow>,
        crouched: bool,
        legs: Option<ambition_sprite_sheet::character::CharacterAnim>,
    ) -> Option<Self> {
        let Some(pin) = pin.filter(|pin| pin.is_pinned()) else { return base; };
        let rows: Vec<String> = pin.rows.iter().map(|row| ambition_sprite_sheet::character::stance_row(row, crouched)).collect();
        let base_time = base.as_ref().and_then(|base| base.time);
        let mut request = match base {
            Some(base) => rows.iter().rev().fold(base, |chain, row| chain.ahead_of(row)),
            None => Self::from_chain(&rows.iter().map(String::as_str).collect::<Vec<_>>())?,
        };
        request.time = base_time;
        request.row_clock = Some((rows, ClipTime::Seconds { elapsed: pin.elapsed, looping: pin.looping }));
        request.legs = legs.filter(|legs| pin.over_legs && legs_go_on_under(*legs));
        Some(request)
    }

    /// The clock of the row the animator selected, before facing.
    pub fn time_for_row(&self, row: &str) -> Option<ClipTime> {
        self.row_clock.as_ref()
            .filter(|(rows, _)| rows.iter().any(|name| name == row))
            .map(|(_, time)| *time)
            .or(self.time)
    }

    /// Build a request from a static preference chain — the shape
    /// `ambition_character_sprites::body_state_clip` answers in.
    pub fn from_chain(chain: &[&str]) -> Option<Self> {
        let (clip, fallbacks) = chain.split_first()?;
        Some(Self {
            clip: (*clip).to_string(),
            fallbacks: fallbacks.iter().map(|f| (*f).to_string()).collect(),
            time: None,
            row_clock: None,
            legs: None,
        })
    }

    /// The row a playing MOVE asks for, slaved to its progress.
    ///
    /// ⛔ This is the contract `MoveSpec` and `MovePlayback::phase` always
    /// documented ("the clip is SLAVED to the move; it never runs its own
    /// clock") and nothing implemented: the animator ran every move row on the
    /// sheet's frame rate, so a 3-frame 180 ms tilt row froze on its last frame
    /// for the rest of a 310 ms move, and a 496 ms throw row was cut off at
    /// half. Both constructions of a move's request go through here.
    pub fn for_move(binding: &ambition_entity_catalog::ClipBinding, phase: f32) -> Self {
        Self {
            clip: binding.clip.clone(),
            fallbacks: binding.fallbacks.clone(),
            time: Some(ClipTime::Phase(phase)),
            row_clock: None,
            legs: None,
        }
    }

    /// The same request, with the row as the picture of the upper body and
    /// `legs` as the pose of the legs under it.
    pub fn over_legs(mut self, legs: Option<ambition_sprite_sheet::character::CharacterAnim>) -> Self {
        self.legs = legs;
        self
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
            // The row placed ahead is a BODY STATE's (a held charge), and the
            // move's progress is frozen while it shows — it runs its own clock.
            time: None,
            row_clock: None,
            legs: None,
        }
    }

    /// A request for `clip` alone — the head of a chain a body state names when
    /// no move is naming one.
    pub fn only(clip: &str) -> Self {
        Self {
            clip: clip.to_string(),
            fallbacks: Vec::new(),
            time: None,
            row_clock: None,
            legs: None,
        }
    }
}

/// The sheet row a body holding a ranged charge asks for, above its hips.
pub const AIM_CLIP: &str = "aim";

/// The poses a body's legs can keep under a row that draws only its upper
/// body (a move's `WindowTag::UpperBody`, a held aim): it walks, runs, jumps,
/// falls, flies or glides. Any other pose is the whole body.
pub fn legs_go_on_under(anim: ambition_sprite_sheet::character::CharacterAnim) -> bool {
    use ambition_sprite_sheet::character::CharacterAnim::*;
    matches!(anim, Walk | Run | Sprint | Jump | Fall | Fly | FloatGlide)
}

/// A held aim over a body that moves: the aim row above the hips and the
/// body's own pose below them. The picker draws a body that stands and aims
/// as `Aim` itself; one that walks while it charges keeps walking.
pub fn aim_over_legs(aiming: bool, anim: ambition_sprite_sheet::character::CharacterAnim) -> Option<ClipRequest> {
    (aiming && legs_go_on_under(anim)).then(|| ClipRequest::only(AIM_CLIP).over_legs(Some(anim)))
}

/// The rows a body stands and walks in while it squares up to a foe: guard
/// up, weight low. Authored on the person rig; a sheet without them draws the
/// body's own idle and walk.
pub const FIGHT_IDLE_CLIP: &str = "fight_idle";
pub const FIGHT_WALK_CLIP: &str = "fight_walk";

/// How near a picked foe must be (px) for a body to square up to it. Farther
/// off it is still on its way, and walks or runs as it would.
pub const ENGAGED_RANGE: f32 = 220.0;

/// The guard a body that stands or walks near a foe is drawn in. `None` for
/// any other pose (a run, a jump, a swing), or with no foe in range: a body
/// that only HAS a target across the room is not yet fighting it, and a body
/// mid-move is drawn as its move.
pub fn engaged_clip(
    anim: ambition_sprite_sheet::character::CharacterAnim,
    free: bool,
    at: ae::Vec2,
    target: Option<&ambition_combat::components::ActorTarget>,
) -> Option<ClipRequest> {
    use ambition_sprite_sheet::character::CharacterAnim::{Idle, Walk};
    if !free {
        return None;
    }
    let row = match anim {
        Idle => FIGHT_IDLE_CLIP,
        Walk => FIGHT_WALK_CLIP,
        _ => return None,
    };
    target
        .is_some_and(|t| t.entity.is_some() && t.pos.distance(at) <= ENGAGED_RANGE)
        .then(|| ClipRequest { time: Some(ClipTime::Looping), ..ClipRequest::only(row) })
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

/// Actor animation data for the frame, indexed by feature ID.
#[derive(Resource, Default, Clone, Debug)]
pub struct ActorAnimIndex {
    rows: Vec<ActorAnimRow>,
    slots: HashMap<Arc<str>, usize>,
    generation: u64,
}

#[derive(Clone, Debug)]
struct ActorAnimRow {
    id: Arc<str>,
    frame: ActorAnimFrame,
    seen: u64,
    passage_clip: Option<&'static [&'static str]>,
}

impl ActorAnimIndex {
    pub fn get(&self, id: &str) -> Option<&ActorAnimFrame> {
        self.slots.get(id).map(|&slot| &self.rows[slot].frame)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ActorAnimFrame)> {
        self.rows.iter().map(|row| (row.id.as_ref(), &row.frame))
    }

    /// Build an index from a list of actor poses.
    pub fn from_rows(entries: impl IntoIterator<Item = (String, ActorAnimFrame)>) -> Self {
        let mut index = Self::default();
        for (id, frame) in entries {
            if let Some(&slot) = index.slots.get(id.as_str()) {
                index.rows[slot].frame = frame;
            } else {
                index.insert(&id, frame);
            }
        }
        index
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(crate) fn apply_passage(
        &mut self,
        id: &str,
        anim: CharacterAnim,
        clip: &'static [&'static str],
        look: crate::PassageLookView,
    ) {
        if let Some(&slot) = self.slots.get(id) {
            let row = &mut self.rows[slot];
            row.frame.anim = anim;
            row.frame.passage_look = Some(look);
            row.passage_clip = Some(clip);
        }
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn insert(&mut self, id: &str, frame: ActorAnimFrame) -> &mut ActorAnimRow {
        let id: Arc<str> = Arc::from(id);
        let slot = self.rows.len();
        self.slots.insert(Arc::clone(&id), slot);
        self.rows.push(ActorAnimRow {
            id,
            frame,
            seen: self.generation,
            passage_clip: None,
        });
        &mut self.rows[slot]
    }

    fn end_rebuild(&mut self) {
        let len = self.rows.len();
        self.rows.retain(|row| row.seen == self.generation);
        if len != self.rows.len() {
            self.slots.clear();
            self.slots.extend(self.rows.iter().enumerate().map(|(slot, row)| (Arc::clone(&row.id), slot)));
        }
    }

    fn slot_at(&self, position: usize, id: &str) -> Option<usize> {
        if self.rows.get(position).is_some_and(|row| row.id.as_ref() == id) {
            Some(position)
        } else {
            self.slots.get(id).copied()
        }
    }

    fn publish_at(&mut self, position: usize, id: &str) -> (&mut ActorAnimFrame, bool) {
        if let Some(slot) = self.slot_at(position, id) {
            let row = &mut self.rows[slot];
            row.seen = self.generation;
            row.passage_clip = None;
            return (&mut row.frame, false);
        }
        (&mut self.insert(id, ActorAnimFrame {
            anim: CharacterAnim::Idle,
            opacity: 1.0,
            conversation_held: false,
            pos: ae::Vec2::ZERO,
            facing: 1.0,
            gravity_dir: ae::DEFAULT_GRAVITY_DIR,
            clip: None,
            move_charge: None,
            passage_look: None,
            stop: crate::StopClock::default(),
            turn: crate::TurnClock::default(),
        }).frame, true)
    }
}

/// Save the pose each tick to detect stops and turns between rendered frames.
pub(crate) fn advance_actor_anim_poses(
    mut index: ResMut<ActorAnimIndex>,
    actors: Query<ActorPoseData, ActorSpriteBody>,
    time: Option<Res<Time>>,
) {
    let dt = time.as_deref().map_or(0.0, Time::delta_secs);
    index.begin_rebuild();
    for (position, a) in actors.iter().enumerate() {
        let free = free_pose(a.playback, a.anim_override);
        let anim = a.anim_override.map_or_else(|| pick_actor_pose(&a), |pose| pose.0);
        let anim = crate::pose_view::seated(anim, a.riding);
        let (frame, added) = index.publish_at(position, a.feature_id.as_str());
        let facing = ae::mirror_side(a.kin.facing, a.unmirrored);
        if !added {
            frame.stop = frame.stop.next(Some(frame.anim), anim, free, dt);
            frame.turn = frame.turn.next(Some((frame.anim, frame.facing)), anim, facing, free, dt);
        }
        frame.anim = anim;
        frame.facing = facing;
        frame.passage_look = None;
    }
    index.end_rebuild();
}

fn pick_actor_pose(a: &ActorPoseDataItem<'_, '_>) -> CharacterAnim {
    let swing = ambition_combat::moveset::melee_swing_of(a.playback, a.moveset);
    ambition_character_sprites::pick_actor_anim(
        a.kin,
        a.ground,
        a.motion_facts,
        a.flight,
        a.body_mode,
        a.env_contact,
        a.abilities,
        a.shield,
        swing.as_ref(),
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
            aiming: a.anim.is_some_and(|f| f.aim_anim_active),
            rolling: a.anim.is_some_and(|f| f.rolling),
            held: a.anim.is_some_and(|f| f.held),
        },
        a.frame.map_or_else(
            || ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR),
            |f| f.basis(),
        ),
        ambition_character_sprites::MotionSpeeds::of(a.model),
    )
}

fn free_pose(
    playback: Option<&ambition_combat::moveset::MovePlayback>,
    pose_override: Option<&ActorAnimOverride>,
) -> bool {
    playback.is_none_or(|playback| playback.settled()) && pose_override.is_none()
}

/// Publish the clip and drawing data for the pose at the end of the frame.
pub(crate) fn rebuild_actor_anim_index(
    mut index: ResMut<ActorAnimIndex>,
    actors: Query<ActorClipData, (ActorSpriteBody, With<ActorConfig>)>,
) {
    for (position, a) in actors.iter().enumerate() {
        let Some(slot) = index.slot_at(position, a.feature_id.as_str()) else { continue; };
        let row = &mut index.rows[slot];
        let frame = &mut row.frame;
        let charge = a.playback.and_then(ambition_combat::moveset::MovePlayback::smash_charge_fraction);
        frame.conversation_held = a.control_holds.is_some_and(|holds| {
            holds.holds(ambition_characters::control::ControlHold::Conversation)
        });
        frame.pos = a.kin.pos;
        frame.opacity = a.death_beat.map_or(1.0, |beat| beat.opacity());
        frame.gravity_dir = a.frame.map_or(ae::DEFAULT_GRAVITY_DIR, |f| f.down());
        frame.move_charge = charge;
        if let Some(chain) = row.passage_clip {
            frame.clip = ClipRequest::from_chain(chain);
            continue;
        }
        let anim = frame.anim;
        let free = free_pose(a.playback, a.anim_override);

        // A charge takes priority over the move and keeps its clip as a fallback.
        let clip = a
            .playback
            .filter(|playback| !playback.settled())
            .map(|playback| ClipRequest::for_move(&playback.spec.clip, playback.phase()))
            .or_else(|| frame.turn.clip(anim, free))
            .or_else(|| frame.stop.clip(anim, free))
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
                        // The actor road's own `hit` read
                        // (`ActorAnimState::hit_flash`).
                        recoil: (a.combat.hit_flash > 0.0).then(|| {
                            ambition_character_sprites::Recoil::of(
                                a.combat.struck_height,
                                ambition_character_sprites::extent_along(a.kin.size(), frame.gravity_dir),
                                !a.ground.on_ground,
                            )
                        }),
                        ..Default::default()
                    }
                    .with_gestures(a.anim),
                )?)
            })
            .or_else(|| engaged_clip(anim, free, a.kin.pos, a.target))
            .map(|chain| match charge {
                Some(_) => chain.ahead_of(SMASH_CHARGE_CLIP),
                None => chain,
            })
            .or_else(|| charge.map(|_| ClipRequest::only(SMASH_CHARGE_CLIP)))
            .or_else(|| aim_over_legs(a.anim.is_some_and(|f| f.aim_anim_active) && free, anim));
        frame.clip = ClipRequest::with_pin(clip, a.pinned, crate::pose_view::crouched(anim), Some(anim));
    }
}

#[cfg(test)]
pub(crate) fn spawn_pose_fixture(app: &mut bevy::prelude::App, id: &str) -> bevy::prelude::Entity {
    use ambition_characters::actor::{BodyCombat, BodyHealth, Health};

    app.world_mut().spawn((
        FeatureId::new(id),
        ActorIdentity::new("test", "Test actor"),
        ActorStatus { respawn_timer: 0.0 },
        ActorConfig {
            tuning: Default::default(),
            brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
            preserves_mirror_symmetry: false,
        },
        BodyHealth::new(Health::new(10)),
        BodyCombat::default(),
        BodyKinematics { facing: 1.0, ..Default::default() },
        ae::BodyGroundState { on_ground: true, ..Default::default() },
        ae::BodyMotionFacts::default(),
        ae::BodyFlightState::default(),
        ae::BodyModeState::default(),
        ae::BodyEnvironmentContact::default(),
        ae::BodyAbilities::default(),
        ae::BodyShieldState::default(),
    )).id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, FixedUpdate, Update};

    #[test]
    fn a_stop_between_rendered_frames_uses_its_tick_clock() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(20));
        app.insert_resource(time)
            .init_resource::<ActorAnimIndex>()
            .add_systems(FixedUpdate, advance_actor_anim_poses)
            .add_systems(Update, rebuild_actor_anim_index);
        let actor = spawn_pose_fixture(&mut app, "walker");
        let world = app.world_mut();
        world.get_mut::<BodyKinematics>(actor).unwrap().vel.x = 100.0;
        world.run_schedule(FixedUpdate);
        assert_eq!(world.resource::<ActorAnimIndex>().get("walker").unwrap().anim, CharacterAnim::Walk);

        world.get_mut::<BodyKinematics>(actor).unwrap().vel.x = 0.0;
        world.run_schedule(FixedUpdate);
        world.get_mut::<BodyKinematics>(actor).unwrap().pos.x = 20.0;
        world.run_schedule(FixedUpdate);
        assert!(world.resource::<ActorAnimIndex>().get("walker").unwrap().clip.is_none());

        world.run_schedule(Update);
        let frame = world.resource::<ActorAnimIndex>().get("walker").unwrap();
        assert_eq!(frame.anim, CharacterAnim::Idle);
        assert_eq!(frame.pos.x, 20.0);
        let stop = frame.clip.clone().unwrap();
        assert_eq!(stop.clip, "walk_stop");
        assert_eq!(frame.stop.secs, 0.02);
        assert!(matches!(stop.time, Some(ClipTime::Phase(phase)) if phase > 0.0 && phase < 1.0));

        world.run_schedule(Update);
        assert_eq!(world.resource::<ActorAnimIndex>().get("walker").unwrap().clip.as_ref(), Some(&stop));

        world.get_mut::<BodyKinematics>(actor).unwrap().facing = -1.0;
        world.run_schedule(FixedUpdate);
        world.run_schedule(Update);
        let frame = world.resource::<ActorAnimIndex>().get("walker").unwrap();
        assert_eq!(frame.facing, -1.0);
        assert_eq!(frame.clip.as_ref().unwrap().clip, "turnaround");
    }
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
    frames: bevy::platform::collections::HashMap<String, (BossFrameView, u64)>,
    generation: u64,
}

#[derive(Clone, Debug)]
pub struct BossFrameView {
    pub anim: ambition_sprite_sheet::boss::BossAnimState,
    pub opacity: f32,
    /// A NAMED row content pinned this boss to (`PinnedRow`): drawn ahead of
    /// the slot cursor when the sheet has it. Presentation only — the cursor
    /// and the geometry it drives carry on underneath.
    pub pinned: Option<ambition_sprite_sheet::character::PinnedRow>,
    /// The SIM-owned draw cursor (`BossAnimFrame`), published by id so the
    /// render's draw-only [`BossAnimator`] can mirror the advancing frame WITHOUT
    /// borrowing the sim entity's component. The render's `FeatureVisual` entity
    /// is a separate by-id mirror (it never carries the sim `BossAnimFrame`), so
    /// the frame — like every other sim→render fact — has to cross the boundary
    /// through this read-model. `drive_boss_animators` advances the cursor earlier
    /// in the sim tick; this captures its current value.
    pub cursor_anim: ambition_sprite_sheet::boss::BossAnim,
    pub cursor_frame: usize,
    /// Seconds of damage flash left on a live boss (`0.0` when dead). The
    /// renderer draws the `Hit` row over the cursor while it runs, for a boss
    /// that is not [`Self::posed_by_rig`]; the sim cursor itself never enters
    /// `Hit`.
    pub hit_flash_secs: f32,
    /// This boss is hit through parts that are posed from the row it is drawn
    /// with (it has a `BodyRig`). Its drawn row is then a fact of the sim:
    /// the renderer draws no presentation-only row over it, because the
    /// visible parts would be in one pose and the hurt parts in another.
    pub posed_by_rig: bool,
    /// The boss's combat AABB (debug health bars anchor here).
    pub aabb: ae::Aabb,
    pub hazard_lane: Option<HazardLaneFact>,
    /// The boss's own toward-feet direction, from its resolved frame (the
    /// default down for a boss with none). The facing flip reads it.
    pub gravity_dir: ae::Vec2,
}

impl crate::RowView for BossFrameIndex {
    fn has_rows(&self) -> bool {
        !self.frames.is_empty()
    }
}

impl BossFrameIndex {
    pub fn get(&self, id: &str) -> Option<&BossFrameView> {
        self.frames.get(id).map(|(frame, _)| frame)
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
    // The row each attack plays is authored data in the boss catalog.
    // REQUIRED, not optional: `engine.character-authority-is-app-local`
    // forbids an optional boss authority and a substituted empty catalog.
    catalog: bevy::prelude::Res<ambition_boss_encounter::BossCatalog>,
    bosses: Query<(
        &FeatureId,
        ambition_boss_encounter::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
        // The SIM-owned draw cursor. `Option` so a boss fixture spawned without the anim cursor
        // still lands in the index (it just draws Rest frame 0).
        Option<&ambition_sprite_sheet::boss::BossAnimFrame>,
        Option<&ambition_sprite_sheet::character::PinnedRow>,
        Option<&ambition_combat::components::DeathBeat>,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        bevy::prelude::Has<ambition_combat::body_rig::BodyRig>,
    )>,
) {
    use ambition_sprite_sheet::boss::BossAnim;
    use ambition_characters::brain::BossAttackProfile;
    index.begin_rebuild();
    for (id, feature, health, combat, attack_state, brain, anim_frame, pinned, death_beat, frame, posed_by_rig) in &bosses {
        let boss = feature.as_boss_ref();
        let anim = boss_anim_state_for(&catalog, boss, health.alive(), attack_state, brain);
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
                opacity: death_beat.map_or(1.0, |beat| beat.opacity()),
                pinned: pinned.filter(|pin| pin.is_pinned() || pin.layer.is_some()).cloned(),
                cursor_anim,
                cursor_frame,
                hit_flash_secs: if health.alive() {
                    combat.hit_flash
                } else {
                    0.0
                },
                posed_by_rig,
                aabb: boss.aabb(),
                hazard_lane,
                gravity_dir: frame.map_or(ae::DEFAULT_GRAVITY_DIR, |f| f.down()),
            },
        );
    }
    index.end_rebuild();
}

#[cfg(test)]
mod pin_tests {
    use super::*;
    use ambition_sprite_sheet::character::PinnedRow;

    fn rows(request: &ClipRequest) -> Vec<&str> {
        std::iter::once(request.clip.as_str()).chain(request.fallbacks.iter().map(String::as_str)).collect()
    }

    /// The firing row uses the content clock in either stance. A missing
    /// variant keeps the fallback crouch on its base clock.
    #[test]
    fn a_crouched_body_draws_a_pin_crouched_or_not_at_all() {
        let mut pin = PinnedRow::default();
        pin.pin(&["eye_laser", "shoot"], 0.4, true);
        let mut crouch = ClipRequest::only("crouch");
        crouch.time = Some(ClipTime::Phase(0.75));

        let crouched = ClipRequest::with_pin(Some(crouch.clone()), Some(&pin), true, None).expect("a request");
        assert_eq!(rows(&crouched), ["eye_laser_crouch", "shoot_crouch", "crouch"]);
        assert_eq!(crouched.time_for_row("eye_laser_crouch"), Some(ClipTime::Seconds { elapsed: 0.4, looping: true }));
        assert_eq!(crouched.time_for_row("crouch"), Some(ClipTime::Phase(0.75)), "a missing variant keeps the crouch clock");

        let standing = ClipRequest::with_pin(Some(crouch), Some(&pin), false, None).expect("a request");
        assert_eq!(rows(&standing), ["eye_laser", "shoot", "crouch"]);
        assert_eq!(standing.time_for_row("eye_laser"), Some(ClipTime::Seconds { elapsed: 0.4, looping: true }));
    }

    /// An upper-body pin keeps a walking body's legs; a whole-body pin and a
    /// standing body draw the pinned row alone.
    #[test]
    fn an_upper_body_pin_keeps_the_legs_that_move() {
        use ambition_sprite_sheet::character::CharacterAnim;
        let mut pin = PinnedRow::default();
        pin.pin_over_legs(&["eye_laser"], 0.2, true);
        let walk = ClipRequest::only("walk");
        let over = ClipRequest::with_pin(Some(walk.clone()), Some(&pin), false, Some(CharacterAnim::Walk)).expect("a request");
        assert_eq!((over.clip.as_str(), over.legs), ("eye_laser", Some(CharacterAnim::Walk)));
        let still = ClipRequest::with_pin(Some(walk.clone()), Some(&pin), false, Some(CharacterAnim::Idle)).expect("a request");
        assert_eq!(still.legs, None, "a body that stands is drawn in the pinned row whole");
        pin.pin(&["eye_laser"], 0.2, true);
        let whole = ClipRequest::with_pin(Some(walk), Some(&pin), false, Some(CharacterAnim::Walk)).expect("a request");
        assert_eq!(whole.legs, None, "a whole-body pin covers the legs");
    }
}
