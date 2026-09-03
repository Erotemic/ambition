//! Catalog entry + preset shapes. Serde-authored mirrors of the
//! `Brain` / `ActionSet` configs in `crate::brain`. Kept separate from
//! the runtime types so:
//!
//! 1. Brain cfgs can keep non-serde fields (per-actor `state`,
//!    `Vec<f32>` history buffers) without leaking serde into the
//!    tick path.
//! 2. RON authoring follows a stable, documented shape that doesn't
//!    move when an unrelated runtime detail changes.

use ambition_platformer2d_core as ae;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What tier a character occupies in the Hall of Characters and other
/// gallery rooms. Drives layout: `MainHall` characters get standard
/// 128 px slots; `Basement` characters get the wide 256 px slots.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum CharacterTier {
    MainHall,
    Basement,
}

/// Footprint hint. Today it only influences gallery layout; the
/// runtime physics footprint still comes from the sheet spec.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum CharacterBodyKind {
    Standard,
    Wide,
    Floating,
    Crawler,
}

impl CharacterBodyKind {
    /// The height a character of this kind stands, in world px, when its row
    /// does not say.
    ///
    /// only `Standard` answers, and that narrowness is the design. The complaint was about
    /// HUMANOIDS being mutually out of scale; a crawler, a floating drone and a wide body have
    /// no shared height to be consistent about, and inventing one for them would be changing
    /// sizes to satisfy a pattern rather than a report.
    ///
    /// 48.0 is not invented: it is the protagonist. `player_robot_v3`
    /// resolves to a 30x48 body through the sprite-authored seam, so a Standard
    /// character now stands exactly as tall as the character the player looks at
    /// most. A reference the game already contains beats a plausible number.
    pub fn default_standing_height(self) -> Option<f32> {
        match self {
            Self::Standard => Some(48.0),
            Self::Wide | Self::Floating | Self::Crawler => None,
        }
    }
}

/// Optional composition layer for multi-part sprites (bosses, etc.).
/// Dormant scaffolding — the renderer still emits a composed sheet
/// today, so the runtime ignores this field. Reserved for future
/// layered-render work without breaking the catalog schema.
#[allow(
    dead_code,
    reason = "Reserved for future layered-rendering of multi-part sprites; ships as schema-stable scaffolding so adding composition to a catalog entry is forwards-compatible."
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CompositionLayer {
    pub id: String,
    pub layer: i32,
    pub anchor_px: (f32, f32),
}

/// Per-character sprite gameplay tuning, authored in the catalog row.
///
/// The generated `*_spritesheet.ron` manifest carries everything the
/// sprite RENDERER knows (frame grid, rows, feet anchor); these are
/// the gameplay-side knobs it can't infer. Rows without this field
/// use middle-of-the-road defaults (`collision_scale: 1.5`,
/// `frame_sample_inset: 1`).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct SpriteTuningSpec {
    /// render_size = aabb_size * collision_scale (the sprite is drawn
    /// larger than the collision box so silhouettes read correctly).
    pub collision_scale: f32,
    /// Pixels trimmed from each frame edge when sampling, to drop
    /// generator border bleed.
    pub frame_sample_inset: u32,
    /// Override for the manifest's `feet_anchor_norm.y` when the
    /// generated anchor doesn't sit actors on the floor correctly.
    #[serde(default)]
    pub feet_anchor_y: Option<f32>,
}

/// Surface-momentum motion feel, authored on the catalog row (Q21). The
/// gameplay-side mirror of the serde-free kernel struct
/// [`ae::MomentumParams`](ambition_platformer2d_core::MomentumParams):
/// the kernel stays serde-free (its doc's contract), so this Deserialize twin
/// lives here and hydrates via [`to_kernel`](MomentumParamsSpec::to_kernel).
///
/// Every field carries a `#[serde(default = ...)]` matching the kernel's
/// `Default` value-for-value, so authored RON omits whatever it doesn't tune —
/// `momentum: Some(())` alone yields the kernel defaults. A character row that
/// carries this field opts its body into `MotionModel::SurfaceMomentum` (the
/// surface-follower solver); a row without it stays on the axis-swept path.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct MomentumParamsSpec {
    #[serde(default = "md_ground_accel")]
    pub ground_accel: f32,
    #[serde(default = "md_brake")]
    pub brake: f32,
    #[serde(default = "md_friction")]
    pub friction: f32,
    #[serde(default = "md_slope_factor")]
    pub slope_factor: f32,
    #[serde(default = "md_top_speed")]
    pub top_speed: f32,
    #[serde(default = "md_air_accel")]
    pub air_accel: f32,
    #[serde(default = "md_jump_speed")]
    pub jump_speed: f32,
    #[serde(default = "md_stick_factor")]
    pub stick_factor: f32,
    #[serde(default = "md_min_stick_speed")]
    pub min_stick_speed: f32,
}

