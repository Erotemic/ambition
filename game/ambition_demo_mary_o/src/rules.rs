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
    pub castle_end: CastleEnd,
    pub flag: Flag,
}

/// The end of a castle: the switch, the bridge, the boss in the lava, the
/// gate and the one she saves.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CastleEnd {
    pub press_speed: f32,
    pub gate_step_seconds: f32,
    pub rescue_line_seconds: f32,
    pub stand_before_him: f32,
    pub step_down_speed: f32,
    pub switch_score: u32,
    pub sink_speed: f32,
    pub splash_ticks: u64,
    pub sunk_under: f32,
    pub fallen_under: f32,
    pub boss_fall_limit_seconds: f32,
    pub beats: CastleBeats,
}

/// The seconds of each beat of the end of a castle that ends on time.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CastleBeats {
    pub pressed: f32,
    pub charges: f32,
    pub aftermath: f32,
    pub gate_opens: f32,
    /// The thanks beat is the rescue line and this much more.
    pub thanks_after_line: f32,
    pub victory: f32,
}

/// The flag at the end of a level.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flag {
    pub slide_speed: f32,
    pub walk_off_px: f32,
    pub walk_off_speed: f32,
    /// The score of a grab, by its height on the pole (0 at the base, 1 at
    /// the top): the first band whose `at` the grab reaches pays.
    pub score_bands: [FlagBand; 5],
}

/// One band of the flag's score ladder.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagBand {
    pub at: f32,
    pub score: u32,
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
            ("castle_end.press_speed", self.castle_end.press_speed),
            ("castle_end.gate_step_seconds", self.castle_end.gate_step_seconds),
            ("castle_end.rescue_line_seconds", self.castle_end.rescue_line_seconds),
            ("castle_end.stand_before_him", self.castle_end.stand_before_him),
            ("castle_end.step_down_speed", self.castle_end.step_down_speed),
            ("castle_end.sink_speed", self.castle_end.sink_speed),
            ("castle_end.sunk_under", self.castle_end.sunk_under),
            ("castle_end.fallen_under", self.castle_end.fallen_under),
            ("castle_end.boss_fall_limit_seconds", self.castle_end.boss_fall_limit_seconds),
            ("castle_end.beats.pressed", self.castle_end.beats.pressed),
            ("castle_end.beats.charges", self.castle_end.beats.charges),
            ("castle_end.beats.aftermath", self.castle_end.beats.aftermath),
            ("castle_end.beats.gate_opens", self.castle_end.beats.gate_opens),
            ("castle_end.beats.thanks_after_line", self.castle_end.beats.thanks_after_line),
            ("castle_end.beats.victory", self.castle_end.beats.victory),
            ("flag.slide_speed", self.flag.slide_speed),
            ("flag.walk_off_px", self.flag.walk_off_px),
            ("flag.walk_off_speed", self.flag.walk_off_speed),
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
        for (field, value) in [
            ("castle_end.press_speed", self.castle_end.press_speed),
            ("castle_end.gate_step_seconds", self.castle_end.gate_step_seconds),
            ("castle_end.step_down_speed", self.castle_end.step_down_speed),
            ("castle_end.sink_speed", self.castle_end.sink_speed),
            ("flag.slide_speed", self.flag.slide_speed),
            ("flag.walk_off_speed", self.flag.walk_off_speed),
        ] {
            if value == 0.0 {
                problems.push(format!("`{field}` is 0; the beat it moves would never end"));
            }
        }
        if self.castle_end.splash_ticks == 0 {
            problems.push("`castle_end.splash_ticks` is 0; it is the ticks from one splash to the next".to_string());
        }
        let bands = &self.flag.score_bands;
        if bands.iter().any(|band| !(0.0..=1.0).contains(&band.at))
            || bands.windows(2).any(|pair| pair[0].at <= pair[1].at || pair[0].score <= pair[1].score)
            || bands[bands.len() - 1].at != 0.0
        {
            problems.push(format!(
                "`flag.score_bands` is {bands:?}; each `at` is from 0 to 1, the bands go down the pole, a lower \
                 band pays less, and the last is at 0 so every grab pays"
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

        // The flag pays by the file's ladder: its top band at the top of the
        // pole, its bottom band at the base.
        assert_eq!(crate::flag::flag_score(1.0), file.flag.score_bands[0].score);
        assert_eq!(crate::flag::flag_score(0.0), file.flag.score_bands[4].score);
        let top = "(at: 0.9, score: 5000),";
        assert_eq!(authored.matches(top).count(), 1, "the probe edits one band of the ladder");
        assert_eq!(rules_of(&authored.replace(top, "(at: 0.9, score: 9000),")).flag.score_bands[0].score, 9000);

        let boxed = "boxed_seconds: 4.0,";
        assert_eq!(authored.matches(boxed).count(), 1, "the probe edits one line of the file");
        assert_eq!(rules_of(&authored.replace(boxed, "boxed_seconds: 6.0,")).shell.boxed_seconds, 6.0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
