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
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SanicRules {
    pub ring_monitor: RingMonitor,
    pub speed_shoes: SpeedShoes,
    pub ring_loss: RingLoss,
    pub act: Act,
    pub badnik: Badnik,
    pub stomp: Stomp,
    pub super_form: SuperForm,
    /// The spin dash: the rev, the launch and the roll.
    pub spin_dash: crate::ball_dash::BallDashTuning,
    /// The acts, in their order: the first is Act 1.
    pub acts: Vec<ActTitle>,
}

/// One act: its room and the name its title card shows.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActTitle {
    pub room: String,
    pub title: String,
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
    pub ring_size: f32,
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
    /// How far before a course's right edge its goal line sits, in px. Each
    /// act's goal is this far from its own end.
    pub goal_from_end: f32,
    /// How fast the course brakes him after the goal: the share of his speed
    /// taken off each second.
    pub outro_brake_per_second: f32,
    /// How tall the goal post stands, in world units.
    pub goal_post_height: f32,
}

/// When a badnik thinks, and how a stomp throws Sanic off it.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badnik {
    pub bounce_speed: f32,
}

/// The beat Sanic holds when he becomes super.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuperForm {
    /// How long the beat holds (wall-clock seconds).
    pub beat_seconds: f32,
    /// The clock scale the beat asks for while it runs. 1.0 asks for none.
    pub beat_clock_scale: f32,
}

/// What counts as landing on a badnik's head or a monitor's lid.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stomp {
    pub band: f32,
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
            ("ring_loss.ring_size", loss.ring_size),
            ("act.clear_dwell", act.clear_dwell),
            ("act.par_seconds", act.par_seconds),
            ("act.title_seconds", act.title_seconds),
            ("act.goal_from_end", act.goal_from_end),
            ("act.outro_brake_per_second", act.outro_brake_per_second),
            ("act.goal_post_height", act.goal_post_height),
            ("badnik.bounce_speed", self.badnik.bounce_speed),
            ("stomp.band", self.stomp.band),
            ("super_form.beat_seconds", self.super_form.beat_seconds),
            ("spin_dash.decay_per_s", self.spin_dash.decay_per_s),
            ("spin_dash.exit_speed", self.spin_dash.exit_speed),
            ("spin_dash.contact_grace_s", self.spin_dash.contact_grace_s),
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
        let scale = self.super_form.beat_clock_scale;
        if !(scale.is_finite() && scale > 0.0 && scale <= 1.0) {
            problems.push(format!("`super_form.beat_clock_scale` is {scale}; a beat slows the clock or leaves it, so it is more than 0 and at most 1"));
        }
        let dash = &self.spin_dash;
        for (field, value) in [
            ("spin_dash.rev_per_tap", dash.rev_per_tap),
            ("spin_dash.launch_speed", dash.launch_speed),
            ("spin_dash.ball_side", dash.ball_side),
        ] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and more than 0"));
            }
        }
        // A charge and a stick deflection are each a share of a whole.
        for (field, value) in [
            ("spin_dash.min_launch_charge", dash.min_launch_charge),
            ("spin_dash.crouch_threshold", dash.crouch_threshold),
            ("spin_dash.rev_tiers[0]", dash.rev_tiers[0]),
            ("spin_dash.rev_tiers[1]", dash.rev_tiers[1]),
        ] {
            if !(value.is_finite() && value > 0.0 && value <= 1.0) {
                problems.push(format!("`{field}` is {value}; it is more than 0 and at most 1"));
            }
        }
        if dash.rev_tiers[0] > dash.rev_tiers[1] {
            problems.push(format!(
                "`spin_dash.rev_tiers` is {:?}; the rev climbs, so the second tier starts at or before the third",
                dash.rev_tiers
            ));
        }
        if self.acts.is_empty() {
            problems.push("`acts` is empty; the demo has at least one act".to_string());
        }
        for (index, act) in self.acts.iter().enumerate() {
            if act.title.trim().is_empty() {
                problems.push(format!("`acts[{index}].title` is empty; a title card names its act"));
            }
            if self.acts[..index].iter().any(|before| before.room == act.room) {
                problems.push(format!("`acts[{index}].room` repeats `{}`; one room is one act", act.room));
            }
        }
        if loss.ring_size <= 0.0 {
            problems.push(format!("`ring_loss.ring_size` is {}; a ring has a size", loss.ring_size));
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
         scatter, the act's score and cards, the badnik's bounce, the spin dash, \
         and what counts as a landing on a badnik or a monitor.",
        SanicRules::problems,
    )
}

