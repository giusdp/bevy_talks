use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
};

use crate::{conversation::Conversation, types::RawScript};

#[derive(Default)]
pub struct ConversationLoader;

impl AssetLoader for ConversationLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let script = serde_json::from_slice::<RawScript>(bytes)?;
            let asset = Conversation::new(script)?;
            load_context.set_default_asset(LoadedAsset::new(asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
