//! Entity-contract + moveset vocabulary — the gameplay-truth schema.
//!
//! This crate is the typed spine of the `EntityCatalog` target in
//! `docs/planning/engine/data-driven-sprites-and-characters.md`: entities as
//! **contract bundles** (not categories), and abilities as **Smash-model move
//! timelines** that every actor plays through the same system.
//!
//! Two rules carry the design:
//!
//! - **One clock per move: the owner's proper time.** Every duration in a
//!   [`MoveSpec`] is seconds of the *owning actor's* clock — its entity dt
//!   (sim dt × whatever dilation that actor experiences: bullet-time, a time
//!   bubble, a relativistic zone). The bound clip's playback is slaved to the
//!   move's normalized phase, so a dilated actor's picture and hit windows
//!   slow together and can never desync. Dilation is a property of the
//!   actor's clock, never of this data — the schema stays
//!   frame-of-reference-free.
//! - **Entity-local logical space.** Move volumes are authored in the
//!   entity's local coordinates (+x = facing, y = up, origin = body center),
//!   never atlas pixels. Quality tiers rescale render textures; they cannot
//!   touch this data.
//!
//! The engine owns the *primitives* here (window, volume, event, gate,
//! cancel edge); content composes them into moves. A move is data — giving
//! the goblin the player's slash is a re-binding, not a Rust change.
//!
//! Authored as RON (this is Rust/hand-authored data; only Python-authored
//! interchange uses JSON). Headless by construction: no Bevy, no assets —
//! a simulation can parse, validate, and play a move without loading a PNG.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Moves: the Smash-model timeline.
// ---------------------------------------------------------------------------

/// What a span of a move's timeline means, gameplay-wise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowTag {
    /// Windup — no hits yet; the tell.
    Startup,
    /// The window's volumes are live hitboxes.
    Active,
    /// Follow-through — vulnerable, no hits.
    Recovery,
    /// The owner cannot be hit.
    Invuln,
    /// The owner takes hits without hitstun.
    Armor,
    /// The move may be canceled into the named moves.
    Cancelable { into: Vec<String> },
}

/// An axis-aligned or circular hit volume in ENTITY-LOCAL logical space
/// (+x = facing; the runtime mirrors x for a left-facing actor).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeShape {
    /// Centered at `offset`, extending `half_extents` each way.
    Rect {
        offset: (f32, f32),
        half_extents: (f32, f32),
    },
    /// Centered at `offset` with radius `radius`.
    Circle { offset: (f32, f32), radius: f32 },
}

/// One hit volume carried by an [`WindowTag::Active`] window, with its hit
/// payload. Volumes live on their window — where the timeline says they are —
/// not in a parallel list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitVolume {
    pub shape: VolumeShape,
    /// Damage dealt on contact.
    pub damage: i32,
    /// Knockback impulse magnitude (engine units; direction is derived from
    /// facing + contact by the combat runtime).
    #[serde(default)]
    pub knockback: f32,
}

/// One span of a move's timeline. Times are seconds of the owner's proper
/// time, relative to move start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveWindow {
    pub start_s: f32,
    pub end_s: f32,
    pub tag: WindowTag,
    /// Hit volumes live during this window (meaningful for `Active`).
    #[serde(default)]
    pub volumes: Vec<HitVolume>,
    /// A SUSTAINED content effect: while this window is active, an `Effect { key }`
    /// is emitted EVERY frame (not one-shot like a `MoveEvent`). This is how a move
    /// expresses a HELD/continuous special — a beam that lingers, a rain that keeps
    /// falling — where the consuming technique times its own cadence off the
    /// per-frame "active this tick" signal (the shape the boss `apple_rain`-style
    /// specials need; the boss fold rides this). `None` for ordinary windows.
    #[serde(default)]
    pub sustain_effect: Option<String>,
}