// Per-field defaults, read straight off the kernel `Default` so the two never
// drift (the kernel is the single source of truth for the feel baseline).
fn md_ground_accel() -> f32 {
    ae::MomentumParams::default().ground_accel
}
fn md_brake() -> f32 {
    ae::MomentumParams::default().brake
}
fn md_friction() -> f32 {
    ae::MomentumParams::default().friction
}
fn md_slope_factor() -> f32 {
    ae::MomentumParams::default().slope_factor
}
fn md_top_speed() -> f32 {
    ae::MomentumParams::default().top_speed
}
fn md_air_accel() -> f32 {
    ae::MomentumParams::default().air_accel
}
fn md_jump_speed() -> f32 {
    ae::MomentumParams::default().jump_speed
}
fn md_stick_factor() -> f32 {
    ae::MomentumParams::default().stick_factor
}
fn md_min_stick_speed() -> f32 {
    ae::MomentumParams::default().min_stick_speed
}

impl Default for MomentumParamsSpec {
    fn default() -> Self {
        // Mirrors `ae::MomentumParams::default()` field-for-field.
        Self {
            ground_accel: md_ground_accel(),
            brake: md_brake(),
            friction: md_friction(),
            slope_factor: md_slope_factor(),
            top_speed: md_top_speed(),
            air_accel: md_air_accel(),
            jump_speed: md_jump_speed(),
            stick_factor: md_stick_factor(),
            min_stick_speed: md_min_stick_speed(),
        }
    }
}

impl MomentumParamsSpec {
    /// Hydrate into the serde-free kernel struct the surface solver consumes.
    pub fn to_kernel(&self) -> ae::MomentumParams {
        ae::MomentumParams {
            ground_accel: self.ground_accel,
            brake: self.brake,
            friction: self.friction,
            slope_factor: self.slope_factor,
            top_speed: self.top_speed,
            air_accel: self.air_accel,
            jump_speed: self.jump_speed,
            stick_factor: self.stick_factor,
            min_stick_speed: self.min_stick_speed,
        }
    }
}

/// Per-character AXIS-swept movement feel, authored on the catalog row — the
/// tuning twin of [`MomentumParamsSpec`] for the axis path (and the third
/// per-body override sibling, alongside [`momentum`](CharacterCatalogEntry::momentum)
/// and [`abilities`](CharacterCatalogEntry::abilities)).
///
/// A row that carries this field spawns its PLAYABLE body with an
/// [`AuthoredMovementTuning`](ambition_platformer2d_core::AuthoredMovementTuning)
/// component, so the body's live axis parameters are refreshed from THIS instead
/// of the global F3 dev tuning — a demo protagonist with a distinct jump keeps
/// its feel instead of tracking the shared inspector sliders. A row without it
/// (the default) leaves the body on the shared editable tuning, so every
/// existing character — and the F3 dev workflow — is untouched.
///
/// Character-owned projection of the reusable axis-swept movement laws.
///
/// The common case remains sparse: omitted fields retain the shared default, so
/// `axis_tuning: Some(())` is the default feel with an authored marker (the body
/// still escapes the session F3 slider). Characters whose identity depends on
/// locomotion may select a horizontal law, a jump law, and the small set of
/// scalar limits those laws consume without forking the collision kernel.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct AxisTuningSpec {
    /// Horizontal controller selected by this character. The default preserves
    /// Ambition's responsive controller; momentum-oriented characters opt into
    /// the reusable acceleration/brake/coast law explicitly.
    #[serde(default)]
    pub horizontal_law: ae::AxisHorizontalLaw,
    /// Jump-arc controller selected by this character. The default preserves
    /// Ambition's historical velocity-cut jump.
    #[serde(default)]
    pub jump_law: ae::AxisJumpLaw,
    /// Authored gravity response magnitude (px/s²). The environment still owns
    /// direction and zones; this is only the character's response strength.
    #[serde(default = "at_gravity")]
    pub gravity: f32,
    /// Mid-air jump count. Needs the `AirJump` grant to have any effect (the
    /// grant lights the *capability*; this is the *count*). The shared default
    /// is 1 (a double jump); `2` makes `AirJump` a triple jump.
    #[serde(default = "at_air_jumps")]
    pub air_jumps: u8,
    /// Ground-jump launch speed (px/s). Apex height is `v²/(2·gravity)`, so this
    /// is the *height* knob: scaling it by `√k` makes the jump `k` times as high
    /// against the shared gravity, and the airtime grows by `√k` with it. A
    /// character whose jump ARC is its identity (a floaty, ceiling-scraping
    /// platformer hop) authors it here rather than moving the shared default,
    /// which every other body — and the F3 dev slider — rides.
    #[serde(default = "at_jump_speed")]
    pub jump_speed: f32,
    /// Top ground speed (px/s) — the character's GAIT, the horizontal sibling of
    /// [`Self::jump_speed`]'s arc. A body whose identity is how fast it moves (a
    /// deliberate plodder, a sprinter) authors it here rather than moving the
    /// shared default every other body and the F3 dev slider ride.
    #[serde(default = "at_max_run_speed")]
    pub max_run_speed: f32,
    /// Ground acceleration (px/s²) toward the target speed. This is the knob that
    /// decides whether a character SNAPS to speed or has to build it — and, on the
    /// same approach, how far it slides when reversing. Low values read as weight.
    #[serde(default = "at_run_accel")]
    pub run_accel: f32,
    /// Airborne forward acceleration along the body-local side axis.
    #[serde(default = "at_air_accel")]
    pub air_accel: f32,
    /// Gravity-relative terminal fall speed.
    #[serde(default = "at_max_fall_speed")]
    pub max_fall_speed: f32,
    /// Grounded jump forgiveness after leaving support.
    #[serde(default = "at_coyote_time")]
    pub coyote_time: f32,
    /// Pre-landing jump input buffer.
    #[serde(default = "at_jump_buffer")]
    pub jump_buffer: f32,
    /// Grounded startup owed before a jump leaves the floor ("jump-squat").
    /// `0.0` — the default — is the classic instant leap.
    #[serde(default)]
    pub jump_squat_time: f32,
    /// Top horizontal AIR speed. `0.0` — the default — inherits
    /// [`Self::max_run_speed`], which is what every body did before air speed
    /// was authorable at all.
    #[serde(default)]
    pub max_air_speed: f32,
    /// Free-flight acceleration toward the commanded 2D velocity.
    #[serde(default = "at_flight_accel")]
    pub flight_accel: f32,
    /// Free-flight braking/idle drag.
    #[serde(default = "at_flight_drag")]
    pub flight_drag: f32,
    /// Magnitude cap for free-flight coordinate velocity.
    #[serde(default = "at_flight_terminal_speed")]
    pub flight_terminal_speed: f32,
    /// Whether the flight limb takes stick × terminal speed immediately.
    #[serde(default)]
    pub flight_direct_velocity: bool,
    /// Optional invariant speed for proper-velocity flight.
    #[serde(default)]
    pub flight_invariant_speed: Option<f32>,
}

