//! **The player robot's incarnations, emitted from one source.**
//!
//! The protagonist has been rebuilt twice, and Ambition's answer to that is not
//! a changelog. `npc_player_robot_v2`'s catalog row says it outright: an old
//! build is *"preserved as a CHARACTER rather than as history, [because]
//! Ambition wants old versions of yourself to be things you can meet, talk to,
//! and fight, so this keeps its own id, its own sheet, and its own pedestal
//! instead of living in a git object."*
//!
//! # These are separate characters, not variants of one
//!
//! That distinction is the whole design and it is easy to lose. A "player robot"
//! with a version *parameter* would be one character wearing three coats, and
//! every system downstream would have to learn what a version is in order to ask
//! anything useful. What exists instead is three characters that happen to share
//! a face: each has its own stable id, its own art, and its own kit — v0 is
//! peaceful, v2 swings the generic striker swipe the protagonist used to carry,
//! v3 carries the host-code kit — and nothing downstream knows they are related.
//!
//! # What the sharing is, and what it is NOT
//!
//! §4.3's rule, stated on [`Lineage`]: *"two independent, fully-resolved
//! products with distinct stable ids, emitted by one generator from shared
//! source. The engine **never learns what a mode is** — there is no patch layer
//! and no override precedence."*
//!
//! So the sharing lives HERE, in a generator, and stops at the door. What comes
//! out is three complete definitions. [`Incarnation`] is the part that differs;
//! [`definition`] is the part they have in common. Adding v4 is a struct literal
//! — which is the same shape `versus_fighters::DuelistNumbers` uses for the two
//! duelists, and for the same reason.
//!
//! ⚠ [`Lineage::derived_from`] is **provenance, not authority**. It records that
//! v3 replaced v2; nothing resolves through it, and no field of v3 is inherited
//! from v2. A reader who treats it as an inheritance edge has reintroduced the
//! patch layer this design exists to refuse.

use ambition_entity_catalog::{
    HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, VolumeShape,
};
use ambition_platformer2d_actor_monolith::character_runtime::{
    CharacterBindings, CharacterDefinition, CharacterDefinitionAppExt, Lineage,
};

