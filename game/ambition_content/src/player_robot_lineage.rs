//! Player Robot incarnations generated from shared source.
//!
//! Each incarnation is a complete character with its own stable id; the engine
//! does not interpret a version parameter or apply an inheritance/patch chain.
//! [`Incarnation`] contains per-version data, while [`definition`] authors the
//! shared body/moves. [`Lineage::derived_from`] records provenance only and is
//! never an authority for field resolution.

use ambition_characters::actor::definition::CharacterDefinition;
use ambition_characters::actor::definition::Lineage;
use ambition_characters::prepared::CharacterBindings;
use ambition_entity_catalog::{
    HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, VolumeShape,
};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinitionAppExt;

/// One incarnation of the player robot: everything about it that is not shared.
///
/// Deliberately TWO fields, and it had four.
///
/// `display_name` and `sheet` lived here AND in `character_catalog.ron`, with
/// nothing deciding which won per field — the AF4b duplicate-authority row. The
/// `voice` field went the same way earlier the same day, and that one was worse
/// than duplication: the catalog outranks a definition's voice, so
/// `player_robot_v2`'s Rust lines could never be heard at all. Reading the row
/// is what makes "content owns the facts" structural instead of a convention.
pub struct Incarnation {
    /// Stable id. Never reused and never repointed — that is what makes an
    /// old build a thing you can meet rather than a thing you remember. A future
    /// v4 does not take v3's id; it takes its own, and v3 keeps standing.
    ///
    /// It is also the key into the catalog: everything else this character is
    /// comes from the row under this id.
    pub id: &'static str,
    /// The incarnation this one replaced. `None` for the original.
    ///
    /// Provenance only — see the module doc. It exists so the lineage is a fact
    /// the code owns rather than a sentence in an authoring description.
    pub replaces: Option<&'static str>,
}

// Voice is catalog-authored for the robot lineage. `fallback_dialogue` provides
// the lowest-precedence lines, so the Rust definitions do not duplicate dialogue.

/// v0 — the original. Its own bark: *"Version zero. Everything after me was
/// a patch note."*
pub const V0: Incarnation = Incarnation {
    id: "robot",
    replaces: None,
};

/// v2 — the build that shipped before the SVG rig.
///
/// There is no v1. v2's own dialogue handles the question (*"There is no v1. Ask
/// someone else why."*) and its row records the reason: it is a joke, not a gap.
pub const V2: Incarnation = Incarnation {
    id: "player_robot_v2",
    replaces: Some(V0.id),
};

/// v3 — the body you are playing right now.
///
/// Named for its version rather than for being current, so v4 costs a struct literal instead of a
/// rename.
pub const V3: Incarnation = Incarnation {
    id: "player_robot_v3",
    replaces: Some(V2.id),
};

/// The whole lineage, oldest first.
pub const LINEAGE: &[&Incarnation] = &[&V0, &V2, &V3];

/// Build one incarnation's complete definition, reading its FACTS from the
/// catalog row under its id.
///
/// Everything the three have in common lives here and nowhere else. What it does
/// NOT do is inherit: no field is copied from `replaces`, and the definition that
/// comes out is complete on its own.
///
/// The sheet comes through
/// [`CatalogEntry::manifest_target`](ambition_characters::actor::character_catalog::CatalogEntry::manifest_target),
/// the same canonical projection `audit_character_authority_parity` compares with — a catalog row
/// names FILES (`sprites/player_robot_v2_spritesheet.ron`) and a definition names a TARGET
/// (`player_robot_v2`).
///
/// A missing row is a panic rather than a fallback: an incarnation the catalog
/// does not describe cannot be registered as a character, and inventing a name
/// for it here would put the duplication back one `unwrap_or` at a time.
pub fn definition(incarnation: &Incarnation) -> CharacterDefinition {
    definition_from(&crate::character_catalog::load_catalog(), incarnation)
}