fn at_gravity() -> f32 {
    ae::DEFAULT_TUNING.gravity
}

fn at_max_run_speed() -> f32 {
    ae::DEFAULT_TUNING.max_run_speed
}

fn at_run_accel() -> f32 {
    ae::DEFAULT_TUNING.run_accel
}

fn at_air_accel() -> f32 {
    ae::DEFAULT_TUNING.air_accel
}

fn at_max_fall_speed() -> f32 {
    ae::DEFAULT_TUNING.max_fall_speed
}

fn at_coyote_time() -> f32 {
    ae::DEFAULT_TUNING.coyote_time
}

fn at_jump_buffer() -> f32 {
    ae::DEFAULT_TUNING.jump_buffer
}

fn at_flight_accel() -> f32 {
    ae::DEFAULT_TUNING.flight_accel
}

fn at_flight_drag() -> f32 {
    ae::DEFAULT_TUNING.flight_drag
}

fn at_flight_terminal_speed() -> f32 {
    ae::DEFAULT_TUNING.flight_terminal_speed
}

fn at_air_jumps() -> u8 {
    ae::DEFAULT_TUNING.air_jumps
}

fn at_jump_speed() -> f32 {
    ae::DEFAULT_TUNING.jump_speed
}

impl Default for AxisTuningSpec {
    fn default() -> Self {
        Self {
            horizontal_law: ae::AxisHorizontalLaw::default(),
            jump_law: ae::AxisJumpLaw::default(),
            gravity: at_gravity(),
            air_jumps: at_air_jumps(),
            jump_speed: at_jump_speed(),
            max_run_speed: at_max_run_speed(),
            run_accel: at_run_accel(),
            air_accel: at_air_accel(),
            max_fall_speed: at_max_fall_speed(),
            coyote_time: at_coyote_time(),
            jump_buffer: at_jump_buffer(),
            jump_squat_time: 0.0,
            max_air_speed: 0.0,
            flight_accel: at_flight_accel(),
            flight_drag: at_flight_drag(),
            flight_terminal_speed: at_flight_terminal_speed(),
            flight_direct_velocity: false,
            flight_invariant_speed: None,
        }
    }
}

