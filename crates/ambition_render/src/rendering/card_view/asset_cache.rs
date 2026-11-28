use bevy::asset::{Asset, Assets, Handle};

/// Hold a derived asset through a short gap in use.
pub(super) struct RecentAsset<A: Asset> {
    handle: Handle<A>,
    used: bool,
}

impl<A: Asset> RecentAsset<A> {
    pub(super) fn new(asset: A, assets: &mut Assets<A>) -> Self {
        Self { handle: assets.add(asset), used: true }
    }

    pub(super) fn get(&mut self) -> Handle<A> {
        self.used = true;
        self.handle.clone()
    }

    /// Keep the asset if a draw used it since the last pass.
    pub(super) fn retain(&mut self) -> bool {
        std::mem::take(&mut self.used)
    }
}
