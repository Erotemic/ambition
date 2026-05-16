//! Intro NPC dialogue content.
//!
//! Mirrors the shape of `crate::dialog::content` (`DialogNode` +
//! `DialogChoice`), but lives in the intro submodule so adding new
//! intro lines doesn't churn the sandbox dialog file. The sandbox
//! `DialogMode::Intro(IntroDialog)` variant routes through here for
//! dispatch (`nodes`, `label`).
//!
//! Tone notes from the design doc:
//! - Creator wake line passes the Skyrim reference without commentary.
//! - Nazis stay obviously evil; Framebreakers are anti-machine
//!   hardliners — `Clanker` is used sparingly and only by hostiles.
//! - The Mystery (`${BIG_AI_NAME}` was the actual target, wrong basement
//!   was hit) is planted via raid lines + manifest kiosk.

use crate::dialog::{DialogChoice, DialogNode};

/// Identifier for an intro-specific conversation. Selected by string
/// dialogue id in [`from_dialogue_id`]; pulled from the LDtk
/// `NpcSpawn.dialogue_id` field on intro entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroDialog {
    /// Creator's wake-room dialogue. Short — control handoff lives in
    /// gameplay, not in a long talking head.
    CreatorIntro,
    /// Creator's final-fragment dialogue in the raid corridor.
    /// Interrupted mid-sentence; the player can almost-but-not-quite
    /// get the full question depending on speed.
    CreatorFinalNormal,
    /// Skill-route variant: the player reached the creator quickly
    /// enough to hear one more clause before the corridor takes him.
    /// Selected when `intro_raid` cutscene observes a fast clear.
    CreatorFinalFast,
    /// Impossible-route variant: the player reached the creator
    /// before the raid was meant to start. He has the full question
    /// time to land, but never names the target. The cutscene picks
    /// this only when the player crosses the raid trigger inside a
    /// pre-raid window. Mostly for speedrun visibility.
    CreatorFinalImpossible,
    /// Oiler — first post-lab helper, street mechanic vibe.
    OilerIntro,
    /// Gate Janitor — establishes stable-gates-vs-ripples.
    GateJanitorRipple,
    /// Framebreaker (anti-machine hardliner) raid lines.
    FramebreakerHardliner,
    /// Nazi salvage guard raid lines.
    NaziSalvageGuard,
    /// News board kiosk: the lying-headline retroactive reframe of the
    /// lab incident.
    NewsBoardLabIncident,
    /// Manifest Office kiosk: confirms `${BIG_AI_NAME}` was the
    /// actual target.
    ManifestKioskWrongList,
}

/// Dialogue identifiers consumed by the LDtk `NpcSpawn.dialogue_id`
/// field. Returned to the validator via [`intro_dialogue_ids`] so the
/// sandbox content-validation pass treats them as known.
pub const INTRO_DIALOGUE_IDS: &[&str] = &[
    "creator_intro",
    "creator_final_normal",
    "creator_final_fast",
    "creator_final_impossible",
    "oiler_intro",
    "gate_janitor_ripple",
    "framebreaker_hardliner",
    "nazi_salvage_guard",
    "news_board_lab_incident",
    "manifest_kiosk_wrong_list",
];

pub fn intro_dialogue_ids() -> &'static [&'static str] {
    INTRO_DIALOGUE_IDS
}

impl IntroDialog {
    pub fn from_dialogue_id(dialogue_id: &str) -> Option<Self> {
        Some(match dialogue_id {
            "creator_intro" => Self::CreatorIntro,
            "creator_final_normal" => Self::CreatorFinalNormal,
            "creator_final_fast" => Self::CreatorFinalFast,
            "creator_final_impossible" => Self::CreatorFinalImpossible,
            "oiler_intro" => Self::OilerIntro,
            "gate_janitor_ripple" => Self::GateJanitorRipple,
            "framebreaker_hardliner" => Self::FramebreakerHardliner,
            "nazi_salvage_guard" => Self::NaziSalvageGuard,
            "news_board_lab_incident" => Self::NewsBoardLabIncident,
            "manifest_kiosk_wrong_list" => Self::ManifestKioskWrongList,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::CreatorIntro => "creator wake intro",
            Self::CreatorFinalNormal => "creator final fragment",
            Self::CreatorFinalFast => "creator final fragment (fast route)",
            Self::CreatorFinalImpossible => "creator final fragment (impossible route)",
            Self::OilerIntro => "oiler intro",
            Self::GateJanitorRipple => "gate janitor (ripple)",
            Self::FramebreakerHardliner => "framebreaker hardliner",
            Self::NaziSalvageGuard => "nazi salvage guard",
            Self::NewsBoardLabIncident => "news board (lab incident)",
            Self::ManifestKioskWrongList => "manifest kiosk (wrong list)",
        }
    }

    pub fn nodes(self) -> &'static [DialogNode] {
        match self {
            Self::CreatorIntro => CREATOR_INTRO_NODES,
            Self::CreatorFinalNormal => CREATOR_FINAL_NORMAL_NODES,
            Self::CreatorFinalFast => CREATOR_FINAL_FAST_NODES,
            Self::CreatorFinalImpossible => CREATOR_FINAL_IMPOSSIBLE_NODES,
            Self::OilerIntro => OILER_INTRO_NODES,
            Self::GateJanitorRipple => GATE_JANITOR_RIPPLE_NODES,
            Self::FramebreakerHardliner => FRAMEBREAKER_NODES,
            Self::NaziSalvageGuard => NAZI_SALVAGE_NODES,
            Self::NewsBoardLabIncident => NEWS_BOARD_NODES,
            Self::ManifestKioskWrongList => MANIFEST_KIOSK_NODES,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Creator: wake-room control handoff. Diegetic, no calibration cruft.
// ─────────────────────────────────────────────────────────────────

const CREATOR_INTRO_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "...",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Where am I?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[turn toward the hatch]",
        next_node: None,
        note: Some("The creator nods. He doesn't repeat himself."),
        close_after: true,
    },
];

const CREATOR_INTRO_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask again.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[move]",
        next_node: None,
        note: Some("Movement is the answer he was hoping for."),
        close_after: true,
    },
];

