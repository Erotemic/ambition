//! The player robot's lineage: `robot` (v0), `player_robot_v2` and
//! `player_robot_v3`, three incarnations of one character. There is no v1. Ambition
//! wants old versions of yourself to be things you can meet, talk to, fight and
//! play as.
//!
//! Each incarnation is its catalog row, like every other character: the row
//! states the body, the kit and the voice, and `derived_from` names the version
//! before it. That is provenance only; nothing is inherited along the chain.
//! What is here checks that the rows say what the lineage means.

/// The robot's lineage in the SHIPPED catalog, oldest first (test-only: it
/// inspects the shipped product, and no composition asks it): the provider's default character and each
/// row its `derived_from` names, followed back to the row that names none.
///
/// # Panics
///
/// When a row on the chain names a character the catalog does not have, or
/// the chain comes back to itself.
#[cfg(test)]
pub fn lineage() -> Vec<String> {
    let catalog = crate::character_catalog::shipped_catalog();
    let mut chain = vec![crate::character_catalog::DEFAULT_CHARACTER.to_string()];
    while let Some(previous) = catalog
        .get(chain.last().expect("the chain starts non-empty"))
        .unwrap_or_else(|| panic!("the lineage names `{}`, which has no row", chain.last().unwrap()))
        .derived_from
        .clone()
    {
        assert!(!chain.contains(&previous), "the lineage comes back to `{previous}`: {chain:?}");
        chain.push(previous);
    }
    chain.reverse();
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared_cast() -> bevy::prelude::App {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
        app
    }

    fn prepared<'a>(
        app: &'a bevy::prelude::App,
        id: &str,
    ) -> &'a ambition_characters::prepared::PreparedCharacterDefinition {
        app.world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .get(id)
            .unwrap_or_else(|| panic!("`{id}` is a character you can meet and not one you can be"))
    }

    /// The chain is well-formed, and it is a chain: one origin, every other
    /// link naming the incarnation before it, and the prepared characters carry
    /// what the rows say. There is no v1: v2 names v0.
    #[test]
    fn the_lineage_is_an_unbroken_chain_of_distinct_characters() {
        assert_eq!(lineage(), ["robot", "player_robot_v2", "player_robot_v3"]);
        let app = prepared_cast();
        let mut previous: Option<&str> = None;
        for id in &lineage() {
            let id = id.as_str();
            let derived_from = prepared(&app, id)
                .lineage
                .as_ref()
                .and_then(|lineage| lineage.derived_from.as_deref());
            assert_eq!(
                derived_from, previous,
                "incarnation '{id}' does not name the one before it — the lineage \
                 is a chain, and a break in it makes provenance a guess"
            );
            previous = Some(id);
        }
    }

    /// v3 stands as tall as the level expects, and their box is their ART.
    ///
    /// It was not, by 1.28× wide and 1.29× tall, because their box was the
    /// engine's default constant while their sprite was drawn through a
    /// hand-tuned `collision_scale` and nothing reconciled the two. Both halves
    /// are asserted because either alone is satisfiable by a bug: a body source
    /// that resolves to nothing would leave the height right and the box
    /// unowned, and a scale read off today's pixel count would leave the box
    /// owned and the height wrong the next time a crop moves.
    #[test]
    fn v3s_body_is_his_sheets_and_he_still_stands_at_the_authored_height() {
        use ambition_characters::actor::definition::BodySource;

        let app = prepared_cast();
        let v3 = prepared(&app, "player_robot_v3");
        let Some(BodySource::SpriteAuthored { world_per_pixel }) = v3.body else {
            panic!(
                "v3 authors no sprite body, so their collision box is still the \
                 engine's default constant: {:?}",
                v3.body
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

    fn v3_standing_body() -> ambition_platformer2d_core::Vec2 {
        let pixels =
            ambition_platformer2d::character_sprites::authored_body_pixel_size("player_robot_v3")
                .expect("v3's sheet authors a body box");
        pixels * (ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT / pixels.y)
    }

    /// A crouching robot's hurtbox stays inside the box a crouching robot wears.
    ///
    /// The volume is placed at the body's CENTRE, and a stance moves the centre
    /// without moving the feet. So a volume measured against the standing box
    /// and worn while crouching hangs through the floor. Guards the OUTPUT:
    /// where the volume's edges land relative to the box it is actually worn
    /// with, not which numbers went in.
    #[test]
    fn a_crouching_robots_hurtbox_stays_inside_a_crouching_robot() {
        use ambition_combat::hurtbox_resolution::POSE_CROUCH;
        use ambition_entity_catalog::VolumeShape;

        let app = prepared_cast();
        let doc = prepared(&app, "player_robot_v3")
            .hurtboxes
            .as_ref()
            .expect("v3 authors a hurtbox");
        let standing = v3_standing_body();
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

    /// The forgiving hurtbox is strictly inside the box that carries it, and the
    /// top by much more than the sides: "under the head" on a body whose head is
    /// most of its silhouette. The failure this guards is an authored volume
    /// that quietly resolves to something as big as the collision box, which is
    /// what the unauthored fallback does.
    #[test]
    fn v3s_hurtbox_is_smaller_than_his_collision_box_on_every_edge() {
        use ambition_entity_catalog::VolumeShape;

        let app = prepared_cast();
        let doc = prepared(&app, "player_robot_v3")
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
        let body = v3_standing_body();
        for (axis, off, half, body_half) in [
            ("x", offset.0, half_extents.0, body.x * 0.5),
            ("y", offset.1, half_extents.1, body.y * 0.5),
        ] {
            assert!(
                off - half > -body_half && off + half < body_half,
                "the hurtbox escapes the collision box on {axis}: it spans \
                 {}..{} against a body half-extent of {body_half}",
                off - half,
                off + half,
            );
        }
        assert!(
            half_extents.1 < body.y * 0.35,
            "the hurtbox is {} tall against a body half-height of {} — 'under \
             the main head' means the head is OUT of it",
            half_extents.1 * 2.0,
            body.y * 0.5,
        );
        assert!(
            offset.1 > 0.0,
            "+y is DOWN in this engine, so a torso box below the body centre has a \
             POSITIVE y offset; {} puts their hurtbox above their head",
            offset.1,
        );
    }

    /// An incarnation built on its art has a sheet that AUTHORS its body box.
    ///
    /// A sheet that only MEASURED its box has a raw alpha silhouette, arms and
    /// all, and a body built on that includes the outstretched arms. The row
    /// states `posed_body`, so this checks the row against its sheet.
    #[test]
    fn an_incarnation_built_on_its_art_has_a_sheet_that_authors_its_body_box() {
        use ambition_characters::actor::definition::BodySource;
        use ambition_platformer2d::character_sprites::authored_body_pixel_size;

        let app = prepared_cast();
        let mut built_on_art = 0;
        for id in &lineage() {
            if !matches!(prepared(&app, id).body, Some(BodySource::SpriteAuthored { .. })) {
                continue;
            }
            built_on_art += 1;
            assert!(
                authored_body_pixel_size(id).is_some(),
                "'{id}' is built on a box its sheet only measured, so its body \
                 includes the outstretched arms",
            );
        }
        assert!(built_on_art >= 1, "no incarnation is built on its art, so this checks nothing");
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
    /// The second half is the one worth having. Eighteen shipped sheets declare
    /// `target: "robot"` (the name of the procedural generator, not of a
    /// character), so "the target resolves" is satisfied by all three resolving
    /// to the same robot. Distinctness is what says three incarnations look like
    /// three characters.
    #[test]
    fn every_incarnation_resolves_its_own_distinct_sheet() {
        use ambition_sprite_sheet::character::sheets;

        let app = prepared_cast();
        let mut seen: Vec<String> = Vec::new();
        for id in &lineage() {
            let id = id.as_str();
            let sheet = prepared(&app, id)
                .sheet
                .clone()
                .expect("every incarnation names a sheet target");
            assert!(
                sheets::record_for_sheet_key(&sheet).is_some(),
                "incarnation '{id}' names sheet target '{sheet}', which resolves to \
                 nothing — it would draw the marked placeholder",
            );
            assert!(
                !seen.contains(&sheet),
                "incarnation '{id}' shares sheet '{sheet}' with an earlier one, so \
                 the lineage is one body wearing three names",
            );
            seen.push(sheet);
        }
    }

    /// Nobody in the lineage stands mute: asked the way the ambient ticker
    /// asks, through the situation pool and then `fallback_dialogue`.
    #[test]
    fn every_incarnation_says_something() {
        let catalog = crate::character_catalog::shipped_catalog();
        for id in &lineage() {
            let id = id.as_str();
            for situation in [
                ambition_characters::actor::character_catalog::BarkSituation::Hall,
                ambition_characters::actor::character_catalog::BarkSituation::Idle,
            ] {
                assert!(
                    catalog.bark_line(id, situation, 0).is_some(),
                    "incarnation '{id}' has nothing to say in {situation:?}, so the \
                     ambient ticker skips it and it stands there silent",
                );
            }
        }
    }
}