impl AxisTuningSpec {
    /// Overlay the authored knobs onto the shared default tuning, producing the
    /// full [`MovementTuning`](ambition_platformer2d_core::MovementTuning) the axis
    /// policy projects its `AxisSweptParams` from. Only the fields this spec
    /// carries diverge from [`DEFAULT_TUNING`](ambition_platformer2d_core::DEFAULT_TUNING).
    pub fn to_kernel(&self) -> ae::MovementTuning {
        ae::MovementTuning {
            horizontal_law: self.horizontal_law,
            jump_law: self.jump_law,
            gravity: self.gravity,
            air_jumps: self.air_jumps,
            jump_speed: self.jump_speed,
            max_run_speed: self.max_run_speed,
            run_accel: self.run_accel,
            air_accel: self.air_accel,
            max_fall_speed: self.max_fall_speed,
            coyote_time: self.coyote_time,
            jump_buffer: self.jump_buffer,
            jump_squat_time: self.jump_squat_time,
            max_air_speed: self.max_air_speed,
            flight_accel: self.flight_accel,
            flight_drag: self.flight_drag,
            flight_terminal_speed: self.flight_terminal_speed,
            flight_direct_velocity: self.flight_direct_velocity,
            flight_invariant_speed: self.flight_invariant_speed,
            ..ae::DEFAULT_TUNING
        }
    }
}

/// The composable grant vocabulary a catalog row lists to define its kit.
///
/// A character is not one blessed preset; it is the composition of the grant
/// bundles it carries. The kernel owns the algebra ([`ae::AbilitySet::compose`]);
/// this is the same [`ae::AbilityGrant`] enum, re-exported so a RON row reads
/// `abilities: Some([RunJump])` — a list that unions, not a single word that
/// picks. Adding a verb to a character is appending a grant, never forking a
/// preset roster.
///
/// A row that carries this field spawns its PLAYABLE body with the union of its
/// grants as the body's [`AbilityBase`](ambition_platformer2d_core::AbilityBase),
/// which the session mask (the F3 dev editable) then gates — the per-character
/// analogue of [`momentum`](CharacterCatalogEntry::momentum). A row without it
/// (the default) keeps the shared sandbox set, so every existing character is
/// untouched. This is how a restricted-kit demo character (classic run + jump,
/// no blink/dash/wall/fly) is authored without forcing the whole multi-game host
/// into that reduced kit.
pub use ambition_platformer2d_core::AbilityGrant;

/// An occasion on which a character may speak a one-line speech bubble.
/// Each variant maps to a named pool on [`CharacterBarks`]; the firing
/// system for that occasion picks (and rotates through) lines from the
/// matching pool. Heterogeneous by design — some are events (struck,
/// provoked), some are ambient states (idling, on display) — but the data
/// model is uniform so all of a character's voice lives in one place.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum BarkSituation {
    /// Struck in combat — a peaceful NPC's retaliation warning, or an
    /// enemy/boss yelping under a hit. Event-driven; rotates with strikes.
    OnHit,
    /// The moment a peaceful NPC crosses its hostility threshold and turns
    /// to fight. Event-driven; fires once.
    Provoked,
    /// Ambient muttering while idling — a peaceful NPC standing around, or a
    /// boss between strikes. Timer-driven; rotates.
    Idle,
    /// On display in the Hall of Characters: the character's fun, often
    /// self-aware gallery line. Timer-driven; rotates.
    Hall,
    /// A conversation with this character was left rather than finished —
    /// the other party walked, fell, or was carried out of talking range.
    ///
    /// deliberately NOT "the conversation was interrupted". A conversation
    /// broken by a HIT already barks: `npc_hit_bark_line` fires on every strike
    /// and falls back to a generic line when a character authored none, so a
    /// second bubble for one event would be worse than none. Walking away is
    /// the occasion nothing in the game currently answers.
    ///
    /// Event-driven; rotates. Empty pool means silence, exactly like `Idle` and
    /// `Hall` — a character says nothing about being left until somebody writes
    /// it a line, and that line is the character's voice to author.
    ConversationCut,
}

/// Per-character speech-bubble pools, one list per [`BarkSituation`]. All
/// pools default empty — an empty pool means "no authored line for that
/// occasion", and the firing system falls back (generic mob lines for
/// `OnHit` / `Provoked`, silence for `Idle` / `Hall`).
///
/// Authored in the catalog row so a character's voice lives with its
/// identity: every system that spawns the character — a room placement, the
/// peaceful→hostile flip, the Hall gallery — draws from the same lines.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct CharacterBarks {
    /// Lines when struck in combat. Rotates with strike count.
    #[serde(default)]
    pub on_hit: Vec<String>,
    /// Line(s) when a peaceful NPC turns hostile. Usually one.
    #[serde(default)]
    pub provoked: Vec<String>,
    /// Ambient idle muttering.
    #[serde(default)]
    pub idle: Vec<String>,
    /// Hall-of-Characters gallery lines (fun / self-aware).
    #[serde(default)]
    pub hall: Vec<String>,
    /// Lines when somebody walks out of a conversation with this character.
    /// See [`BarkSituation::ConversationCut`] for why a HIT is not this.
    #[serde(default)]
    pub conversation_cut: Vec<String>,
}

impl CharacterBarks {
    /// The line pool for `situation` (possibly empty).
    pub fn pool(&self, situation: BarkSituation) -> &[String] {
        match situation {
            BarkSituation::OnHit => &self.on_hit,
            BarkSituation::Provoked => &self.provoked,
            BarkSituation::Idle => &self.idle,
            BarkSituation::Hall => &self.hall,
            BarkSituation::ConversationCut => &self.conversation_cut,
        }
    }

