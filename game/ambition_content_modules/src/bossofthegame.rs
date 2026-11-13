//! The final boss's techniques: the bossofthegame and Full Stack, the giant
//! amp-stack robot he stands on (`docs/planning/game/final-boss.md`).
//!
//! Each entry is a `Special("<key>")` of the `bossofthegame` row of
//! `boss_profiles.ron`. The rider is the caster: the robot under him is a
//! mount with no conductor, so its moves come out where he stands.
//!
//! - `stack_power_chord`: a chord of sound rings rolls out to each side and
//!   bounces along the floor. Jump it, or be on a riser.
//! - `stack_riff`: a fan of notes at the player that arcs and falls.
//! - `stack_feedback`: the telegraph locks the player's height; a level beam
//!   of sound crosses the stage at that height, out to each side.
//! - `boss_keystroke_rain`: while he types, keycaps fall in the order of the
//!   words he types (`WORDS`), each word's letters side by side.
//! - `boss_backspace`: the telegraph locks the player; a row of backspace keys
//!   flies at that height from him toward the player's side.
//! - `boss_subagents`: two Agent Swarm bodies drop onto the stage.

use ambition_boss_special_port::{BossCaster, BossSpecialCast, BossSummon, BossSummonPort, SummonSide};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, PortKey, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::strike;

pub const POWER_CHORD: &str = "stack_power_chord";
pub const RIFF: &str = "stack_riff";
pub const FEEDBACK: &str = "stack_feedback";
pub const KEYSTROKE_RAIN: &str = "boss_keystroke_rain";
pub const BACKSPACE: &str = "boss_backspace";
pub const SUBAGENTS: &str = "boss_subagents";

const SOUND: &str = "sound_ring";

const CHORD_SPEEDS: [f32; 3] = [240.0, 320.0, 400.0];
const CHORD_GRAVITY: f32 = 900.0;
const CHORD_BOUNCES: u8 = 4;

const RIFF_NOTES: i32 = 5;
const RIFF_SPREAD_DEG: f32 = 10.0;
const RIFF_SPEED: f32 = 380.0;
const RIFF_GRAVITY: f32 = 500.0;

const BEAM_SEGMENTS: u32 = 6;
const BEAM_SPACING: f32 = 34.0;
const BEAM_SPEED: f32 = 640.0;

/// What he types, in order, round again after the last.
pub const WORDS: &[&str] = &["kill", "gg", "rm", "die", "ez", "git", "push", "force", "nerf", "noob", "ctrl", "esc", "del"];
const KEY_INTERVAL_S: f32 = 0.24;
const KEY_HALF: f32 = 15.0;
const KEY_PITCH: f32 = 46.0;
const KEY_GRAVITY: f32 = 600.0;
/// Down from the top of the room.
const KEY_SPAWN_Y: f32 = 48.0;
const PHI_FRAC: f32 = 0.618_033_99;
const KEYS_PER_CALL: u32 = 4;

const BACKSPACE_KEYS: u32 = 3;
const BACKSPACE_SPEED: f32 = 420.0;

const SUBAGENT_COUNT: u32 = 2;
const SUBAGENT_LABEL: &str = "bossofthegame_subagent";
const SUBAGENT_CHARACTER: &str = "npc_agent_swarm";
const SUBAGENT_HALF_SIZE: [f32; 2] = [15.0, 24.0];
const SUBAGENT_SPREAD: f32 = 260.0;
const SUBAGENT_DROP_Y: f32 = 80.0;

pub const CHORD_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.chord", 1);
pub const RIFF_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.riff", 1);
pub const FEEDBACK_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.feedback", 1);
pub const BACKSPACE_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.backspace", 1);
pub const SUBAGENT_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.subagents", 1);
pub const SUBAGENT_NUMBER: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.subagent_number", 1);

record! {
    /// The typing clock: gameplay seconds since the last keystroke of this strike.
    pub struct Typing = SchemaKey::new(crate::PROVIDER, "bossofthegame.typing", 1);
    1 since_key_s: f32,
}

record! {
    /// The next letter he types, counted across strikes through `WORDS`.
    pub struct Cursor = SchemaKey::new(crate::PROVIDER, "bossofthegame.cursor", 1);
    1 letter: u32,
}

