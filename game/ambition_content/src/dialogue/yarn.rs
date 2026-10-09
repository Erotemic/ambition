//! The game's authored Yarn dialogue set — CONTENT data, evicted from the
//! engine core (R3.2: the engine ships no dialogue).
//!
//! One `.yarn` file per zone under `assets/dialogue/sandbox/`; the sources
//! are embedded and handed to `bevy_yarnspinner` IN MEMORY, so no asset-root
//! coupling remains and desktop / web / Android all load the same bytes.
//!
//! Single source of truth: [`yarn_spinner_plugin`] registers exactly
//! [`yarn_sources`]; the `yarn_compile` integration test compiles exactly the
//! same set as one project (matching startup); [`known_dialogue_ids`] derives
//! the validator's accepted ids from the same texts. A new `.yarn` added here
//! is automatically covered by all three.

/// Every EXECUTABLE region of a `.yarn` file, as `(1-based line, body)`.
///
/// The one Rust definition of what the interpreter evaluates, so a guard over
/// authored dialogue checks the same text the game runs. A `.yarn` file is
/// mostly spoken lines; only `<<…>>` is evaluated. A character may say
/// anything, including the exact spelling of a call.
///
/// For example, `kernel.yarn` has the Kernel Guide explain a call in prose:
/// `boss_cleared("mockingbird") returned TRUE.` A scanner that reads whole
/// files over-reports authored demand, and a misspelling in dialogue could fail
/// CI over text nothing evaluates. Find regions first, then calls. A consumer
/// that filters prose by recognising it has the rule backwards.
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

/// Each Yarn file the game loads: its logical name, and its text when this
/// build embeds it (`static_content`: web, Android, a build without the source
/// tree). Otherwise the text is read off disk at startup, so a dialogue edit
/// costs a restart, not a rebuild (see [`crate::pack::source_text`]).
macro_rules! yarn_files {
    ($($file:literal),* $(,)?) => {
        #[cfg(feature = "static_content")]
        const YARN_FILES: &[(&str, Option<&'static str>)] = &[$(
            (
                concat!("dialogue/sandbox/", $file),
                Some(include_str!(concat!("../../assets/dialogue/sandbox/", $file))),
            ),
        )*];
        #[cfg(not(feature = "static_content"))]
        const YARN_FILES: &[(&str, Option<&'static str>)] =
            &[$((concat!("dialogue/sandbox/", $file), None),)*];
    };
}
yarn_files!("intro.yarn", "kernel.yarn", "factions.yarn", "cove.yarn", "dojo.yarn", "symmetry.yarn", "hall.yarn");

/// `(logical name, source text)` for every Yarn file the game loads, read
/// once per process.
pub fn yarn_sources() -> &'static [(&'static str, &'static str)] {
    static SOURCES: std::sync::OnceLock<Vec<(&'static str, &'static str)>> = std::sync::OnceLock::new();
    SOURCES.get_or_init(|| {
        YARN_FILES
            .iter()
            .map(|(name, embedded)| {
                let text: &'static str = Box::leak(crate::pack::source_text(name, *embedded).into_boxed_str());
                (*name, text)
            })
            .collect()
    })
}

/// Registers Yarn Spinner with the game's dialogue set as IN-MEMORY sources
/// (no folder scan, no asset-root dependency — identical on desktop, web,
/// and Android).
#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> bevy_yarnspinner::prelude::YarnSpinnerPlugin {
    use bevy_yarnspinner::prelude::{YarnFile, YarnFileSource, YarnSpinnerPlugin};
    YarnSpinnerPlugin::with_yarn_sources(
        yarn_sources()
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

/// The flags a `<<command "world.set_flag" "<id>" true>>` in the game's Yarn
/// sets, read from the executable regions only. The content validator counts
/// them as authored flags, as it counts a story-flag pickup.
pub fn dialogue_set_flags() -> Vec<String> {
    let mut flags: Vec<String> = yarn_sources()
        .iter()
        .flat_map(|(_, source)| executable_regions(source))
        .filter_map(|(_, body)| {
            let rest = body.strip_prefix("command")?.trim_start();
            let rest = rest.strip_prefix("\"world.set_flag\"")?.trim_start();
            let rest = rest.strip_prefix('"')?;
            Some(rest[..rest.find('"')?].to_string())
        })
        .collect();
    flags.sort_unstable();
    flags.dedup();
    flags
}

/// Validator surface (the LDtk content validator reads this): every Yarn node
/// id `NpcSpawn.dialogue_id` may reference. Folds in the per-character
/// Hall-of-Characters dialogue ids declared in the catalog
/// (`hall_dialogue_id`), so authored `hall_<id>` nodes are accepted without a
/// second hand-maintained list — the catalog is their single source of truth.
///
/// Exact titles only. The runtime starts a dialogue id exactly as named
/// (`DialogueNodeIndex::entry_node`), so the root of a `root__1` title is not
/// an id that can start unless a node is titled `root` too.
pub fn known_dialogue_ids(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for (_, source) in yarn_sources() {
        ids.extend(yarn_title_ids(source).map(str::to_string));
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

    /// A region must not swallow the lines beneath it. An unmatched `<<` in prose
    /// is a possible typo; if it ran to the next `>>` two lines down, this filter
    /// would give a guard more prose than a whole-file scan.
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
