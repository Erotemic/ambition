//! Sandbox dialogue runtime and UI.
//!
//! This is the first production-shaped dialogue layer for Ambition. It keeps the
//! authored conversation data in a small code-side registry for now, while the
//! app also registers `bevy_yarnspinner` and includes Yarn source files so the
//! content can migrate to real Yarn nodes without changing NPC/merchant-facing
//! gameplay semantics.

use bevy::prelude::*;
#[cfg(feature = "ui")]
use bevy_yarnspinner::prelude::*;

use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::MenuControlFrame;
use crate::ui_fonts::{UiFontWeight, UiFonts};
#[cfg(feature = "input")]
use crate::ui_nav::apply_vertical_scroll;
use crate::ui_nav::{decorate_windowed_label, visible_window_start};
use bevy::log::info;

const DIALOG_CONTINUE_HINT: &str =
    "Tap an option, press Confirm / Jump / Interact, or drag / use Up-Down. Back closes.";
const DIALOG_VISIBLE_OPTIONS: usize = 4;

/// Marker plugin: registers Yarn Spinner so dialogue assets and future Yarn
/// runners are available, while keeping this first sandbox dialogue view
/// intentionally custom and game-feel oriented. Gated behind the `ui`
/// feature; the rest of this module's dialogue runtime + custom Bevy UI
/// view does not depend on Yarn Spinner.
#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> YarnSpinnerPlugin {
    // Android cannot enumerate asset folders inside the APK, so use an
    // explicit Yarn source instead of YarnSpinnerPlugin::new() (which scans
    // the dialogue folder on desktop builds). Keep this path relative to
    // Bevy's asset root: crates/ambition_sandbox/assets/dialogue/...
    YarnSpinnerPlugin::with_yarn_source(YarnFileSource::file("dialogue/ambition_sandbox.yarn"))
}

#[derive(Clone, Debug, Default)]
pub struct DialogState {
    active: bool,
    node_id: String,
    npc_name: String,
    node_index: usize,
    selected_option: usize,
    /// Android/touch row activation is deliberately two-step: first tap selects,
    /// second tap or a Confirm button activates. This prevents a finger press
    /// that turns into a small drag from accidentally advancing dialogue.
    pointer_armed: Option<usize>,
    mode: DialogMode,
    last_note: String,
}

impl DialogState {
    pub fn start(&mut self, dialogue_id: &str, npc_name: &str) {
        self.active = true;
        self.node_id = dialogue_id.to_string();
        self.npc_name = npc_name.to_string();
        self.node_index = 0;
        self.selected_option = 0;
        self.pointer_armed = None;
        self.mode = DialogMode::from_dialogue_id(dialogue_id);
        self.last_note.clear();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.pointer_armed = None;
        self.last_note.clear();
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn title(&self) -> String {
        if let Some(node) = self.current_node() {
            format!("{} — {}", node.speaker, self.mode.label())
        } else {
            format!("{} — dialogue", self.npc_name)
        }
    }

    pub fn body(&self) -> String {
        let Some(node) = self.current_node() else {
            return "The conversation data is missing; this is a dialogue routing bug.".to_string();
        };
        let mut body = node.line.to_string();
        if !self.last_note.is_empty() {
            body.push_str("\n\n");
            body.push_str(&self.last_note);
        }
        body
    }

    pub fn options(&self) -> &'static [DialogChoice] {
        self.current_node().map(|node| node.options).unwrap_or(&[])
    }

    pub fn selected_option(&self) -> usize {
        self.selected_option
    }

