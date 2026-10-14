//! The `mary_o_rules` schema: the demo's own tuning, as data in its pack
//! (`assets/data/mary_o_rules.ron`).
//!
//! The spark, the snake plant, the pipe, the snake's shell, the star, the
//! coin block, the bridge, the walk and the cosmic form's beats are this
//! game's policy, so the demo owns the schema and no engine capability reads
//! it. The pack names it (`content_pack! { schemas: [..] }`), and the pack's
//! compile checks and lowers the file.

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind: the demo's tuning.
pub const MARY_O_RULES_SCHEMA: &str = "mary_o_rules";

/// The demo's tuning, as its pack states it.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaryORules {
    pub spark: Spark,
    pub snake_plant: SnakePlant,
    pub pipe: Pipe,
    pub shell: Shell,
    pub star_seconds: f32,
    pub coins_per_block: i32,
    pub plank_ticks: u64,
    pub walk_throttle: f32,
    pub cosmic: Cosmic,
}

/// The spark the fire form throws.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spark {
    pub refire_seconds: f32,
    pub half_extent: f32,
    pub speed: f32,
    pub damage: i32,
    pub gravity: f32,
    pub bounces: u8,
    pub lifetime_seconds: f32,
}

/// The cycle of the snake plant in its pipe.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnakePlant {
    pub cycle_ticks: u64,
    pub travel_ticks: u64,
    pub out_ticks: u64,
    pub shy_reach: f32,
    pub harmless_below: f32,
}

/// A body's trip through a pipe.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pipe {
    pub swallow_seconds: f32,
    pub emerge_seconds: f32,
    pub center_seconds: f32,
    pub travel_tiles: f32,
}

/// The stomped snake and its shell.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shell {
    pub stomp_bounce_speed: f32,
    pub slide_speed: f32,
    pub enemy_damage: i32,
    pub player_damage: i32,
    pub kick_grace_seconds: f32,
    pub retreat_seconds: f32,
    pub boxed_seconds: f32,
    pub peek_seconds: f32,
    pub emerge_seconds: f32,
}

/// The beats of the cosmic form.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cosmic {
    pub leave_hold_seconds: f32,
    pub hit_stop_max_seconds: f32,
}

impl MaryORules {
    /// Each value the demo cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let (spark, plant, pipe, shell) = (&self.spark, &self.snake_plant, &self.pipe, &self.shell);
        for (field, value) in [
            ("spark.refire_seconds", spark.refire_seconds),
            ("spark.half_extent", spark.half_extent),
            ("spark.speed", spark.speed),
            ("spark.gravity", spark.gravity),
            ("spark.lifetime_seconds", spark.lifetime_seconds),
            ("snake_plant.shy_reach", plant.shy_reach),
            ("snake_plant.harmless_below", plant.harmless_below),
            ("pipe.swallow_seconds", pipe.swallow_seconds),
            ("pipe.emerge_seconds", pipe.emerge_seconds),
            ("pipe.center_seconds", pipe.center_seconds),
            ("pipe.travel_tiles", pipe.travel_tiles),
            ("shell.stomp_bounce_speed", shell.stomp_bounce_speed),
            ("shell.slide_speed", shell.slide_speed),
            ("shell.kick_grace_seconds", shell.kick_grace_seconds),
            ("shell.retreat_seconds", shell.retreat_seconds),
            ("shell.boxed_seconds", shell.boxed_seconds),
            ("shell.peek_seconds", shell.peek_seconds),
            ("shell.emerge_seconds", shell.emerge_seconds),
            ("star_seconds", self.star_seconds),
            ("walk_throttle", self.walk_throttle),
            ("cosmic.leave_hold_seconds", self.cosmic.leave_hold_seconds),
            ("cosmic.hit_stop_max_seconds", self.cosmic.hit_stop_max_seconds),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        }
        for (field, value) in [
            ("spark.damage", spark.damage),
            ("shell.enemy_damage", shell.enemy_damage),
            ("shell.player_damage", shell.player_damage),
            ("coins_per_block", self.coins_per_block),
        ] {
            if value < 0 {
                problems.push(format!("`{field}` is {value}; it is not negative"));
            }
        }
        if plant.travel_ticks == 0 {
            problems.push("`snake_plant.travel_ticks` is 0; the plant moves by one part in it each tick".to_string());
        }
        if plant.cycle_ticks < 2 * plant.travel_ticks + plant.out_ticks {
            problems.push(format!(
                "`snake_plant.cycle_ticks` is {}; a cycle holds the rise, the time out and the fall ({} ticks)",
                plant.cycle_ticks,
                2 * plant.travel_ticks + plant.out_ticks
            ));
        }
        if self.plank_ticks == 0 {
            problems.push("`plank_ticks` is 0; a bridge breaks one plank at a time".to_string());
        }
        problems
    }
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn mary_o_rules_schema() -> SchemaRegistration {
    rules_schema(
        MARY_O_RULES_SCHEMA,
        1,
        "mary_o_demo",
        "The Mary-O demo's tuning: the spark, the snake plant, the pipe, the \
         snake's shell, the star, the coin block, the bridge, the walk and the \
         cosmic form's beats.",
        MaryORules::problems,
    )
}

/// The demo's tuning, as its pack states it (`assets/data/mary_o_rules.ron`).
pub fn mary_o_rules() -> MaryORules {
    *lowered_rules::<MaryORules>(crate::pack::PACK.prepared(), MARY_O_RULES_SCHEMA)
        .expect("the demo's pack states its tuning (`data/mary_o_rules.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo plays its pack's `mary_o_rules.ron`, and an edit to the file
    /// is the rule the next start plays: no Rust names these numbers.
    #[test]
    fn mary_os_tuning_is_its_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/mary_o_rules.ron"))
            .expect("the demo's tuning file is readable");
        let root = std::env::temp_dir().join(format!("mary_o_rules_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "mary_o_rules_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/mary_o_rules.ron", schema: "{MARY_O_RULES_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let rules_of = |text: &str| {
            std::fs::write(root.join("data/mary_o_rules.ron"), text).unwrap();
            let pack = EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![mary_o_rules_schema()]);
            let prepared = pack.compile().unwrap_or_else(|failure| panic!("{}", failure.render()));
            *lowered_rules::<MaryORules>(&prepared, MARY_O_RULES_SCHEMA).expect("the probe pack lowers the tuning")
        };

        let file = rules_of(&authored);
        assert_eq!(file, mary_o_rules(), "the demo plays the file");
        // The plant rises over the file's travel ticks: half out halfway up,
        // all the way out at the end of the rise.
        let travel = file.snake_plant.travel_ticks;
        assert_eq!(crate::plant::wanted_out(travel / 2), (travel / 2) as f32 / travel as f32);
        assert_eq!(crate::plant::wanted_out(travel), 1.0);

        let boxed = "boxed_seconds: 4.0,";
        assert_eq!(authored.matches(boxed).count(), 1, "the probe edits one line of the file");
        assert_eq!(rules_of(&authored.replace(boxed, "boxed_seconds: 6.0,")).shell.boxed_seconds, 6.0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
