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
//! - `boss_call_geppetto`: his named agent Geppetto drops onto the stage
//!   (phase 2 opens with it: "Do I need to get Geppetto to do it?").
//! - `boss_call_clause_wren`: Clause and Wren drop onto the stage, one each
//!   side of him (the encore opens with it: "Spawn subagents.").
//!
//! And one conductor entry (`conduct`, bound to the boss `bossofthegame`):
//! it moves nothing; it draws Full Stack in the row of the move it plays
//! (`strum`, `riff`, `feedback`), and in its `idle` strum the rest of the
//! time; its `solo` while he holds his beat between phases, and its `outro`
//! when he is down (it plays him off). As the show opens it stages the band (`BAND`): Geppetto on the bass
//! on the west truss, Wren on the rhythm guitar and Clause at the drums on
//! the east one, each playing the row of the instrument. A call sends the
//! called agent's band body away as the fighter drops in: it put the
//! instrument down. The microphone is the Boss's alone.

use ambition_boss_special_port::conduct::{BossConduct, BossConductPort, DrawnPart, DrawnRow, DrawnRowPort};
use ambition_boss_special_port::{
    BossCaster, BossDismissSummon, BossDismissSummonPort, BossSpecialCast, BossSummon, BossSummonPort, SummonSide,
};
use ambition_extension_sdk::{
    phases::{BOSS_CONDUCT, TECHNIQUE_EXECUTION}, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
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
pub const CALL_GEPPETTO: &str = "boss_call_geppetto";
pub const CALL_CLAUSE_WREN: &str = "boss_call_clause_wren";
/// The boss the conductor entry is bound to (`BossConfig::behavior.id`).
pub const BOSS_ID: &str = "bossofthegame";

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

/// The named agents he calls, by the move that calls them: each one's
/// catalog character, its side of him (-1 west, 1 east) and how far out, and
/// its health. He stands on Full Stack's right shoulder, 140 right of its
/// middle, and its top is 400 wide. Geppetto drops onto the open stage,
/// clear of the robot and its guitar, to hunt the player on the floor: a
/// brute dropped on the guitar's neck stays up there over a player below.
/// Clause drops onto the robot's top, his master's guard where the climb
/// arrives; Wren, the quick one, onto the stage east of the robot.
const AGENT_LABEL: &str = "bossofthegame_agent";
const AGENT_HALF_SIZE: [f32; 2] = [15.0, 30.0];
const GEPPETTO_CALL: &[(&str, f32, f32, u32)] = &[("npc_geppetto", -1.0, 760.0, 8)];
const CLAUSE_WREN_CALL: &[(&str, f32, f32, u32)] = &[("npc_clause", -1.0, 250.0, 6), ("npc_wren", 1.0, 360.0, 6)];

/// The band as the show opens: each agent's band body (its catalog
/// character) and where it stands, as an offset from the middle of the
/// stage. They stand on the trusses, at the height of Full Stack's top where
/// the Boss stands (`boss_stage/make_specs.py`: the west truss is 920 to 280
/// west of the middle, the east one 280 to 920 east of it; Clause's drum kit
/// stands at his place). The serial of each is its place here; a call names
/// the ones it sends away.
const BAND_LABEL: &str = "bossofthegame_band";
const BAND: [(&str, f32); 3] = [("npc_geppetto_band", -500.0), ("npc_wren_band", 520.0), ("npc_clause_band", 760.0)];
/// The band bodies each call sends away, by their place in `BAND`.
const GEPPETTO_BAND: &[u32] = &[0];
const CLAUSE_WREN_BAND: &[u32] = &[2, 1];

pub const CHORD_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.chord", 1);
pub const RIFF_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.riff", 1);
pub const FEEDBACK_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.feedback", 1);
pub const BACKSPACE_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.backspace", 1);
pub const SUBAGENT_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.subagents", 1);
pub const SUBAGENT_NUMBER: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.subagent_number", 1);
pub const GEPPETTO_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.call_geppetto", 1);
pub const CLAUSE_WREN_STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "bossofthegame.call_clause_wren", 1);