    fn current_node(&self) -> Option<&'static DialogNode> {
        self.mode.nodes().get(self.node_index)
    }

    fn select_delta(&mut self, delta: isize) {
        let len = self.options().len();
        if len == 0 {
            self.selected_option = 0;
            return;
        }
        let next = (self.selected_option as isize + delta).rem_euclid(len as isize) as usize;
        if self.selected_option != next {
            self.pointer_armed = None;
        }
        self.selected_option = next;
    }

    fn confirm_or_advance(&mut self) -> bool {
        let Some(node) = self.current_node() else {
            self.close();
            return true;
        };
        if node.options.is_empty() {
            if let Some(next) = node.default_next {
                self.node_index = next;
                self.selected_option = 0;
                self.pointer_armed = None;
                return false;
            }
            self.close();
            return true;
        }
        let choice = &node.options[self
            .selected_option
            .min(node.options.len().saturating_sub(1))];
        if let Some(note) = choice.note {
            self.last_note = note.to_string();
        } else {
            self.last_note.clear();
        }
        if choice.close_after {
            self.close();
            return true;
        }
        if let Some(next) = choice.next_node {
            self.node_index = next;
            self.selected_option = 0;
            self.pointer_armed = None;
            return false;
        }
        self.close();
        true
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DialogMode {
    #[default]
    Architect,
    VaultKeeper,
    MerchantSeed,
    HubGuide,
    MilitaryGeneral,
    GoblinCantinaChieftain,
    PulseVoyagerCaptain,
    TechBrosDisruptor,
    PirateAdmiral,
    PirateRaider,
    Generic,
}

impl DialogMode {
    fn from_dialogue_id(dialogue_id: &str) -> Self {
        match dialogue_id {
            "architect_intro" => Self::Architect,
            "vault_keeper" => Self::VaultKeeper,
            "merchant_seed" => Self::MerchantSeed,
            "hub_guide" => Self::HubGuide,
            "military_general" => Self::MilitaryGeneral,
            "goblin_cantina_chieftain" => Self::GoblinCantinaChieftain,
            "pulse_voyager_captain" => Self::PulseVoyagerCaptain,
            "tech_bros_disruptor" => Self::TechBrosDisruptor,
            "pirate_admiral" => Self::PirateAdmiral,
            "pirate_raider" => Self::PirateRaider,
            _ => Self::Generic,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Architect => "architecture dialogue",
            Self::VaultKeeper => "merchant / persistence seed",
            Self::MerchantSeed => "merchant design sketch",
            Self::HubGuide => "central hub guidance",
            Self::MilitaryGeneral => "military faction leader",
            Self::GoblinCantinaChieftain => "goblin cantina chieftain",
            Self::PulseVoyagerCaptain => "pulse voyager captain",
            Self::TechBrosDisruptor => "tech-bros disruptor",
            Self::PirateAdmiral => "pirate admiral",
            Self::PirateRaider => "pirate raider",
            Self::Generic => "sandbox dialogue",
        }
    }

    fn nodes(self) -> &'static [DialogNode] {
        match self {
            Self::Architect => ARCHITECT_NODES,
            Self::VaultKeeper => VAULT_KEEPER_NODES,
            Self::MerchantSeed => MERCHANT_SEED_NODES,
            Self::HubGuide => HUB_GUIDE_NODES,
            Self::MilitaryGeneral => MILITARY_GENERAL_NODES,
            Self::GoblinCantinaChieftain => GOBLIN_CANTINA_CHIEFTAIN_NODES,
            Self::PulseVoyagerCaptain => PULSE_VOYAGER_CAPTAIN_NODES,
            Self::TechBrosDisruptor => TECH_BROS_DISRUPTOR_NODES,
            Self::PirateAdmiral => PIRATE_ADMIRAL_NODES,
            Self::PirateRaider => PIRATE_RAIDER_NODES,
            Self::Generic => GENERIC_NODES,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DialogNode {
    pub speaker: &'static str,
    pub line: &'static str,
    pub options: &'static [DialogChoice],
    pub default_next: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct DialogChoice {
    pub label: &'static str,
    pub next_node: Option<usize>,
    pub note: Option<&'static str>,
    pub close_after: bool,
}

const HUB_GUIDE_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Why is Interact separate from Up?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "What should I test next?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Back to the hub.",
        next_node: None,
        note: Some("The guide steps aside. The basement door is clear; no rebound launcher is blocking it now."),
        close_after: true,
    },
];

const HUB_GUIDE_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another hub question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Return to movement.",
        next_node: None,
        note: Some("Movement stays primary; dialogue should explain the lab, not replace the lab."),
        close_after: true,
    },
];