/// One incarnation of the player robot: everything about it that is not shared.
///
/// Deliberately TWO fields, and it had four. **Jon ruled 2026-07-31 that each
/// robot version is a different character** — so a version's FACTS (its name,
/// its sheet, its physicals, its voice) belong to that character's content row,
/// and what Rust owns is the reusable lineage COMPOSITION: who exists, and which
/// one replaced which.
///
/// `display_name` and `sheet` lived here AND in `character_catalog.ron`, with
/// nothing deciding which won per field — the AF4b duplicate-authority row. The
/// `voice` field went the same way earlier the same day, and that one was worse
/// than duplication: the catalog outranks a definition's voice, so
/// `player_robot_v2`'s Rust lines could never be heard at all. Reading the row
/// is what makes "content owns the facts" structural instead of a convention.
pub struct Incarnation {
    /// Stable id. **Never reused and never repointed** — that is what makes an
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

// ⚠ **no `voice` field, and its removal is AF4b** (Jon ruled 2026-07-31: each
// version is a different CHARACTER, so version-specific facts belong to the
// per-character content row and Rust owns reusable lineage COMPOSITION).
//
// It was authored here AND in `character_catalog.ron`, with v0's two lines
// duplicated verbatim between them — and the duplicate was not symmetric. The
// catalog outranks a definition's voice (`npc_ambient_bark_line` asks
// `catalog.bark_line` first; the definition answers only when the catalog had
// nothing), and `CatalogEntry::bark` falls through `barks.pick` to
// `fallback_dialogue`. So `player_robot_v2`, which authored BOTH, could never
// reach its Rust voice at all — it was dead, and the test asserting every
// incarnation "says something" was green over it because it read the struct
// rather than the runtime.
//
// v0 and v3 authored only `barks.hall`, so their Rust lines DID speak — but only
// away from a pedestal, which is the one place they are usually seen. Both rows
// gained a `fallback_dialogue` carrying exactly those lines, so the voice they
// had is the voice they keep, from one authority.

/// **v0 — the original.** Its own bark: *"Version zero. Everything after me was
/// a patch note."*
pub const V0: Incarnation = Incarnation {
    id: "robot",
    replaces: None,
};

/// **v2 — the build that shipped before the SVG rig.**
///
/// There is no v1. v2's own dialogue handles the question (*"There is no v1. Ask
/// someone else why."*) and its row records the reason: it is a joke, not a gap.
pub const V2: Incarnation = Incarnation {
    id: "player_robot_v2",
    replaces: Some(V0.id),
};

/// **v3 — the body you are playing right now.**
///
/// Named for its version rather than for being current, so v4 costs a struct
/// literal instead of a rename. Until 2026-07-29 this was the one incarnation
/// whose id meant "whichever is latest", which would have made preserving it a
/// retroactive rename of every sheet, rig and reference it owns.
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
/// ⚠ **the row is the authority for the name and the art, and this used to
/// duplicate both.** `load_catalog` is a pure parse of an `include_str!`
/// constant — no `App`, no plugin order, no asset load — so there is no ordering
/// reason for the Rust side to carry its own copy, which is the objection that
/// kept AF4b open. The sheet comes through
/// [`CatalogEntry::manifest_target`](ambition_characters::actor::character_catalog::CatalogEntry::manifest_target),
/// the same canonical projection `audit_character_authority_parity` compares
/// with — a catalog row names FILES (`sprites/player_robot_v2_spritesheet.ron`)
/// and a definition names a TARGET (`player_robot_v2`).
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
    // **Hand the body to the art, for whichever incarnation authored one.**
    //
    // Jon, on Mary-O and then again on v3: *"The box and the sprite seem to be
    // not independent of each other. Shouldn't the sprite sheet generator be
    // authoring the collision boxes for the characters?"* It should, and the
    // engine has offered `BodySource::SpriteAuthored { world_per_pixel }` since
    // §4.11 — every NPC and enemy derives its box from published sprite metrics
    // and Mary-O's three forms use this exact seam. The player robot used
    // neither: they kept the engine's default 30×48 constant while their sprite was
    // drawn through a hand-tuned `collision_scale`, and the two were never
    // reconciled. Measured 2026-08-03 with `scripts/show_sprite_gameplay_box.py`:
    // their collider ran **1.28× wider and 1.29× taller than the body inside it**,
    // its top edge 17 px above the tip of their antenna. That is the report.
    //
    // ⚠ **the SCALE is derived and the HEIGHT is the authored quantity**, the
    // same direction Mary-O's `MARY_O_STANDING_HEIGHT` takes and for the same
    // reason: the sheets are regenerated regularly, every regeneration
    // re-measures, and a scale pinned to today's pixel count silently changes
    // how tall they stand the first time a crop moves by a pixel. Levels are
    // authored against the standing height, so that is what must hold still.
    //
    // ⚠ **only an AUTHORED body qualifies**, which is why this can be a blanket
    // rule over the lineage instead of a per-version flag. `authored_body_pixel_size`
    // returns `None` for a sheet that merely MEASURED its alpha bbox, so v0 and
    // v2 — whose boxes are still raw silhouettes, arms and all — keep exactly
    // the path they have today and opt in when someone authors them. Absence is
    // the answer, not an omission to fix here.
    if let Some(body_px) = ambition_platformer2d::character_sprites::authored_body_pixel_size(sheet)
    {
        let world_per_pixel = ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT / body_px.y;
        definition = definition
            .with_sprite_authored_body(world_per_pixel)
            .with_hurtboxes(forgiving_hurtbox(body_px * world_per_pixel));
    }
    // ⭐⭐ **THE CURRENT INCARNATION CARRIES THE MOVES.** (Jon's redirect §15.)
    //
    // ⛔ **the protagonist's repertoire was Smash-only until now.** The eleven
    // authored timelines — jab, three tilts, three smashes, five aerials, with
    // landing lag and auto-cancel — lived in `ambition_demo_smash` attached to
    // shadow identities (`smash_duelist_a/b`) wearing Robot art, so the real
    // robot could not throw any of them and the demo was proving the
    // architecture on characters nobody plays.
    //
    // ⚠ **v3 only, and that is not a shortcut.** v0 and v2 are the bodies the
    // player USED to be — a lineage the game shows you rather than a roster it
    // seats — and giving a retired incarnation the current one's frame data
    // would be inventing content, not migrating it.
    //
    // ⚠ the ACTION SET is still the host's (`playable_kit: HostCode`): what the
    // robot may DO is progression-gated, and what its swings ARE is not. Those
    // are different questions and this answers only the second.
    // ⭐⭐ **THE CURRENT INCARNATION CARRIES THE MOVES** (Jon's redirect §15,
    // ledger D82). Eleven authored timelines — jab, three tilts, three smashes,
    // five aerials, with landing lag and auto-cancel — lived in
    // `ambition_demo_smash` attached to shadow identities wearing Robot art, so
    // the real robot could throw none of them.
    //
    // ⚠ **v3 only.** v0 and v2 are bodies the player USED to be — a lineage the
    // game shows you rather than a roster it seats — and giving a retired
    // incarnation the current one's frame data would be inventing content.
    //
    // ⚠ the ACTION SET is still the host's (`playable_kit: HostCode`): what the
    // robot may DO is progression-gated, and what its swings ARE is not.
    // ⭐⭐ **THE BODY EVERY INCARNATION SHARES**, migrated off the
    // `player_robot` ARCHETYPE row (2026-08-11). That row was eighty lines with
    // all three authorities fused into it — a body (health, top speed, gait,
    // contact damage, movement feel), a controller (aggro distances, the duelist
    // neutral game), and a placement policy (respawn) — which is exactly the
    // shape Jon's brief says must separate rather than migrate wholesale.
    //
    // ⚠ **the lineage shares one body**, so this is stated once rather than per
    // incarnation: v0, v2 and v3 are the same robot at three ages, and the
    // exhibition duel in the arena fields v2 against the PCA precisely because
    // it IS the player's body seen from outside.
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
        // The CONTROLLER half, by name — shared rather than inlined, because a
        // policy that only one character can use is a policy fused to a body all
        // over again.
        .with_autonomous_profile_named("robot_duelist")
        // ⭐⭐ **AND THE VERBS ITS BODY HAS** (Jon's redirect §18).
        //
        // ⛔ **the protagonist authored none**, so a match seating it took
        // the *migration bridge*: `seat_abilities` hands an unauthored
        // character the MODE's declared set verbatim, because almost nothing
        // in the repo states its own verbs and removing that row today would
        // strip the Smash cast bare. The bridge is documented as meant to
        // shrink, and it shrinks one character at a time — this is the
        // first, and it is the right first because it is the one body both
        // games are supposed to share.
        //
        // ⚠ **no behaviour change in Smash, by construction**: the stage
        // declares a subset of this, and `authored ∩ mask` is the mask. What
        // changes is WHY — the robot may shield because the robot can
        // shield, not because nobody asked it.
        //
        // ⚠ **`fly` IS one of the robot's verbs, and my first pass had this
        // wrong.** It reads like a dev toggle from the player's side and it is
        // not: the archetype row granted `can_fly` beside `is_aerial: false`,
        // with the reason written down — *"grounded-base hybrid, exactly like
        // the player: fights on the ground and takes to the air via the fly
        // toggle when it needs the vertical space."* The duel arena's exhibition
        // robot uses it, and a body that could not would be a different creature.
        //
        // ⛔ **`reset` stays out**, and that one really is a debug affordance:
        // authoring it would hand every game that seats the robot a way to
        // teleport home.
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
    // ⭐ **THE SIGNATURE PROJECTILE.** The robot fires a Hadouken, and that was a
    // fact only an enemy ARCHETYPE row could state (ledger D83) — so a
    // character-first robot fired an unadorned rock while the archetype road drew
    // the real thing.
    definition = definition.with_ranged_vfx("hadouken");
    // ⭐⭐ **AND IT CHARGES.** Hold to build, release to fire — the mechanic the
    // protagonist has always had, authored on the CHARACTER for the first time
    // (GPT 5.6 §4, 2026-08-11).
    //
    // ⛔ **this was a property of `PlayableKitSource::HostCode`**, which made
    // *delete HostCode* read as *delete the charge*. Jon's product rule is the
    // opposite: Player Robot v3 is the same character with the same repertoire in
    // Ambition and in Smash, and a mode changes interpretation and restrictions
    // rather than silently replacing its moves. Authoring it here is what lets
    // `HostCode` be deleted without the robot losing the Hadouken.
    definition = definition
        .with_ranged_execution(ambition_characters::brain::RangedExecution::ChargedProjectile);
    // ⭐ **THEOREM CHAIN, on the incarnation the duel fields.** v3 carries the
    // platform-fighter table instead; two incarnations of one robot with
    // different repertoires is what a lineage IS.
    if incarnation.id == V2.id {
        definition = definition.with_moveset(crate::player_robot_moveset::theorem_chain_moveset());
    }
    if incarnation.id == V3.id {
        definition = definition.with_moveset(crate::player_robot_moveset::player_robot_moveset());
        // ⭐⭐ **AND WHAT ACTIONS IT HAS**, not only what those actions ARE (GPT
        // 5.6 §5). The moveset says the swing's timeline; this says the robot has
        // a swing, a bolt and a bubble shield at all — the half that was
        // `default_player_action_set` in host code, keyed off
        // `playable_kit: HostCode` in the catalog row.
        //
        // ⚠ authoring it is what makes this character `PreparedKit::Authored`,
        // and §4's `ranged_execution` is why that no longer costs it the charge.
        definition =
            definition.with_action_set(crate::player_robot_moveset::player_robot_action_set());
    }
    definition.lineage = Some(Lineage {
        derived_from: incarnation.replaces.map(str::to_string),
        // Left `None` deliberately. These are hand-authored incarnations, not
        // the output of a crossover generator, so there is no revision or source
        // fingerprint to state — and inventing one would make provenance that
        // cannot be traced look like provenance that can.
        generator_revision: None,
        source_fingerprint: None,
    });
    definition
}

