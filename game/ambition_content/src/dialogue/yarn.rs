//! The game's authored Yarn dialogue set — CONTENT data, evicted from the
//! engine core (R3.2: the engine ships no dialogue).
//!
//! One `.yarn` file per zone under `assets/dialogue/sandbox/`; the sources
//! are embedded and handed to `bevy_yarnspinner` IN MEMORY, so no asset-root
//! coupling remains and desktop / web / Android all load the same bytes.
//!
//! Single source of truth: [`yarn_spinner_plugin`] registers exactly
//! [`YARN_SOURCES`]; the `yarn_compile` integration test compiles exactly the
//! same set as one project (matching startup); [`known_dialogue_ids`] derives
//! the validator's accepted ids from the same texts. A new `.yarn` added here
//! is automatically covered by all three.

/// Every EXECUTABLE region of a `.yarn` file, as `(1-based line, body)`.
///
/// ⭐⭐ THE ONE RUST DEFINITION OF "WHAT THE INTERPRETER EVALUATES", so a guard
/// over authored dialogue asks about the same text the game runs. A `.yarn`
/// file is mostly SPOKEN LINES; only `<<…>>` is evaluated. Everything else is a
/// character talking, and a character may say anything — including the exact
/// spelling of a call.
///
/// ⛔⛔ THIS IS NOT A CONVENIENCE, IT IS THE FIX FOR A REAL CLASS OF DEFECT.
/// `kernel.yarn` has the Kernel Guide EXPLAIN a call in prose:
/// `boss_cleared("mockingbird") returned TRUE.` Three separate instruments
/// scanned whole files and each grew its own private prose heuristic instead —
/// measured 2026-09-05, they over-reported authored demand by 25% (`boss_cleared`
/// 5 raw / 3 executable, `quest_active` 3 / 1, generic `condition(` 10 / 8), and
/// a misspelling IN DIALOGUE could redden CI over text nothing evaluates: a
/// guard reporting a defect in the WRITING. ⇒ Region first, calls second. A
/// consumer that filters prose by recognising it has the rule backwards.
///
/// Regions do not span lines, so a stray `<<` in prose cannot swallow the lines
/// beneath it, and every hit carries a line an author can be pointed at.
pub fn executable_regions(text: &str) -> Vec<(usize, &str)> {
    let mut regions = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(open) = rest.find("<<") {
            let after = &rest[open + 2..];
            let Some(close) = after.find(">>") else {
                break;
            };
            regions.push((i + 1, after[..close].trim()));
            rest = &after[close + 2..];
        }
    }
    regions
}

/// `(logical name, source text)` for every Yarn file the game loads.
pub const YARN_SOURCES: &[(&str, &str)] = &[
    (
        "dialogue/sandbox/intro.yarn",
        include_str!("../../assets/dialogue/sandbox/intro.yarn"),
    ),
    (
        "dialogue/sandbox/kernel.yarn",
        include_str!("../../assets/dialogue/sandbox/kernel.yarn"),
    ),
    (
        "dialogue/sandbox/factions.yarn",
        include_str!("../../assets/dialogue/sandbox/factions.yarn"),
    ),
    (
        "dialogue/sandbox/cove.yarn",
        include_str!("../../assets/dialogue/sandbox/cove.yarn"),
    ),
    (
        "dialogue/sandbox/dojo.yarn",
        include_str!("../../assets/dialogue/sandbox/dojo.yarn"),
    ),
    (
        "dialogue/sandbox/symmetry.yarn",
        include_str!("../../assets/dialogue/sandbox/symmetry.yarn"),
    ),
    (
        "dialogue/sandbox/hall.yarn",
        include_str!("../../assets/dialogue/sandbox/hall.yarn"),
    ),
];

