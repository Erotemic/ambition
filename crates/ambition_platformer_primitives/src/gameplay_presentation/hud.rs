//! **Declared HUD readouts** — a game says what its HUD shows; the engine
//! never learns what any of it means.
//!
//! The engine already owned WHERE a HUD may live
//! ([`ResolvedGameplayPresentation::hud_region`](super::ResolvedGameplayPresentation::hud_region)
//! and the surround/occupancy vocabulary). What it did not own was a way for a
//! game to say WHAT to show there without the engine growing a field per game:
//! the only HUD in the tree is a fixed three-readout widget hardcoded to
//! `HP`/`MP`/`$`, so a demo wanting `RINGS: 12` or `SCORE 004200` had no seam
//! and shipped no HUD at all.
//!
//! This module is that seam, and it is deliberately two halves:
//!
//! * a **declaration** ([`HudDeclaration`]) — the slots a game's HUD has, in
//!   what order, preferring which surround region. Declared once at
//!   plugin-build time through the provider's authoring builder, exactly like
//!   presentation profiles and loading specs; and
//! * a **live value** ([`HudReadouts`]) — what each slot currently reads,
//!   written every frame by a system the GAME owns, from whatever state the
//!   game considers authoritative.
//!
//! The engine matches on neither. A [`HudSlotId`] is an opaque string it uses
//! only as a map key and a spawn identity; "RINGS", "SCORE", "TIME" are values a
//! game writes into [`HudReadout::label`]. That is the whole point — a second
//! game gets a HUD by declaring slots and writing readouts, with no core edit,
//! which is the same rule the presentation profiles follow.
//!
//! # What this deliberately is not
//!
//! Not a layout engine. A declaration names a preferred [`SurroundRegion`] and a
//! minimum size; the renderer asks [`hud_region`] for it and falls back to
//! overlaying gameplay when this profile and display leave none — the same
//! ladder the built-in player HUD already walks. Slots stack in `order` within
//! their region and size themselves to their text. A game that wants real
//! layout builds its own HUD; this covers the readout row that every one of
//! them wants first.
//!
//! [`hud_region`]: super::ResolvedGameplayPresentation::hud_region

use std::collections::BTreeMap;

use ambition_engine_core as ae;
use bevy::prelude::Resource;

use super::SurroundRegion;

/// A game's opaque name for one HUD readout.
///
/// The engine uses this ONLY as a map key and a spawned-entity identity. It
/// never branches on the value, so a game may use whatever ids it likes without
/// the engine learning a content vocabulary.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HudSlotId(pub String);

impl HudSlotId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for HudSlotId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

/// One declared readout: where it wants to live and how it should look.
///
/// Authored by the game, read by the renderer. Carries no value — the value
/// arrives every frame through [`HudReadouts`], so a declaration stays an
/// immutable build-time fact.
#[derive(Clone, Debug, PartialEq)]
pub struct HudSlotSpec {
    pub id: HudSlotId,
    /// Stacking order within the slot's region. Ties break on `id`, so a
    /// declaration always lays out deterministically.
    ///
    /// `u32::MAX` is the "unstated" sentinel: [`HudDeclaration::slot`] replaces
    /// it with the slot's declaration index. Zero is therefore an ORDINARY
    /// explicit order that survives, which is why the default is not zero.
    pub order: u32,
    /// Where this readout would PREFER to live. The renderer honours it when
    /// the active profile reserves a surround and the region is big enough;
    /// otherwise the slot overlays gameplay.
    pub region: SurroundRegion,
    /// Smallest surround rect this readout is willing to occupy. A region
    /// narrower than this is treated as "no room", and the slot overlays.
    pub min_px: ae::Vec2,
    pub font_size: f32,
    /// `sRGBA`. Kept as plain components so this crate stays free of the render
    /// stack's colour types.
    pub color: [f32; 4],
    /// Centre this readout across the gameplay rectangle instead of stacking it
    /// at the start of its region.
    ///
    /// This is what makes a transient CARD — a level title, a course-clear
    /// tally — expressible as a slot rather than needing its own surface. A
    /// game publishes text into the slot only while the card should be up, and
    /// the readout vanishes on its own when the game stops publishing, because
    /// an unpublished slot draws nothing.
    pub centered: bool,
}

impl HudSlotSpec {
    /// A readout with the ordinary defaults: top surround, modest minimum,
    /// legible size, near-white.
    pub fn new(id: impl Into<HudSlotId>) -> Self {
        Self {
            id: id.into(),
            // Sentinel means "use declaration sequence". Zero is a valid,
            // explicit order and must not be mistaken for the default.
            order: u32::MAX,
            region: SurroundRegion::Top,
            min_px: ae::Vec2::new(96.0, 24.0),
            font_size: 18.0,
            color: [0.94, 0.96, 1.0, 0.98],
            centered: false,
        }
    }

