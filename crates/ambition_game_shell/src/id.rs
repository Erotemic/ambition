//! Stable identifiers for shell routes, experiences, holds, and sequence segments.

use std::fmt;

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

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

string_id!(ShellRouteId);
string_id!(ShellExperienceId);
string_id!(ShellSegmentId);
string_id!(ShellSegmentKindId);
string_id!(ShellHoldId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShellActivationId(pub u64);
