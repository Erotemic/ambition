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
    /// Third pirate variant — Quartermaster keeps the books and the
    /// powder, complains about both, and is the player's path to
    /// the (new) sky-lookout sandbox room above the cove.
    PirateQuartermaster,
    /// Lady pirate — Lookout. Stands on the cove ladder watching
    /// the sky; warns about the rider grudge from a different
    /// angle than the Quartermaster.
    PirateLookout,
    /// Lady pirate — Navigator. Lives on the sky lookout deck;
    /// gives the player the geography of the route, including the
    /// double-back orbits the riders fly.
    PirateNavigator,
    /// Pirate heavy — Broadside Bess. Cove gunner. Brash, loud,
    /// obsessed with firepower. Crew member NPC; the shark-rider
    /// "Iron Mary on Shark" in the sky is still hostile combat.
    PirateHeavyBroadsideBess,
    /// Pirate heavy — Iron Mary. Cove armorer. Stoic, terse, all
    /// about plating and salt-rust.
    PirateHeavyIronMary,
    /// Pirate heavy — Salt Annet. Oldest soul in the cove.
    /// Weathered, salty, half her vocabulary is "salt".
    PirateHeavySaltAnnet,
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
    "pirate_quartermaster",
    "pirate_lookout",
    "pirate_navigator",
    "pirate_heavy_broadside_bess",
    "pirate_heavy_iron_mary",
    "pirate_heavy_salt_annet",
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
            "pirate_quartermaster" => Self::PirateQuartermaster,
            "pirate_lookout" => Self::PirateLookout,
            "pirate_navigator" => Self::PirateNavigator,
            "pirate_heavy_broadside_bess" => Self::PirateHeavyBroadsideBess,
            "pirate_heavy_iron_mary" => Self::PirateHeavyIronMary,
            "pirate_heavy_salt_annet" => Self::PirateHeavySaltAnnet,
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
            Self::PirateQuartermaster => "pirate quartermaster",
            Self::PirateLookout => "pirate lookout",
            Self::PirateNavigator => "pirate navigator",
            Self::PirateHeavyBroadsideBess => "pirate heavy — broadside bess",
            Self::PirateHeavyIronMary => "pirate heavy — iron mary",
            Self::PirateHeavySaltAnnet => "pirate heavy — salt annet",
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
            Self::PirateQuartermaster => PIRATE_QUARTERMASTER_NODES,
            Self::PirateLookout => PIRATE_LOOKOUT_NODES,
            Self::PirateNavigator => PIRATE_NAVIGATOR_NODES,
            Self::PirateHeavyBroadsideBess => PIRATE_HEAVY_BROADSIDE_BESS_NODES,
            Self::PirateHeavyIronMary => PIRATE_HEAVY_IRON_MARY_NODES,
            Self::PirateHeavySaltAnnet => PIRATE_HEAVY_SALT_ANNET_NODES,
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
// Pirate Quartermaster — keeps the books and the powder. Lives in
// the cove next to the Admiral and the Raider, points the player
// at the sky lookout above the cove so the flying-shark combat is
// reachable as a sandbox test surface.
// ─────────────────────────────────────────────────────────────────

const PIRATE_QUARTERMASTER_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "How's the hold lookin'?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "What's above decks?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[step aside]",
        next_node: None,
        note: Some("Quartermaster nods and goes back to a ledger that nods back."),
        close_after: true,
    },
];

const PIRATE_QUARTERMASTER_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Aught else, master?",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[step aside]",
        next_node: None,
        note: Some("Quartermaster waves you off with the eraser end of a pencil."),
        close_after: true,
    },
];

const PIRATE_QUARTERMASTER_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Quartermaster",
        line: "Aye-aye, landstrider. I be the only soul in this here cove what can spell 'manifest' — th' Admiral signs it, the Raider DEVOURS it. Pray ye don't end up a Raider, savvy?",
        options: PIRATE_QUARTERMASTER_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Quartermaster",
        line: "Powder: low. Salt pork: lower. Rum: don't ask. Shanties: somehow runnin' a deficit since the bird flew off with the chest — we be tradin' IOUs for verses now, which is as seaworthy as a sieve in a squall.",
        options: PIRATE_QUARTERMASTER_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Quartermaster",
        line: "Up the riggin', through the hatch in the overhead — that's where the lookout's perched. Mind ye: the crew be in a black humor today. Every shark wears a rider with a grudge an' a flintlock, an' they fire before they ask yer name. Ye try splittin' six pork rations 'tween six pirates an' three on-fire sharks an' see how SWEET-tempered ye'd be.",
        options: PIRATE_QUARTERMASTER_RETURN_OPTIONS,
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Pirate Lookout — lady pirate at the foot of the cove ladder.
// Watches the sky for incoming sharks; second-source warning to
// match the Quartermaster's "crew is grumpy" beat from a different
// angle (eyes-on-the-sky vs ledger-side).
// ─────────────────────────────────────────────────────────────────