    /// Centre this readout — the transient-card shape. See [`Self::centered`].
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    pub fn with_region(mut self, region: SurroundRegion) -> Self {
        self.region = region;
        self
    }

    pub fn with_min_px(mut self, min_px: ae::Vec2) -> Self {
        self.min_px = min_px;
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// A game's whole HUD declaration: which readouts exist.
///
/// Empty by default, and an empty declaration means "this game has no declared
/// HUD" — not an error. A provider that never calls the builder simply gets no
/// HUD surface, which is what every demo does today.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HudDeclaration {
    pub slots: Vec<HudSlotSpec>,
}

impl HudDeclaration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare one readout. Order defaults to declaration order, so the common
    /// case needs no explicit `order`.
    pub fn slot(mut self, spec: HudSlotSpec) -> Self {
        let mut spec = spec;
        assert!(
            self.slots.iter().all(|slot| slot.id != spec.id),
            "duplicate declared HUD slot id: {}",
            spec.id.as_str(),
        );
        if spec.order == u32::MAX {
            spec.order = self.slots.len() as u32;
        }
        self.slots.push(spec);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Slots in the region+order the renderer should lay them out in. Stable:
    /// region first, then `order`, then `id`, so a declaration never depends on
    /// `Vec` order for its visual result.
    pub fn laid_out(&self) -> Vec<&HudSlotSpec> {
        let mut out: Vec<&HudSlotSpec> = self.slots.iter().collect();
        out.sort_by(|a, b| {
            (a.region as u8, a.order, a.id.as_str()).cmp(&(b.region as u8, b.order, b.id.as_str()))
        });
        out
    }
}

/// What one readout currently says.
///
/// `label` and `value` are drawn as `"{label} {value}"` when both are present,
/// so a game controls its own vocabulary and formatting entirely — including
/// zero-padding, units, and whether there is a label at all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudReadout {
    pub label: String,
    pub value: String,
}

impl HudReadout {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }

    /// A readout with no label — the game formats the whole string itself.
    pub fn bare(value: impl Into<String>) -> Self {
        Self {
            label: String::new(),
            value: value.into(),
        }
    }

    /// The exact text to draw.
    pub fn text(&self) -> String {
        match (self.label.is_empty(), self.value.is_empty()) {
            (true, _) => self.value.clone(),
            (false, true) => self.label.clone(),
            (false, false) => format!("{} {}", self.label, self.value),
        }
    }
}

/// Live readout values, written by the GAME every frame.
///
/// The renderer only mirrors what it finds here into the slots the active
/// declaration named. A slot with no entry draws nothing, so a game may publish
/// a readout conditionally without the declaration changing.
#[derive(Resource, Clone, Debug, Default)]
pub struct HudReadouts {
    by_slot: BTreeMap<HudSlotId, HudReadout>,
}

impl HudReadouts {
    pub fn set(&mut self, id: impl Into<HudSlotId>, readout: HudReadout) {
        self.by_slot.insert(id.into(), readout);
    }

    /// Convenience for the overwhelmingly common `LABEL value` case.
    pub fn set_labelled(
        &mut self,
        id: impl Into<HudSlotId>,
        label: impl Into<String>,
        value: impl std::fmt::Display,
    ) {
        self.set(id, HudReadout::new(label, value.to_string()));
    }

    pub fn get(&self, id: &HudSlotId) -> Option<&HudReadout> {
        self.by_slot.get(id)
    }

    /// Stop publishing one slot. It then draws nothing — which is how a
    /// transient card retires without a hide path or a despawn.
    pub fn clear_slot(&mut self, id: impl Into<HudSlotId>) {
        self.by_slot.remove(&id.into());
    }

    pub fn clear(&mut self) {
        self.by_slot.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.by_slot.is_empty()
    }
}

/// Route-keyed HUD declarations, exactly parallel to
/// [`GameplayPresentationProfileCatalog`](super::GameplayPresentationProfileCatalog).
///
/// The engine holds declarations keyed by an opaque route id and never learns
/// an experience or a game name.
#[derive(Resource, Default)]
pub struct HudDeclarationCatalog {
    by_route: BTreeMap<String, HudDeclaration>,
}

impl HudDeclarationCatalog {
    pub fn insert(&mut self, route_id: impl Into<String>, declaration: HudDeclaration) {
        self.by_route.insert(route_id.into(), declaration);
    }

    pub fn get(&self, route_id: &str) -> Option<&HudDeclaration> {
        self.by_route.get(route_id)
    }

    pub fn is_empty(&self) -> bool {
        self.by_route.is_empty()
    }

    pub fn routes(&self) -> impl Iterator<Item = &str> {
        self.by_route.keys().map(String::as_str)
    }
}

/// The declaration selected for the active route. `None` while no gameplay
/// route with a declared HUD is active — the launcher, loading, and every
/// route a game chose not to give a HUD.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActiveHudDeclaration(pub Option<HudDeclaration>);