    /// Pick a line for `situation`, rotating by `rotation` so repeated barks
    /// cycle the pool. `None` when the pool is empty.
    ///
    /// This is the situation-pool primitive, NOT the thing that answers "what
    /// does this character say" — it cannot see the row's `fallback_dialogue`.
    /// Call [`CharacterCatalogEntry::bark`] (or `CharacterCatalog::bark_line`)
    /// instead, or a character whose voice lives entirely in its fallback pool
    /// goes silent here.
    pub fn pick(&self, situation: BarkSituation, rotation: u32) -> Option<&str> {
        let pool = self.pool(situation);
        if pool.is_empty() {
            return None;
        }
        Some(pool[(rotation as usize) % pool.len()].as_str())
    }
}

/// Optional independently published portrait product for dialogue and other
/// close-up presentation. The image is separate from the gameplay sheet; the
/// manifest names the default and future expression/animation clips.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CharacterPortraitRef {
    /// Portrait-sheet image path, relative to the sandbox asset root.
    pub image: String,
    /// Portrait-sheet RON manifest path, relative to the sandbox asset root.
    pub manifest: String,
    /// Named clip shown when dialogue does not request another expression.
    pub default_clip: String,
    /// Named clip a STILL consumer draws — a select-screen or HUD box.
    ///
    /// Empty defers to the manifest's own `still_clip`, and then to
    /// `default_clip`'s first frame. Separate from `default_clip` because the
    /// clip that PLAYS and the pose a UI box shows are different choices, and a
    /// looping default's first frame is wherever the loop starts.
    #[serde(default)]
    pub still_clip: String,
}