/// A timed one-shot on the move timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveEventKind {
    /// Play a sound cue by key.
    Sfx { cue: String },
    /// Emit a content-defined effect by key (the `Effect` vocabulary /
    /// technique seam resolves it).
    Effect { key: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveEvent {
    /// Seconds (owner's proper time) from move start.
    pub at_s: f32,
    pub kind: MoveEventKind,
}

/// Which semantic clip presents this move, with a declared fallback chain
/// (e.g. `tilt_up → slash → idle`). Resolution happens against the entity's
/// visual (pack or sheet) at bind time; a missing clip degrades presentation,
/// never simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipBinding {
    pub clip: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

/// Activation gates for a move. Narrow on purpose — add knobs when real
/// moves need them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveGates {
    /// `Some(true)` = grounded only; `Some(false)` = airborne only;
    /// `None` = either.
    #[serde(default)]
    pub grounded: Option<bool>,
}

/// One ability activation: a clip binding plus the full gameplay meaning of
/// the ability on one timeline. **The move timeline is authoritative for both
/// gameplay and presentation** — windows advance on the owner's proper time
/// and the bound clip is sampled by normalized move phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveSpec {
    /// Stable move id (`"jab"`, `"tilt_up"`, `"sandbag_swat"`).
    pub id: String,
    pub clip: ClipBinding,
    /// Total move time, seconds of the owner's proper time.
    pub duration_s: f32,
    pub windows: Vec<MoveWindow>,
    #[serde(default)]
    pub events: Vec<MoveEvent>,
    #[serde(default)]
    pub gates: MoveGates,
}

impl MoveSpec {
    /// The windows carrying `tag`, in declaration order.
    pub fn windows_tagged(
        &self,
        want: fn(&WindowTag) -> bool,
    ) -> impl Iterator<Item = &MoveWindow> {
        self.windows.iter().filter(move |w| want(&w.tag))
    }

    /// The active hit volumes at proper-time `t` seconds into the move.
    pub fn active_volumes_at(&self, t: f32) -> impl Iterator<Item = &HitVolume> {
        self.windows
            .iter()
            .filter(move |w| matches!(w.tag, WindowTag::Active) && w.start_s <= t && t < w.end_s)
            .flat_map(|w| w.volumes.iter())
    }

    /// Normalized phase (`0..=1`) at proper-time `t` — what presentation
    /// samples the bound clip by.
    pub fn phase_at(&self, t: f32) -> f32 {
        if self.duration_s <= 0.0 {
            return 1.0;
        }
        (t / self.duration_s).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Entities: contract bundles.
// ---------------------------------------------------------------------------

/// Physics body contract: entity-local collision half-extents.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Body2dContract {
    pub half_extents: (f32, f32),
}

/// Presentation contract: which visual this entity binds to. `visual_id` is
/// the packer/sheet target name resolved through the sprite pack (or the
/// per-target sheet compatibility path).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationContract {
    pub visual_id: String,
}

/// Moveset contract: the entity's moves plus which input verb activates
/// which move. `moves` is the composition surface — re-binding an existing
/// move onto a different actor is a data edit here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MovesetContract {
    /// Input verb → move id (e.g. `"attack" → "sandbag_swat"`). BTreeMap for
    /// deterministic iteration (query-order discipline).
    #[serde(default)]
    pub verbs: BTreeMap<String, String>,
    pub moves: Vec<MoveSpec>,
}

impl MovesetContract {
    pub fn move_by_id(&self, id: &str) -> Option<&MoveSpec> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// Resolve an input verb to its move.
    pub fn move_for_verb(&self, verb: &str) -> Option<&MoveSpec> {
        self.move_by_id(self.verbs.get(verb)?)
    }
}

/// The contracts an entity exposes. All optional: the engine asks "does this
/// entity expose the contract this system consumes?", never "what category
/// is it?". Narrow seed set — grow per real consumer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityContracts {
    #[serde(default)]
    pub body: Option<Body2dContract>,
    #[serde(default)]
    pub presentation: Option<PresentationContract>,
    #[serde(default)]
    pub moveset: Option<MovesetContract>,
}

/// One catalog entity: a stable id plus its contract bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDef {
    pub id: String,
    pub contracts: EntityContracts,
}

/// An authored entity-catalog document (one or many entities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityCatalogDoc {
    pub schema_version: u32,
    pub entities: Vec<EntityDef>,
}

// ---------------------------------------------------------------------------
// Validation: headless, structural, exhaustive.
// ---------------------------------------------------------------------------