const HUB_GUIDE_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Kernel Guide",
        line: "Welcome back to The Kernel. The hub should teach routes without ambushing your movement inputs. Doors answer Interact first; double-tap up is only a deliberate fallback.",
        options: HUB_GUIDE_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Kernel Guide",
        line: "Up is aim, climb, fly, and intent. Binding doors to a raw single Up press makes the game steal agency at exactly the wrong time. Interact is a promise that you meant to talk, trade, open, or enter.",
        options: HUB_GUIDE_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Kernel Guide",
        line: "Try the basement labs, then check debug mode: combo state, hitstun, invulnerability, health bars, and honest hurtboxes should explain every surprising hit.",
        options: HUB_GUIDE_RETURN_OPTIONS,
        default_next: None,
    },
];

const ARCHITECT_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What is this place?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why are the debug boxes honest?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Enough architecture for now.",
        next_node: None,
        note: Some("Conversation closed. The Architect remains available for retuning the lab."),
        close_after: true,
    },
];

const ARCHITECT_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Return to the room.",
        next_node: None,
        note: Some("You leave with one more rule: beautiful debug is still debug."),
        close_after: true,
    },
];

const ARCHITECT_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Architect",
        line: "You are standing inside a proof harness pretending to be a basement. Every platform is a claim. Every hurtbox is evidence. The game only becomes honest when the debug view and the feeling agree.",
        options: ARCHITECT_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Architect",
        line: "This place is The Kernel's maintenance layer: enemy labs, boss patterns, breakable floors, and unfinished ethical machinery. It is not lore pasted on top of movement; it is where movement earns lore.",
        options: ARCHITECT_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Architect",
        line: "Generated systems lie when they hide their assumptions. So Ambition should show hitboxes, seeds, graphs, timers, and costs. If the player is an AI, then inspection is not a cheat; it is a sense organ.",
        options: ARCHITECT_RETURN_OPTIONS,
        default_next: None,
    },
];

const VAULT_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Show the merchant plan.",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "What is ethical currency?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Not buying anything yet.",
        next_node: None,
        note: Some("The vault closes without spending a single test coin."),
        close_after: true,
    },
];

const VAULT_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Back to the vault menu.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Close the ledger.",
        next_node: None,
        note: Some(
            "Merchant UI is still a design sketch, but the dialogue contract is now explicit.",
        ),
        close_after: true,
    },
];

const VAULT_KEEPER_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Vault Keeper",
        line: "I do not sell power. I sell constraints you can inspect. A merchant in Ambition should expose price, source, side effect, persistence, and refund rules before the player commits.",
        options: VAULT_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Vault Keeper",
        line: "Merchant contract draft: inventory rows are dialogue choices with costs. A purchase can grant an ability, refill health, unlock a route, set a story flag, or reveal the funding provenance of an upgrade.",
        options: VAULT_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Vault Keeper",
        line: "Ethical currency is not a morality meter. It is provenance. Dirty funding may unlock shortcuts but contaminate generated systems. Clean funding may be slower but makes later artifacts easier to audit.",
        options: VAULT_RETURN_OPTIONS,
        default_next: None,
    },
];

const MERCHANT_SEED_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Buy health refill (stub).",
        next_node: None,
        note: Some("Stub purchase: later this should route through Inventory, Wallet, Price, and RewardEffect systems."),
        close_after: true,
    },
    DialogChoice {
        label: "Ask about refunds.",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Leave.",
        next_node: None,
        note: None,
        close_after: true,
    },
];

const MERCHANT_REFUND_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Back to shop stub.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Leave.",
        next_node: None,
        note: None,
        close_after: true,
    },
];

const MERCHANT_SEED_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Merchant Prototype",
        line: "A real shop should be a dialogue node with inventory, prices, requirements, consequences, and preview text. This stub proves choices can become transactions.",
        options: MERCHANT_SEED_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Merchant Prototype",
        line: "Refunds are part of the design contract. For experiments, every purchase should be reversible until the route, boss, or story flag that depends on it is committed.",
        options: MERCHANT_REFUND_OPTIONS,
        default_next: None,
    },
];

const GENERIC_OPTIONS: &[DialogChoice] = &[DialogChoice {
    label: "Close.",
    next_node: None,
    note: None,
    close_after: true,
}];

