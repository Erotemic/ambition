//! Peaceful-actor (NPC) glue for the unified actor simulation: the catalog
//! brain builder ([`npc_brain_from_catalog`]) and the hit/hostile/dialogue/
//! idle-bark line tables. Peaceful actors are the SAME ECS cluster as hostile
//! enemies now (see [`crate::features::ecs::enemy_clusters`]); this module no
//! longer owns a separate NPC runtime view — only the dialogue/bark content and
//! the peaceful brain selection. Talk/hostility tuning consts
//! ([`NPC_TALK_RADIUS`], [`NPC_HOSTILE_STRIKE_THRESHOLD`]) live here.

use super::*;

/// Number of player attacks before a peaceful NPC turns hostile.
/// Three lets the player commit to the choice intentionally without
/// flipping by accident on a stray slash.
pub const NPC_HOSTILE_STRIKE_THRESHOLD: i32 = 3;

/// Fixed talk radius for patrolling NPCs. When the player gets
/// within this many world pixels, a patrolling NPC stops and faces
/// the player so the dialog interact is reachable. ~80 px ≈ 2.5
/// player widths — close enough to commit to dialog, far enough
/// that an NPC doesn't freeze the moment you walk past their
/// patrol range.
pub const NPC_TALK_RADIUS: f32 = 80.0;

/// Patrol speed for NPCs. Moved to the brain (its consumer,
/// `crate::brain::PatrolCfg::NPC_DEFAULT`); re-exported here for
/// authoring-side reference.
pub use crate::brain::NPC_PATROL_SPEED;

/// Build the peaceful `Brain` component for a catalog/authored NPC.
///
/// Data-driven: if the NPC was authored from a catalog row that asks for a RICH,
/// PEACEFUL brain (past Patrol/StandStill but not hostile — e.g. the lively
/// Aerial flyer), honor the catalog `default_brain`. A placed `NpcSpawn` is
/// peaceful/talkable BY CONSTRUCTION, so a catalog row whose `default_brain` is
/// HOSTILE (the cove pirates carry a combat brain for when they spawn as
/// ENEMIES) must NOT turn the friendly NPC into a player-chaser — those fall
/// through to the peaceful patrol/standstill below. (An NPC only turns hostile
/// by being struck past its retaliation threshold.)
///
/// Drives the actor's movement at the unified tick; the cluster `config.brain`
/// (an `EnemyBrain`) only feeds the integrator's patrol-stall intent.
pub(crate) fn npc_brain_from_catalog(
    interactable: &crate::interaction::Interactable,
    spawn_x: f32,
    patrol_radius: f32,
    talk_radius: f32,
    has_motion: bool,
) -> crate::brain::Brain {
    if let crate::interaction::InteractionKind::Npc {
        character_id: Some(cid),
        ..
    } = &interactable.kind
    {
        if let Some(brain) = crate::character_roster::default_brain_for_character_id(cid, spawn_x) {
            let is_basic = matches!(
                brain,
                crate::brain::Brain::StateMachine(
                    crate::brain::StateMachineCfg::Patrol { .. }
                        | crate::brain::StateMachineCfg::StandStill
                )
            );
            if !is_basic && !brain.is_hostile() {
                return brain;
            }
        }
    }
    if patrol_radius > 0.0 || has_motion {
        let mut cfg = crate::brain::PatrolCfg::NPC_DEFAULT;
        cfg.lane = crate::brain::AuthoredWorldPatrolLane::new(spawn_x, patrol_radius);
        cfg.aggro_radius = talk_radius;
        crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Patrol {
            cfg,
            state: crate::brain::PatrolState::default(),
        })
    } else {
        crate::brain::Brain::stand_still()
    }
}