/// A structural problem in an authored catalog. Every violation is reported
/// (not just the first) so an author fixes a file in one pass.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogError {
    DuplicateEntityId {
        id: String,
    },
    DuplicateMoveId {
        entity: String,
        id: String,
    },
    /// A window lies outside `[0, duration_s]` or is inverted/empty.
    WindowOutOfRange {
        entity: String,
        mv: String,
        index: usize,
    },
    /// A non-Active window carries hit volumes (they would never fire).
    VolumesOnInactiveWindow {
        entity: String,
        mv: String,
        index: usize,
    },
    /// A `Cancelable { into }` edge names an undeclared move.
    UnknownCancelTarget {
        entity: String,
        mv: String,
        target: String,
    },
    /// A verb maps to an undeclared move.
    UnknownVerbMove {
        entity: String,
        verb: String,
        target: String,
    },
    /// An event fires outside the move's duration.
    EventOutOfRange {
        entity: String,
        mv: String,
        index: usize,
    },
    /// Non-positive move duration.
    NonPositiveDuration {
        entity: String,
        mv: String,
    },
    /// Degenerate volume (non-positive extent/radius).
    DegenerateVolume {
        entity: String,
        mv: String,
        window: usize,
    },
    /// An entity declares a moveset but no presentation clip could ever bind.
    /// (Warning-grade in spirit, but structural: an empty clip name is a typo.)
    EmptyClipBinding {
        entity: String,
        mv: String,
    },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::DuplicateEntityId { id } => write!(f, "duplicate entity id `{id}`"),
            CatalogError::DuplicateMoveId { entity, id } => {
                write!(f, "{entity}: duplicate move id `{id}`")
            }
            CatalogError::WindowOutOfRange { entity, mv, index } => {
                write!(f, "{entity}/{mv}: window[{index}] outside [0, duration]")
            }
            CatalogError::VolumesOnInactiveWindow { entity, mv, index } => {
                write!(
                    f,
                    "{entity}/{mv}: window[{index}] carries volumes but is not Active"
                )
            }
            CatalogError::UnknownCancelTarget { entity, mv, target } => {
                write!(
                    f,
                    "{entity}/{mv}: cancel target `{target}` is not a declared move"
                )
            }
            CatalogError::UnknownVerbMove {
                entity,
                verb,
                target,
            } => {
                write!(
                    f,
                    "{entity}: verb `{verb}` maps to undeclared move `{target}`"
                )
            }
            CatalogError::EventOutOfRange { entity, mv, index } => {
                write!(
                    f,
                    "{entity}/{mv}: event[{index}] fires outside the move duration"
                )
            }
            CatalogError::NonPositiveDuration { entity, mv } => {
                write!(f, "{entity}/{mv}: non-positive duration")
            }
            CatalogError::DegenerateVolume { entity, mv, window } => {
                write!(f, "{entity}/{mv}: window[{window}] has a degenerate volume")
            }
            CatalogError::EmptyClipBinding { entity, mv } => {
                write!(f, "{entity}/{mv}: empty clip binding")
            }
        }
    }
}