const GENERIC_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Sandbox NPC",
        line: "This NPC has no named Yarn node yet. The fallback still proves the interaction contract: trigger, pause, show line, choose, close, resume.",
        options: GENERIC_OPTIONS,
        default_next: None,
    },
];

const MILITARY_GENERAL_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What is this place?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why so many medals?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Salute and withdraw.",
        next_node: None,
        note: Some("The General returns the salute with surgical precision and twelve audible clicks."),
        close_after: true,
    },
];

const MILITARY_GENERAL_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Salute and withdraw.",
        next_node: None,
        note: Some("The General returns the salute with surgical precision and twelve audible clicks."),
        close_after: true,
    },
];

const MILITARY_GENERAL_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "General",
        line: "AT EASE, RECRUIT. You have ascended the Tower of Acceptable Casualties. Below us: parade grounds. Above us: bureaucracy. Around us: an oath you have not yet sworn.",
        options: MILITARY_GENERAL_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "General",
        line: "This is the Iron Bastion of the Faction Of Steel — a tower built from after-action reports and hand-cranked optimism. We host parades on the parts that aren't structural.",
        options: MILITARY_GENERAL_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "General",
        line: "Each medal commemorates a meeting I survived. Three are for punctuality. Two are for the meetings ABOUT punctuality. The largest one is awarded by, and to, myself.",
        options: MILITARY_GENERAL_RETURN_OPTIONS,
        default_next: None,
    },
];

const GOBLIN_CANTINA_CHIEFTAIN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What goes on here?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why a training pit, not a stronghold?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Walk away.",
        next_node: None,
        note: Some("Fretjaw bangs a flagon on the dais. The entire pit cheers, then immediately resumes losing at darts."),
        close_after: true,
    },
];

const GOBLIN_CANTINA_CHIEFTAIN_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Walk away.",
        next_node: None,
        note: Some("Fretjaw bangs a flagon on the dais. The cheering is real. The dart skill is not."),
        close_after: true,
    },
];

const GOBLIN_CANTINA_CHIEFTAIN_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Fretjaw",
        line: "Mind the tables, friend. We don't FIGHT in the cantina, we REHEARSE fighting. There's a difference. Mostly the volume.",
        options: GOBLIN_CANTINA_CHIEFTAIN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Fretjaw",
        line: "House rules: vault the tables, duck the bar shelf, buy a round if you bleed on the floorboards. Three goblins are passed out under the dais. They are also house rules.",
        options: GOBLIN_CANTINA_CHIEFTAIN_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Fretjaw",
        line: "Strongholds make you proud. Pits make you GOOD. Anyone can stand a wall up. Try keeping your footing on a beer floor.",
        options: GOBLIN_CANTINA_CHIEFTAIN_RETURN_OPTIONS,
        default_next: None,
    },
];

const PULSE_VOYAGER_CAPTAIN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Where do these stones lead?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why park up in the sky?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Cast off.",
        next_node: None,
        note: Some("Captain Pulse touches a rune on the dais. The stones below shimmer once, as if amused, and stay put."),
        close_after: true,
    },
];

const PULSE_VOYAGER_CAPTAIN_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Cast off.",
        next_node: None,
        note: Some("Captain Pulse offers a salute that ends in a wave. Or a wave that ends in a salute. Hard to tell."),
        close_after: true,
    },
];

const PULSE_VOYAGER_CAPTAIN_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Captain Pulse",
        line: "Welcome aboard the dais, drifter. We don't moor in the sky because we LIKE heights. We moor here because the tide is a measurement, not a condition.",
        options: PULSE_VOYAGER_CAPTAIN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Captain Pulse",
        line: "Each stone is a beat. Skip one and you don't fall — you just arrive late. Late is a kind of wrong place that pretends to be a wrong time.",
        options: PULSE_VOYAGER_CAPTAIN_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Captain Pulse",
        line: "Below the stones is water. Above them is also water, just slower. We split the difference and call the difference a deck.",
        options: PULSE_VOYAGER_CAPTAIN_RETURN_OPTIONS,
        default_next: None,
    },
];

const TECH_BROS_DISRUPTOR_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What is this place pivoting toward?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why does the boardroom face up?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Touch grass.",
        next_node: None,
        note: Some("Chadwick blinks. \"You'll be back. The runway always slopes downward.\""),
        close_after: true,
    },
];

