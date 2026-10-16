//! The `versus_arena` schema: the versus stage's room, as data in its pack
//! (`assets/data/versus_arena.ron`).
//!
//! The room is this stage's content, so the app owns the schema and no
//! engine capability reads it. Its blocks are the engine's `AuthoredBlock`,
//! the shape each game that states a room as data writes.

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use ambition_platformer2d::engine_core::{AuthoredBlock, BlockKind};
use serde::Deserialize;

/// The authored FILE kind: the versus stage's room.
pub const VERSUS_ARENA_SCHEMA: &str = "versus_arena";

/// The versus stage's room, as its pack states it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersusArena {
    pub name: String,
    /// (width, height).
    pub size: (f32, f32),
    /// The centre of the main platform's top, a little above it. Seating
    /// places the fighters symmetrically about this point.
    pub spawn: (f32, f32),
    pub blast: ArenaBlast,
    /// The first block is the main platform.
    pub blocks: Vec<AuthoredBlock>,
}

/// How far beyond the room each blast line is, in px. A body that crosses
/// one has left the world, and the rounds score that as a defeat.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArenaBlast {
    pub fall: f32,
    pub side: f32,
    /// `None`: no line above the room.
    pub rise: Option<f32>,
}

impl VersusArena {
    /// Each value the room cannot be built from, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let positive = |value: f32| value.is_finite() && value > 0.0;
        if self.name.trim().is_empty() {
            problems.push("`name` is empty".to_string());
        }
        if !positive(self.size.0) || !positive(self.size.1) {
            problems.push(format!("`size` must be finite and positive, not {:?}", self.size));
        }
        if !self.spawn.0.is_finite() || !self.spawn.1.is_finite() {
            problems.push(format!("`spawn` must be finite, not {:?}", self.spawn));
        }
        let margins = [("fall", Some(self.blast.fall)), ("side", Some(self.blast.side)), ("rise", self.blast.rise)];
        for (field, margin) in margins {
            if let Some(margin) = margin.filter(|margin| !positive(*margin)) {
                problems.push(format!("`blast.{field}` must be finite and positive, not {margin}"));
            }
        }
        match self.blocks.first() {
            None => problems.push("`blocks` is empty; the fighters stand on the first block".to_string()),
            Some(main) if main.kind != BlockKind::Solid => {
                problems.push("`blocks[0]` is the main platform and must be `Solid`".to_string());
            }
            Some(_) => {}
        }
        for (index, block) in self.blocks.iter().enumerate() {
            for problem in block.problems() {
                problems.push(format!("`blocks[{index}]` (`{}`) {problem}", block.name));
            }
        }
        problems
    }

    /// The room's geometry.
    pub fn world(&self) -> ambition_platformer2d::engine_core::World {
        use ambition_platformer2d::engine_core::{Vec2, World};
        let mut world = World::new(
            self.name.clone(),
            Vec2::new(self.size.0, self.size.1),
            Vec2::new(self.spawn.0, self.spawn.1),
            self.blocks.iter().map(AuthoredBlock::block).collect(),
        );
        world.edges.fall = self.blast.fall;
        world.edges.side = Some(self.blast.side);
        world.edges.rise = self.blast.rise;
        world
    }
}

/// The schema, for the versus pack's `content_pack! { schemas: [..] }`.
pub fn versus_arena_schema() -> SchemaRegistration {
    rules_schema(
        VERSUS_ARENA_SCHEMA,
        1,
        super::super::versus::VERSUS_EXPERIENCE,
        "The versus stage's room: its size, the seating point, the blast lines \
         and the blocks.",
        VersusArena::problems,
    )
}

/// The room, as `pack` states it (`assets/data/versus_arena.ron`).
pub fn versus_arena_in(pack: &ambition_platformer2d::content::PreparedContentPack) -> Option<&VersusArena> {
    lowered_rules::<VersusArena>(pack, VERSUS_ARENA_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stage plays its pack's `versus_arena.ron`: an edit to a block is
    /// the room the next start builds, and a room the stage cannot use is
    /// refused at admission with the file and the field named.
    #[test]
    fn the_versus_room_is_its_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/versus_arena.ron"))
            .expect("the versus room file is readable");
        let root = std::env::temp_dir().join(format!("versus_arena_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "versus_arena_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/versus_arena.ron", schema: "{VERSUS_ARENA_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let compile = |text: &str| {
            std::fs::write(root.join("data/versus_arena.ron"), text).unwrap();
            EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![versus_arena_schema()])
                .compile()
        };
        let arena_of = |text: &str| {
            let prepared = compile(text).unwrap_or_else(|failure| panic!("{}", failure.render()));
            lowered_rules::<VersusArena>(&prepared, VERSUS_ARENA_SCHEMA).expect("the probe pack lowers the room").clone()
        };
        let blocks = |world: &ambition_platformer2d::engine_core::World| {
            world.blocks.iter().map(|block| (block.name.clone(), block.kind, block.aabb.min, block.aabb.max)).collect::<Vec<_>>()
        };

        let room_of = |text: &str| {
            let prepared = compile(text).unwrap_or_else(|failure| panic!("{}", failure.render()));
            crate::app::versus::versus_arena_of(&prepared)
        };

        // The shipped pack's room is the file's.
        let shipped = crate::app::versus::versus_arena_of(crate::app::versus_fighters::PACK.prepared());
        assert_eq!(blocks(&shipped.world), blocks(&room_of(&authored).world), "the stage's room is not the file's");
        // An edited ledge is the room the stage builds from that pack.
        let left = r#"(name: "arena_platform_left", kind: Solid, at: (96.0, 304.0), size: (128.0, 20.0)),"#;
        assert_eq!(authored.matches(left).count(), 1, "the probe edits one ledge");
        let wider = room_of(&authored.replace(left, r#"(name: "arena_platform_left", kind: OneWay, at: (80.0, 304.0), size: (160.0, 20.0)),"#));
        let ledge = &wider.world.blocks[1];
        assert_eq!((ledge.kind, ledge.aabb.min.x, ledge.aabb.max.x), (BlockKind::OneWay, 80.0, 240.0), "the stage did not build the edited ledge");
        assert_eq!(wider.world.edges.side, Some(arena_of(&authored).blast.side), "the side blast line is not the file's");

        let refused = match compile(&authored.replace(left, r#"(name: "arena_platform_left", kind: Solid, at: (96.0, 304.0), size: (0.0, 20.0)),"#)) {
            Ok(_) => panic!("a ledge with no width was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("data/versus_arena.ron"), "the refusal names the file: {refused}");
        assert!(refused.contains("`blocks[1]` (`arena_platform_left`) `size`"), "the refusal names the field: {refused}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