impl EntityCatalogDoc {
    /// Parse a catalog document from RON text.
    pub fn parse(ron_text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron_text)
    }

    /// Serialize to pretty RON (authoring round-trips).
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Structural validation. Empty ⇒ sound. Filesystem-free: clip bindings
    /// are checked for shape here; whether a clip resolves in the bound
    /// visual is the publish-time validator's job (it has the visual data).
    pub fn validate(&self) -> Vec<CatalogError> {
        let mut errors = Vec::new();
        let mut seen_entities = HashSet::new();
        for entity in &self.entities {
            if !seen_entities.insert(entity.id.as_str()) {
                errors.push(CatalogError::DuplicateEntityId {
                    id: entity.id.clone(),
                });
            }
            let Some(moveset) = &entity.contracts.moveset else {
                continue;
            };
            let declared: HashSet<&str> = moveset.moves.iter().map(|m| m.id.as_str()).collect();
            let mut seen_moves = HashSet::new();
            for mv in &moveset.moves {
                if !seen_moves.insert(mv.id.as_str()) {
                    errors.push(CatalogError::DuplicateMoveId {
                        entity: entity.id.clone(),
                        id: mv.id.clone(),
                    });
                }
                if mv.duration_s <= 0.0 {
                    errors.push(CatalogError::NonPositiveDuration {
                        entity: entity.id.clone(),
                        mv: mv.id.clone(),
                    });
                }
                if mv.clip.clip.is_empty() {
                    errors.push(CatalogError::EmptyClipBinding {
                        entity: entity.id.clone(),
                        mv: mv.id.clone(),
                    });
                }
                for (index, w) in mv.windows.iter().enumerate() {
                    if !(0.0..=mv.duration_s).contains(&w.start_s)
                        || !(0.0..=mv.duration_s).contains(&w.end_s)
                        || w.start_s >= w.end_s
                    {
                        errors.push(CatalogError::WindowOutOfRange {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            index,
                        });
                    }
                    if !w.volumes.is_empty() && !matches!(w.tag, WindowTag::Active) {
                        errors.push(CatalogError::VolumesOnInactiveWindow {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            index,
                        });
                    }
                    for v in &w.volumes {
                        let degenerate = match v.shape {
                            VolumeShape::Rect { half_extents, .. } => {
                                half_extents.0 <= 0.0 || half_extents.1 <= 0.0
                            }
                            VolumeShape::Circle { radius, .. } => radius <= 0.0,
                        };
                        if degenerate {
                            errors.push(CatalogError::DegenerateVolume {
                                entity: entity.id.clone(),
                                mv: mv.id.clone(),
                                window: index,
                            });
                        }
                    }
                    if let WindowTag::Cancelable { into } = &w.tag {
                        for target in into {
                            if !declared.contains(target.as_str()) {
                                errors.push(CatalogError::UnknownCancelTarget {
                                    entity: entity.id.clone(),
                                    mv: mv.id.clone(),
                                    target: target.clone(),
                                });
                            }
                        }
                    }
                }
                for (index, ev) in mv.events.iter().enumerate() {
                    if !(0.0..=mv.duration_s).contains(&ev.at_s) {
                        errors.push(CatalogError::EventOutOfRange {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            index,
                        });
                    }
                }
            }
            for (verb, target) in &moveset.verbs {
                if !declared.contains(target.as_str()) {
                    errors.push(CatalogError::UnknownVerbMove {
                        entity: entity.id.clone(),
                        verb: verb.clone(),
                        target: target.clone(),
                    });
                }
            }
        }
        errors
    }

    pub fn entity(&self, id: &str) -> Option<&EntityDef> {
        self.entities.iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seed catalog: one actor-like entity (a moveset + body +
    /// presentation) and one prop-like entity (body + presentation only).
    /// The actor's `swat` is the SwipeSpec shape as data: three windows,
    /// the active one carrying one rect hit volume.
    const SEED: &str = r#"
    (
        schema_version: 1,
        entities: [
            (
                id: "sandbag_seed",
                contracts: (
                    body: Some((half_extents: (15.0, 24.0))),
                    presentation: Some((visual_id: "sandbag")),
                    moveset: Some((
                        verbs: { "attack": "swat" },
                        moves: [
                            (
                                id: "swat",
                                clip: (clip: "slash", fallbacks: ["idle"]),
                                duration_s: 0.68,
                                windows: [
                                    (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                    (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                        (shape: Rect(offset: (28.0, 0.0), half_extents: (14.0, 10.0)),
                                         damage: 1, knockback: 40.0),
                                    ]),
                                    (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                                    (start_s: 0.5, end_s: 0.68, tag: Cancelable(into: ["swat"]), volumes: []),
                                ],
                                events: [
                                    (at_s: 0.28, kind: Sfx(cue: "swing_light")),
                                ],
                                gates: (grounded: Some(true)),
                            ),
                        ],
                    )),
                ),
            ),
            (
                id: "crate_seed",
                contracts: (
                    body: Some((half_extents: (16.0, 16.0))),
                    presentation: Some((visual_id: "intro_cart")),
                ),
            ),
        ],
    )
    "#;

    #[test]
    fn seed_catalog_parses_and_validates() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        assert!(doc.validate().is_empty(), "{:?}", doc.validate());
        assert_eq!(doc.entities.len(), 2);
        let actor = doc.entity("sandbag_seed").unwrap();
        let moveset = actor.contracts.moveset.as_ref().unwrap();
        let swat = moveset.move_for_verb("attack").unwrap();
        assert_eq!(swat.id, "swat");
        // Prop exposes body+presentation, no moveset — contracts, not
        // categories: nothing marks it "a prop".
        let prop = doc.entity("crate_seed").unwrap();
        assert!(prop.contracts.moveset.is_none());
        assert!(prop.contracts.presentation.is_some());
    }

    #[test]
    fn round_trips_through_ron() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        let text = doc.to_ron().unwrap();
        let back = EntityCatalogDoc::parse(&text).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn move_timeline_queries_answer_the_sim() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        let moveset = doc
            .entity("sandbag_seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap();
        let swat = moveset.move_by_id("swat").unwrap();
        // Proper-time queries: nothing live during startup, one volume
        // mid-active, nothing during recovery.
        assert_eq!(swat.active_volumes_at(0.1).count(), 0);
        assert_eq!(swat.active_volumes_at(0.30).count(), 1);
        assert_eq!(swat.active_volumes_at(0.5).count(), 0);
        // Phase is normalized move progress — what the clip samples by.
        assert!((swat.phase_at(0.34) - 0.5).abs() < 1e-6);
        assert_eq!(swat.phase_at(2.0), 1.0);
    }

    #[test]
    fn validators_catch_structural_violations() {
        let bad = r#"
        (
            schema_version: 1,
            entities: [
                (
                    id: "bad",
                    contracts: (
                        moveset: Some((
                            verbs: { "attack": "missing" },
                            moves: [
                                (
                                    id: "broken",
                                    clip: (clip: ""),
                                    duration_s: 0.5,
                                    windows: [
                                        (start_s: 0.4, end_s: 0.9, tag: Startup, volumes: []),
                                        (start_s: 0.0, end_s: 0.2, tag: Recovery, volumes: [
                                            (shape: Circle(offset: (0.0, 0.0), radius: 0.0),
                                             damage: 1, knockback: 0.0),
                                        ]),
                                        (start_s: 0.2, end_s: 0.4, tag: Cancelable(into: ["nowhere"]), volumes: []),
                                    ],
                                    events: [ (at_s: 0.9, kind: Effect(key: "boom")) ],
                                ),
                            ],
                        )),
                    ),
                ),
                ( id: "bad", contracts: () ),
            ],
        )
        "#;
        let doc = EntityCatalogDoc::parse(bad).unwrap();
        let errors = doc.validate();
        let has = |f: &dyn Fn(&CatalogError) -> bool| errors.iter().any(|e| f(e));
        assert!(has(&|e| matches!(
            e,
            CatalogError::DuplicateEntityId { .. }
        )));
        assert!(has(&|e| matches!(e, CatalogError::WindowOutOfRange { .. })));
        assert!(has(&|e| matches!(
            e,
            CatalogError::VolumesOnInactiveWindow { .. }
        )));
        assert!(has(&|e| matches!(e, CatalogError::DegenerateVolume { .. })));
        assert!(has(&|e| matches!(
            e,
            CatalogError::UnknownCancelTarget { .. }
        )));
        assert!(has(&|e| matches!(e, CatalogError::UnknownVerbMove { .. })));
        assert!(has(&|e| matches!(e, CatalogError::EventOutOfRange { .. })));
        assert!(has(&|e| matches!(e, CatalogError::EmptyClipBinding { .. })));
    }

    /// The relativity contract, pinned as behavior: the timeline is queried
    /// in the OWNER'S proper time, so a dilated actor advancing at 0.25×
    /// world rate reaches its active window after 4× the world time — by
    /// construction, because the caller integrates proper time from the
    /// owner's dt. The schema carries no world-time anywhere.
    #[test]
    fn proper_time_integration_is_callers_dt_sum() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        let moveset = doc
            .entity("sandbag_seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap();
        let swat = moveset.move_by_id("swat").unwrap();
        // Simulate a 0.25×-dilated owner: 60 world frames of 16ms reach only
        // 0.24s proper — still in startup. An undilated owner is active.
        let dilated: f32 = (0..60).map(|_| 0.016 * 0.25).sum();
        let undilated: f32 = (0..60).map(|_| 0.016).sum();
        assert_eq!(swat.active_volumes_at(dilated).count(), 0);
        assert_eq!(swat.active_volumes_at(undilated - 0.65).count(), 1);
    }
}
