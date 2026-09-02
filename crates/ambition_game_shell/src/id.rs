//! Stable identifiers for shell routes, experiences, holds, and sequence segments.

use ambition_load::string_id;

string_id!(ShellRouteId);
string_id!(ShellExperienceId);
string_id!(ShellSegmentId);
string_id!(ShellSegmentKindId);
string_id!(ShellHoldId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShellActivationId(pub u64);
