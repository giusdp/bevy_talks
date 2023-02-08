use bevy::prelude::{AddAsset, App, Plugin};
use conversation::Conversation;
use loader::ConversationLoader;

pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Conversation>()
            .init_asset_loader::<ConversationLoader>();
    }
}

pub mod conversation;
pub mod dialogue_line;
pub mod errors;
pub mod loader;
pub mod prelude;
pub mod talker;