/// [`definition`] against an already-parsed catalog, so registering the whole
/// lineage parses the roster ONCE instead of once per incarnation.
fn definition_from(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    incarnation: &Incarnation,
) -> CharacterDefinition {
    let row = catalog.get(incarnation.id).unwrap_or_else(|| {
        panic!(
            "player-robot incarnation `{}` has no row in character_catalog.ron — \
             the lineage names who exists; the row says what they are",
            incarnation.id
        )
    });
    let sheet = row.manifest_target().unwrap_or_else(|| {
        panic!(
            "`{}`'s catalog manifest `{}` does not follow the \
             `<target>_spritesheet.ron` convention, so no sheet target can be \
             derived from it",
            incarnation.id, row.manifest
        )
    });
    let mut definition = CharacterDefinition::new(
        incarnation.id,
        row.display_name.clone(),
        crate::AMBITION_CONTENT_PROVIDER,
    )
    .with_sheet(sheet);
    // Derive collision scale from published sprite metrics while keeping the
    // canonical standing height stable across sheet regeneration. Incarnations
    // without authored body metrics retain their existing body source.
    if let Some(body_px) = ambition_platformer2d::character_sprites::authored_body_pixel_size(sheet)
    {
        // the robot's canonical height IS the engine's default playable body: 48 world pixels,
        // exactly three tiles.
        let canonical_height = ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT;
        if let Some(world_per_pixel) =
            ambition_characters::actor::definition::world_per_pixel_for_height(
                canonical_height,
                body_px.y,
            )
        {
            definition = definition
                .with_canonical_height(canonical_height)
                .with_sprite_authored_body(world_per_pixel)
                .with_hurtboxes(forgiving_hurtbox(body_px * world_per_pixel));
        }
    }
    // All incarnations share the same authored body and moveset. The host still
    // owns progression-gated action availability; these timelines define what
    // the robot's attacks are, not which actions are currently permitted.
    definition.vitals.max_health = Some(60);
    definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 200.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ambition_characters::actor::ContactDamage {
            strength: 0.6,
            amount: 1,
        })
        // Autonomous behavior is a named policy shared independently of body identity.
        .with_autonomous_profile_named("robot_duelist")
        // Character-authored body capabilities. Matches may mask these abilities but do not
        // invent them. Flight is intentional for this grounded hybrid; reset is debug-only.
        .with_abilities(ambition_platformer2d_core::AbilitySet {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            wall_jump: true,
            wall_cling: true,
            wall_climb: true,
            dash: true,
            double_dash: true,
            blink: true,
            precision_blink: true,
            blink_through_soft_walls: true,
            blink_through_hard_walls: true,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            ledge_grab: true,
            swim: true,
            glide: true,
            dodge: true,
            shield: true,
            interact: true,
            fly: true,
            fly_toggle: true,
            ..ambition_platformer2d_core::AbilitySet::NONE
        });
    // Character-owned ranged presentation.
    definition = definition.with_ranged_vfx("hadouken");
    // Hold to charge, release to fire.
    definition = definition
        .with_ranged_execution(ambition_characters::brain::RangedExecution::ChargedProjectile);
    // Incarnations share lineage while authoring different repertoires.
    if incarnation.id == V2.id {
        definition = definition.with_moveset(crate::player_robot_moveset::theorem_chain_moveset());
    }
    if incarnation.id == V3.id {
        definition = definition.with_moveset(crate::player_robot_moveset::player_robot_moveset());
        // V3 authors both move timelines and the action slots it exposes.
        definition =
            definition.with_action_set(crate::player_robot_moveset::player_robot_action_set());
    }
    definition.lineage = Some(Lineage {
        derived_from: incarnation.replaces.map(str::to_string),
        // Hand-authored incarnations have no generator provenance.
        generator_revision: None,
        source_fingerprint: None,
    });
    definition
}

