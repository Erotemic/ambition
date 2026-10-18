//! The `ambition_rules` schema: Ambition's own tuning, as data in its pack
//! (`assets/data/ambition_rules.ron`).

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const AMBITION_RULES_SCHEMA: &str = "ambition_rules";

/// Ambition's own tuning.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbitionRules {
    pub boss_idle_bark: IdleBark,
    pub gnu_ton_duel: GnuTonDuelRules,
    pub african_swallow: SwallowFetch,
    /// The thrown items that hurt the body they strike, by held-item id. An
    /// item with no row lands and hurts nobody.
    pub thrown_impacts: std::collections::BTreeMap<String, ThrownImpact>,
    pub day: Day,
    /// The character a player starts as: a row of the pack's catalog.
    pub starting_character: String,
    pub symmetry_attunement: SymmetryAttunement,
    /// How a quest's payout is shown.
    pub quest_payout: QuestPayout,
    /// Where a dropped portal gun lies, and when it can be taken again.
    pub portal_gun_drop: PortalGunDrop,
    pub cut_rope_victory: VictoryNpc,
    pub cut_rope_trap: CutRopeTrap,
    pub duel: Duel,
}

/// The trap over the Smirking Behemoth: what hangs from the rope, and the
/// show when it lands on him.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutRopeTrap {
    /// What hangs from the rope, in the order the room's replays take them.
    /// A session starts on the first.
    pub heavy_objects: Vec<HeavyObject>,
    /// The banner when it lands on him; `{object}` is the name of what fell.
    pub flattened_banner: String,
    /// How long that banner stays (seconds).
    pub banner_seconds: f32,
    /// Seconds between two sparks on the rope while it waits to be cut.
    pub rope_spark_seconds: f32,
}

/// One thing the rope can hold.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeavyObject {
    /// The prop sheet it is drawn with (`prop_sheets.ron`).
    pub prop: String,
    /// Its name: the banner says it, and the dialogue asks for it
    /// (`cut_rope_heavy_object_is("anvil")`).
    pub name: String,
}

impl CutRopeTrap {
    /// The heavy object the cycle's `index` names: the list wraps.
    pub fn heavy_object(&self, index: usize) -> &HeavyObject {
        &self.heavy_objects[index % self.heavy_objects.len()]
    }
}

/// The spectator duel: fighters that hold a grudge against each other. The
/// arena room stages them each time its contents are staged, and the
/// `<<duel>>` dialogue command stages them beside the player.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Duel {
    /// The level id of the arena room.
    pub room: String,
    /// The duel's centre in the arena: this far right of where the player
    /// arrives, in px, so the fight is on the screen at once.
    pub arena_offset_x: f32,
    /// The duel's centre for `<<duel>>`: this far in front of the player, in px.
    pub beside_player_x: f32,
    pub fighters: Vec<DuelFighter>,
}

/// One fighter of the spectator duel.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuelFighter {
    /// Its feature id, unique in the room.
    pub id: String,
    /// The character it is: a row of the pack's catalog. Its art, its
    /// hitbox and its shown name come from that row.
    pub character: String,
    /// The custom brain that decides its moves.
    pub brain: String,
    /// Its position from the duel's centre, and its half size, in px.
    pub offset: (f32, f32),
    pub half_size: (f32, f32),
    /// The id of the fighter it fights. The grudge, not a hostile faction,
    /// makes the pair fight, so neither aims at the player.
    pub grudge_against: String,
}

/// The NPC that comes out of the Smirking Behemoth when it dies.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VictoryNpc {
    /// The name it shows. The speaker of its dialogue is written in the
    /// Yarn file, so an edit here does not change that text.
    pub name: String,
    /// The dialogue node it starts when the player talks to it.
    pub dialogue: String,
    /// Its box, in px. Its feet stand on the bottom of the boss's box.
    pub width: f32,
    pub height: f32,
}

/// The Noether attunement: the signals that complete it, and its banner.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymmetryAttunement {
    /// The puzzle completes when each of these signals came in. The level's
    /// switches send them (`encounter.signal encounter:symmetry_attunement <signal>`).
    pub signals: Vec<String>,
    /// The banner when it completes, and how long it stays (seconds).
    pub banner: String,
    pub banner_seconds: f32,
}