/// Registers Yarn Spinner with the game's dialogue set as IN-MEMORY sources
/// (no folder scan, no asset-root dependency — identical on desktop, web,
/// and Android).
#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> bevy_yarnspinner::prelude::YarnSpinnerPlugin {
    use bevy_yarnspinner::prelude::{YarnFile, YarnFileSource, YarnSpinnerPlugin};
    YarnSpinnerPlugin::with_yarn_sources(
        YARN_SOURCES
            .iter()
            .map(|(name, text)| YarnFileSource::InMemory(YarnFile::new(*name, *text))),
    )
}

fn yarn_title_ids(source: &'static str) -> impl Iterator<Item = &'static str> {
    source.lines().filter_map(|line| {
        let title = line.strip_prefix("title:")?.trim();
        (!title.is_empty()).then_some(title)
    })
}

/// Validator surface (the LDtk content validator reads this): every Yarn node
/// id `NpcSpawn.dialogue_id` may reference. Folds in the per-character
/// Hall-of-Characters dialogue ids declared in the catalog
/// (`hall_dialogue_id`), so authored `hall_<id>` nodes are accepted without a
/// second hand-maintained list — the catalog is their single source of truth.
pub fn known_dialogue_ids(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for (_, source) in YARN_SOURCES {
        for title in yarn_title_ids(source) {
            ids.push(title.to_string());
            if let Some((root, _)) = title.split_once("__") {
                ids.push(root.to_string());
            }
        }
    }
    ids.extend(
        catalog
            .data()
            .characters
            .values()
            .filter_map(|entry| entry.hall_dialogue_id.clone()),
    );
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod executable_region_tests {
    use super::executable_regions;

    /// The pair that names the whole point: the SAME spelling is invisible in
    /// prose and visible in a region. A guard reading whole files cannot tell
    /// these two lines apart, and one of them is a character talking.
    #[test]
    fn the_same_call_is_prose_in_one_line_and_a_call_in_the_next() {
        let spoken = "Kernel Guide: boss_cleared(\"not_a_boss\") returned TRUE.";
        let evaluated = "<<if boss_cleared(\"not_a_boss\")>>";

        assert!(
            executable_regions(spoken).is_empty(),
            "a character SAYING a call is not a call; scanning this line is how a \
             misspelling in DIALOGUE reddens CI over text nothing evaluates"
        );
        assert_eq!(
            executable_regions(evaluated),
            vec![(1, "if boss_cleared(\"not_a_boss\")")],
            "the identical spelling inside `<<…>>` IS evaluated and must be checked"
        );
    }

    /// Same pair for the generic verb — the one `kernel.yarn` actually explains
    /// in prose, and the sentence that made the app guard's first run report a
    /// defect that was not there.
    #[test]
    fn the_generic_verb_is_prose_when_a_guide_explains_it() {
        let spoken = "Guide: condition() reads the world-fact domain directly.";
        let evaluated = "<<if condition(\"world.flag_set\", \"lamp\")>>";

        assert!(executable_regions(spoken).is_empty());
        assert_eq!(
            executable_regions(evaluated).len(),
            1,
            "an evaluated `condition(...)` must survive the region filter however spaced"
        );
    }

    /// ⛔ A REGION MUST NOT SWALLOW THE LINES BENEATH IT. An unmatched `<<` in
    /// prose is a typo an author can make; if it ran to the next `>>` two lines
    /// down, this filter would hand a guard MORE prose than a whole-file scan.
    #[test]
    fn an_unclosed_marker_in_prose_does_not_swallow_the_lines_below() {
        let text = "Guide: I said <<loudly, and then\nhe left.\n<<if quest_active(\"a\")>>";
        assert_eq!(
            executable_regions(text),
            vec![(3, "if quest_active(\"a\")")],
            "only the well-formed region on line 3 is executable"
        );
    }

    /// Two regions on one line, and the line number is the author's.
    #[test]
    fn every_region_on_a_line_is_found_and_carries_that_line() {
        let text = "prose\n<<set $a to 1>> spoken between <<set $b to 2>>";
        assert_eq!(
            executable_regions(text),
            vec![(2, "set $a to 1"), (2, "set $b to 2")]
        );
    }
}