/// Combat targets the torso rather than the full collision outline.
///
/// The collision body still includes the head for world collision. The hurtbox
/// begins near the shoulders, retains the feet, and excludes arm/head overhang.
///
/// ⛔ **A STANCE MOVES THE CENTRE, AND THIS VOLUME IS PLACED AT THE CENTRE.**
/// The edges below are fractions of the body box, but they are baked to world
/// offsets here and `hurtbox_world_aabb` puts them at `pos`. A crouch halves
/// the box and slides `pos` toward the feet, so a volume measured against the
/// STANDING box hangs a quarter of the standing height through the floor — you
/// could see it under the platform with the combat overlay on. So the crouch
/// gets its own profile, measured against the box it will actually be worn
/// with. `HurtboxDoc::poses` and `BodyPoseClock` already carried this seam
/// end to end; nothing authored it.
fn forgiving_hurtbox(body_world: ambition_platformer2d_core::Vec2) -> HurtboxDoc {
    HurtboxDoc {
        default: Some(forgiving_timeline(body_world)),
        poses: std::iter::once((
            ambition_combat::hurtbox_resolution::POSE_CROUCH.to_string(),
            // The same rule the stance applies to the collision box, applied to
            // the volume worn inside it — asked of `BodyMode::shape` rather than
            // restated, so the two cannot disagree about what crouching means.
            forgiving_timeline(
                ambition_platformer2d_core::player_state::BodyMode::Crouching
                    .shape(body_world)
                    .size,
            ),
        ))
        .collect(),
        moves: Default::default(),
    }
}

/// The forgiving torso volume for one body box, in that box's own frame.
fn forgiving_timeline(body_world: ambition_platformer2d_core::Vec2) -> HurtboxTimeline {
    // Fractions of the body box, per edge.
    const LEFT: f32 = 0.09;
    const RIGHT: f32 = 0.21;
    const TOP: f32 = 0.43;
    const BOTTOM: f32 = 0.01;

    // World +y is down, so a lower hurtbox center has a positive y offset.
    let offset = ambition_platformer2d_core::Vec2::new(
        ((LEFT + (1.0 - RIGHT)) * 0.5 - 0.5) * body_world.x,
        ((TOP + (1.0 - BOTTOM)) * 0.5 - 0.5) * body_world.y,
    );
    let half_extents = ambition_platformer2d_core::Vec2::new(
        (1.0 - LEFT - RIGHT) * 0.5 * body_world.x,
        (1.0 - TOP - BOTTOM) * 0.5 * body_world.y,
    );
    HurtboxTimeline {
        keyframes: vec![HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![HurtboxVolume {
                shape: VolumeShape::Rect {
                    offset: (offset.x, offset.y),
                    half_extents: (half_extents.x, half_extents.y),
                },
            }],
        }],
    }
}