/// True when feet at `feet` land on a top at `top` (screen +y is down): within
/// `stomp.band` of it on either side. The badnik's head and the monitor's lid
/// use this one rule.
pub fn feet_on_top(feet: f32, top: f32) -> bool {
    let band = sanic_rules().stomp.band;
    feet >= top - band && feet <= top + band
}

/// The demo's tuning, as its pack states it (`assets/data/sanic_rules.ron`).
pub fn sanic_rules() -> &'static SanicRules {
    lowered_rules::<SanicRules>(crate::pack::PACK.prepared(), SANIC_RULES_SCHEMA)
        .expect("the demo's pack states its tuning (`data/sanic_rules.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A landing on a badnik or a monitor is the band the rules state, on both
    /// sides of the top: the badnik and the monitor ask `feet_on_top`, and no
    /// Rust names the band.
    #[test]
    fn a_landing_is_the_stomp_band_the_rules_state() {
        let band = sanic_rules().stomp.band;
        let top = 400.0;
        assert!(feet_on_top(top - band, top), "feet {band} px above the top are not a landing");
        assert!(feet_on_top(top + band, top), "feet {band} px into the top are not a landing");
        assert!(!feet_on_top(top - band - 0.5, top), "feet above the band ({band} px) are a landing");
        assert!(!feet_on_top(top + band + 0.5, top), "feet below the band ({band} px) are a landing");
    }

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
            lowered_rules::<SanicRules>(&prepared, SANIC_RULES_SCHEMA).expect("the probe pack lowers the tuning").clone()
        };

        let file = rules_of(&authored);
        assert_eq!(&file, sanic_rules(), "the demo plays the file");
        // The acts are the file's: a renamed act is the title its card shows.
        let title = r#"title: "COVE HIGHWAY""#;
        assert_eq!(authored.matches(title).count(), 1, "the probe renames one act");
        let renamed = rules_of(&authored.replace(title, r#"title: "COVE EXPRESSWAY""#));
        assert_eq!(renamed.acts[1].title, "COVE EXPRESSWAY");
        // The goal line is the file's distance from each course's end.
        let goal = "goal_from_end: 400.0,";
        assert_eq!(authored.matches(goal).count(), 1, "the probe moves the goal");
        let nearer = rules_of(&authored.replace(goal, "goal_from_end: 300.0,"));
        let course = crate::sanic_speedway().world;
        assert_eq!(crate::goal_x_with(&nearer.act, &course), course.size.x - 300.0, "the goal line is not the file's");
        // The score the demo pays is the file's par and bonuses.
        assert_eq!(
            crate::act_score(0.0, 3),
            (file.act.par_seconds * file.act.time_bonus_per_second as f32) as i32 + 3 * file.act.ring_bonus,
        );

        // The spin dash is the file's, and a value it cannot use is refused at
        // admission with its field named.
        let launch = "launch_speed: 1600.0,";
        assert_eq!(authored.matches(launch).count(), 1, "the probe edits one launch speed");
        assert_eq!(rules_of(&authored.replace(launch, "launch_speed: 1800.0,")).spin_dash.launch_speed, 1800.0);
        let side = "ball_side: 24.0,";
        assert_eq!(authored.matches(side).count(), 1, "the probe edits one ball side");
        std::fs::write(root.join("data/sanic_rules.ron"), authored.replace(side, "ball_side: 0.0,")).unwrap();
        let pack = EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
            .with_schemas(|| vec![sanic_rules_schema()]);
        let refusal = pack.compile().err().map(|failure| failure.render()).unwrap_or_default();
        assert!(refusal.contains("spin_dash.ball_side"), "a ball with no size is not refused by its field: {refusal}");

        let par = "par_seconds: 30.0,";
        assert_eq!(authored.matches(par).count(), 1, "the probe edits one line of the file");
        assert_eq!(rules_of(&authored.replace(par, "par_seconds: 45.0,")).act.par_seconds, 45.0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