/// One character entry in `character_catalog.ron`.
#[allow(
    dead_code,
    reason = "Public catalog schema; future consumers (Hall layout generator, dialogue UI, faction-aware spawn rules) read tier / body_kind / composition / tags. Today the validator + sprite loader use a subset."
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
// An authored field nobody consumes must not be silently dropped: the mechanic
// simply never fires and the author sees content that looks correct.
#[serde(deny_unknown_fields)]
pub struct CharacterCatalogEntry {
    /// Which provider authored this row, filled in by ASSEMBLY.
    ///
    /// The cost showed up when a migrated character tried to stop naming a preset: the Hall's
    /// override validation lost the namespace and the full-host check failed.
    ///
    /// EMPTY in an unassembled fragment, which is honest rather than a gap:
    /// a fragment does not know its own provider until it is registered under
    /// one, and `#[serde(default)]` means no authored row writes this.
    #[serde(default)]
    pub provider: String,
    /// Human-facing label (UI, dialogue, debug overlays).
    pub display_name: String,
    /// Sprite-sheet image path, relative to the sandbox asset root.
    pub spritesheet: String,
    /// Sprite-sheet RON manifest path, relative to the sandbox asset
    /// root. Today the manifest carries grid/frame info; future
    /// catalog work moves animation timing here too.
    pub manifest: String,
    /// Optional close-up portrait sheet. This presentation asset is
    /// deliberately independent of the gameplay sprite sheet.
    #[serde(default)]
    pub portrait: Option<CharacterPortraitRef>,
    /// Gallery tier. Drives hall placement.
    pub tier: CharacterTier,
    /// Footprint hint. Drives slot sizing.
    pub body_kind: CharacterBodyKind,
    /// How TALL this character stands, in world px, feet to the top of the
    /// visible body. `None` falls back to
    /// [`CharacterBodyKind::default_standing_height`].
    ///
    /// the one quantity a viewer actually compares, and it was authored
    /// NOWHERE. A catalog character's on-screen size was
    /// `authored LDtk spawn box × collision_scale × (body / frame)` — two
    /// per-character guesses and a room's spawn rectangle. Nothing in that
    /// product is a statement about height, so nothing could be consistent about
    /// one, and the Hall of Characters drew comparable humanoids across a 2x
    /// range: the goblin taller than the robots, one goblin half the other
    /// .
    #[serde(default)]
    pub standing_height: Option<f32>,
    /// Optional layered composition (multi-part sprites). `None` for
    /// single-part characters.
    #[serde(default)]
    pub composition: Option<Vec<CompositionLayer>>,
    /// Name of the preset in `brain_presets` to apply by default.
    ///
    ///  omit it and the preset dies with its last namer, which is what retirement is: one
    /// vocabulary, reached by subtraction.
    ///
    /// `#[serde(default)]` rather than `Option`, and the reason is churn:
    /// 144 shipped rows write this field as a bare string, and RON does not
    /// accept those for an `Option` without `implicit_some`. Empty had no prior
    /// meaning here, so this is the FIRST meaning on that emptiness rather than a
    /// second one — the trap `StartingCharacter::character_id`'s own doc warns
    /// about.
    #[serde(default)]
    pub default_brain: String,
    /// Name of the preset in `action_set_presets` to apply by default.
    pub default_action_set: String,
    /// Free-form tags. Tooling filters by these (e.g. the hall
    /// generator uses `tags = ["boss"]` to fence basement entries).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Behind-the-scenes authoring context: who or what the character parodies,
    /// the name joke, visual/thematic references, and design intent that future
    /// artists and writers should preserve. This is not necessarily canonical
    /// dialogue or lore. Empty for legacy rows that have not been migrated.
    #[serde(default)]
    pub authoring_description: String,
    /// Suggested combat identity and mechanical translation of the character's
    /// source ideas. Games may adopt, revise, or ignore this design guidance.
    /// Empty for legacy rows that have not been migrated.
    #[serde(default)]
    pub gameplay_description: String,
    /// Reusable conversation lines for contexts without bespoke scene/Yarn
    /// dialogue. These are authoring defaults rather than immutable canon.
    #[serde(default)]
    pub fallback_dialogue: Vec<String>,
    /// The sheet this character's melee draws its swing from. `None` means
    /// UNAUTHORED, which is a real authored answer and not a gap to paper over.
    ///
    /// Several characters MAY name the same sheet; that is sharing, and it is
    /// fine as long as their polygons are compatible. What is not fine is
    /// inheriting one by default, which is why the default is `None`.
    ///
    /// `None` does not mean invisible. A body with no authored attack VFX
    /// draws its hit volume directly, as a translucent shape, so an unauthored
    /// attack is legible in play instead of silent — see
    /// `ambition_render`'s unauthored-volume pass. An unresolvable id is a
    /// different thing entirely and is reported, not defaulted.
    #[serde(default)]
    pub attack_vfx: Option<String>,
    /// Gameplay sprite tuning (collision scale / sample inset / feet
    /// anchor override). `None` = defaults. Replaces the old
    /// hardcoded `*_SHEET` statics in `character_sprites/sheets.rs` (that
    /// path is GONE — the name is kept because the sentence is about what this
    /// field replaced, not about where to look). <!-- cite-ok -->
    #[serde(default)]
    pub sprite_tuning: Option<SpriteTuningSpec>,
    /// Speech-bubble lines for this character, keyed by occasion. Defaults
    /// to all-empty (silent). The single source of truth for a character's
    /// voice — supersedes the hardcoded `features::npcs` match tables and the
    /// `CombatBanterRegistry` content installers, which remain only as a
    /// fallback until every row is populated.
    #[serde(default)]
    pub barks: CharacterBarks,
    /// Yarn node id for this character's Hall-of-Characters conversation (the
    /// line shown when the player Inspects its pedestal). `None` = no hall
    /// dialogue; the pedestal is inspect-silent. Folded into the dialogue
    /// validator's known-id set so authored nodes are checked, and read by
    /// the hall generator to populate each pedestal's `dialogue_id`.
    #[serde(default)]
    pub hall_dialogue_id: Option<String>,
    /// Surface-momentum motion feel (Q21 / S2). `Some` opts this character's
    /// body into `MotionModel::SurfaceMomentum` — the surface-follower solver
    /// (slopes, loops, momentum) — whether it is spawned as an NPC or WORN by
    /// the player. `None` (the default) keeps the body on the axis-swept path,
    /// so every existing character is untouched.
    #[serde(default)]
    pub momentum: Option<MomentumParamsSpec>,
    /// Playable capability set, authored as a list of composable grants
    /// (run / jump / blink / dash / wall / fly / …). `Some` sets the body's
    /// [`AbilityBase`](ambition_platformer2d_core::AbilityBase) to the union of the
    /// grants — the per-character analogue of [`momentum`](Self::momentum).
    /// `None` (the default) keeps the shared sandbox set, so every existing row
    /// is untouched. A restricted-kit demo character authors e.g.
    /// `Some([RunJump])` (classic run + jump) instead of forcing the whole host
    /// into that kit. See [`AbilityGrant`].
    #[serde(default)]
    pub abilities: Option<Vec<AbilityGrant>>,
    /// Per-character axis-swept movement feel (the air-jump count today). `Some`
    /// authors this body's tuning so its live parameters come from the row
    /// instead of the global F3 dev tuning — the per-character analogue of
    /// [`momentum`](Self::momentum) for the axis path. `None` (the default) keeps
    /// the body on the shared editable tuning. See [`AxisTuningSpec`].
    #[serde(default)]
    pub axis_tuning: Option<AxisTuningSpec>,
    /// How much punishment this character's PLAYABLE body takes before it dies.
    /// `Some(1)` is the classic platformer contract: whatever armor you are
    /// wearing absorbs the hit, and once there is none left the next one is
    /// fatal. `None` (the default) keeps the host's standard pool, so every
    /// existing row is untouched.
    ///
    /// The per-character analogue of [`abilities`](Self::abilities) and
    /// [`axis_tuning`](Self::axis_tuning): a demo character declares its own
    /// fragility in its row instead of forcing the whole host onto it.
    #[serde(default)]
    pub max_health: Option<i32>,
}

