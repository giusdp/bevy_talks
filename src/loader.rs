//! Asset loader for screenplays with json format.
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
};

use crate::prelude::RawScreenplay;

/// Load screenplays from json assets.
#[derive(Default)]
pub struct ScreenplayLoader;

impl AssetLoader for ScreenplayLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let script_str = std::str::from_utf8(bytes)?;
            let raw_sp: RawScreenplay = ron::from_str(script_str)?;
            load_context.set_default_asset(LoadedAsset::new(raw_sp));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["screenplay.ron"]
    }
}

#[cfg(test)]
mod tests {}
