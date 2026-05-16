#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogMode {
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
    /// Post-treasure-return variant. Chosen by `redirect_post_quest_dialog`
    /// when the player walks up to the admiral after the mockingbird is
    /// dead. Carries the "thank you for the chest" beat so the admiral
    /// stops asking the player to go kill a bird that is, in fact,
    /// already on the floor of the arena.
    PirateAdmiralAfterTreasure,
    PirateRaider,
    /// Post-treasure-return variant for the raider, mirroring the
    /// admiral. Cheaper banter — the raider gets a different beat to
    /// keep the cove from feeling like one looping admiral.
    PirateRaiderAfterTreasure,
    /// Ninja-faction leader. Runs the Shadow Dojo and proposes a
    /// truce-of-convenience with the pirates so the two crews can
    /// settle the Mockingbird together. Dialog hangs on the rivalry
    /// (ninjas vs pirates) softening into a temporary alliance.
    NinjaLeader,
    /// Ninja-faction grunt. Trash-talks the pirates, then grudgingly
    /// admits the bird is the bigger problem.
    NinjaDuelist,
    /// Intro story content (Creator wake / final, Oiler, Gate Janitor,
    /// Framebreaker, Nazi salvage guard, news board, manifest kiosk).
    /// The wrapped enum lives in `crate::intro::dialog` so adding a
    /// new intro NPC doesn't churn this file.
    Intro(crate::intro::dialog::IntroDialog),
    Generic,
}

pub(crate) const KNOWN_DIALOGUE_IDS: &[&str] = &[
    "architect_intro",
    "vault_keeper",
    "merchant_seed",
    "hub_guide",
    "military_general",
    "goblin_cantina_chieftain",
    "pulse_voyager_captain",
    "tech_bros_disruptor",
    "pirate_admiral",
    "pirate_raider",
    "ninja_leader",
    "ninja_duelist",
    "generic_npc",
];

/// Aggregate of sandbox sandbox-owned dialogue ids plus story-content
/// ids contributed by sibling submodules (currently
/// [`crate::intro::dialog::intro_dialogue_ids`]). The validator
/// (`content_validation::validate_npc_dialogue_ids`) walks this list to
/// approve `NpcSpawn.dialogue_id` fields, so a content-only crate split
/// can keep ids local to its module while staying validation-clean.
pub(crate) fn known_dialogue_ids() -> Vec<&'static str> {
    let mut all = Vec::with_capacity(
        KNOWN_DIALOGUE_IDS.len() + crate::intro::dialog::intro_dialogue_ids().len(),
    );
    all.extend(KNOWN_DIALOGUE_IDS.iter().copied());
    all.extend(crate::intro::dialog::intro_dialogue_ids().iter().copied());
    all
}