impl ActiveHudDeclaration {
    pub fn slots(&self) -> &[HudSlotSpec] {
        self.0.as_ref().map(|d| d.slots.as_slice()).unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_order_defaults_to_declaration_sequence() {
        let declaration = HudDeclaration::new()
            .slot(HudSlotSpec::new("score"))
            .slot(HudSlotSpec::new("coins"))
            .slot(HudSlotSpec::new("time"));
        let ids: Vec<&str> = declaration
            .laid_out()
            .iter()
            .map(|slot| slot.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec!["score", "coins", "time"],
            "unordered slots lay out in the order the game declared them"
        );
    }

    /// Layout must not depend on `Vec` order once a game states an order — two
    /// declarations with the same slots in different sequences must render the
    /// same.
    #[test]
    #[should_panic(expected = "duplicate declared HUD slot id")]
    fn duplicate_slot_ids_are_rejected_at_authoring_time() {
        let _ = HudDeclaration::new()
            .slot(HudSlotSpec::new("score"))
            .slot(HudSlotSpec::new("score"));
    }

    #[test]
    fn explicit_zero_order_is_not_rewritten_to_declaration_sequence() {
        let declaration = HudDeclaration::new()
            .slot(HudSlotSpec::new("later"))
            .slot(HudSlotSpec::new("first").with_order(0));
        let ids: Vec<&str> = declaration
            .laid_out()
            .iter()
            .map(|slot| slot.id.as_str())
            .collect();
        assert_eq!(ids, vec!["first", "later"]);
    }

    #[test]
    fn explicit_order_beats_declaration_sequence_and_is_stable() {
        let forwards = HudDeclaration::new()
            .slot(HudSlotSpec::new("time").with_order(30))
            .slot(HudSlotSpec::new("score").with_order(10))
            .slot(HudSlotSpec::new("coins").with_order(20));
        let backwards = HudDeclaration::new()
            .slot(HudSlotSpec::new("coins").with_order(20))
            .slot(HudSlotSpec::new("time").with_order(30))
            .slot(HudSlotSpec::new("score").with_order(10));

        let ids = |d: &HudDeclaration| -> Vec<String> {
            d.laid_out()
                .iter()
                .map(|slot| slot.id.as_str().to_string())
                .collect()
        };
        assert_eq!(ids(&forwards), vec!["score", "coins", "time"]);
        assert_eq!(
            ids(&forwards),
            ids(&backwards),
            "declaration sequence must not change the rendered order"
        );
    }

    #[test]
    fn slots_group_by_region_before_order() {
        let declaration = HudDeclaration::new()
            .slot(
                HudSlotSpec::new("bottom_first")
                    .with_region(SurroundRegion::Bottom)
                    .with_order(1),
            )
            .slot(
                HudSlotSpec::new("top_second")
                    .with_region(SurroundRegion::Top)
                    .with_order(2),
            );
        let ids: Vec<&str> = declaration
            .laid_out()
            .iter()
            .map(|slot| slot.id.as_str())
            .collect();
        assert_eq!(
            ids[0], "top_second",
            "region groups the row before order sorts within it"
        );
    }

    #[test]
    fn readout_text_is_the_games_own_vocabulary() {
        assert_eq!(HudReadout::new("RINGS", "12").text(), "RINGS 12");
        assert_eq!(HudReadout::bare("004200").text(), "004200");
        assert_eq!(HudReadout::new("LIVES", "").text(), "LIVES");
    }

    /// A slot the game never published draws nothing — publishing is
    /// per-frame and conditional, so an absent value must not be an error.
    #[test]
    fn an_unpublished_slot_has_no_readout() {
        let mut readouts = HudReadouts::default();
        readouts.set_labelled("rings", "RINGS", 12);
        assert_eq!(
            readouts.get(&HudSlotId::new("rings")).map(HudReadout::text),
            Some("RINGS 12".to_string())
        );
        assert!(readouts.get(&HudSlotId::new("score")).is_none());
    }

    /// Two games hold two different declarations in one process — the same
    /// property the world manifest needed, for the same reason.
    #[test]
    fn two_routes_hold_two_declarations() {
        let mut catalog = HudDeclarationCatalog::default();
        catalog.insert(
            "sanic_gameplay",
            HudDeclaration::new().slot(HudSlotSpec::new("rings")),
        );
        catalog.insert(
            "mary_o_gameplay",
            HudDeclaration::new()
                .slot(HudSlotSpec::new("score"))
                .slot(HudSlotSpec::new("time")),
        );
        assert_eq!(catalog.get("sanic_gameplay").unwrap().slots.len(), 1);
        assert_eq!(catalog.get("mary_o_gameplay").unwrap().slots.len(), 2);
        assert!(catalog.get("a_route_that_declared_none").is_none());
    }
}