/// Register every incarnation as a character in its own right.
///
/// Kits remain catalog-authored and are folded in during preparation; do not
/// duplicate them on these definitions.
pub fn register(app: &mut bevy::prelude::App) {
    // Parsed ONCE for the whole lineage. Three strings do not justify three
    // parses of the roster, and the cast is only going to grow.
    let catalog = crate::character_catalog::load_catalog();
    for incarnation in LINEAGE {
        app.try_register_character(
            definition_from(&catalog, incarnation),
            // the seam fills the engine's sheet AND portrait vocabularies itself
            // (`with_engine_vocabularies`), so a target that names nothing is reported at load
            // with a did-you-mean rather than silently drawing the marked rectangle — whether
            // or not a provider remembered to ask.
            CharacterBindings::default(),
        )
        .unwrap_or_else(|error| panic!("player-robot incarnation rejected: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chain is well-formed, and it is a chain.
    ///
    /// Exactly one origin, every other link naming the incarnation before it, and no id
    /// repeated.
    #[test]
    fn the_lineage_is_an_unbroken_chain_of_distinct_characters() {
        let ids: Vec<&str> = LINEAGE.iter().map(|inc| inc.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            ids.len(),
            "two incarnations share an id, so one of them cannot be met: {ids:?}"
        );

        let mut previous: Option<&str> = None;
        for incarnation in LINEAGE {
            assert_eq!(
                incarnation.replaces, previous,
                "incarnation '{}' does not name the one before it — the lineage \
                 is a chain, and a break in it makes provenance a guess",
                incarnation.id
            );
            previous = Some(incarnation.id);
        }
    }

    /// v3 stands as tall as the level expects, and their box is their ART.
    ///
    /// player sprite."* It was, by 1.28× wide and 1.29× tall, because their box
    /// was the engine's default constant while their sprite was drawn through a
    /// hand-tuned `collision_scale` and nothing reconciled the two.
    ///
    /// Both halves are asserted because either alone is satisfiable by a bug:
    /// a body source that resolves to nothing would leave the height right and
    /// the box unowned, and a scale read off today's pixel count would leave the
    /// box owned and the height wrong the next time a crop moves.
    #[test]
    fn v3s_body_is_his_sheets_and_he_still_stands_at_the_authored_height() {
        use ambition_characters::actor::definition::BodySource;

        let catalog = crate::character_catalog::load_catalog();
        let definition = definition_from(&catalog, &V3);
        let Some(BodySource::SpriteAuthored { world_per_pixel }) = definition.body else {
            panic!(
                "v3 authors no sprite body, so their collision box is still the \
                 engine's default constant and their sprite is still drawn by a \
                 hand-tuned collision_scale: {:?}",
                definition.body
            );
        };

        let pixels =
            ambition_platformer2d::character_sprites::authored_body_pixel_size("player_robot_v3")
                .expect("v3's sheet publishes an AUTHORED body box, not a measured alpha bbox");
        let standing = pixels * world_per_pixel;
        assert!(
            (standing.y - ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT).abs() < 0.01,
            "v3 stands {} units tall against the {} the levels are authored \
             around — the scale is DERIVED from the height, never the reverse",
            standing.y,
            ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT,
        );
    }

    /// A crouching robot's hurtbox stays inside the box a crouching robot wears.
    ///
    /// The volume is placed at the body's CENTRE, and a stance moves the centre
    /// without moving the feet. So a volume measured against the standing box
    /// and worn while crouching hangs through the floor — a quarter of the
    /// standing height of it, visible under the platform with the combat overlay
    /// on. Guards the OUTPUT: where the volume's edges land relative to the box
    /// it is actually worn with, not which numbers went in.
    #[test]
    fn a_crouching_robots_hurtbox_stays_inside_a_crouching_robot() {
        use ambition_combat::hurtbox_resolution::POSE_CROUCH;
        use ambition_entity_catalog::VolumeShape;

        let catalog = crate::character_catalog::load_catalog();
        let definition = definition_from(&catalog, &V3);
        let doc = definition.hurtboxes.as_ref().expect("v3 authors a hurtbox");
        let pixels =
            ambition_platformer2d::character_sprites::authored_body_pixel_size("player_robot_v3")
                .expect("v3's sheet authors a body box");
        let standing = pixels * (ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT / pixels.y);

        for (pose, body) in [
            (None, standing),
            (
                Some((POSE_CROUCH, 0.0)),
                ambition_platformer2d_core::player_state::BodyMode::Crouching
                    .shape(standing)
                    .size,
            ),
        ] {
            let volumes = doc
                .volumes_for(None, pose)
                .unwrap_or_else(|| panic!("{pose:?} resolves to a timeline"));
            let VolumeShape::Rect {
                offset,
                half_extents,
            } = volumes[0].shape
            else {
                panic!("the torso is a rect: {:?}", volumes[0].shape);
            };
            // Feet are the +gravity face of the box the body wears in this pose.
            let feet = body.y * 0.5;
            assert!(
                offset.1 + half_extents.1 <= feet,
                "{pose:?}: the hurtbox reaches {} below the body centre against \
                 feet at {feet} — it is {} units through the floor",
                offset.1 + half_extents.1,
                offset.1 + half_extents.1 - feet,
            );
            assert!(
                offset.1 - half_extents.1 >= -feet,
                "{pose:?}: the hurtbox reaches above the body's own crown"
            );
        }
    }

    /// The forgiving hurtbox is strictly inside the box that carries it.
    ///
    /// the main head and well within the arms — and the only way that claim can
    /// be wrong without anyone noticing is if the authored volume quietly
    /// resolves to something as big as the collision box, which is exactly what
    /// the unauthored fallback does. So this asserts the CONTAINMENT rather than
    /// the numbers: every edge strictly inside, and the top by much more than
    /// the sides, which is what "under the head" means on a body whose head is
    /// most of its silhouette.
    #[test]
    fn v3s_hurtbox_is_smaller_than_his_collision_box_on_every_edge() {
        use ambition_entity_catalog::VolumeShape;

        let catalog = crate::character_catalog::load_catalog();
        let definition = definition_from(&catalog, &V3);
        let doc = definition
            .hurtboxes
            .as_ref()
            .expect("v3 authors a hurtbox; without one the hit lands on the coarse body box");
        let volumes = doc
            .volumes_for(None, None)
            .expect("their default timeline resolves at rest");
        assert_eq!(volumes.len(), 1, "one torso volume, not a part list");
        let VolumeShape::Rect {
            offset,
            half_extents,
        } = volumes[0].shape
        else {
            panic!("the torso is a rect: {:?}", volumes[0].shape);
        };

        let pixels =
            ambition_platformer2d::character_sprites::authored_body_pixel_size("player_robot_v3")
                .expect("v3's sheet authors a body box");
        let body = pixels * (ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT / pixels.y);

        // Every edge of the hurtbox, against the matching edge of the body box.
        for (axis, off, half, body_half) in [
            ("x", offset.0, half_extents.0, body.x * 0.5),
            ("y", offset.1, half_extents.1, body.y * 0.5),
        ] {
            assert!(
                off - half > -body_half && off + half < body_half,
                "the hurtbox escapes the collision box on {axis}: it spans \
                 {}..{} against a body half-extent of {body_half} — a hurtbox \
                 wider than the body it belongs to is not forgiving, it is the \
                 bug this replaced",
                off - half,
                off + half,
            );
        }
        assert!(
            half_extents.1 < body.y * 0.35,
            "the hurtbox is {} tall against a body half-height of {} — 'under \
             the main head' means the head is OUT of it, and their head is more \
             than a third of them",
            half_extents.1 * 2.0,
            body.y * 0.5,
        );
        assert!(
            offset.1 > 0.0,
            "+y is DOWN in this engine (DEFAULT_GRAVITY_DIR is (0, 1)), so a \
             torso box sitting below the body centre must have a POSITIVE y \
             offset; {} puts their hurtbox in the air above their head",
            offset.1,
        );
    }

    /// v0 and v2 keep the path they have, and the reason is a fact about
    /// their sheets rather than a decision spelled out per version.
    ///
    /// Their boxes are still raw alpha silhouettes — arms and all — so the
    /// lineage's blanket rule declines them on its own. If someone authors one,
    /// it opts in with no edit here, which is the point of asking the sheet.
    #[test]
    fn an_incarnation_that_only_measured_its_box_is_not_given_a_sprite_body() {
        use ambition_platformer2d::character_sprites::authored_body_pixel_size;

        let catalog = crate::character_catalog::load_catalog();
        for incarnation in [&V0, &V2] {
            if authored_body_pixel_size(incarnation.id).is_some() {
                continue; // someone authored it since; the rule opts it in.
            }
            assert!(
                definition_from(&catalog, incarnation).body.is_none(),
                "'{}' measured its box rather than authoring one, so scaling \
                 them by it would hand them a collision body that includes their \
                 outstretched arms",
                incarnation.id,
            );
        }
    }

    /// The authored body box is INSET from the art it belongs to, asked of
    /// the sheet rather than of a number typed here.
    ///
    /// Only a comparison against the drawing catches that.
    ///
    /// the sheet already publishes its own alpha extent, so nothing has to
    /// decode a PNG. The atlas packer trims every frame to its opaque alpha
    /// bounding box and records where that box sat inside the logical frame
    /// (`FrameRect::off`), so the union over a row's frames IS the drawn
    /// silhouette — in the very same logical-frame pixel space
    /// `body_pixel_bbox` is expressed in. That is why this can assert a
    /// RELATIONSHIP instead of pixel constants, and stay true when the art is
    /// redrawn.
    ///
    /// the bottom edge is deliberately NOT required to be inset: that is the
    /// shoe line, and lifting a collision box off the floor is how a character
    /// starts hovering. "Under the main head" is likewise the HURTBOX's job
    /// (see above) — this box only has to clear the antenna.
    #[test]
    fn v3s_authored_body_box_is_inset_from_his_drawn_silhouette() {
        use ambition_sprite_sheet::character::sheets;

        let record = sheets::record_for_sheet_key("player_robot_v3")
            .expect("v3's spritesheet is baked into the sheet index");
        let metrics = record
            .body_metrics
            .as_ref()
            .expect("v3's sheet publishes body metrics");
        assert!(
            metrics.authored_body,
            "v3's sheet only MEASURED its box, so `authored_body_pixel_size` \
             refuses it and the lineage hands them back the engine's default \
             constant — the bug this closes",
        );
        let body = metrics
            .body_pixel_bbox
            .expect("an authored body is a rectangle");

        let idle = record
            .rows
            .iter()
            .find(|row| row.animation == "idle")
            .expect("v3 has an idle row; it is the pose the standing body is read from");
        let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for rect in &idle.rects {
            left = left.min(rect.off.0);
            top = top.min(rect.off.1);
            right = right.max(rect.off.0 + rect.w);
            bottom = bottom.max(rect.off.1 + rect.h);
        }

        // NON-VACUITY, three ways. Without these the comparisons below pass
        // on a sheet that says nothing: an empty row leaves the union inverted,
        // and an UNTRIMMED row reports `off == (0, 0)` with `w`/`h` equal to the
        // whole logical frame, which is trivially bigger than any body box.
        assert!(
            !idle.rects.is_empty() && right > left && bottom > top,
            "v3's idle row publishes no frame extent, so there is no silhouette \
             to be inset from and every assertion below is vacuous",
        );
        assert!(
            right - left < record.frame_width as i32 && bottom - top < record.frame_height as i32,
            "v3's frames are untrimmed, so `off`/`w`/`h` describe the whole \
             {}×{} logical frame instead of his alpha extent — this test would \
             then pass on a body box of any size at all",
            record.frame_width,
            record.frame_height,
        );

        assert!(
            body.x > left && body.x + body.w < right,
            "v3's body box spans x {}..{} against a drawn silhouette of \
             {left}..{right}: it reaches his arms, which is what 'well within \
             the player arms' rules out",
            body.x,
            body.x + body.w,
        );
        assert!(
            (body.w as f32) < 0.9 * (right - left) as f32,
            "v3's body box is {} px wide against a {} px silhouette — a hair \
             narrower is not 'well within the arms', and a hurtbox that \
             forgiving has to clear the arm span, not graze it",
            body.w,
            right - left,
        );
        assert!(
            body.y > top,
            "v3's body box starts at y {} against a silhouette starting at \
             {top}, so his antenna is inside his collision box and he is hit by \
             things that pass over his head",
            body.y,
        );
        assert!(
            body.y + body.h <= bottom,
            "v3's body box ends at y {} below his own art, which ends at \
             {bottom} — a box that overhangs the shoe line plants his feet under \
             the floor",
            body.y + body.h,
        );
    }

    /// Every incarnation's art resolves, and to a DIFFERENT sheet.
    ///
    /// the second half is the one worth having. Eighteen shipped sheets
    /// declare `target: "robot"` — the name of the procedural generator, not of
    /// a character — so "the target resolves" is satisfied by all three
    /// resolving to the same robot. Distinctness is what says three incarnations
    /// actually look like three characters.
    #[test]
    fn every_incarnation_resolves_its_own_distinct_sheet() {
        use ambition_sprite_sheet::character::sheets;

        let mut seen: Vec<String> = Vec::new();
        for incarnation in LINEAGE {
            let sheet = definition(incarnation)
                .sheet
                .expect("the lineage always names a sheet target");
            assert!(
                sheets::record_for_sheet_key(&sheet).is_some(),
                "incarnation '{}' names sheet target '{sheet}', which resolves to \
                 nothing — it would draw the marked placeholder",
                incarnation.id,
            );
            assert!(
                !seen.contains(&sheet),
                "incarnation '{}' shares sheet '{sheet}' with an earlier one, so \
                 the lineage is one body wearing three names",
                incarnation.id,
            );
            seen.push(sheet);
        }
    }

    /// The name comes from the row, and there is only one row. (AF4b)
    ///
    /// The duplication this closes: `Incarnation` carried a `display_name` and
    /// so does the catalog, with nothing deciding which won per field. Now the
    /// definition IS the row's answer, so `DisplayNameDisagreement` cannot fire
    /// for these three by construction rather than by luck.
    #[test]
    fn every_incarnation_presents_under_its_catalog_name() {
        let catalog = crate::character_catalog::load_catalog();
        for incarnation in LINEAGE {
            let row = catalog
                .get(incarnation.id)
                .expect("every incarnation has a catalog row");
            assert_eq!(
                definition(incarnation).display_name,
                row.display_name,
                "incarnation '{}' presents under a name the catalog does not \
                 give it",
                incarnation.id,
            );
        }
    }

    /// Nobody in the lineage stands mute — asked of the RUNTIME, not the
    /// struct. (AF4b)
    ///
    /// It was green while `player_robot_v2`'s lines were unreachable: the catalog outranks a
    /// definition's voice, and v2's row authored both a `barks.hall` pool AND a
    /// `fallback_dialogue`, so `CatalogEntry::bark` always answered first.
    ///
    /// So ask the question the ticker asks. `bark` falls through the situation
    /// pool to `fallback_dialogue`, and a row with neither returns `None` — which
    /// is exactly the silence this test is named for.
    #[test]
    fn every_incarnation_says_something() {
        let catalog = crate::character_catalog::load_catalog();
        for incarnation in LINEAGE {
            for situation in [
                ambition_characters::actor::character_catalog::BarkSituation::Hall,
                ambition_characters::actor::character_catalog::BarkSituation::Idle,
            ] {
                assert!(
                    catalog.bark_line(incarnation.id, situation, 0).is_some(),
                    "incarnation '{}' has nothing to say in {situation:?}, so the \
                     ambient ticker skips it and it stands there silent",
                    incarnation.id,
                );
            }
        }
    }

    /// Every incarnation is in the playable cast.
    ///
    /// The point of the whole arrangement: "play as the build before this one"
    /// is a selection, not a content edit.
    #[test]
    fn every_incarnation_can_be_worn() {
        for incarnation in LINEAGE {
            assert!(
                crate::character_catalog::PLAYABLE_ROSTER.contains(&incarnation.id),
                "incarnation '{}' is a character you can meet and not one you \
                 can be — put it in PLAYABLE_ROSTER",
                incarnation.id,
            );
        }
    }
}

/// Register every playable character declared by this provider.
///
/// Definitions are projected from catalog rows so the catalog remains the
/// authority for names and sheets. Registration makes those characters available
/// to `PreparedCharacterRegistry` for match construction.
pub fn register_declared_cast(app: &mut bevy::prelude::App) {
    let catalog = crate::character_catalog::load_catalog();
    // The lineage registers itself above with authored bodies and hurtboxes;
    // re-registering here would be a duplicate and would also throw those away.
    let lineage: std::collections::BTreeSet<&str> =
        LINEAGE.iter().map(|incarnation| incarnation.id).collect();
    // Register only the declared playable cast. A bare registration for an
    // exploration NPC would incorrectly replace its archetype-authored body.
    //
    // Empty build-only list today, so this iterates exactly what it always did.
    for id in crate::character_catalog::buildable_cast() {
        if lineage.contains(id) {
            continue;
        }
        let Some(row) = catalog.get(id) else {
            continue;
        };
        let id = id.to_string();
        // No derivable sheet target means nothing to wear. The load ledger
        // already reports that class; a registration that could not draw would
        // be a second reporter of one fact.
        let Some(sheet) = row.manifest_target() else {
            continue;
        };
        let definition = CharacterDefinition::new(
            id.clone(),
            row.display_name.clone(),
            crate::AMBITION_CONTENT_PROVIDER,
        )
        .with_sheet(sheet);
        // What this character says about its own body, for the ones that
        // have taken their facts back from the archetype roster. A character
        // still awaiting migration adds nothing here and stays a bare
        // registration.
        let definition = crate::character_catalog::authored_intrinsics(&id, definition);
        // `try_`, and a SKIP rather than a panic: another provider legitimately
        // owns some of these ids in a multi-game composition, and losing a race
        // for one is not this provider's error to raise.
        let _ = app.try_register_character(
            definition,
            // See the sibling registration above: the seam fills both engine
            // vocabularies, so a provider passing nothing is checked identically.
            CharacterBindings::default(),
        );
    }
}