/// How a quest's payout is shown. The banner's words are each quest's own
/// (`quests.ron`); how long it stays is one rule for all of them.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestPayout {
    /// How long the payout banner stays (seconds).
    pub banner_seconds: f32,
}

/// The pickup a player leaves when it drops the portal gun.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalGunDrop {
    /// How far ahead of the body, in the way it faces, the gun lies (px).
    pub ahead_px: f32,
    /// The half size of its pickup box (px).
    pub half_extent_px: f32,
    /// How long before it can be taken again (seconds), so the press that
    /// dropped it cannot take it back.
    pub arm_seconds: f32,
}

/// Ambition's day: how long it is, the hour each phase begins at, the hour a
/// session begins at, and the light over a sunlit room by the hour.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Day {
    /// Sim seconds in one whole day.
    pub length_seconds: f32,
    pub morning_at: f32,
    pub day_at: f32,
    pub evening_at: f32,
    pub night_at: f32,
    pub start_hour: f32,
    /// The veil over a sunlit room: (hour, (r, g, b, a)) in sRGB, in the
    /// order of the hours. Between two keys the veil blends.
    pub daylight: Vec<(f32, (f32, f32, f32, f32))>,
}

/// What one kind of thrown item does to a body it strikes.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrownImpact {
    /// At or above this speed (px/s) the item hurts what it strikes.
    pub min_speed: f32,
    pub damage: i32,
    /// The fraction of its speed the item keeps, reversed, when it strikes.
    pub rebound: f32,
    /// The most of `min_speed` the rebound may be. Under 1, so however fast
    /// the item arrived, it leaves under the bar and does not strike the same
    /// body on the next tick.
    pub rebound_cap: f32,
}

/// What an African swallow does with a coconut (`primeval`): it fetches one
/// at rest, carries it over its target, and lets it go there.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwallowFetch {
    /// It goes for a coconut at rest no farther from it than this (px).
    pub fetch_reach: f32,
    /// How fast it flies to a coconut (px/s).
    pub fetch_speed: f32,
    /// How fast it flies with one (px/s).
    pub laden_speed: f32,
    /// It carries the coconut this far over its target (px).
    pub carry_over: f32,
    /// It lets go within this of the place over its target, side to side (px).
    pub let_go_within: f32,
    /// It lets go no lower over its target than this (px).
    pub let_go_over: f32,
    /// It aims where its target will be after this long (s).
    pub lead_seconds: f32,
}

/// How often an active boss says an idle line.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdleBark {
    /// The mean time between two idle lines, in seconds.
    pub base_seconds: f32,
    /// Each interval is the mean plus or minus at most this, in seconds.
    pub jitter_seconds: f32,
}

/// How a move of the fighter `npc_gnu_ton` is made for the duel on the
/// giant's back (`bosses::gnu_ton::duel`).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GnuTonDuelRules {
    /// The table's damage at and over which a move is a heavy one.
    pub heavy_damage: i32,
    /// What a move under `heavy_damage` does in the main game.
    pub light: DuelStrike,
    /// What a move at or over `heavy_damage` does in the main game.
    pub heavy: DuelStrike,
    /// A move's reach is larger by this on the giant.
    pub reach: f32,
    /// A volume with its middle at or over this height (up is negative) is
    /// over his head; the others are put down at the player's height.
    pub overhead: f32,
    /// The height a volume that is not over his head is put at.
    pub at_the_player: f32,
    /// A volume that is put down is at least this tall, as a half height.
    pub least_half_height: f32,
    /// A volume over his head is moved up by this (its middle's height is
    /// multiplied), and its half height grows by `overhead_grow`.
    pub overhead_rise: f32,
    pub overhead_grow: f32,
    /// The reach of his book about his body, in the fighter's units: a move
    /// that calls the fist of his giant strikes here instead.
    pub book: DuelVolume,
}

/// A box about a body: its middle's offset and its half size.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuelVolume {
    pub offset: (f32, f32),
    pub half_extents: (f32, f32),
}

/// A duel move's effect in the main game.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuelStrike {
    /// Hearts of damage.
    pub hearts: i32,
    /// The fixed launch (a boss's strike is 1.25).
    pub launch: f32,
}