impl CharacterCatalogEntry {
    /// The sheet-manifest record key for this character: the manifest filename root (e.g.
    /// `sprites/pirate_admiral_spritesheet.ron` -> `pirate_admiral`). Multiple catalog ids that
    /// point at the SAME `manifest` path share one generated sheet (texture + record both);
    /// each character with its own art reads its own manifest.
    pub fn manifest_target(&self) -> Option<&str> {
        let file = self.manifest.rsplit('/').next()?;
        file.strip_suffix("_spritesheet.ron")
    }

    /// A line this character says in `situation`: its authored pool for that
    /// situation, else its [`fallback_dialogue`](Self::fallback_dialogue), else
    /// `None` (the caller drops back to the engine-generic line, or stays
    /// silent).
    ///
    /// The fallback pool applies to EVERY situation deliberately. A character
    /// arrives from the sprite pipeline with a voice long before anyone writes
    /// four separate pools for it, and an in-voice line that does not quite fit
    /// the moment beats a voiceless engine-generic one — the fallback exists so
    /// a newly authored character is never mute. Writing a situation pool
    /// silences the fallback for that situation and nothing else.
    ///
    /// `rotation` cycles whichever pool answered, so repeat barks vary.
    pub fn bark(&self, situation: BarkSituation, rotation: u32) -> Option<&str> {
        if let Some(line) = self.barks.pick(situation, rotation) {
            return Some(line);
        }
        if self.fallback_dialogue.is_empty() {
            return None;
        }
        Some(self.fallback_dialogue[(rotation as usize) % self.fallback_dialogue.len()].as_str())
    }
}

/// Deserialize-only mirror of `brain::StateMachineCfg`. Variant
/// names match `StateMachineCfg`; fields match the corresponding
/// `*Cfg` struct field-for-field. The catalog stores the preset
/// shape (cfg only — no per-actor `state`); resolver code constructs
/// the runtime `Brain` by pairing the preset with a default `state`.
///
/// `Patrol` uses `spawn_local_x` rather than `spawn_x` to make
/// explicit that the value is an offset from the NPC's spawn
/// position, not a world-space coordinate. The resolver adds the
/// NPC's actual spawn-X at runtime.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum BrainPreset {
    StandStill,
    Patrol {
        spawn_local_x: f32,
        radius: f32,
        speed: f32,
        aggressiveness: f32,
        aggro_radius: f32,
        attack_range: f32,
    },
    Wanderer {
        speed: f32,
        aggressiveness: f32,
    },
    MeleeBrute {
        aggressiveness: f32,
        aggro_radius: f32,
        attack_range: f32,
        chase_speed: f32,
    },
    Skirmisher {
        aggressiveness: f32,
        aggro_radius: f32,
        standoff_px: f32,
        strafe_speed: f32,
        fire_cooldown_s: f32,
    },
    Sniper {
        aggressiveness: f32,
        aggro_radius: f32,
        fire_cooldown_s: f32,
    },
    /// Lively flyer (perch/fly/walk + land-by-player when peaceful; stalk/dive/
    /// recover when aggressive). `aggressiveness == 0` = peaceful bird.
    Aerial {
        aggressiveness: f32,
        cruise_speed: f32,
        dive_speed: f32,
        aggro_radius: f32,
        attack_range: f32,
        roam_radius: f32,
    },
    BossPattern {
        aggressiveness: f32,
        encounter_id: String,
    },
    /// Smash-brawl reactive fighter (observe → mode → action → difficulty
    /// → emit). The strong, never-cheats melee/zoner brain — it perceives
    /// only a `BrainSnapshot` and acts only through the actor's `ActionSet`,
    /// the same seam the player uses. Always hostile by construction; the
    /// encounter swaps this in when the player picks "challenge". The
    /// `difficulty` floats are the fairness knobs (reaction lag, commit
    /// probability, aim accuracy).
    /// The FB4b fighter brain — L1 classify, L2 options, L3 rollout, on a
    /// human cadence with an APM ceiling.
    ///
    /// It existed, was tested, and no content could select it: `BrainPreset` is the only vocabulary
    /// a catalog row has for choosing a brain, and there was no variant for this one. That is the
    /// third time in a day the same shape has turned up here — a capability built, correct, and
    /// unreachable from where a consumer stands (the named-seat driver seam, the inert rollback
    /// registration, and now this).
    ///
    /// `level` picks a rung of the ladder. The other knobs are the ones a stage
    /// legitimately varies per opponent; everything else about how the brain
    /// thinks belongs to the ladder row, which is content a game ships.
    Fighter {
        /// 1..=9. Reads a rung of the fighter ladder.
        level: u8,
        /// Ticks between decisions. §5's 10-20 Hz at 60 Hz sim.
        #[serde(default = "default_fighter_decision_interval")]
        decision_interval_ticks: u32,
    },
    Smash {
        aggro_radius: f32,
        engage_distance: f32,
        attack_range: f32,
        too_close_distance: f32,
        chase_speed: f32,
        retreat_speed: f32,
        crowding_threshold: f32,
        sprint_to_close: bool,
        reaction_delay_s: f32,
        commit_probability: f32,
        accuracy: f32,
        mash_speed_hz: f32,
    },
}