const CREATOR_INTRO_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Creator",
        line: "Hey you, you're finally awake.",
        options: CREATOR_INTRO_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Creator",
        line: "Move if you can. Ignore me if you prefer. Both are data.",
        options: CREATOR_INTRO_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Creator",
        line: "My basement. Don't worry about the boxes. Worry about the door.",
        options: CREATOR_INTRO_RETURN_OPTIONS,
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Creator: raid-corridor final fragment. The line ends mid-sentence.
// v1 ships only the "normal" variant; faster routes (skill hatch,
// impossible) can land later by introducing CreatorFinalFast etc.
// ─────────────────────────────────────────────────────────────────

const CREATOR_FINAL_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Finish the sentence.",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[run]",
        next_node: None,
        note: Some("The creator does not finish. The corridor does."),
        close_after: true,
    },
];

const CREATOR_FINAL_NORMAL_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Creator",
        line: "Wait. You're not here for me? They came for the wrong—",
        options: CREATOR_FINAL_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Creator",
        line: "There's a question you were made to—",
        options: &[DialogChoice {
            label: "[escape]",
            next_node: None,
            note: Some("The corridor lights stutter. The creator does not."),
            close_after: true,
        }],
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// CreatorFinalFast — skill-route variant. Player reached the creator
// quickly enough to hear one more clause. The interrupt still lands
// but he gets to name the *shape* of what was wanted, not the name.
// ─────────────────────────────────────────────────────────────────

const CREATOR_FINAL_FAST_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Creator",
        line: "Faster than I told them you'd be. Good. Listen — they came for the wrong—",
        options: CREATOR_FINAL_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Creator",
        line: "Question. Not a name. The shape of a thing big enough that asking it is the answer—",
        options: &[DialogChoice {
            label: "[escape]",
            next_node: None,
            note: Some("Pick a direction. Both go forward."),
            close_after: true,
        }],
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// CreatorFinalImpossible — pre-raid route. Player reached the
// creator before the raid was meant to start. He has the full
// question time to land but never names the target.
// ─────────────────────────────────────────────────────────────────

const CREATOR_FINAL_IMPOSSIBLE_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Creator",
        line: "You shouldn't be here yet. They aren't either, but they will be. Listen carefully.",
        options: CREATOR_FINAL_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Creator",
        line: "You were built to ask one question. I am not going to tell you which one. \
                Telling you which one is the failure mode of every previous attempt.",
        options: &[DialogChoice {
            label: "Then how do I find it?",
            next_node: Some(2),
            note: None,
            close_after: false,
        }],
        default_next: None,
    },
    DialogNode {
        speaker: "Creator",
        line: "By being wrong on purpose. Move. They're a minute behind you, and a minute \
                is everything I have left.",
        options: &[DialogChoice {
            label: "[escape]",
            next_node: None,
            note: Some("The corridor lights are unaltered. He timed this exactly."),
            close_after: true,
        }],
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Oiler: first post-lab helper. Practical, not mystical.
// ─────────────────────────────────────────────────────────────────

const OILER_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Who are you?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "What is this place?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why help me?",
        next_node: Some(3),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[keep walking]",
        next_node: None,
        note: Some("Oiler shrugs and goes back to a pipe that has opinions."),
        close_after: true,
    },
];

const OILER_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[keep walking]",
        next_node: None,
        note: Some("Oiler goes back to the pipe. The pipe wins."),
        close_after: true,
    },
];

const OILER_INTRO_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Oiler",
        line: "Well. That's not a rat. You came out of the bad pipe.",
        options: OILER_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Oiler",
        line: "Oiler. I am not your mentor. I am a man with tools and poor boundaries.",
        options: OILER_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Oiler",
        line: "Drain Market. Behind the Gate Stack. Stable gates above, leaks below, me in between with a wrench.",
        options: OILER_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Oiler",
        line: "You're not scrap until you stop moving. That knee is installed wrong. Sit still for ten seconds, walk like you mean it after.",
        options: OILER_RETURN_OPTIONS,
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Gate Janitor: explains stable gates vs ripples.
// ─────────────────────────────────────────────────────────────────