impl AmbitionRules {
    /// Each value the game cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (id, impact) in &self.thrown_impacts {
            if !(impact.min_speed.is_finite() && impact.min_speed > 0.0) {
                problems.push(format!(
                    "`thrown_impacts.{id}.min_speed` is {}; it is a finite speed more than 0",
                    impact.min_speed
                ));
            }
            if impact.damage < 1 {
                problems.push(format!("`thrown_impacts.{id}.damage` is {}; a row that hurts deals at least 1", impact.damage));
            }
            if !(impact.rebound.is_finite() && (0.0..=1.0).contains(&impact.rebound)) {
                problems.push(format!(
                    "`thrown_impacts.{id}.rebound` is {}; it is the fraction of its speed the item keeps, 0 to 1",
                    impact.rebound
                ));
            }
            if !(impact.rebound_cap.is_finite() && (0.0..1.0).contains(&impact.rebound_cap)) {
                problems.push(format!(
                    "`thrown_impacts.{id}.rebound_cap` is {}; at 1 or more the rebound can reach the bar and strike the same body again",
                    impact.rebound_cap
                ));
            }
        }
        let bark = self.boss_idle_bark;
        if !(bark.base_seconds.is_finite() && bark.jitter_seconds.is_finite() && bark.jitter_seconds >= 0.0) {
            problems.push(format!(
                "`boss_idle_bark` is {bark:?}; both are finite seconds and the jitter is not negative"
            ));
        } else if bark.base_seconds - bark.jitter_seconds <= 0.0 {
            problems.push(format!(
                "`boss_idle_bark.base_seconds` {} less `jitter_seconds` {} is not more than 0: \
                 an interval could be zero or negative, so a boss would bark each frame",
                bark.base_seconds, bark.jitter_seconds
            ));
        }
        let duel = self.gnu_ton_duel;
        if duel.heavy_damage < 1 {
            problems.push(format!("`gnu_ton_duel.heavy_damage` is {}; it is at least 1", duel.heavy_damage));
        }
        for (field, strike) in [("light", duel.light), ("heavy", duel.heavy)] {
            if strike.hearts < 0 || !(strike.launch.is_finite() && strike.launch >= 0.0) {
                problems.push(format!(
                    "`gnu_ton_duel.{field}` is {strike:?}; hearts are not negative and the launch is finite and not negative"
                ));
            }
        }
        for (field, value) in [
            ("reach", duel.reach),
            ("least_half_height", duel.least_half_height),
            ("overhead_rise", duel.overhead_rise),
            ("overhead_grow", duel.overhead_grow),
            ("book.half_extents.0", duel.book.half_extents.0),
            ("book.half_extents.1", duel.book.half_extents.1),
        ] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!("`gnu_ton_duel.{field}` is {value}; it is finite and more than 0"));
            }
        }
        if self.starting_character.trim().is_empty() {
            problems.push("`starting_character` is empty; a player starts as a character of the catalog".to_string());
        }
        let attunement = &self.symmetry_attunement;
        if attunement.signals.is_empty() {
            problems.push("`symmetry_attunement.signals` is empty; the puzzle would complete when it starts".to_string());
        }
        for (index, signal) in attunement.signals.iter().enumerate() {
            if signal.trim().is_empty() || signal.contains(char::is_whitespace) {
                problems.push(format!("`symmetry_attunement.signals[{index}]` is `{signal}`; a signal is one word"));
            }
            if attunement.signals[..index].contains(signal) {
                problems.push(format!("`symmetry_attunement.signals[{index}]` repeats `{signal}`"));
            }
        }
        let trap = &self.cut_rope_trap;
        if trap.heavy_objects.is_empty() {
            problems.push("`cut_rope_trap.heavy_objects` is empty; the rope holds something".to_string());
        }
        for (index, object) in trap.heavy_objects.iter().enumerate() {
            if object.prop.trim().is_empty() || object.name.trim().is_empty() {
                problems.push(format!("`cut_rope_trap.heavy_objects[{index}]` has no prop or no name"));
            }
            if trap.heavy_objects[..index].iter().any(|before| before.name == object.name) {
                problems.push(format!(
                    "`cut_rope_trap.heavy_objects[{index}].name` repeats `{}`; the dialogue tells them apart by name",
                    object.name
                ));
            }
        }
        for (field, value) in [("cut_rope_trap.banner_seconds", trap.banner_seconds), ("cut_rope_trap.rope_spark_seconds", trap.rope_spark_seconds)] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and more than 0"));
            }
        }
        let drop = self.portal_gun_drop;
        for (field, value) in [("portal_gun_drop.ahead_px", drop.ahead_px), ("portal_gun_drop.arm_seconds", drop.arm_seconds)] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        }
        if !(drop.half_extent_px.is_finite() && drop.half_extent_px > 0.0) {
            problems.push(format!("`portal_gun_drop.half_extent_px` is {}; a pickup has a size", drop.half_extent_px));
        }
        let payout = self.quest_payout.banner_seconds;
        if !(payout.is_finite() && payout > 0.0) {
            problems.push(format!("`quest_payout.banner_seconds` is {payout}; a banner stays a while"));
        }
        if !(attunement.banner_seconds.is_finite() && attunement.banner_seconds > 0.0) {
            problems.push(format!("`symmetry_attunement.banner_seconds` is {}; a banner stays a while", attunement.banner_seconds));
        }
        let victory = &self.cut_rope_victory;
        if victory.name.trim().is_empty() {
            problems.push("`cut_rope_victory.name` is empty; the NPC shows a name".to_string());
        }
        if victory.dialogue.trim().is_empty() || victory.dialogue.contains(char::is_whitespace) {
            problems.push(format!("`cut_rope_victory.dialogue` is `{}`; a dialogue node is one word", victory.dialogue));
        }
        for (field, value) in [("width", victory.width), ("height", victory.height)] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!("`cut_rope_victory.{field}` is {value}; it is finite and more than 0"));
            }
        }
        let spectator = &self.duel;
        if spectator.room.trim().is_empty() {
            problems.push("`duel.room` is empty; the duel is staged in a room".to_string());
        }
        for (field, value) in [("arena_offset_x", spectator.arena_offset_x), ("beside_player_x", spectator.beside_player_x)] {
            if !value.is_finite() {
                problems.push(format!("`duel.{field}` is {value}; it is finite"));
            }
        }
        if spectator.fighters.len() < 2 {
            problems.push(format!("`duel.fighters` has {} fighters; a duel has two or more", spectator.fighters.len()));
        }
        for (index, fighter) in spectator.fighters.iter().enumerate() {
            for (field, value) in [("id", &fighter.id), ("character", &fighter.character), ("brain", &fighter.brain)] {
                if value.trim().is_empty() {
                    problems.push(format!("`duel.fighters[{index}].{field}` is empty"));
                }
            }
            if spectator.fighters[..index].iter().any(|other| other.id == fighter.id) {
                problems.push(format!("`duel.fighters[{index}].id` repeats `{}`", fighter.id));
            }
            if fighter.grudge_against == fighter.id || !spectator.fighters.iter().any(|other| other.id == fighter.grudge_against) {
                problems.push(format!(
                    "`duel.fighters[{index}].grudge_against` is `{}`; it names another fighter of the duel",
                    fighter.grudge_against
                ));
            }
            if !(fighter.offset.0.is_finite() && fighter.offset.1.is_finite()) {
                problems.push(format!("`duel.fighters[{index}].offset` is {:?}; it is finite", fighter.offset));
            }
            let (w, h) = fighter.half_size;
            if !(w.is_finite() && w > 0.0 && h.is_finite() && h > 0.0) {
                problems.push(format!("`duel.fighters[{index}].half_size` is {:?}; each is finite and more than 0", fighter.half_size));
            }
        }
        let day = &self.day;
        if !(day.length_seconds.is_finite() && day.length_seconds > 0.0) {
            problems.push(format!("`day.length_seconds` is {}; a day lasts a while", day.length_seconds));
        }
        let hours = [
            ("morning_at", day.morning_at),
            ("day_at", day.day_at),
            ("evening_at", day.evening_at),
            ("night_at", day.night_at),
            ("start_hour", day.start_hour),
        ];
        for (field, hour) in hours {
            if !(hour.is_finite() && (0.0..24.0).contains(&hour)) {
                problems.push(format!("`day.{field}` is {hour}; it is an hour from 0 to under 24"));
            }
        }
        if !(day.morning_at < day.day_at && day.day_at < day.evening_at && day.evening_at < day.night_at) {
            problems.push(format!(
                "`day`'s phases begin at {}, {}, {} and {}; morning, day, evening and night begin in that order",
                day.morning_at, day.day_at, day.evening_at, day.night_at
            ));
        }
        for (index, (hour, (r, g, b, a))) in day.daylight.iter().enumerate() {
            if !(hour.is_finite() && (0.0..=24.0).contains(hour)) || index > 0 && *hour <= day.daylight[index - 1].0 {
                problems.push(format!("`day.daylight[{index}]` is at hour {hour}; the keys are hours from 0 to 24, each after the one before"));
            }
            if ![r, g, b, a].iter().all(|c| c.is_finite() && (0.0..=1.0).contains(*c)) {
                problems.push(format!("`day.daylight[{index}]`'s colour is {:?}; each part is from 0 to 1", (r, g, b, a)));
            }
        }
        if day.daylight.is_empty() {
            problems.push("`day.daylight` is empty; a sunlit room has a veil at every hour".to_string());
        }
        let bird = self.african_swallow;
        for (field, value) in [
            ("fetch_reach", bird.fetch_reach),
            ("fetch_speed", bird.fetch_speed),
            ("laden_speed", bird.laden_speed),
            ("carry_over", bird.carry_over),
            ("let_go_within", bird.let_go_within),
            ("let_go_over", bird.let_go_over),
            ("lead_seconds", bird.lead_seconds),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`african_swallow.{field}` is {value}; it is finite and not negative"));
            }
        }
        if bird.let_go_over > bird.carry_over {
            problems.push(format!(
                "`african_swallow.let_go_over` {} is more than `carry_over` {}: the bird carries the coconut \
                 lower than it may let go, so it never lets go",
                bird.let_go_over, bird.carry_over
            ));
        }
        for (field, value) in [
            ("overhead", duel.overhead),
            ("at_the_player", duel.at_the_player),
            ("book.offset.0", duel.book.offset.0),
            ("book.offset.1", duel.book.offset.1),
        ] {
            if !value.is_finite() {
                problems.push(format!("`gnu_ton_duel.{field}` is {value}; it is a finite height"));
            }
        }
        problems
    }
}