fn entry(
    key: &'static str,
    selector: &'static str,
    writes: Vec<SchemaKey>,
    request: PortKey,
    max_requests: u32,
    on_idle: IdlePolicy,
    run: fn(&mut Invocation<'_>) -> Result<(), Fault>,
) -> EntryDescriptor {
    EntryDescriptor {
        key: key.into(),
        phase: TECHNIQUE_EXECUTION,
        trigger: TriggerBinding { port: BossSpecialCast::KEY, selector: selector.into() },
        reads: Vec::new(),
        writes,
        requests: vec![request],
        after: Vec::new(),
        limits: Limits { max_requests },
        on_idle,
        run: EntryCode::Native(run),
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "bossofthegame"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![
            strike::once_schema(CHORD_STRIKE),
            strike::once_schema(RIFF_STRIKE),
            strike::locked_schema(FEEDBACK_STRIKE),
            strike::locked_schema(BACKSPACE_STRIKE),
            strike::once_schema(SUBAGENT_STRIKE),
            strike::number_schema(SUBAGENT_NUMBER),
            Typing::schema(),
            Cursor::schema(),
        ],
        entries: vec![
            entry("power_chord", POWER_CHORD, vec![CHORD_STRIKE], ProjectileSpawnPort::KEY,
                  2 * CHORD_SPEEDS.len() as u32, IdlePolicy::ResetState, power_chord),
            entry("riff", RIFF, vec![RIFF_STRIKE], ProjectileSpawnPort::KEY, RIFF_NOTES as u32, IdlePolicy::ResetState, riff),
            entry("feedback", FEEDBACK, vec![FEEDBACK_STRIKE], ProjectileSpawnPort::KEY, 2 * BEAM_SEGMENTS,
                  IdlePolicy::ResetState, feedback),
            // An idle tick resets the clock; the cursor goes on into the next strike.
            entry("keystroke_rain", KEYSTROKE_RAIN, vec![Typing::KEY, Cursor::KEY], ProjectileSpawnPort::KEY, KEYS_PER_CALL,
                  IdlePolicy::ResetStateExcept(vec![Cursor::KEY]), keystroke_rain),
            entry("backspace", BACKSPACE, vec![BACKSPACE_STRIKE], ProjectileSpawnPort::KEY, BACKSPACE_KEYS,
                  IdlePolicy::ResetState, backspace),
            entry("subagents", SUBAGENTS, vec![SUBAGENT_STRIKE, SUBAGENT_NUMBER], BossSummonPort::KEY, SUBAGENT_COUNT,
                  IdlePolicy::ResetStateExcept(vec![SUBAGENT_NUMBER]), subagents),
        ],
    }
}

fn shot(origin: Vec2, dir: Vec2, speed: f32, gravity: f32, half: f32, lifetime: f32, visual: &str) -> ProjectileSpawn {
    ProjectileSpawn {
        origin,
        dir,
        speed,
        damage: 1,
        max_lifetime: lifetime,
        half_extent: Vec2::splat(half),
        gravity,
        visual_id: visual.into(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    }
}

fn power_chord(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    if !strike::once(inv, &CHORD_STRIKE, &caster)? {
        return Ok(());
    }
    let origin = Vec2::from(caster.body_center);
    for side in [-1.0_f32, 1.0] {
        for speed in CHORD_SPEEDS {
            let mut ring = shot(origin, Vec2::new(side, -0.25).normalize(), speed, CHORD_GRAVITY, 16.0, 3.0, SOUND);
            ring.bounces = CHORD_BOUNCES;
            ring.bounce_on_world_contact = true;
            inv.submit::<ProjectileSpawnPort>(ring)?;
        }
    }
    Ok(())
}

fn muzzle(caster: &BossCaster) -> Vec2 {
    Vec2::from(caster.position) + Vec2::new(caster.projectile_offset[0] * caster.facing, caster.projectile_offset[1])
}

fn riff(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    if !strike::once(inv, &RIFF_STRIKE, &caster)? {
        return Ok(());
    }
    let origin = muzzle(&caster);
    let aim = caster.target.map(Vec2::from).unwrap_or(origin + Vec2::new(caster.facing, 0.0)) - origin;
    let base = if aim.length_squared() < 1e-4 { Vec2::new(caster.facing, 0.0) } else { aim.normalize() };
    for i in 0..RIFF_NOTES {
        let turn = ((i - RIFF_NOTES / 2) as f32 * RIFF_SPREAD_DEG).to_radians();
        let dir = Vec2::from_angle(turn).rotate(base);
        inv.submit::<ProjectileSpawnPort>(shot(origin, dir, RIFF_SPEED, RIFF_GRAVITY, 14.0, 2.5, SOUND))?;
    }
    Ok(())
}

fn feedback(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let Some(target) = strike::locked(inv, &FEEDBACK_STRIKE, &caster)? else {
        return Ok(());
    };
    for side in [-1.0_f32, 1.0] {
        for i in 0..BEAM_SEGMENTS {
            let origin = Vec2::new(caster.position[0] + side * (40.0 + BEAM_SPACING * i as f32), target[1]);
            inv.submit::<ProjectileSpawnPort>(shot(origin, Vec2::new(side, 0.0), BEAM_SPEED, 0.0, 16.0, 2.2, SOUND))?;
        }
    }
    Ok(())
}

/// The word and the letter in it of the `letter`-th keystroke, and the
/// number of the word counted from the first.
pub fn typed(letter: u32) -> (u32, usize, char) {
    let per_round: u32 = WORDS.iter().map(|w| w.len() as u32).sum();
    let round = letter / per_round;
    let mut left = letter % per_round;
    for (i, word) in WORDS.iter().enumerate() {
        let len = word.len() as u32;
        if left < len {
            let ch = word.as_bytes()[left as usize] as char;
            return (round * WORDS.len() as u32 + i as u32, left as usize, ch);
        }
        left -= len;
    }
    unreachable!("the cursor is inside a round")
}

/// The world x of a keystroke: its word starts at a golden-ratio place across
/// the room, the word's letters side by side; a key over the caster moves to
/// the nearer side of it.
pub fn key_x(word_number: u32, index: usize, word_len: usize, room_width: f32, body_min_x: f32, body_max_x: f32) -> f32 {
    let margin = KEY_HALF + 8.0;
    let span = (room_width - 2.0 * margin - KEY_PITCH * word_len as f32).max(0.0);
    let start = margin + ((word_number as f32) * PHI_FRAC).fract() * span;
    let mut x = start + KEY_PITCH * index as f32 + KEY_HALF;
    let (left, right) = (body_min_x - KEY_HALF, body_max_x + KEY_HALF);
    if x > left && x < right {
        x = if x - left < right - x { left } else { right };
    }
    x.clamp(margin, (room_width - margin).max(margin))
}

fn keystroke_rain(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let mut typing = Typing::load(inv)?;
    if !caster.pressed {
        typing.since_key_s = 0.0;
        return typing.store(inv);
    }
    let Some(room) = caster.room_size.filter(|_| caster.alive) else {
        return Ok(());
    };
    let mut cursor = Cursor::load(inv)?;
    typing.since_key_s += inv.dt();
    let mut keys = Vec::new();
    while typing.since_key_s >= KEY_INTERVAL_S && (keys.len() as u32) < KEYS_PER_CALL {
        typing.since_key_s -= KEY_INTERVAL_S;
        let (word_number, index, ch) = typed(cursor.letter);
        let len = WORDS[(word_number as usize) % WORDS.len()].len();
        let body_min = caster.body_center[0] - caster.body_half_size[0];
        let body_max = caster.body_center[0] + caster.body_half_size[0];
        keys.push((key_x(word_number, index, len, room[0], body_min, body_max), ch));
        cursor.letter = cursor.letter.wrapping_add(1);
    }
    typing.store(inv)?;
    cursor.store(inv)?;
    for (x, ch) in keys {
        let visual = format!("keycap_{ch}");
        inv.submit::<ProjectileSpawnPort>(shot(Vec2::new(x, KEY_SPAWN_Y), Vec2::new(0.0, 1.0), 60.0, KEY_GRAVITY, KEY_HALF, 4.0, &visual))?;
    }
    Ok(())
}

fn backspace(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let Some(target) = strike::locked(inv, &BACKSPACE_STRIKE, &caster)? else {
        return Ok(());
    };
    let side = if (target[0] - caster.position[0]).abs() < 1.0 { caster.facing } else { (target[0] - caster.position[0]).signum() };
    for i in 0..BACKSPACE_KEYS {
        let origin = Vec2::new(caster.position[0] + side * (60.0 + 2.0 * KEY_HALF * i as f32), target[1]);
        inv.submit::<ProjectileSpawnPort>(shot(origin, Vec2::new(side, 0.0), BACKSPACE_SPEED, 0.0, KEY_HALF, 4.0, "keycap_back"))?;
    }
    Ok(())
}

fn subagents(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let Some(number) = strike::once_numbered(inv, &SUBAGENT_STRIKE, &SUBAGENT_NUMBER, &caster)? else {
        return Ok(());
    };
    for i in 0..SUBAGENT_COUNT {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        inv.submit::<BossSummonPort>(BossSummon {
            label: SUBAGENT_LABEL.into(),
            serial: vec![number, i],
            position: [caster.position[0] + side * SUBAGENT_SPREAD, SUBAGENT_DROP_Y],
            half_size: SUBAGENT_HALF_SIZE,
            character_id: SUBAGENT_CHARACTER.into(),
            health: Some(3),
            keeps_contact_damage: true,
            side: SummonSide::Enemy,
            holds: Vec::new(),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keystrokes_spell_the_words_in_order_and_go_round() {
        let typed_text: String = (0..9).map(|n| typed(n).2).collect();
        assert_eq!(typed_text, "killggrmd");
        let per_round: u32 = WORDS.iter().map(|w| w.len() as u32).sum();
        assert_eq!(typed(per_round), (WORDS.len() as u32, 0, 'k'));
    }

    #[test]
    fn a_words_keys_stand_side_by_side_in_the_room_and_miss_the_caster() {
        let xs: Vec<f32> = (0..4).map(|i| key_x(0, i, 4, 2560.0, 1240.0, 1280.0)).collect();
        assert!(xs.windows(2).all(|w| w[1] > w[0]), "{xs:?}");
        for word in 0..40 {
            for i in 0..5 {
                let x = key_x(word, i, 5, 2560.0, 1240.0, 1280.0);
                assert!((0.0..=2560.0).contains(&x));
                assert!(x <= 1240.0 - KEY_HALF || x >= 1280.0 + KEY_HALF, "{x}");
            }
        }
    }
}