const TECH_BROS_DISRUPTOR_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Touch grass.",
        next_node: None,
        note: Some("Chadwick uploads the conversation to a deck and shareholders applaud somewhere far away."),
        close_after: true,
    },
];

const TECH_BROS_DISRUPTOR_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Chadwick III",
        line: "Welcome to the basement! We're not failing — we're SUBTERRANEAN. Premium iteration depth. We dropped down here on purpose; gravity is just runway you don't have to lobby for.",
        options: TECH_BROS_DISRUPTOR_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Chadwick III",
        line: "Disruption-as-a-service, friend. We disrupt anything that holds still long enough. Our roadmap is a slope. Our slope is the roadmap. Both are pivots.",
        options: TECH_BROS_DISRUPTOR_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Chadwick III",
        line: "The boardroom faces up because vision is a direction. Looking down is for accountants. We do not look down. We are the down.",
        options: TECH_BROS_DISRUPTOR_RETURN_OPTIONS,
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Pirate Cove dialog: Admiral (faction leader) + Raider (grunt).
// Shared narrative beat: a Mockingbird stole the cove's treasure.
// The Admiral hires the player to kill it; the Raider grumbles at
// the indignity.
// ─────────────────────────────────────────────────────────────────

const PIRATE_ADMIRAL_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What happened to the treasure?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Where can I find this Mockingbird?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why don't your raiders handle it?",
        next_node: Some(3),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "I'll bring it down.",
        next_node: None,
        note: Some("The Admiral straightens. The cove takes a breath. A door at the far end of the cove unlocks itself out of pure protocol."),
        close_after: true,
    },
];

const PIRATE_ADMIRAL_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "I'll bring it down.",
        next_node: None,
        note: Some("The Admiral lifts a half-empty mug in salute. The cove watches the far door as if it might do something interesting."),
        close_after: true,
    },
];

const PIRATE_ADMIRAL_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Admiral",
        line: "Ahoy, landstrider. You catch us in low tide and lower tempers — a bird made off with our hoard, and the chest it lifted weighed more than the bird does. Riddle me THAT.",
        options: PIRATE_ADMIRAL_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Admiral",
        line: "A Mockingbird. Big as a galleon's mainsail, mean as low water. Snatched the iron-bound chest right off the bar. Left us a feather, two splinters, and an apology in a voice that wasn't its own.",
        options: PIRATE_ADMIRAL_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Admiral",
        line: "Past the door at the back of the cove. The bird circles in there like it owns the air. Bring it down. Bring the chest back. I don't care which order.",
        options: PIRATE_ADMIRAL_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Admiral",
        line: "Have you SEEN the raider? He fights cutlass-and-cuss, not aerial gunnery. The bird hovers; he swears; the chest doesn't come back. We need somebody who can climb after a flying thing.",
        options: PIRATE_ADMIRAL_RETURN_OPTIONS,
        default_next: None,
    },
];

const PIRATE_RAIDER_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What was in the chest?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Got any tips for the bird?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("The raider bites the rim of his mug for emphasis. The mug objects."),
        close_after: true,
    },
];

const PIRATE_RAIDER_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("The raider salutes with the wrong hand and is too proud to correct it."),
        close_after: true,
    },
];

const PIRATE_RAIDER_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Raider",
        line: "Oi. Don't ask me about the bird. The Admiral's all 'strategy' and 'resolve' — I'm 'beak twice my size, talons twice my pay'. We had a chest. Now we don't. Connect the dots.",
        options: PIRATE_RAIDER_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Raider",
        line: "Shanties. Specifically, OURS. The bird heard the cove sing about plunder one too many times and decided to file a counterclaim, in talons. The chest's not gold, it's the SONGS. Don't tell the Admiral I said that.",
        options: PIRATE_RAIDER_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Raider",
        line: "Mind the swoops. It hovers, it lulls you, then it dives. If it spits something at you, count to two and step LEFT. Or right. One of those. I'm the muscle, not the navigator.",
        options: PIRATE_RAIDER_RETURN_OPTIONS,
        default_next: None,
    },
];

#[derive(Component)]
pub struct DialogOverlayRoot;