/// The schema.
pub fn ambition_rules_schema() -> SchemaRegistration {
    rules_schema::<AmbitionRules>(
        AMBITION_RULES_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's own tuning: the idle bark cadence of a boss, how the GNU-ton \
         duel makes a fighter's move for the giant's back, and how an African \
         swallow fetches and drops a coconut.",
        AmbitionRules::problems,
    )
}

/// The tuning the pack states, or `None` when it has no `ambition_rules`
/// source.
pub fn lowered_ambition_rules(pack: &PreparedContentPack) -> Option<AmbitionRules> {
    lowered_rules::<AmbitionRules>(pack, AMBITION_RULES_SCHEMA).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> AmbitionRules {
        AmbitionRules {
            boss_idle_bark: IdleBark { base_seconds: 7.0, jitter_seconds: 3.0 },
            gnu_ton_duel: GnuTonDuelRules {
                heavy_damage: 12,
                light: DuelStrike { hearts: 1, launch: 1.0 },
                heavy: DuelStrike { hearts: 2, launch: 1.7 },
                reach: 1.3,
                overhead: -30.0,
                at_the_player: 24.0,
                least_half_height: 30.0,
                overhead_rise: 1.6,
                overhead_grow: 1.5,
                book: DuelVolume { offset: (42.0, -4.0), half_extents: (30.0, 24.0) },
            },
            african_swallow: SwallowFetch {
                fetch_reach: 900.0,
                fetch_speed: 250.0,
                laden_speed: 185.0,
                carry_over: 150.0,
                let_go_within: 18.0,
                let_go_over: 90.0,
                lead_seconds: 0.35,
            },
            thrown_impacts: [(
                "coconut".to_string(),
                ThrownImpact { min_speed: 240.0, damage: 2, rebound: 0.3, rebound_cap: 0.8 },
            )]
            .into(),
            day: Day {
                length_seconds: 1440.0,
                morning_at: 5.0,
                day_at: 10.0,
                evening_at: 17.0,
                night_at: 21.0,
                start_hour: 8.0,
                daylight: vec![(0.0, (0.05, 0.08, 0.26, 0.86)), (10.0, (1.0, 1.0, 1.0, 0.0))],
            },
            starting_character: "player_robot_v3".to_string(),
            symmetry_attunement: SymmetryAttunement {
                signals: vec!["gravity_down".to_string()],
                banner: "done".to_string(),
                banner_seconds: 4.0,
            },
            quest_payout: QuestPayout { banner_seconds: 3.0 },
            portal_gun_drop: PortalGunDrop { ahead_px: 44.0, half_extent_px: 20.0, arm_seconds: 0.35 },
            cut_rope_trap: CutRopeTrap {
                heavy_objects: vec![HeavyObject { prop: "cut_rope_anvil".to_string(), name: "anvil".to_string() }],
                flattened_banner: "flattened by a {object}".to_string(),
                banner_seconds: 2.8,
                rope_spark_seconds: 0.22,
            },
            cut_rope_victory: VictoryNpc {
                name: "The Rope Appreciator".to_string(),
                dialogue: "smirking_behemoth_victory_npc".to_string(),
                width: 28.0,
                height: 48.0,
            },
            duel: Duel {
                room: "duel_arena".to_string(),
                arena_offset_x: 360.0,
                beside_player_x: 220.0,
                fighters: [("a", "b"), ("b", "a")]
                    .map(|(id, foe)| DuelFighter {
                        id: id.to_string(),
                        character: "fixture".to_string(),
                        brain: "fixture".to_string(),
                        offset: (0.0, 0.0),
                        half_size: (14.0, 23.0),
                        grudge_against: foe.to_string(),
                    })
                    .to_vec(),
            },
        }
    }

    #[test]
    fn a_value_the_game_cannot_use_is_named_by_its_field() {
        assert!(rules().problems().is_empty(), "{:?}", rules().problems());
        let mut barks_each_frame = rules();
        barks_each_frame.boss_idle_bark.jitter_seconds = 7.0;
        let problems = barks_each_frame.problems();
        assert!(problems.iter().any(|problem| problem.contains("boss_idle_bark.base_seconds")), "{problems:?}");
        let mut no_reach = rules();
        no_reach.gnu_ton_duel.reach = 0.0;
        let problems = no_reach.problems();
        assert!(problems.iter().any(|problem| problem.contains("gnu_ton_duel.reach")), "{problems:?}");
        let mut slow = rules();
        slow.thrown_impacts.insert(
            "coconut".to_string(),
            ThrownImpact { min_speed: f32::NAN, damage: 2, rebound: 0.3, rebound_cap: 0.8 },
        );
        let problems = slow.problems();
        assert!(problems.iter().any(|problem| problem.contains("thrown_impacts.coconut.min_speed")), "{problems:?}");
        let mut strikes_twice = rules();
        strikes_twice.thrown_impacts.get_mut("coconut").unwrap().rebound_cap = 1.0;
        let problems = strikes_twice.problems();
        assert!(problems.iter().any(|problem| problem.contains("thrown_impacts.coconut.rebound_cap")), "{problems:?}");
        let mut flat = rules();
        flat.cut_rope_victory.height = 0.0;
        let problems = flat.problems();
        assert!(problems.iter().any(|problem| problem.contains("cut_rope_victory.height")), "{problems:?}");
        let mut alone = rules();
        alone.duel.fighters[1].grudge_against = "b".to_string();
        let problems = alone.problems();
        assert!(problems.iter().any(|problem| problem.contains("duel.fighters[1].grudge_against")), "{problems:?}");
        let mut gone = rules();
        gone.quest_payout.banner_seconds = 0.0;
        let problems = gone.problems();
        assert!(problems.iter().any(|problem| problem.contains("quest_payout.banner_seconds")), "{problems:?}");
        let mut empty = rules();
        empty.cut_rope_trap.heavy_objects.clear();
        let problems = empty.problems();
        assert!(problems.iter().any(|problem| problem.contains("cut_rope_trap.heavy_objects")), "{problems:?}");
        let mut twins = rules();
        let first = twins.cut_rope_trap.heavy_objects[0].clone();
        twins.cut_rope_trap.heavy_objects.push(first);
        let problems = twins.problems();
        assert!(problems.iter().any(|problem| problem.contains("cut_rope_trap.heavy_objects[1].name")), "{problems:?}");
        let mut still = rules();
        still.cut_rope_trap.rope_spark_seconds = 0.0;
        let problems = still.problems();
        assert!(problems.iter().any(|problem| problem.contains("cut_rope_trap.rope_spark_seconds")), "{problems:?}");
    }
}