impl DialogMode {
    pub(crate) fn from_dialogue_id(dialogue_id: &str) -> Self {
        // Intro story dialogue dispatches first so adding intro ids
        // doesn't require touching this match. Sandbox-owned ids
        // remain authoritative; the Generic fallback stays last.
        if let Some(intro) = crate::intro::dialog::IntroDialog::from_dialogue_id(dialogue_id) {
            return Self::Intro(intro);
        }
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
            "ninja_leader" => Self::NinjaLeader,
            "ninja_duelist" => Self::NinjaDuelist,
            "generic_npc" => Self::Generic,
            _ => Self::Generic,
        }
    }

    pub(in crate::dialog) fn label(self) -> &'static str {
        match self {
            Self::Architect => "architecture dialogue",
            Self::VaultKeeper => "merchant / persistence seed",
            Self::MerchantSeed => "merchant design sketch",
            Self::HubGuide => "central hub guidance",
            Self::MilitaryGeneral => "military faction leader",
            Self::GoblinCantinaChieftain => "goblin cantina chieftain",
            Self::PulseVoyagerCaptain => "pulse voyager captain",
            Self::TechBrosDisruptor => "tech-bros disruptor",
            Self::PirateAdmiral | Self::PirateAdmiralAfterTreasure => "pirate admiral",
            Self::PirateRaider | Self::PirateRaiderAfterTreasure => "pirate raider",
            Self::NinjaLeader => "ninja shadow oni leader",
            Self::NinjaDuelist => "ninja shadow duelist",
            Self::Intro(intro) => intro.label(),
            Self::Generic => "sandbox dialogue",
        }
    }

    pub(in crate::dialog) fn nodes(self) -> &'static [DialogNode] {
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
            Self::PirateAdmiralAfterTreasure => PIRATE_ADMIRAL_AFTER_TREASURE_NODES,
            Self::PirateRaider => PIRATE_RAIDER_NODES,
            Self::PirateRaiderAfterTreasure => PIRATE_RAIDER_AFTER_TREASURE_NODES,
            Self::NinjaLeader => NINJA_LEADER_NODES,
            Self::NinjaDuelist => NINJA_DUELIST_NODES,
            Self::Intro(intro) => intro.nodes(),
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
        note: Some(
            "The General returns the salute with surgical precision and twelve audible clicks.",
        ),
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
        note: Some(
            "The General returns the salute with surgical precision and twelve audible clicks.",
        ),
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
        note: Some(
            "Fretjaw bangs a flagon on the dais. The cheering is real. The dart skill is not.",
        ),
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

// Post-defeat / post-return cove dialog. Lights up when
// `redirect_post_quest_dialog` swaps the dialog mode after the
// mockingbird's encounter state flips to `Cleared`. The chest may or
// may not have been physically retrieved by the player yet; the
// `quest::PIRATE_TREASURE_REWARD_FLAG` flag tracks whether the
// admiral has actually paid out, but at the conversation level, the
// admiral's mood changes the moment the bird stops being a problem.

const PIRATE_ADMIRAL_AFTER_TREASURE_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "About the chest…",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Anything else?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on, Admiral.",
        next_node: None,
        note: Some("The Admiral lifts a fresh mug. The cove, for once, looks like the inside of a cove and not the outside of a grievance."),
        close_after: true,
    },
];

const PIRATE_ADMIRAL_AFTER_TREASURE_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on, Admiral.",
        next_node: None,
        note: Some("The Admiral salutes with the right hand. Possibly the only correct salute the cove has filed all season."),
        close_after: true,
    },
];

const PIRATE_ADMIRAL_AFTER_TREASURE_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Admiral",
        line: "Landstrider returns. The bird is feathers and the chest is OURS again — I owe you a hold full of thanks and a small fortune in goods. The cove sleeps tonight without one ear cocked at the sky.",
        options: PIRATE_ADMIRAL_AFTER_TREASURE_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Admiral",
        line: "Iron-bound, salt-cured, three locks the Mockingbird picked with a beak — proves the bird had hands somewhere a respectable bird shouldn't. Whatever was inside is yours; whatever's in the cove's books is square.",
        options: PIRATE_ADMIRAL_AFTER_TREASURE_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Admiral",
        line: "Stay sharp out there. There's always another bird, another cove, another bar that thinks our shanties belong to them. If something needs hunting, we know where to find you now.",
        options: PIRATE_ADMIRAL_AFTER_TREASURE_RETURN_OPTIONS,
        default_next: None,
    },
];

const PIRATE_RAIDER_AFTER_TREASURE_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What's the cove drinking to tonight?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Sing me a shanty.",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("The raider toasts with a mug that, against all odds, has stayed full for an entire conversation."),
        close_after: true,
    },
];

const PIRATE_RAIDER_AFTER_TREASURE_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("The raider salutes with the wrong hand. Old habits."),
        close_after: true,
    },
];

