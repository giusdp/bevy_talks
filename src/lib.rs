use bevy::prelude::{AddAsset, App, Plugin};
use conversation::Conversation;
use loader::ConversationLoader;
use prelude::{ChoicePickedEvent, ChoicesReachedEvent, NextAction};

pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NextAction>()
            .add_event::<ChoicesReachedEvent>()
            .add_event::<ChoicePickedEvent>()
            .add_asset::<Conversation>()
            .init_asset_loader::<ConversationLoader>();
    }
}

pub mod conversation;
pub mod errors;
pub mod events;
pub mod loader;
pub mod prelude;
pub mod types;