record! {
    /// The row Full Stack is drawn in (`STACK_ROWS`), and for how long.
    pub struct Stage = SchemaKey::new(crate::PROVIDER, "bossofthegame.stage", 1);
    1 row: u32,
    2 elapsed_s: f32,
}

/// Full Stack's rows: (the move that plays it, the row, whether it loops).
/// Row 0 is its idle strum, drawn when none of these moves plays; the solo
/// and the outro belong to no move (`stack_row_now`).
const STACK_ROWS: [(&str, &str, bool); 6] = [
    ("", "idle", true),
    (POWER_CHORD, "strum", false),
    (RIFF, "riff", true),
    (FEEDBACK, "feedback", true),
    ("", "solo", true),
    ("", "outro", true),
];
const SOLO_ROW: u32 = 4;
const OUTRO_ROW: u32 = 5;

/// The row Full Stack plays now: the outro once he is down (it plays him
/// off), its solo while he holds his beat between two phases, else the row
/// of his move.
pub fn stack_row_now(alive: bool, between_phases: bool, telegraph: Option<&str>, active: Option<&str>) -> u32 {
    if !alive {
        OUTRO_ROW
    } else if between_phases {
        SOLO_ROW
    } else {
        stack_row(telegraph, active)
    }
}

/// The row of the move the boss plays now (its strike, else its tell).
pub fn stack_row(telegraph: Option<&str>, active: Option<&str>) -> u32 {
    let of = |key: &str| STACK_ROWS.iter().position(|(k, ..)| !k.is_empty() && *k == key).map(|i| i as u32);
    active.and_then(of).or_else(|| telegraph.and_then(of)).unwrap_or(0)
}

record! {
    /// Whether the band is on the stage: it is staged once a fight.
    pub struct Band = SchemaKey::new(crate::PROVIDER, "bossofthegame.band", 1);
    1 staged: u32,
}

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
            strike::once_schema(GEPPETTO_STRIKE),
            strike::once_schema(CLAUSE_WREN_STRIKE),
            Typing::schema(),
            Cursor::schema(),
            Stage::schema(),
            Band::schema(),
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
            EntryDescriptor {
                key: "conduct".into(),
                phase: BOSS_CONDUCT,
                trigger: TriggerBinding { port: BossConductPort::KEY, selector: BOSS_ID.into() },
                reads: Vec::new(),
                writes: vec![Stage::KEY, Band::KEY],
                requests: vec![DrawnRowPort::KEY, BossSummonPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 1 + BAND.len() as u32 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(conduct),
            },
            entry("subagents", SUBAGENTS, vec![SUBAGENT_STRIKE, SUBAGENT_NUMBER], BossSummonPort::KEY, SUBAGENT_COUNT,
                  IdlePolicy::ResetStateExcept(vec![SUBAGENT_NUMBER]), subagents),
            EntryDescriptor {
                requests: vec![BossSummonPort::KEY, BossDismissSummonPort::KEY],
                ..entry("call_geppetto", CALL_GEPPETTO, vec![GEPPETTO_STRIKE], BossSummonPort::KEY,
                        (GEPPETTO_CALL.len() + GEPPETTO_BAND.len()) as u32, IdlePolicy::ResetState, call_geppetto)
            },
            EntryDescriptor {
                requests: vec![BossSummonPort::KEY, BossDismissSummonPort::KEY],
                ..entry("call_clause_wren", CALL_CLAUSE_WREN, vec![CLAUSE_WREN_STRIKE], BossSummonPort::KEY,
                        (CLAUSE_WREN_CALL.len() + CLAUSE_WREN_BAND.len()) as u32, IdlePolicy::ResetState, call_clause_wren)
            },
        ],
    }
}

fn conduct(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let boss: BossConduct = inv.trigger::<BossConductPort>()?.clone();
    let mut stage = Stage::load(inv)?;
    let row = stack_row_now(
        boss.alive,
        boss.between_phases,
        boss.telegraph.as_ref().map(|m| m.key.as_str()),
        boss.active.as_ref().map(|m| m.key.as_str()),
    );
    if row == stage.row {
        stage.elapsed_s += inv.dt();
    } else {
        stage.row = row;
        stage.elapsed_s = 0.0;
    }
    stage.store(inv)?;
    let (_, name, looping) = STACK_ROWS[stage.row as usize % STACK_ROWS.len()];
    inv.submit::<DrawnRowPort>(DrawnRow { on: DrawnPart::Mount, names: vec![name.into()], elapsed: stage.elapsed_s, looping })?;
    stage_the_band(inv, &boss)
}