const PIRATE_RAIDER_AFTER_TREASURE_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Raider",
        line: "There they are. The bird-killer. I owe you a drink and the Admiral owes you a fortune. I'll let you collect from the one that pays better.",
        options: PIRATE_RAIDER_AFTER_TREASURE_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Raider",
        line: "You. Loud. The bird, who's not here. The bar, who's still here. And the chest, which is BACK. We do not toast the chest — the chest toasts itself. That's just respect.",
        options: PIRATE_RAIDER_AFTER_TREASURE_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Raider",
        line: "Yo-ho, the Mockingbird's done / Yo-ho, the songs are won / Yo-ho, the chest is RETURNED / and the chest is the actual ledger of every yo-ho ever filed. ...you don't need the third verse.",
        options: PIRATE_RAIDER_AFTER_TREASURE_RETURN_OPTIONS,
        default_next: None,
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

// ─────────────────────────────────────────────────────────────────
// Ninja Shadow Dojo dialog: Leader (Shadow Oni) + Duelist (grunt).
// Shared narrative beat: ninjas and pirates loathe each other on
// principle, but the Mockingbird is everybody's problem and the
// Leader is willing to broker a one-engagement truce. The Duelist
// grumbles about it the way a soldier grumbles about a peace treaty
// they secretly agree with.
// ─────────────────────────────────────────────────────────────────

const NINJA_LEADER_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Why team up with the pirates?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "What's your grievance with the bird?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Where does the dojo stand on the cove?",
        next_node: Some(3),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Open the arena door.",
        next_node: None,
        note: Some("The Leader inclines her horned mask. Somewhere in the rafters, a paper lantern unrolls a sigil that wasn't there a moment ago. The far door slides aside."),
        close_after: true,
    },
];

const NINJA_LEADER_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Open the arena door.",
        next_node: None,
        note: Some("The Leader's blade rests an inch out of its saya. The dojo holds its breath in the way only a room of disciplined breathers can."),
        close_after: true,
    },
];

const NINJA_LEADER_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Shadow Oni",
        line: "Stranger. You walked through our doorway without my permission, which means either you are very rude or very good. The Mockingbird stole a song that wasn't theirs to sing. We will end them. Even if it means standing shoulder to shoulder with a pirate.",
        options: NINJA_LEADER_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Shadow Oni",
        line: "We are knives in the dark; the pirates are shouts in broad daylight. We don't share a worldview — we share a target. The bird mocks our katas as fluently as it mocks their shanties. A loud enemy and a quiet enemy still bury the same corpse.",
        options: NINJA_LEADER_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Shadow Oni",
        line: "It learned a kata. ONE of ours. Not perfectly — but the bird performed it back at us in a courtyard at dawn, with our own footwork, and laughed. A bird does not laugh. A bird that does is no longer a bird; it is an insult that flies.",
        options: NINJA_LEADER_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Shadow Oni",
        line: "Tell the Admiral the dojo will hold position on the eastern thermals. We will not steal his songs and he will not steal our shadows. The bird falls. After that, we go back to politely loathing each other from a distance.",
        options: NINJA_LEADER_RETURN_OPTIONS,
        default_next: None,
    },
];

const NINJA_DUELIST_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Heard you're working with pirates now.",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Any tips for the bird?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("The duelist resumes a kata that consists mostly of stepping on a single tile in five slightly different ways."),
        close_after: true,
    },
];

const NINJA_DUELIST_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Ask another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("The duelist nods once. The nod is also a kata."),
        close_after: true,
    },
];

const NINJA_DUELIST_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Duelist",
        line: "Quiet. The Leader is meditating, which means anyone who slams a door dies. Don't slam the door. ...You walked in without slamming. Fine. Acceptable. State your business.",
        options: NINJA_DUELIST_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Duelist",
        line: "Working WITH them. Not joining. There's a distinction. A pirate has never washed his hands in his life, and a ninja is mostly hands. The arithmetic is ugly. But the bird is uglier. After the bird's down, we go back to being insulted by their cologne and their volume.",
        options: NINJA_DUELIST_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Duelist",
        line: "It mimics. Don't believe a sound it makes — your own voice in your own kata could come out of its beak. Strike on the silence between its calls. If you hear yourself, you're already losing.",
        options: NINJA_DUELIST_RETURN_OPTIONS,
        default_next: None,
    },
];