const PIRATE_LOOKOUT_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What's on the horizon?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Why be the crew so cross?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[climb past]",
        next_node: None,
        note: Some("She tips her cap with a knife — point first."),
        close_after: true,
    },
];

const PIRATE_LOOKOUT_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Aught else, sentry?",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[climb past]",
        next_node: None,
        note: Some("She rolls her eyes and goes back to scanning the cloud line."),
        close_after: true,
    },
];

const PIRATE_LOOKOUT_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Lookout",
        line: "Hist. Shark trouble brewin' aloft — riders been comin' about since first light. Same arc, then a sharp double-back when they reckon no one's watchin'. They reckon WRONG.",
        options: PIRATE_LOOKOUT_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Lookout",
        line: "Sky an' clouds, lubber. That cursed Mockingbird drops shanties from up yonder — riders dive at whatever twitches below. Mark me: when ye see one peel off the orbit an' reverse her heading, that be the dive — step PORT, smartly.",
        options: PIRATE_LOOKOUT_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Lookout",
        line: "Short rations, plain an' simple. Quartermaster halved the salt pork. Six pirates an' three on-fire sharks on half-portions? That be a grudge with a heartbeat, an' the riders bark loudest about it. Empty belly makes a sharp cutlass.",
        options: PIRATE_LOOKOUT_RETURN_OPTIONS,
        default_next: None,
    },
];

// ─────────────────────────────────────────────────────────────────
// Pirate Navigator — lady pirate on the sky-lookout deck. Lives
// upstairs with the sharks; explains the orbit pattern and the
// double-back to the player after they've climbed up.
// ─────────────────────────────────────────────────────────────────

const PIRATE_NAVIGATOR_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What's their course aloft?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "How d'ye dodge their broadside?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[ready your blade]",
        next_node: None,
        note: Some("She steps back so you have a clear line to the first orbit."),
        close_after: true,
    },
];

const PIRATE_NAVIGATOR_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "One more question, cap'n cartographer.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "[ready your blade]",
        next_node: None,
        note: Some("Navigator nods at the sky. 'Three of 'em. Mind the reverse.'"),
        close_after: true,
    },
];

const PIRATE_NAVIGATOR_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Navigator",
        line: "Three sharks, three riders, three sets o' teeth. They orbit on station at altitude 'til somethin' they fancy steps onto the planks — that be you, lubber — then they peel off and broadside. Here be the rub: every few breaths they come about and reverse the orbit, so ye can't lead 'em like a duck shoot.",
        options: PIRATE_NAVIGATOR_OPTIONS,
        default_next: None,
        },
    DialogNode {
        speaker: "Navigator",
        line: "Three roles aloft, picked at sail-out an' kept 'til death. HOVER holds the weather gauge an' fires. SWOOP dives every three breaths. RETREAT climbs an' stretches the cadence so ye lose the beat. An' atop all of it — the reverse. Watch the SHARK, not the rider; the rider just pulls the trigger.",
        options: PIRATE_NAVIGATOR_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Navigator",
        line: "Easy now — they don't fire on the navigator, see. Mockingbird made off with the chest, the shanties went short, the crew turned mean, and I drew the chart that says the cartographer eats first. Sit a spell if ye like; the sharks won't.",
        options: PIRATE_NAVIGATOR_RETURN_OPTIONS,
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

// ─────────────────────────────────────────────────────────────────
// Pirate heavies — three cove crew NPCs (Broadside Bess, Iron
// Mary, Salt Annet) who share the heavy-bruiser silhouette but
// each have their own bit. Authored as `NpcSpawn` so the player
// can press Interact to talk to them; the sky-rider "Iron Mary
// on Shark" in `pirate_sky_lookout` is still hostile combat and
// doesn't use these nodes.
// ─────────────────────────────────────────────────────────────────

const PIRATE_HEAVY_BROADSIDE_BESS_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "What's a 'broadside', exactly?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "How d'ye end up in this cove?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("Bess hefts her cleaver in a salute, knocks a lantern off a hook, swears like a thunderclap, and pretends it was on purpose."),
        close_after: true,
    },
];

const PIRATE_HEAVY_BROADSIDE_BESS_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Aye, another question.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("Bess bellows YO-HO at the ceiling. The ceiling, accustomed, does not flinch."),
        close_after: true,
    },
];

