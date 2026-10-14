//! The `sanic_rules` schema: the demo's own tuning, as data in its pack
//! (`assets/data/sanic_rules.ron`).
//!
//! The monitors, the ring scatter, the act's score and the badnik's bounce
//! are this game's policy, so the demo owns the schema and no engine
//! capability reads it. The pack names it (`content_pack! { schemas: [..] }`),
//! and the pack's compile checks and lowers the file.

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind: the demo's tuning.
pub const SANIC_RULES_SCHEMA: &str = "sanic_rules";

/// The demo's tuning, as its pack states it.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SanicRules {
    pub ring_monitor: RingMonitor,
    pub speed_shoes: SpeedShoes,
    pub ring_loss: RingLoss,
    pub act: Act,
    pub badnik: Badnik,
}

/// What a ring monitor holds.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RingMonitor {
    pub rings: i32,
}

/// How long the speed shoes last and what they multiply.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedShoes {
    pub seconds: f32,
    pub top_speed_factor: f32,
    pub accel_factor: f32,
}

/// What a hit does to the rings Sanic carries.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RingLoss {
    pub invulnerable_seconds: f32,
    pub max_scattered: usize,
    pub burst_speed: f32,
    pub rings_per_shell: usize,
    pub inner_shell_scale: f32,
    pub gravity: f32,
    pub air_drag: f32,
    pub restitution: f32,
    pub lock_seconds: f32,
    pub life_seconds: f32,
}

/// The act's clock, score and cards.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Act {
    pub clear_dwell: f32,
    pub par_seconds: f32,
    pub time_bonus_per_second: i32,
    pub ring_bonus: i32,
    pub title_seconds: f32,
}

/// When a badnik thinks, and how a stomp throws Sanic off it.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badnik {
    pub wake_radius: f32,
    pub bounce_speed: f32,
}

impl SanicRules {
    /// Each value the demo cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut amount = |field: &str, value: f32| {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        };
        let shoes = &self.speed_shoes;
        let loss = &self.ring_loss;
        let act = &self.act;
        for (field, value) in [
            ("speed_shoes.seconds", shoes.seconds),
            ("speed_shoes.top_speed_factor", shoes.top_speed_factor),
            ("speed_shoes.accel_factor", shoes.accel_factor),
            ("ring_loss.invulnerable_seconds", loss.invulnerable_seconds),
            ("ring_loss.burst_speed", loss.burst_speed),
            ("ring_loss.inner_shell_scale", loss.inner_shell_scale),
            ("ring_loss.gravity", loss.gravity),
            ("ring_loss.air_drag", loss.air_drag),
            ("ring_loss.restitution", loss.restitution),
            ("ring_loss.lock_seconds", loss.lock_seconds),
            ("ring_loss.life_seconds", loss.life_seconds),
            ("act.clear_dwell", act.clear_dwell),
            ("act.par_seconds", act.par_seconds),
            ("act.title_seconds", act.title_seconds),
            ("badnik.wake_radius", self.badnik.wake_radius),
            ("badnik.bounce_speed", self.badnik.bounce_speed),
        ] {
            amount(field, value);
        }
        for (field, value) in [
            ("ring_monitor.rings", self.ring_monitor.rings),
            ("act.time_bonus_per_second", act.time_bonus_per_second),
            ("act.ring_bonus", act.ring_bonus),
        ] {
            if value < 0 {
                problems.push(format!("`{field}` is {value}; it is not negative"));
            }
        }
        if loss.rings_per_shell == 0 && loss.max_scattered > 0 {
            problems.push(
                "`ring_loss.rings_per_shell` is 0; a scatter of rings has at least one ring in a shell"
                    .to_string(),
            );
        }
        problems
    }
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn sanic_rules_schema() -> SchemaRegistration {
    rules_schema(
        SANIC_RULES_SCHEMA,
        1,
        "sanic_demo",
        "The Sanic demo's tuning: the ring monitor, the speed shoes, the ring \
         scatter, the act's score and cards, the badnik's wake radius and bounce.",
        SanicRules::problems,
    )
}

/// The demo's tuning, as its pack states it (`assets/data/sanic_rules.ron`).
pub fn sanic_rules() -> SanicRules {
    *lowered_rules::<SanicRules>(crate::pack::PACK.prepared(), SANIC_RULES_SCHEMA)
        .expect("the demo's pack states its tuning (`data/sanic_rules.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo plays its pack's `sanic_rules.ron`, and an edit to the file
    /// is the rule the next start plays: no Rust names these numbers.
    #[test]
    fn sanics_tuning_is_its_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/sanic_rules.ron"))
            .expect("the demo's tuning file is readable");
        let root = std::env::temp_dir().join(format!("sanic_rules_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "sanic_rules_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/sanic_rules.ron", schema: "{SANIC_RULES_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let rules_of = |text: &str| {
            std::fs::write(root.join("data/sanic_rules.ron"), text).unwrap();
            let pack = EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![sanic_rules_schema()]);
            let prepared = pack.compile().unwrap_or_else(|failure| panic!("{}", failure.render()));
            *lowered_rules::<SanicRules>(&prepared, SANIC_RULES_SCHEMA).expect("the probe pack lowers the tuning")
        };

        let file = rules_of(&authored);
        assert_eq!(file, sanic_rules(), "the demo plays the file");
        // The score the demo pays is the file's par and bonuses.
        assert_eq!(
            crate::act_score(0.0, 3),
            (file.act.par_seconds * file.act.time_bonus_per_second as f32) as i32 + 3 * file.act.ring_bonus,
        );

        let par = "par_seconds: 30.0,";
        assert_eq!(authored.matches(par).count(), 1, "the probe edits one line of the file");
        assert_eq!(rules_of(&authored.replace(par, "par_seconds: 45.0,")).act.par_seconds, 45.0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
