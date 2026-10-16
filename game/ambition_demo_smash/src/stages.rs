//! The `smash_stages` schema: Smash's stages, as data in its pack
//! (`assets/data/smash_stages.ron`).
//!
//! The pack states the room every stage shares (its size, the place a lone
//! visitor lands and the blast margins) and the blocks of each stage. A stage
//! is a game's content, so the demo owns the schema and no engine capability
//! reads it. The select screen offers the stages `offered` names
//! ([`SmashStageChoice`](crate::SmashStageChoice)), so no Rust names a stage.

use std::collections::BTreeMap;

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use ambition_platformer2d::engine_core::{AuthoredBlock, BlockKind};
use serde::Deserialize;

/// The authored FILE kind: Smash's stages.
pub const SMASH_STAGES_SCHEMA: &str = "smash_stages";

/// Smash's stages, as its pack states them.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmashStages {
    /// The room around each stage, (width, height).
    pub size: (f32, f32),
    /// Where a lone visitor lands. Seating places the fighters.
    pub spawn: (f32, f32),
    /// How far beyond the room each blast line is.
    pub blast: BlastMargins,
    /// Where and how a knocked-out fighter comes back.
    pub respawn: Respawn,
    /// The room ids the select screen offers, in its cycle order. The first
    /// is the default.
    pub offered: Vec<String>,
    /// Each stage by room id.
    pub stages: BTreeMap<String, StageRoom>,
}

/// How far beyond the room each blast line is, in world pixels.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlastMargins {
    pub fall: f32,
    pub side: f32,
    pub rise: f32,
}

/// Where and how a knocked-out fighter comes back, on every stage.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Respawn {
    /// How far above the stage centre the fighter appears, in world pixels.
    pub rise: f32,
    /// How long the returned fighter cannot be hit, in seconds.
    pub protection_seconds: f32,
    /// How far apart two seats' respawn points are, in world pixels.
    pub seat_spacing: f32,
    /// The size (width, height) of the platform a returning fighter stands on.
    pub platform: (f32, f32),
}

/// One stage: its name, its button label and its blocks.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRoom {
    pub name: String,
    /// What the select screen's stage button shows.
    pub label: String,
    /// The first block is the main platform.
    pub blocks: Vec<AuthoredBlock>,
}

impl SmashStages {
    /// Each value the stages cannot be built from, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let positive = |value: f32| value.is_finite() && value > 0.0;
        if !positive(self.size.0) || !positive(self.size.1) {
            problems.push(format!("`size` must be finite and positive, not {:?}", self.size));
        }
        if !self.spawn.0.is_finite() || !self.spawn.1.is_finite() {
            problems.push(format!("`spawn` must be finite, not {:?}", self.spawn));
        }
        for (field, margin) in [("fall", self.blast.fall), ("side", self.blast.side), ("rise", self.blast.rise)] {
            if !positive(margin) {
                problems.push(format!("`blast.{field}` must be finite and positive, not {margin}"));
            }
        }
        for (field, value) in [("rise", self.respawn.rise), ("protection_seconds", self.respawn.protection_seconds)] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`respawn.{field}` must be finite and not negative, not {value}"));
            }
        }
        for (field, value) in [
            ("seat_spacing", self.respawn.seat_spacing),
            ("platform.0", self.respawn.platform.0),
            ("platform.1", self.respawn.platform.1),
        ] {
            if !positive(value) {
                problems.push(format!("`respawn.{field}` must be finite and positive, not {value}"));
            }
        }
        // Two fighters that return on one frame stand side by side, so the
        // seats are more than a standing body apart.
        let body = ambition_platformer2d::engine_core::DEFAULT_PLAYER_BODY_WIDTH;
        if self.respawn.seat_spacing <= body {
            problems.push(format!(
                "`respawn.seat_spacing` is {}; two fighters that return on one frame would overlap, so it is more than a standing body ({body} px)",
                self.respawn.seat_spacing
            ));
        }
        if self.offered.is_empty() {
            problems.push("`offered` is empty; the select screen offers at least one stage".to_string());
        }
        let mut labels = std::collections::BTreeSet::new();
        for (index, id) in self.offered.iter().enumerate() {
            if self.offered[..index].contains(id) {
                problems.push(format!("`offered[{index}]` repeats `{id}`; the stage button would visit it twice"));
            }
            match self.stages.get(id) {
                None => problems.push(format!("`offered[{index}]` is `{id}`, and `stages` has no `{id}`")),
                // `ladder_rig --stage <label>` finds a stage by its label.
                Some(stage) if !labels.insert(stage.label.to_ascii_lowercase()) => problems.push(format!(
                    "`stages.{id}.label` is `{}`, which another offered stage shows",
                    stage.label
                )),
                Some(_) => {}
            }
        }
        for (id, stage) in &self.stages {
            match stage.blocks.first() {
                None => problems.push(format!("`stages.{id}` has no blocks")),
                Some(main) if main.kind != BlockKind::Solid => {
                    problems.push(format!("`stages.{id}.blocks[0]` is the main platform and must be `Solid`"));
                }
                Some(_) => {}
            }
            for (index, block) in stage.blocks.iter().enumerate() {
                for problem in block.problems() {
                    problems.push(format!("`stages.{id}.blocks[{index}]` (`{}`) {problem}", block.name));
                }
            }
        }
        problems
    }
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn smash_stages_schema() -> SchemaRegistration {
    rules_schema(
        SMASH_STAGES_SCHEMA,
        1,
        "smash_demo",
        "Smash's stages: the room every stage shares, its blast margins and \
         each stage's blocks.",
        SmashStages::problems,
    )
}