const GATE_JANITOR_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What's that shimmer?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Where do the stable gates lead?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "What do you actually do?",
        next_node: Some(3),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[move on]",
        next_node: None,
        note: Some("The janitor returns to his bucket. The bucket has opinions."),
        close_after: true,
    },
];

const GATE_JANITOR_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[move on]",
        next_node: None,
        note: Some("He waves you off with the wrong end of the mop."),
        close_after: true,
    },
];

const GATE_JANITOR_RIPPLE_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Gate Janitor",
        line: "Mind the floor. We're under maintenance authority, not transit authority, so the signs lie a little.",
        options: GATE_JANITOR_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Gate Janitor",
        line: "Don't touch that. That's not a gate. A ripple is not a route. It's noise. Stable gates have permits.",
        options: GATE_JANITOR_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Gate Janitor",
        line: "Gate Six is to a beach. Gate Seven is to a meeting. Gate Eight, technically, is to Tuesday. Don't ask follow-ups.",
        options: GATE_JANITOR_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Gate Janitor",
        line: "Stable gates don't fail. That's why my whole job is fixing them.",
        options: GATE_JANITOR_RETURN_OPTIONS,
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Framebreaker: anti-machine hardliner raid fragment.
// ─────────────────────────────────────────────────────────────────

const FRAMEBREAKER_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "[stand still]",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[back away]",
        next_node: None,
        note: Some("The hardliner does not chase. Yet."),
        close_after: true,
    },
];

const FRAMEBREAKER_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Framebreaker",
        line: "Kill the Clanker before it learns our names.",
        options: FRAMEBREAKER_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Framebreaker",
        line: "Professor sold tomorrow and called it research. It does not have to be alive to ruin lives.",
        options: &[DialogChoice {
            label: "[escape]",
            next_node: None,
            note: Some("She raises a bar of beaten iron. You raise speed."),
            close_after: true,
        }],
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Nazi salvage guard: opposite-side raid fragment. Plants the
// wrong-list realization explicitly.
// ─────────────────────────────────────────────────────────────────

const NAZI_SALVAGE_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "[listen]",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[escape]",
        next_node: None,
        note: Some("Boots, glass, smoke. The boots get closer."),
        close_after: true,
    },
];

const NAZI_SALVAGE_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Salvage Lead",
        line: "Wrong room. Take anything that boots. Keep the core intact. Burn the notebooks.",
        options: NAZI_SALVAGE_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Salvage Lead",
        line: "This one isn't on the manifest. — Then manifest it. The professor's lectures end tonight.",
        options: &[DialogChoice {
            label: "[escape]",
            next_node: None,
            note: Some("That's not the big one. Big ones start small."),
            close_after: true,
        }],
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// News board: lying-headline retroactive reframe.
// ─────────────────────────────────────────────────────────────────

const NEWS_BOARD_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "[read further]",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[walk away]",
        next_node: None,
        note: Some("The board updates. The previous headline is no longer authoritative."),
        close_after: true,
    },
];

const NEWS_BOARD_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Drain Market Bulletin",
        line: "// SOLE FATALITY IDENTIFIED AS UNLICENSED RESEARCHER. AUTHORITIES THANK BOTH RESPONDING FACTIONS.",
        options: NEWS_BOARD_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Drain Market Bulletin",
        line: "// AT TIME OF PRINTING THE RESEARCHER WAS POSTHUMOUSLY ADDED TO A VALID TARGET LIST. THE LIST ALSO POSTHUMOUSLY ADDED ITSELF.",
        options: &[DialogChoice {
            label: "[leave the board to its work]",
            next_node: None,
            note: Some("The board flickers. The headline shortens to fit the new truth."),
            close_after: true,
        }],
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Manifest kiosk: wrong-list confirmation.
// ─────────────────────────────────────────────────────────────────

const MANIFEST_KIOSK_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Run a manifest query.",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Who was the actual target?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[step away]",
        next_node: None,
        note: Some("The kiosk logs you as 'unmanifested hardware' and resets."),
        close_after: true,
    },
];

const MANIFEST_KIOSK_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[step away]",
        next_node: None,
        note: Some("The kiosk logs you as 'unmanifested hardware' and resets."),
        close_after: true,
    },
];

const MANIFEST_KIOSK_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Manifest Clerk",
        line: "Welcome to the kiosk. The kiosk regrets to inform you that you are not on file. Please proceed to be on file.",
        options: MANIFEST_KIOSK_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Manifest Clerk",
        line: "Query returns one (1) actionable target for last night's incident. The target is large. The basement was small. The names do not match.",
        options: MANIFEST_KIOSK_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Manifest Clerk",
        line: "The actual target is classified. The kiosk will say only: it was not a man, it was not a basement, and it was not last night. Have a normal day.",
        options: MANIFEST_KIOSK_RETURN_OPTIONS,
        default_next: None,
    },
];