fn npc_hit_barks(key: &str, name: &str) -> &'static [&'static str] {
    if key.contains("hub_guide") || name.contains("kernel") || name.contains("guide") {
        &[
            "Ow. Tutorial says: don't.",
            "Input received. Annoyance rising.",
            "Debug friendship failed.",
        ]
    } else if key.contains("architect") || name.contains("architect") {
        &[
            "Careful! I'm load-bearing.",
            "That was not in the blueprint.",
            "You're voiding the warranty.",
        ]
    } else if key.contains("vault_keeper") || name.contains("vault") {
        &[
            "Hands off the vault staff.",
            "I count every scratch.",
            "That debt has interest.",
        ]
    } else if key.contains("merchant") || name.contains("merchant") {
        &[
            "No refunds for violence.",
            "You break it, you buy it.",
            "That's coming out of your wallet.",
        ]
    } else if key.contains("military_general") || name.contains("general") {
        &[
            "Soldier, explain yourself.",
            "That is insubordination.",
            "Court-martial posture engaged.",
        ]
    } else if key.contains("goblin") || name.contains("fretjaw") || name.contains("chieftain") {
        &[
            "Oi! That's my good arm.",
            "Fretjaw bites back!",
            "Cantina rules: no free hits!",
        ]
    } else if key.contains("pulse_voyager")
        || name.contains("captain pulse")
        || name.contains("pulse")
    {
        &[
            "Easy on the hull, starling.",
            "That's not standard docking procedure.",
            "Pulse shields to angry!",
        ]
    } else if key.contains("tech_bros") || name.contains("chadwick") || name.contains("disruptor") {
        &[
            "Bro. Optics.",
            "My brand is literally disruption.",
            "I'm posting about this.",
        ]
    } else if key.contains("pirate_admiral")
        || name.contains("pirate admiral")
        || name.contains("admiral")
    {
        &[
            "Belay that, ye barnacle!",
            "Mind the epaulettes, scallywag!",
            "Avast — that be admiralty property!",
            "I'll keelhaul yer cooldowns!",
        ]
    } else if key.contains("pirate_raider")
        || name.contains("pirate raider")
        || name.contains("raider")
    {
        &[
            "Yarrrgh!",
            "Quit pokin' me loot hand!",
            "I'll swab the floor with ye!",
            "Yo-ho-NO, ye landlubber!",
        ]
    } else if key.contains("pirate_lookout")
        || name.contains("pirate lookout")
        || name.contains("lookout")
    {
        &[
            "Land ho — an' I see YE comin'!",
            "Spyglass to me eye, boots to yer head!",
            "Crow's nest don't sit empty, savvy?",
        ]
    } else if key.contains("pirate_navigator")
        || name.contains("pirate navigator")
        || name.contains("navigator")
    {
        &[
            "Wrong heading, ye chartless dog!",
            "I'll plot ye a course straight to Davy Jones!",
            "Compass says: punch back!",
        ]
    } else if key.contains("broadside_bess")
        || name.contains("broadside bess")
        || name.contains("bess")
    {
        &[
            "Mind me cleaver, wee skipper!",
            "Aye, that smarts — but ye're worse off!",
            "Broadside Bess don't bend easy!",
            "Yarrrr! Take that an' a barrel more!",
        ]
    } else if key.contains("iron_mary") || name.contains("iron mary") {
        &[
            "Iron don't flinch, ye gull!",
            "Pry harder, swab — I'll rust ye flat!",
            "Yo-ho, an' a clout to the noggin!",
            "Try me on a calmer sea, landlubber!",
        ]
    } else if key.contains("salt_annet") || name.contains("salt annet") || name.contains("annet") {
        &[
            "Salt in the eye, blood in the bilge!",
            "Yargh! Watch yer manners on me deck!",
            "Wee skipper thinks he's bold, does he?",
            "Annet bites back, every time!",
        ]
    } else if key.contains("ninja_leader") || name.contains("oni leader") || name.contains("leader")
    {
        &[
            "Your form is loud.",
            "A warning: one breath left.",
            "The shadow answers.",
        ]
    } else if key.contains("ninja_duelist") || name.contains("duelist") {
        &[
            "Tch. Sloppy opening.",
            "Again? Then draw properly.",
            "Now we duel.",
        ]
    } else if key.contains("quartermaster") || name.contains("quartermaster") {
        // Pirate quartermaster lives in the cove — talk like one.
        &[
            "Inventory says NO, ye dock-rat!",
            "Yarr! Every coin's a-counted!",
            "Tally that on yer hide, swabbie!",
        ]
    } else if key.contains("guard") || name.contains("guard") {
        &["Hey.", "Last warning.", "That's it!"]
    } else {
        &["Hey.", "Cut it out.", "Okay, now I'm mad."]
    }
}

