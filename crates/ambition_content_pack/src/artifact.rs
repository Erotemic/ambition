//! The loadable content envelope — fast-iteration packet I2, step 1.
//!
//! ⭐⭐ **THE ENVELOPE KNOWS NOTHING ABOUT MOVES, AND THAT IS THE DESIGN.** A
//! section's payload is opaque text here; the codec for a move section lives
//! beside `MoveSpec`, in the crate that owns it. The alternative — an envelope
//! that names every domain — makes this crate depend on each one and turns
//! "add a content family" into "edit the envelope", which is the shape the
//! packet's own step 1 tells us to avoid.
//!
//! ⛔⛔ **TWO VERSIONS, NOT ONE, AND THEY ARE NOT THE SAME QUESTION.** The
//! ENVELOPE version is "can this reader parse the outer shape at all"; a SECTION
//! version is "does this reader understand this family's payload". One number
//! would force a lockstep bump of every section whenever the envelope changed,
//! and — worse — would let a host that understands a newer envelope silently
//! misread an older section's payload as the shape it expects now.
//!
//! ⚠ **ADMISSION IS A REFUSAL LIST, NOT A PARSE.** `ron::from_str` succeeding
//! says the bytes were well-formed; it says nothing about whether this host can
//! honour what they ask for. Every rule in [`ContentArtifact::admit`] is a state
//! the format can EXPRESS and a host must not act on.

use serde::{Deserialize, Serialize};

/// The outer shape this module reads and writes.
///
/// ⛔ Bumped only when the ENVELOPE changes — the section list's own encoding.
/// A new content family is a new section kind, not a new envelope.
pub const ENVELOPE_VERSION: u32 = 1;

/// The most sections one artifact may carry.
///
/// ⚠ A BOUND, because "bounded lengths" is step 1's own words and an unbounded
/// list is a reader that allocates whatever a file says. It is generous on
/// purpose: this refuses a corrupt or hostile file, not a large game.
pub const MAX_SECTIONS: usize = 256;

/// One content family's data, as the envelope sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSection {
    /// WHICH family this is — a logical reference, never a Rust type name.
    ///
    /// ⛔ A type name would make the wire format a fact about the host's source
    /// layout, so renaming a struct would invalidate every artifact on disk.
    pub kind: String,
    /// The version of THIS family's payload encoding.
    pub section_version: u32,
    /// The family's own encoding, opaque here.
    pub payload: String,
}

/// A versioned bundle of content sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentArtifact {
    pub envelope_version: u32,
    pub sections: Vec<ArtifactSection>,
}

/// Why a well-formed artifact may not be acted on.
///
/// ⭐ EVERY VARIANT NAMES A STATE THE FORMAT CAN EXPRESS. A refusal for
/// something the format cannot represent would be a check that cannot fire.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactRefusal {
    #[error(
        "artifact envelope version {found} is not {expected}: this host cannot \
         be sure it is reading the outer shape the writer meant"
    )]
    EnvelopeVersion { found: u32, expected: u32 },
    #[error(
        "section `{kind}` is version {found} and this host understands at most \
         {supported}: a newer payload read as an older shape is a silent \
         misread, so it is refused rather than parsed"
    )]
    SectionTooNew {
        kind: String,
        found: u32,
        supported: u32,
    },
    #[error(
        "section kind `{kind}` appears {count} times: which one wins would be \
         decided by read order, and a partial write or a watcher firing mid-copy \
         is exactly how a mixed pack gets selected"
    )]
    DuplicateSection { kind: String, count: usize },
    #[error(
        "the artifact carries no sections at all — an empty pack admits cleanly \
         and replaces a host's content with nothing, which reads as success"
    )]
    Empty,
    #[error("{count} sections exceeds the bound of {max}")]
    TooManySections { count: usize, max: usize },
    #[error("section kind is empty: nothing can ask for it by name")]
    UnnamedSection,
}

impl ContentArtifact {
    /// A new artifact at this module's envelope version.
    pub fn new(sections: Vec<ArtifactSection>) -> Self {
        Self {
            envelope_version: ENVELOPE_VERSION,
            sections,
        }
    }