/// **Being hit is judged on their torso, not on their outline.**
///
/// Jon: *"It should be under the main head, and well within the player arms.
/// The player hitbox needs to be very forgiving to the player."*
///
/// ⭐ **that sentence describes TWO boxes, which is why it read as
/// contradictory.** The collision box has to keep their head, or a robot whose
/// head is nearly half their height walks it through every ceiling; the hurtbox is
/// the one that can stop under it. They were one rectangle until this, so
/// "forgiving" had nowhere to live — the `HurtboxDoc` seam has existed since A7
/// and the protagonist authored nothing, so their hurtbox fell back to the coarse
/// body AABB.
///
/// The insets are FRACTIONS of the sheet's authored body box, not pixels, for
/// the same reason `body_inset` is fractional: they survive a regeneration that
/// re-crops them. What they were measured against, in their 224 px idle frame:
///
/// | | |
/// |---|---|
/// | antenna | `y 59..67`, `x 80..92` — a 13 px stalk |
/// | head | `y 68..104`, out to `x 148` at its widest |
/// | shoulders/neck | `y 104..110`, narrowing to `x 95..145` |
/// | torso and arms | `y 110..140`, `x 83..138` |
/// | legs and feet | `y 140..157`, `x 87..136` |
///
/// So `top` clears the head and lands on the shoulder line, `bottom` keeps the
/// shoe line the box already stood on, and the sides come in inside the arm
/// span — further on the right because their head, and only their head, is drawn
/// off-centre that way.
fn forgiving_hurtbox(body_world: ambition_platformer2d_core::Vec2) -> HurtboxDoc {
    // Fractions of the authored body box, per edge.
    const LEFT: f32 = 0.09;
    const RIGHT: f32 = 0.21;
    const TOP: f32 = 0.43;
    const BOTTOM: f32 = 0.01;

    // ⚠ **+y is DOWN** — `DEFAULT_GRAVITY_DIR` is `(0, 1)`, and sheet pixel
    // space and world space share that handedness. A box that sits low on the
    // body therefore takes a POSITIVE y offset; the opposite sign would put their
    // hurtbox in the air above their head and nothing would ever hit them.
    let offset = ambition_platformer2d_core::Vec2::new(
        ((LEFT + (1.0 - RIGHT)) * 0.5 - 0.5) * body_world.x,
        ((TOP + (1.0 - BOTTOM)) * 0.5 - 0.5) * body_world.y,
    );
    let half_extents = ambition_platformer2d_core::Vec2::new(
        (1.0 - LEFT - RIGHT) * 0.5 * body_world.x,
        (1.0 - TOP - BOTTOM) * 0.5 * body_world.y,
    );
    HurtboxDoc {
        // One timeline, no poses and no moves. The duelists vary theirs by pose
        // because their archetypes trade reach against exposure; nothing about
        // the protagonist's body changes shape, and authoring a pose entry that
        // restates the default is a second place for the number to drift.
        default: Some(HurtboxTimeline {
            keyframes: vec![HurtboxKeyframe {
                at_s: 0.0,
                volumes: vec![HurtboxVolume {
                    shape: VolumeShape::Rect {
                        offset: (offset.x, offset.y),
                        half_extents: (half_extents.x, half_extents.y),
                    },
                }],
            }],
        }),
        poses: Default::default(),
        moves: Default::default(),
    }
}

