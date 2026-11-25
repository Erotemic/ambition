//! A game can replace any default button row without an engine edit.

use bevy::prelude::{Query, Res, Resource};
use crate::{AxisBinding, Binding, BindingDefaults, BindingRecipe, InputParticipant, ParticipantId};
use crate::touch_state::MobileTouchState;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingLayout {
    pub rows: Vec<Binding>,
    /// None inherits the defaults; Some replaces the complete axis declaration.
    pub axes: Option<Vec<AxisBinding>>,
}

impl BindingLayout {
    pub fn new(rows: impl IntoIterator<Item = Binding>) -> Self {
        Self { rows: rows.into_iter().collect(), axes: None }
    }

    pub fn with_axes(mut self, axes: impl IntoIterator<Item = AxisBinding>) -> Self {
        self.axes = Some(axes.into_iter().collect());
        self
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct DeclaredBindingLayout {
    pub declared_by: String,
    pub layout: BindingLayout,
}

impl DeclaredBindingLayout {
    pub fn new(declared_by: impl Into<String>, layout: BindingLayout) -> Self {
        Self { declared_by: declared_by.into(), layout }
    }
    pub fn is_declared_by(&self, owner: &str) -> bool { self.declared_by == owner }
}

/// Propagate game policy changes to every participant. Removing a declaration
/// restores the host's defaults. The user's remaps are applied after this layer.
pub fn apply_active_binding_layout_to_recipes(
    declared: Option<Res<DeclaredBindingLayout>>,
    defaults: Option<Res<BindingDefaults>>,
    touch: Option<Res<MobileTouchState>>,
    mut recipes: Query<(&InputParticipant, &mut BindingRecipe)>,
) {
    let wanted = declared.as_ref().map(|d| &d.layout).cloned().unwrap_or_default();
    for (participant, mut recipe) in &mut recipes {
        let touch_enabled = touch.is_some() && participant.id == ParticipantId::PRIMARY;
        if recipe.touch != touch_enabled { recipe.touch = touch_enabled; }
        if recipe.layout != wanted { recipe.layout = wanted.clone(); }
        if let Some(defaults) = defaults.as_deref() {
            if recipe.defaults != *defaults { recipe.defaults = defaults.clone(); }
        }
    }
}