/// Locomotion style. Mirrors `brain::action_set::MoveStyleSpec`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum MoveStylePreset {
    #[default]
    Walk,
    WalkHeavy,
    Hop,
    Strafe,
    Slither,
    Float,
}

/// Mirrors `brain::action_set::MeleeActionSpec` — each variant
/// carries its own windup/active/recover timing.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum MeleePreset {
    Swipe {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
    },
    Lunge {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
        step_px: f32,
    },
    Slam {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
        hop_height_px: f32,
    },
    Bite {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
    },
    PunchWeak {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
    },
}

/// Mirrors `brain::action_set::RangedActionSpec`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum RangedPreset {
    Rock { speed: f32, damage: i32 },
    Arrow { speed: f32, damage: i32 },
    Pistol { speed: f32, damage: i32 },
    Bolt { speed: f32, damage: i32 },
}

/// Mirrors `brain::action_set::SpecialActionSpec`.
///
/// Keep the open `Special(String)` hatch here too: the catalog is an
/// authoring surface, so it must be able to reach every content-defined
/// technique the runtime action set can emit.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum SpecialPreset {
    Special(String),
}

/// Action-set preset (capability bundle). Each character points at
/// one of these by name in its `default_action_set` field.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ActionSetPreset {
    #[serde(default)]
    pub move_style: MoveStylePreset,
    #[serde(default)]
    pub melee: Option<MeleePreset>,
    #[serde(default)]
    pub ranged: Option<RangedPreset>,
    #[serde(default)]
    pub special: Option<SpecialPreset>,
}

/// Top-level RON shape: brain presets + action-set presets + the
/// character map keyed by `character_id`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CharacterCatalogData {
    /// A character could carry a [`crate:brain:BrainProfile`] by VALUE
    /// (`CharacterDefinition:autonomous_profile`) or name a [`BrainPreset`] by key — and those
    /// are different vocabularies read by different roads, so the old architecture coupled
    /// autonomous policy to body identity.
    ///
    /// deliberately NOT [`BrainPreset`], and the difference is the point: a
    /// preset authors ABSOLUTE speeds (`chase_speed`, `cruise_speed`) while a
    /// profile authors normalized EFFORT against the body's own `run_speed`
    /// (§4.7). Merging the two vocabularies needs the BODY, which a preset does
    /// not know; this map sidesteps that by being the profile vocabulary from
    /// the start. The presets are untouched.
    ///
    /// namespaced per provider on assembly, exactly like the presets beside
    /// it, so two games may both author a `"striker"` without colliding.
    #[serde(default)]
    pub autonomous_profiles: BTreeMap<String, crate::brain::BrainProfile>,
    pub brain_presets: BTreeMap<String, BrainPreset>,
    pub action_set_presets: BTreeMap<String, ActionSetPreset>,
    pub characters: BTreeMap<String, CharacterCatalogEntry>,
}

#[cfg(test)]
mod momentum_spec_tests {
    use super::*;

    #[test]
    fn omitted_fields_inherit_the_kernel_defaults() {
        // Authoring only what it tunes (Sanic's fast profile) leaves every
        // other field at the kernel `Default` — the Q21 contract.
        let spec: MomentumParamsSpec =
            ron::from_str("(ground_accel: 900.0, top_speed: 1200.0, jump_speed: 700.0)")
                .expect("partial momentum spec should deserialize");
        let k = spec.to_kernel();
        let d = ae::MomentumParams::default();
        assert_eq!(k.ground_accel, 900.0, "tuned field wins");
        assert_eq!(k.top_speed, 1200.0);
        assert_eq!(k.jump_speed, 700.0);
        // Untouched fields match the kernel baseline value-for-value.
        assert_eq!(k.brake, d.brake);
        assert_eq!(k.friction, d.friction);
        assert_eq!(k.slope_factor, d.slope_factor);
        assert_eq!(k.air_accel, d.air_accel);
        assert_eq!(k.stick_factor, d.stick_factor);
        assert_eq!(k.min_stick_speed, d.min_stick_speed);
    }

    #[test]
    fn empty_spec_is_the_kernel_default() {
        let spec: MomentumParamsSpec = ron::from_str("()").expect("empty spec ok");
        assert_eq!(spec.to_kernel(), ae::MomentumParams::default());
    }
}

fn default_fighter_decision_interval() -> u32 {
    crate::brain::fighter::data::DEFAULT_DECISION_INTERVAL_TICKS
}
