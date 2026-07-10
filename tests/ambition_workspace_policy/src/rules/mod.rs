//! The declarative rule kinds. Each is a pure function
//! `(&Workspace, &Policy, &mut Report)` that appends a [`crate::Diagnostic`] per
//! violation. Repetitive, data-driven checks live here; one-off semantic
//! scanners live in [`crate::custom`].

pub mod dependency;
pub mod paths;
pub mod source_reference;
pub mod workspace_member;