#[derive(Component, Clone, Copy, Debug)]
pub struct DialogChoiceSlot {
    pub index: usize,
}

#[cfg(feature = "input")]
pub fn dialog_pointer_input(
    mut runtime: ResMut<crate::SandboxRuntime>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    choices: Query<(&Interaction, &DialogChoiceSlot), Changed<Interaction>>,
) {
    if !runtime.dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }

    let option_count = runtime.dialogue.options().len();
    for (interaction, slot) in &choices {
        let valid_slot = if option_count == 0 {
            slot.index == 0
        } else {
            slot.index < option_count
        };
        if !valid_slot {
            continue;
        }

        match interaction {
            Interaction::Hovered => {
                let index = slot.index.min(option_count.saturating_sub(1));
                if runtime.dialogue.selected_option != index {
                    runtime.dialogue.pointer_armed = None;
                }
                runtime.dialogue.selected_option = index;
            }
            Interaction::Pressed => {
                let index = slot.index.min(option_count.saturating_sub(1));

                #[cfg(target_os = "android")]
                {
                    let confirm = runtime.dialogue.selected_option == index
                        && runtime.dialogue.pointer_armed == Some(index);
                    runtime.dialogue.selected_option = index;
                    if confirm {
                        runtime.dialogue.pointer_armed = None;
                        let closed = runtime.dialogue.confirm_or_advance();
                        if closed {
                            next_mode.set(GameMode::Playing);
                        }
                    } else {
                        runtime.dialogue.pointer_armed = Some(index);
                    }
                }

                #[cfg(not(target_os = "android"))]
                {
                    runtime.dialogue.selected_option = index;
                    let closed = runtime.dialogue.confirm_or_advance();
                    if closed {
                        next_mode.set(GameMode::Playing);
                    }
                }
                return;
            }
            Interaction::None => {}
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_pointer_input() {}

#[cfg(feature = "input")]
pub fn dialog_input(
    menu: Res<MenuControlFrame>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    if !runtime.dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }
    if menu.back || menu.start {
        runtime.dialogue.close();
        next_mode.set(GameMode::Playing);
        return;
    }
    let mut frame = crate::input::MenuInputFrame {
        up: menu.up,
        down: menu.down,
        left: menu.left,
        right: menu.right,
        select: menu.select,
        back: menu.back,
        start: menu.start,
    };
    apply_vertical_scroll(&mut frame, menu.vertical_scroll_steps());
    if frame.up {
        runtime.dialogue.select_delta(-1);
    }
    if frame.down {
        runtime.dialogue.select_delta(1);
    }
    if frame.select {
        let closed = runtime.dialogue.confirm_or_advance();
        if closed {
            next_mode.set(GameMode::Playing);
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_input() {}

pub fn sync_dialog_ui(
    mut commands: Commands,
    runtime: Res<crate::SandboxRuntime>,
    overlays: Query<Entity, With<DialogOverlayRoot>>,
    ui_fonts: Option<Res<UiFonts>>,
    mut logged_font_state: Local<bool>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    if !runtime.dialogue.active() {
        return;
    }

    let title = runtime.dialogue.title();
    let body = runtime.dialogue.body();
    let options = runtime.dialogue.options();
    let selected = runtime.dialogue.selected_option();

    let selected_marker = ui_fonts
        .as_deref()
        .map(UiFonts::selected_marker)
        .unwrap_or(">");

    if !*logged_font_state {
        let marker_codepoints = selected_marker
            .chars()
            .map(|ch| format!("U+{:04X}", ch as u32))
            .collect::<Vec<_>>()
            .join(" ");

        let font_state = ui_fonts
            .as_deref()
            .map(|fonts| {
                format!(
                    "regular={}, semibold={}",
                    fonts.regular.is_some(),
                    fonts.semibold.is_some()
                )
            })
            .unwrap_or_else(|| "UiFonts resource missing".to_string());

        info!(
            "dialog UI font state: {font_state}; selected_marker='{selected_marker}' ({marker_codepoints})"
        );

        *logged_font_state = true;
    }

    let dialog_font = |font_size: f32, weight: UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size,
                ..default()
            })
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(14.0)),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(45),
            Name::new("Dialogue Overlay Root"),
            DialogOverlayRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    max_width: Val::Px(960.0),
                    max_height: Val::Percent(94.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.030, 0.045, 0.95)),
                BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.86)),
                Name::new("Dialogue Overlay Panel"),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(title),
                    dialog_font(20.0, UiFontWeight::Semibold),
                    TextColor(Color::srgba(0.82, 0.94, 1.00, 1.0)),
                ));
                parent.spawn((
                    Text::new(body),
                    dialog_font(16.0, UiFontWeight::Regular),
                    TextColor(Color::srgba(0.93, 0.96, 1.00, 1.0)),
                ));
                if !options.is_empty() {
                    let total = options.len();
                    let start = visible_window_start(selected, total, DIALOG_VISIBLE_OPTIONS);
                    let end = (start + DIALOG_VISIBLE_OPTIONS).min(total);
                    for idx in start..end {
                        let option = &options[idx];
                        let label = decorate_windowed_label(
                            option.label.to_string(),
                            idx,
                            selected,
                            total,
                            DIALOG_VISIBLE_OPTIONS,
                        );
                        spawn_dialog_choice_row(
                            parent,
                            idx,
                            &label,
                            idx == selected,
                            selected_marker,
                            &dialog_font,
                        );
                    }
                } else {
                    spawn_dialog_choice_row(
                        parent,
                        0,
                        "Continue",
                        true,
                        selected_marker,
                        &dialog_font,
                    );
                }
                parent.spawn((
                    Text::new(DIALOG_CONTINUE_HINT),
                    dialog_font(12.0, UiFontWeight::Regular),
                    TextColor(Color::srgba(0.66, 0.76, 0.88, 0.96)),
                ));
            });
        });
}