    /// Parse the outer shape. ⚠ WELL-FORMED IS NOT ADMISSIBLE — call
    /// [`Self::admit`] before acting on anything this returns.
    pub fn parse(ron_text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron_text)
    }

    /// Serialize to pretty RON. The first implementation favours clarity over
    /// compression, which is step 1's own instruction.
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// May this host act on it?
    ///
    /// `supported` answers "what is the newest version of this section kind I
    /// understand" — a host that does not know a kind at all passes `None` for
    /// it and the section is carried without being acted on, which is how a
    /// pack can hold a family this build did not compile.
    ///
    /// ⛔ EVERY VIOLATION IS RETURNED, not just the first: an author fixing a
    /// pack should fix it in one pass.
    pub fn admit(
        &self,
        supported: &dyn Fn(&str) -> Option<u32>,
    ) -> Vec<ArtifactRefusal> {
        let mut out = Vec::new();
        if self.envelope_version != ENVELOPE_VERSION {
            out.push(ArtifactRefusal::EnvelopeVersion {
                found: self.envelope_version,
                expected: ENVELOPE_VERSION,
            });
        }
        if self.sections.is_empty() {
            out.push(ArtifactRefusal::Empty);
        }
        if self.sections.len() > MAX_SECTIONS {
            out.push(ArtifactRefusal::TooManySections {
                count: self.sections.len(),
                max: MAX_SECTIONS,
            });
        }
        let mut seen: std::collections::BTreeMap<&str, usize> = Default::default();
        for section in &self.sections {
            if section.kind.is_empty() {
                out.push(ArtifactRefusal::UnnamedSection);
                continue;
            }
            *seen.entry(section.kind.as_str()).or_default() += 1;
            if let Some(supported_version) = supported(&section.kind) {
                if section.section_version > supported_version {
                    out.push(ArtifactRefusal::SectionTooNew {
                        kind: section.kind.clone(),
                        found: section.section_version,
                        supported: supported_version,
                    });
                }
            }
        }
        for (kind, count) in seen {
            if count > 1 {
                out.push(ArtifactRefusal::DuplicateSection {
                    kind: kind.to_string(),
                    count,
                });
            }
        }
        out
    }

    /// The one section of this kind, or `None`.
    ///
    /// ⚠ Only meaningful after [`Self::admit`] is empty — before that, "the one
    /// section" is a claim [`ArtifactRefusal::DuplicateSection`] exists to deny.
    pub fn section(&self, kind: &str) -> Option<&ArtifactSection> {
        self.sections.iter().find(|s| s.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(kind: &str, version: u32) -> ArtifactSection {
        ArtifactSection {
            kind: kind.to_string(),
            section_version: version,
            payload: "()".to_string(),
        }
    }

    /// Everything this host knows, for a fixture that is not about support.
    fn knows_everything(_kind: &str) -> Option<u32> {
        Some(u32::MAX)
    }

    #[test]
    fn an_artifact_round_trips_through_its_own_text() {
        let a = ContentArtifact::new(vec![section("moves", 1), section("rooms", 3)]);
        let back = ContentArtifact::parse(&a.to_ron().expect("serializes"))
            .expect("parses back");
        assert_eq!(a, back);
    }

    #[test]
    fn a_sound_artifact_is_admitted() {
        let a = ContentArtifact::new(vec![section("moves", 1)]);
        assert_eq!(a.admit(&knows_everything), vec![]);
    }

    /// ⛔⛔ THE ONE THAT READS AS SUCCESS. An empty pack admits cleanly and
    /// replaces a host's content with nothing — the same shape as every
    /// empty-corpus failure this repository keeps rediscovering.
    #[test]
    fn an_empty_artifact_is_refused_rather_than_admitted() {
        let a = ContentArtifact::new(vec![]);
        assert_eq!(a.admit(&knows_everything), vec![ArtifactRefusal::Empty]);
    }

    /// ⛔ A PARTIAL WRITE OR A WATCHER FIRING MID-COPY is how a mixed pack gets
    /// selected, and "which one wins" would otherwise be read order.
    #[test]
    fn two_sections_of_one_kind_are_refused() {
        let a = ContentArtifact::new(vec![section("moves", 1), section("moves", 1)]);
        assert_eq!(
            a.admit(&knows_everything),
            vec![ArtifactRefusal::DuplicateSection {
                kind: "moves".to_string(),
                count: 2
            }]
        );
    }

    /// ⭐ THE REASON THE TWO VERSIONS ARE SEPARATE. A newer payload read as the
    /// shape this host expects now is a SILENT misread, so it is refused.
    #[test]
    fn a_section_newer_than_this_host_understands_is_refused() {
        let a = ContentArtifact::new(vec![section("moves", 4)]);
        assert_eq!(
            a.admit(&|kind| (kind == "moves").then_some(2)),
            vec![ArtifactRefusal::SectionTooNew {
                kind: "moves".to_string(),
                found: 4,
                supported: 2
            }]
        );
    }

    /// ⭐ AND THE CONTROL: a kind this build does not know at all is CARRIED,
    /// not refused. A pack may hold a family this host did not compile, and
    /// refusing it would make every artifact build-specific.
    #[test]
    fn a_section_this_host_does_not_know_is_carried_rather_than_refused() {
        let a = ContentArtifact::new(vec![section("weather", 9)]);
        assert_eq!(a.admit(&|_| None), vec![]);
    }

    #[test]
    fn an_envelope_from_another_version_is_refused() {
        let mut a = ContentArtifact::new(vec![section("moves", 1)]);
        a.envelope_version = ENVELOPE_VERSION + 1;
        assert_eq!(
            a.admit(&knows_everything),
            vec![ArtifactRefusal::EnvelopeVersion {
                found: ENVELOPE_VERSION + 1,
                expected: ENVELOPE_VERSION
            }]
        );
    }

    /// ⛔ EVERY VIOLATION, NOT THE FIRST — an author fixes a pack in one pass.
    #[test]
    fn a_pack_with_several_problems_reports_all_of_them() {
        let mut a = ContentArtifact::new(vec![section("moves", 9), section("moves", 9)]);
        a.envelope_version = 99;
        let refusals = a.admit(&|_| Some(1));
        // ⚠ THE SET, NOT THE COUNT. Both duplicated sections are also too new, so
        // the count is four — and asserting `3` here was my own first version,
        // which would have made a correct "report every violation" look wrong.
        // What the row claims is that no problem is HIDDEN by another.
        assert!(
            refusals
                .iter()
                .any(|r| matches!(r, ArtifactRefusal::EnvelopeVersion { .. })),
            "the envelope problem was hidden: {refusals:?}"
        );
        assert!(
            refusals
                .iter()
                .any(|r| matches!(r, ArtifactRefusal::SectionTooNew { .. })),
            "the version problem was hidden: {refusals:?}"
        );
        assert!(
            refusals
                .iter()
                .any(|r| matches!(r, ArtifactRefusal::DuplicateSection { .. })),
            "the duplicate was hidden: {refusals:?}"
        );
    }
}