const PIRATE_HEAVY_BROADSIDE_BESS_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Broadside Bess",
        line: "YARRRGH — wee skipper! Mind the cleaver, mind the cleaver! Bess be the loudest gun on this here cove, an' I will out-shout, out-shoot, an' out-drink anyone who says different. The Admiral, bless 'im, signs me chits an' covers his ears.",
        options: PIRATE_HEAVY_BROADSIDE_BESS_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Broadside Bess",
        line: "A BROADSIDE, ye landlubber? 'Tis EVERY GUN on the port side firin' at once — boom, boom, boom-boom-boom — twelve cannons singin' the same hymn at the same wave. Yo-ho! When Bess fires a broadside, the SKY apologizes. The Mockingbird heard one once an' has been writin' diss-shanties about us ever since.",
        options: PIRATE_HEAVY_BROADSIDE_BESS_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Broadside Bess",
        line: "Story is — yarrr — I was THREE crews ago, gunner on the Stormpetrel, an' the captain says 'Bess, ye fire when I say.' I says 'I fire when the SHIP says.' Ship said now. Captain said never. Captain swam back to port. I sailed here. Cove ain't paid me proper since, but the Admiral lets me yell, so we be square.",
        options: PIRATE_HEAVY_BROADSIDE_BESS_RETURN_OPTIONS,
        default_next: None,
    },
];

const PIRATE_HEAVY_IRON_MARY_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Why 'Iron'?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Any tip for the bird?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("Iron Mary nods once. The nod is, somehow, also rust-flaked."),
        close_after: true,
    },
];

const PIRATE_HEAVY_IRON_MARY_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "Another, then.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("Mary taps her chestplate twice. It rings like a small bell with a grudge."),
        close_after: true,
    },
];

const PIRATE_HEAVY_IRON_MARY_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Iron Mary",
        line: "Hmmph. Iron Mary. Don't shake me hand, ye'll lose yer grip on it. Yarrgh. The Admiral keeps me 'round 'cause when the negotiatin' fails, I'm what the negotiatin' WAS about. Wee skipper, what brings ye to a cove full o' shouters?",
        options: PIRATE_HEAVY_IRON_MARY_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Iron Mary",
        line: "Plate. Rivets. Stubborn. Three reasons. Plate — me whole top half be hammered scrap salvaged off a sunk navy cutter, bolted to me leathers. Rivets — anything that ain't bolted shut, I bolted shut, twice. Stubborn — I'm older than this cove an' younger than the Admiral's grudges, an' that be a long stretch o' stubborn. Yo-ho.",
        options: PIRATE_HEAVY_IRON_MARY_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Iron Mary",
        line: "Aye. Bird's a mimic. Bird's a thief. Bird's a coward at close range — they always be. If ye can get a hand on it, ye don't NEED a sword. Yer fists, me fists — same arithmetic. But ye ain't got me fists, so bring somethin' heavy. An' don't listen to its voice; it'll wear yours.",
        options: PIRATE_HEAVY_IRON_MARY_RETURN_OPTIONS,
        default_next: None,
    },
];

const PIRATE_HEAVY_SALT_ANNET_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "How long ye been here?",
        next_node: Some(1),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Got a shanty for me?",
        next_node: Some(2),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("Salt Annet spits a precise arc over the rail and into the wash. A gull, paid, applauds."),
        close_after: true,
    },
];

const PIRATE_HEAVY_SALT_ANNET_RETURN_OPTIONS: &[DialogChoice] = &[
    DialogChoice {
        label: "One more, ye old salt.",
        next_node: Some(0),
        note: None,
        close_after: false,
    },
    DialogChoice {
        label: "Carry on.",
        next_node: None,
        note: Some("Annet waves a hand crusted in dried sea-spray. 'Pleasure was MINE,' she lies."),
        close_after: true,
    },
];

const PIRATE_HEAVY_SALT_ANNET_NODES: &[DialogNode] = &[
    DialogNode {
        speaker: "Salt Annet",
        line: "Yarrrr. Salt in me eyes, salt in me bones, salt in me bloody MORNIN' COFFEE — yo-ho, the cove gives an' the cove takes an' mostly the cove takes salt. Sit a spell, wee skipper. Old Annet's seen yer kind come an' go like tides — most go.",
        options: PIRATE_HEAVY_SALT_ANNET_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Salt Annet",
        line: "Too long, an' not long enough — yarr. The Admiral was a midshipman when I signed on; now he salutes me with both hands an' I salute him with neither. I've watched three ships rot at this very dock. The bird? The bird be a NEW kind o' weather. An' weather always passes, wee skipper. Always.",
        options: PIRATE_HEAVY_SALT_ANNET_RETURN_OPTIONS,
        default_next: None,
    },
    DialogNode {
        speaker: "Salt Annet",
        line: "(coughs, clears throat, sings) 'Yo-ho an' a barrel o' salt — yo-ho an' the captain's at fault — yo-ho an' the bird stole the chest — yo-ho an' we'll be GETTIN' it back — yarrgh.' Aye, I made the last verse up just now. Ye get what ye pay for in shanties, an' ye paid in a HALLO.",
        options: PIRATE_HEAVY_SALT_ANNET_RETURN_OPTIONS,
        default_next: None,
    },
];
