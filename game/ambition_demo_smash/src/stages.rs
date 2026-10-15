//! The `smash_stages` schema: Smash's stages, as data in its pack
//! (`assets/data/smash_stages.ron`).
//!
//! The pack states the room every stage shares (its size, the place a lone
//! visitor lands and the blast margins) and the blocks of each stage. A stage
//! is a game's content, so the demo owns the schema and no engine capability
//! reads it. The Rust names only which stages the select screen offers
//! ([`SmashStageChoice`](crate::SmashStageChoice)).

use std::collections::BTreeMap;

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
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
}

/// One stage: its name and its blocks.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRoom {
    pub name: String,
    pub blocks: Vec<StageBlock>,
}

/// One block of a stage. The first block is the main platform.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageBlock {
    pub name: String,
    pub kind: StageBlockKind,
    /// The top-left corner (y grows down).
    pub at: (f32, f32),
    /// (width, height).
    pub size: (f32, f32),
}

/// What a body does when it meets a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum StageBlockKind {
    /// Stops a body from every side.
    Solid,
    /// Holds a body that lands from above; a body can drop through it.
    OneWay,
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
        for id in crate::SmashStageChoice::ALL.map(crate::SmashStageChoice::room_id) {
            if !self.stages.contains_key(id) {
                problems.push(format!("`stages` has no `{id}`, which the select screen offers"));
            }
        }
        for (id, stage) in &self.stages {
            match stage.blocks.first() {
                None => problems.push(format!("`stages.{id}` has no blocks")),
                Some(main) if main.kind != StageBlockKind::Solid => {
                    problems.push(format!("`stages.{id}.blocks[0]` is the main platform and must be `Solid`"));
                }
                Some(_) => {}
            }
            for (index, block) in stage.blocks.iter().enumerate() {
                if !positive(block.size.0) || !positive(block.size.1) {
                    problems.push(format!(
                        "`stages.{id}.blocks[{index}]` (`{}`) `size` must be finite and positive, not {:?}",
                        block.name, block.size
                    ));
                }
                if !block.at.0.is_finite() || !block.at.1.is_finite() {
                    problems.push(format!(
                        "`stages.{id}.blocks[{index}]` (`{}`) `at` must be finite, not {:?}",
                        block.name, block.at
                    ));
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

        // The respawn is the file's too: an edit is the next start's rise and
        // protection, and a negative clock is refused with its field named.
        let respawn = "respawn: (rise: 160.0, protection_seconds: 2.0),";
        assert_eq!(authored.matches(respawn).count(), 1, "the probe edits the respawn");
        assert_eq!(file.respawn, Respawn { rise: 160.0, protection_seconds: 2.0 });
        // The runtime places a returning fighter by the file's rise.
        let placed = crate::respawn_placement(bevy::math::Vec2::ZERO, 0);
        assert_eq!(placed.y, -file.respawn.rise, "the respawn point is the file's rise above the centre");
        let edited = stages_of(&authored.replace(respawn, "respawn: (rise: 96.0, protection_seconds: 3.5),"));
        assert_eq!(edited.respawn, Respawn { rise: 96.0, protection_seconds: 3.5 });
        let refused = match compile(&authored.replace(respawn, "respawn: (rise: 160.0, protection_seconds: -1.0),")) {
            Ok(_) => panic!("a negative respawn protection was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`respawn.protection_seconds`"), "the refusal names the field: {refused}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