/// The stages, as the demo's pack states them (`assets/data/smash_stages.ron`).
pub fn smash_stages() -> &'static SmashStages {
    lowered_rules::<SmashStages>(crate::smash_pack::PACK.prepared(), SMASH_STAGES_SCHEMA)
        .expect("the demo's pack states its stages (`data/smash_stages.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo plays its pack's `smash_stages.ron`: an edit to the file is
    /// the stage the next start builds, and a file the select screen cannot
    /// play is refused at admission with the file and the field named.
    #[test]
    fn smashs_stages_are_its_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/smash_stages.ron"))
            .expect("the demo's stages file is readable");
        let root = std::env::temp_dir().join(format!("smash_stages_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "smash_stages_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/smash_stages.ron", schema: "{SMASH_STAGES_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let compile = |text: &str| {
            std::fs::write(root.join("data/smash_stages.ron"), text).unwrap();
            EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![smash_stages_schema()])
                .compile()
        };
        let stages_of = |text: &str| {
            let prepared = compile(text).unwrap_or_else(|failure| panic!("{}", failure.render()));
            lowered_rules::<SmashStages>(&prepared, SMASH_STAGES_SCHEMA).expect("the probe pack lowers the stages").clone()
        };

        let file = stages_of(&authored);
        assert_eq!(&file, smash_stages(), "the demo plays the file");
        let blocks = |room: ambition_platformer2d::world::rooms::RoomSpec| {
            room.world.blocks.iter().map(|block| (block.name.clone(), block.aabb.min, block.aabb.max)).collect::<Vec<_>>()
        };
        assert_eq!(blocks(crate::stage_room(&file, crate::SMASH_STAGE_ROOM_ID)), blocks(crate::smash_stage()));

        let narrow = r#"(name: "smash_platform", kind: Solid, at: (160.0, 300.0), size: (320.0, 32.0)),"#;
        assert_eq!(authored.matches(narrow).count(), 1, "the probe edits one block");
        let wider = stages_of(&authored.replace(narrow, r#"(name: "smash_platform", kind: Solid, at: (144.0, 300.0), size: (352.0, 32.0)),"#));
        let room = crate::stage_room(&wider, crate::SMASH_NARROW_STAGE_ROOM_ID);
        let platform = room.world.blocks[0].aabb;
        assert_eq!(platform.max.x - platform.min.x, 352.0, "the edited block is the stage the next start builds");

        let entry = r#""smash_narrow_stage": ("#;
        assert_eq!(authored.matches(entry).count(), 1, "the probe renames one stage");
        let refused = match compile(&authored.replace(entry, r#""smash_narrower_stage": ("#)) {
            Ok(_) => panic!("a file without a stage the select screen offers was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("data/smash_stages.ron"), "the refusal names the file: {refused}");
        assert!(refused.contains("`stages` has no `smash_narrow_stage`"), "the refusal names the field: {refused}");

        // A new stage is a row and its id in `offered`: it is in the select
        // screen's cycle with its own label, and no Rust names it.
        let offered = r#"offered: ["smash_stage", "smash_platform_stage", "smash_narrow_stage"],"#;
        assert_eq!(authored.matches(offered).count(), 1, "the probe edits the offered list");
        // One literal on each line: the repository's line scanners strip a
        // literal only when it closes on its own line.
        let probe = [
            r#"    stages: {"#,
            r#"        "probe_stage": (name: "Probe", label: "Probe", blocks: ["#,
            r#"            (name: "probe_platform", kind: Solid, at: (80.0, 300.0), size: (480.0, 32.0)),"#,
            r#"        ]),"#,
        ]
        .join("\n");
        let fourth = authored
            .replace(offered, r#"offered: ["smash_stage", "smash_platform_stage", "smash_narrow_stage", "probe_stage"],"#)
            .replacen("    stages: {", &probe, 1);
        let more = stages_of(&fourth);
        assert_eq!(more.offered.last().map(String::as_str), Some("probe_stage"));
        assert_eq!(more.stages["probe_stage"].label, "Probe");
        assert_eq!(crate::stage_room(&more, "probe_stage").world.blocks[0].name, "probe_platform");
        // Two offered stages that show one label are one `--stage` name.
        let twins = authored.replace(r#"label: "Narrow","#, r#"label: "Flat","#);
        let refused = match compile(&twins) {
            Ok(_) => panic!("two offered stages with one label were admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`stages.smash_narrow_stage.label` is `Flat`"), "the refusal names the field: {refused}");

        // The respawn is the file's too: an edit is the next start's rise and
        // protection, and a negative clock is refused with its field named.
        let respawn = "respawn: (rise: 160.0, protection_seconds: 2.0, seat_spacing: 64.0, platform: (96.0, 12.0)),";
        assert_eq!(authored.matches(respawn).count(), 1, "the probe edits the respawn");
        let shipped = Respawn { rise: 160.0, protection_seconds: 2.0, seat_spacing: 64.0, platform: (96.0, 12.0) };
        assert_eq!(file.respawn, shipped);
        // The runtime places a returning fighter by the file's rise.
        let placed = crate::respawn_placement(bevy::math::Vec2::ZERO, 0);
        assert_eq!(placed.y, -file.respawn.rise, "the respawn point is the file's rise above the centre");
        // Seat 1 is half a spacing right of the centre, and its platform is
        // the file's size.
        assert_eq!(crate::respawn_placement(bevy::math::Vec2::ZERO, 1).x, file.respawn.seat_spacing * 0.5);
        assert_eq!(crate::respawn_platform_size(), bevy::math::Vec2::new(96.0, 12.0));
        let edited = stages_of(&authored.replace(
            respawn,
            "respawn: (rise: 96.0, protection_seconds: 3.5, seat_spacing: 80.0, platform: (128.0, 10.0)),",
        ));
        assert_eq!(
            edited.respawn,
            Respawn { rise: 96.0, protection_seconds: 3.5, seat_spacing: 80.0, platform: (128.0, 10.0) }
        );
        for (bad, field) in [
            ("respawn: (rise: 160.0, protection_seconds: -1.0, seat_spacing: 64.0, platform: (96.0, 12.0)),", "`respawn.protection_seconds`"),
            ("respawn: (rise: 160.0, protection_seconds: 2.0, seat_spacing: 64.0, platform: (96.0, 0.0)),", "`respawn.platform.1`"),
            ("respawn: (rise: 160.0, protection_seconds: 2.0, seat_spacing: 20.0, platform: (96.0, 12.0)),", "`respawn.seat_spacing`"),
        ] {
            let refused = match compile(&authored.replace(respawn, bad)) {
                Ok(_) => panic!("{field}: the respawn was admitted"),
                Err(failure) => failure.render(),
            };
            assert!(refused.contains(field), "the refusal names the field {field}: {refused}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