/// Where the band body at `place` in `BAND` stands: on the truss at the
/// height of the Boss's feet, `boss` being his middle, in a stage `room_w` wide.
pub fn band_spot(place: usize, boss: [f32; 2], room_w: f32) -> [f32; 2] {
    // He is 96 tall and they are 60: a little over the truss, to land on it.
    [room_w * 0.5 + BAND[place].1, boss[1] + 48.0 - AGENT_HALF_SIZE[1] - 4.0]
}

/// Bring the band on, once a fight, while he stands.
fn stage_the_band(inv: &mut Invocation<'_>, boss: &BossConduct) -> Result<(), Fault> {
    let mut band = Band::load(inv)?;
    let Some([room_w, _]) = boss.room else {
        return Ok(());
    };
    if band.staged != 0 || !boss.alive {
        return Ok(());
    }
    band.staged = 1;
    band.store(inv)?;
    for (place, &(character, _)) in BAND.iter().enumerate() {
        inv.submit::<BossSummonPort>(BossSummon {
            label: BAND_LABEL.into(),
            serial: vec![place as u32],
            position: band_spot(place, boss.position, room_w),
            half_size: AGENT_HALF_SIZE,
            character_id: character.into(),
            health: None,
            keeps_contact_damage: false,
            side: SummonSide::Boss,
            holds: Vec::new(),
        })?;
    }
    Ok(())
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

fn call_geppetto(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    call_agents(inv, &GEPPETTO_STRIKE, 0, GEPPETTO_CALL, GEPPETTO_BAND)
}

fn call_clause_wren(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    call_agents(inv, &CLAUSE_WREN_STRIKE, 1, CLAUSE_WREN_CALL, CLAUSE_WREN_BAND)
}

/// Drop the named agents of one call onto the stage, once a strike, and
/// send their band bodies away (`band`, by place). `call` keeps each call's
/// bodies apart from the other's (their serials).
fn call_agents(
    inv: &mut Invocation<'_>,
    strike_key: &SchemaKey,
    call: u32,
    agents: &[(&str, f32, f32, u32)],
    band: &[u32],
) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    if !strike::once(inv, strike_key, &caster)? {
        return Ok(());
    }
    for &place in band {
        inv.submit::<BossDismissSummonPort>(BossDismissSummon { label: BAND_LABEL.into(), serial: vec![place] })?;
    }
    let room_w = caster.room_size.map(|r| r[0]);
    for (i, &(character, side, reach, health)) in agents.iter().enumerate() {
        let mut x = caster.position[0] + side * reach;
        if let Some(w) = room_w {
            x = x.clamp(AGENT_HALF_SIZE[0] + 48.0, (w - AGENT_HALF_SIZE[0] - 48.0).max(AGENT_HALF_SIZE[0] + 48.0));
        }
        inv.submit::<BossSummonPort>(BossSummon {
            label: AGENT_LABEL.into(),
            serial: vec![call, i as u32],
            position: [x, SUBAGENT_DROP_Y],
            half_size: AGENT_HALF_SIZE,
            character_id: character.into(),
            health: Some(health),
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
    fn full_stack_plays_the_row_of_the_move_and_strums_between() {
        assert_eq!(stack_row(None, None), 0);
        assert_eq!(stack_row(Some(POWER_CHORD), None), 1);
        assert_eq!(stack_row(Some(RIFF), Some(FEEDBACK)), 3);
        assert_eq!(stack_row(Some(KEYSTROKE_RAIN), Some(KEYSTROKE_RAIN)), 0);
        assert_eq!(stack_row_now(true, true, Some(RIFF), None), SOLO_ROW);
        assert_eq!(stack_row_now(false, false, Some(RIFF), None), OUTRO_ROW);
    }

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