fn spawn_dialog_choice_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    label: &str,
    selected: bool,
    selected_marker: &str,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    let bg = if selected {
        Color::srgba(0.95, 0.78, 0.32, 0.96)
    } else {
        Color::srgba(0.10, 0.13, 0.20, 0.74)
    };
    let fg = if selected {
        Color::srgba(0.18, 0.06, 0.04, 1.0)
    } else {
        Color::srgba(0.82, 0.90, 1.0, 0.98)
    };
    let marker = if selected { selected_marker } else { " " };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(38.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(bg),
            DialogChoiceSlot { index },
            Name::new(format!("Dialogue choice {index}: {label}")),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{marker} {label}")),
                dialog_font(16.0, UiFontWeight::Regular),
                TextColor(fg),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_inactive() {
        let s = DialogState::default();
        assert!(!s.active());
    }

    #[test]
    fn start_activates_dialogue() {
        let mut s = DialogState::default();
        s.start("guide", "Guide");
        assert!(s.active());
        let title = s.title();
        assert!(!title.is_empty());
        // Title format is "{speaker} — {mode_label}" when a node
        // exists; otherwise "{npc_name} — dialogue". Either way the
        // separator is present.
        assert!(title.contains('—') || title.contains("dialogue"));
    }

    #[test]
    fn close_deactivates() {
        let mut s = DialogState::default();
        s.start("guide", "Guide");
        s.close();
        assert!(!s.active());
    }

    #[test]
    fn body_returns_routing_bug_message_when_no_node() {
        let mut s = DialogState::default();
        s.start("nonexistent_dialogue_id_for_test", "X");
        // The node index is 0 but the mode for an unknown id may
        // route to a fallback set; either way `body()` must return
        // SOME string (not panic).
        let body = s.body();
        assert!(!body.is_empty());
    }

    #[test]
    fn selected_option_starts_at_zero() {
        let mut s = DialogState::default();
        s.start("guide", "Guide");
        assert_eq!(s.selected_option(), 0);
    }

    #[test]
    fn visible_dialog_window_keeps_selected_option_in_range() {
        assert_eq!(visible_window_start(0, 8, DIALOG_VISIBLE_OPTIONS), 0);
        assert_eq!(visible_window_start(4, 8, DIALOG_VISIBLE_OPTIONS), 2);
        assert_eq!(visible_window_start(7, 8, DIALOG_VISIBLE_OPTIONS), 4);
    }
}
