//! Stable load, barrier, and work identifiers.

/// One non-empty `String` newtype, with the identifier policy this project has
/// settled on three times independently: construction PANICS on a blank value,
/// `as_str`/`Display` read it back, `From<&str>`/`From<String>` build it, and it
/// is deliberately NOT serialisable.
///
/// ⛔ THIS MACRO EXISTED THREE TIMES — here, in `ambition_game_shell::id` and in
/// `ambition_load_presentation::model` — byte-identical modulo whitespace, over
/// eleven types. `ambition_load` owns it because the dependency graph says so:
/// this crate depends on `bevy` and on no other workspace crate, and both former
/// definers already depend on it, so consolidating adds no edge and no cycle.
///
/// ⛔ THE BODY MUST SPELL `::core::fmt` RATHER THAN `fmt`. An exported macro
/// expands at the CALL SITE, where a `use std::fmt;` may not exist — relying on
/// one is the difference between a macro that moves and a macro that only
/// appears to.
#[macro_export]
macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                let value = value.into();
                assert!(
                    !value.trim().is_empty(),
                    concat!(stringify!($name), " cannot be empty")
                );
                Self(value)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

string_id!(LoadId);
string_id!(LoadBarrierId);
string_id!(LoadWorkId);

#[cfg(test)]
mod string_id_tests {
    use super::LoadId;

    /// ⛔ `Display` MUST NOT QUOTE. The macro body used to read `self.0.fmt(f)`,
    /// which resolves to the trait being implemented — correct, but silently so.
    /// Spelling it `::core::fmt::Display::fmt` is what makes the export portable,
    /// and one wrong trait there would put quotes around every id.
    ///
    /// This is not hypothetical: `ambition_game_shell::router` builds route keys
    /// as `format!("shell.{}.{}", ..)`, so a `Debug` resolution would produce
    /// `shell."a"."b"` and no test in the workspace pinned it before this one.
    #[test]
    fn a_string_id_displays_as_its_str_without_quotes() {
        let id = LoadId::new("hall_of_characters");
        assert_eq!(id.to_string(), "hall_of_characters");
        assert_eq!(id.to_string(), id.as_str());
        assert_eq!(format!("shell.{id}"), "shell.hall_of_characters");
    }

    /// The identity policy the three copies silently agreed on, now stated once.
    #[test]
    fn a_blank_string_id_panics_rather_than_existing() {
        assert!(std::panic::catch_unwind(|| LoadId::new("   ")).is_err());
    }
}