/// Register every incarnation as a character in its own right.
///
/// The KIT is deliberately not authored here: each incarnation's catalog row
/// already states what it can do, and preparation folds that row in at the
/// finalization barrier. Authoring it a second time on the definition would be
/// two declarations of one fact — exactly the split the character-authority
/// campaign exists to remove.
pub fn register(app: &mut bevy::prelude::App) {
    // Parsed ONCE for the whole lineage. Three strings do not justify three
    // parses of the roster, and the cast is only going to grow.
    let catalog = crate::character_catalog::load_catalog();
    for incarnation in LINEAGE {
        app.try_register_character(
            definition_from(&catalog, incarnation),
            // The engine's sheet vocabulary, so a target that names nothing is
            // reported at load with a did-you-mean instead of silently drawing
            // the marked rectangle.
            CharacterBindings::default().with_engine_sheet_vocabulary(),
        )
        .unwrap_or_else(|error| panic!("player-robot incarnation rejected: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The chain is well-formed, and it is a chain.**
    ///
    /// Exactly one origin, every other link naming the incarnation before it,
    /// and no id repeated. A lineage that forked or looped would still compile
    /// and would quietly make "the version before this one" unanswerable.
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

    /// **v3 stands as tall as the level expects, and their box is their ART.**
    ///
    /// Jon: *"The current player V3 collision / hurt box is larger than the
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
        use ambition_platformer2d_actor_monolith::character_runtime::BodySource;

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

    /// **The forgiving hurtbox is strictly inside the box that carries it.**
    ///
    /// Jon asked for a hitbox that is *"very forgiving to the player"* — under
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

    /// **v0 and v2 keep the path they have**, and the reason is a fact about
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

    /// Every incarnation's art resolves, and to a DIFFERENT sheet.
    ///
    /// ⚠ the second half is the one worth having. Eighteen shipped sheets
    /// declare `target: "robot"` — the name of the procedural generator, not of
    /// a character — so "the target resolves" is satisfied by all three
    /// resolving to the same robot. Distinctness is what says three incarnations
    /// actually look like three characters.
    ///
    /// ⚠ **it asks the DEFINITION now, not the struct.** The sheet used to be a
    /// `&'static str` on `Incarnation`, so this test read a Rust literal and
    /// would have stayed green while the catalog row — which is what the art
    /// pipeline actually resolves — said something else entirely. That is the
    /// same mistake `every_incarnation_says_something` had to be rewritten out
    /// of on the voice field the same day.
    #[test]
    fn every_incarnation_resolves_its_own_distinct_sheet() {
        use ambition_sprite_sheet::character::sheets;

        let mut seen: Vec<String> = Vec::new();
        for incarnation in LINEAGE {
            let sheet = definition(incarnation)
                .sheet
                .expect("the lineage always names a sheet target");
            assert!(
                sheets::record_for_target(&sheet).is_some(),
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

    /// **The name comes from the row, and there is only one row.** (AF4b)
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

    /// **Nobody in the lineage stands mute — asked of the RUNTIME, not the
    /// struct.** (AF4b)
    ///
    /// This used to assert `!definition.voice.is_empty()`, which is a fact about
    /// a Rust literal and not about what anybody hears. It was green while
    /// `player_robot_v2`'s lines were unreachable: the catalog outranks a
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

/// **Every character this provider DECLARES, registered so it can be BUILT.**
///
/// ⛔ **a catalog row is not a registration, and the difference had never been
/// visible.** The row says what a character IS; `register_character` is what
/// puts it in `PreparedCharacterRegistry`, which is the population match
/// preparation can construct a body from. Ambition's Hall cast was catalog-only,
/// so the crossover grid OFFERED eight fighters this host could not seat.
///
/// ⛔ **and the old adoption path HID it.** A human seat took over the session's
/// existing body, and that branch consulted the registry optionally — so a
/// catalog-only character worked in seat 0 and nowhere else. Eight of the twelve
/// grid fighters were playable only as player one, and picking one for anybody
/// else deadlocked the whole match in silence (Jon, 2026-08-06). Building every
/// seat the same way is what turned that asymmetry into a single question, and
/// this is the answer to it.
///
/// ⚠ **projected from the rows, never re-authored.** Display name and sheet come
/// from the catalog exactly as [`definition_from`] takes them, because naming
/// them again in Rust is the second source of truth the character-authority
/// campaign exists to remove.
///
/// ⚠ **a BLANKET rule over the provider's own catalog, not a list of eight.**
/// A hand-kept list is the shape this repo has been bitten by five times — the
/// pairing that drifts because nothing checks it. "A character we declare is a
/// character we can build" needs no maintenance.
pub fn register_declared_cast(app: &mut bevy::prelude::App) {
    let catalog = crate::character_catalog::load_catalog();
    // The lineage registers itself above with authored bodies and hurtboxes;
    // re-registering here would be a duplicate and would also throw those away.
    let lineage: std::collections::BTreeSet<&str> =
        LINEAGE.iter().map(|incarnation| incarnation.id).collect();
    // ⛔ **THE BUILDABLE CAST, not every catalog row.** Registering the whole
    // catalog was tried and is measurably wrong — see the note on
    // `PLAYABLE_ROSTER`, where the population is declared and the measurement
    // recorded. In one line: a bare registration says "this character authors no
    // body", preparation correctly retracts what a persona does not author, and
    // ~100 exploration NPCs lost their archetype-built vitals.
    //
    // ⭐ **and it is no longer the SELECTION list** (D73 phase 2): what a game
    // can BUILD and what it OFFERS on a character-select grid are two questions,
    // and `buildable_cast()` is the union that answers the first. Empty
    // build-only list today, so this iterates exactly what it always did.
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
        // **What this character says about its own body**, for the ones that
        // have taken their facts back from the archetype roster. A character
        // still awaiting migration adds nothing here and stays a bare
        // registration.
        let definition = crate::character_catalog::authored_intrinsics(&id, definition);
        // `try_`, and a SKIP rather than a panic: another provider legitimately
        // owns some of these ids in a multi-game composition, and losing a race
        // for one is not this provider's error to raise.
        let _ = app.try_register_character(
            definition,
            CharacterBindings::default().with_engine_sheet_vocabulary(),
        );
    }
}