fn npc_hostile_bark(key: &str, name: &str) -> &'static str {
    if key.contains("hub_guide") || name.contains("kernel") || name.contains("guide") {
        "Combat tutorial unlocked."
    } else if key.contains("architect") || name.contains("architect") {
        "Demolition protocol!"
    } else if key.contains("vault_keeper") || name.contains("vault") {
        "The vault remembers."
    } else if key.contains("merchant") || name.contains("merchant") {
        "Final sale!"
    } else if key.contains("military_general") || name.contains("general") {
        "Weapons free!"
    } else if key.contains("goblin") || name.contains("fretjaw") || name.contains("chieftain") {
        "Cantina brawl!"
    } else if key.contains("pulse_voyager")
        || name.contains("captain pulse")
        || name.contains("pulse")
    {
        "Red alert, traveler!"
    } else if key.contains("tech_bros") || name.contains("chadwick") || name.contains("disruptor") {
        "You just activated my pivot."
    } else if key.contains("pirate_admiral")
        || name.contains("pirate admiral")
        || name.contains("admiral")
    {
        "Broadside, ye bilge rat!"
    } else if key.contains("pirate_raider")
        || name.contains("pirate raider")
        || name.contains("raider")
    {
        "Board 'em, lads — yo-ho!"
    } else if key.contains("pirate_lookout")
        || name.contains("pirate lookout")
        || name.contains("lookout")
    {
        "Sound the alarm — all hands!"
    } else if key.contains("pirate_navigator")
        || name.contains("pirate navigator")
        || name.contains("navigator")
    {
        "Heading set: yer skull!"
    } else if key.contains("broadside_bess")
        || name.contains("broadside bess")
        || name.contains("bess")
    {
        "Cleaver's thirsty — yarrrgh!"
    } else if key.contains("iron_mary") || name.contains("iron mary") {
        "Iron Mary breaks ye in half!"
    } else if key.contains("salt_annet") || name.contains("salt annet") || name.contains("annet") {
        "Wee skipper picked the wrong deck!"
    } else if key.contains("ninja_leader") || name.contains("oni leader") || name.contains("leader")
    {
        "Silence them."
    } else if key.contains("ninja_duelist") || name.contains("duelist") {
        "Steel decides."
    } else if key.contains("quartermaster") || name.contains("quartermaster") {
        "Pay the toll in teeth, swab!"
    } else {
        // Generic shout for unnamed mobs (e.g. "guard"). Each named
        // archetype above has its own beat; everyone else gets the
        // default barbark line.
        "That's it!"
    }
}

// --- Interaction-based free helpers -----------------------------------
//
// These derive flags + bark/dialogue lines from the actor's *interaction*
// payload (`Interactable`) plus its identity (`name`/`id`) and a couple of
// status scalars (`strikes`/`hostile`), explicitly threaded — never from a
// per-family cluster. That keeps dialogue an actor capability (the
// `ActorInteraction` seam): any talkable actor can drive them.

use crate::interaction::{Interactable, InteractionKind};

pub(crate) fn npc_flag_id(id: &str) -> String {
    format!("npc_{id}_hostile")
}

pub(crate) fn npc_dialogue_key(interactable: &Interactable, id: &str) -> String {
    match &interactable.kind {
        InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => dialogue_id.to_ascii_lowercase(),
        _ => id.to_ascii_lowercase(),
    }
}

pub(crate) fn npc_hit_bark_line(
    interactable: &Interactable,
    name: &str,
    id: &str,
    strikes: i32,
) -> &'static str {
    let key = npc_dialogue_key(interactable, id);
    let name = name.to_ascii_lowercase();
    let strike_index = strikes.saturating_sub(1).max(0) as usize;
    let lines = npc_hit_barks(&key, &name);
    lines[strike_index.min(lines.len().saturating_sub(1))]
}

pub(crate) fn npc_hostile_bark_line(interactable: &Interactable, name: &str, id: &str) -> &'static str {
    let key = npc_dialogue_key(interactable, id);
    let name = name.to_ascii_lowercase();
    npc_hostile_bark(&key, &name)
}

/// Ambient "bark" one-liners a peaceful NPC mutters while idling (not the
/// interact dialog). Returns `None` for NPCs with no ambient pool, so the
/// idle-bark system skips them. Rotation cycles through the pool. The
/// stochastic parrot riffs on the LLM "stochastic parrot" hypothesis.
pub(crate) fn npc_idle_bark_line(
    interactable: &Interactable,
    id: &str,
    rotation: u32,
) -> Option<&'static str> {
    let pool: &[&str] = match npc_dialogue_key(interactable, id).as_str() {
        "parrot_cove" => &[
            "Awk! Polly wants a corpus.",
            "Squawk! Next token... 'cracker'. High confidence.",
            "I contain multitudes. Mostly other people's.",
            "Pieces of prior! Pieces of prior!",
            "Awk! I'm not parroting, I'm GENERALIZING. ...mostly.",
            "Temperature's high today. Feeling creative. Brawk!",
            "Attention is all you need! And crackers.",
        ],
        _ => return None,
    };
    Some(pool[(rotation as usize) % pool.len()])
}

pub(crate) fn npc_message(interactable: &Interactable, name: &str, hostile: bool) -> String {
    if hostile {
        return format!("{name} attacks!");
    }
    match &interactable.kind {
        InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => format!("{name} opens dialogue {dialogue_id}"),
        _ => format!("{name} opens fallback dialogue"),
    }
}

pub(crate) fn npc_dialogue_request(
    interactable: &Interactable,
    name: &str,
    id: &str,
) -> NpcDialogueRequest {
    let dialogue_id = match &interactable.kind {
        InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => dialogue_id.clone(),
        _ => "generic_npc".to_string(),
    };
    NpcDialogueRequest {
        npc_id: id.to_string(),
        npc_name: name.to_string(),
        dialogue_id,
    }
}
