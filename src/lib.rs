use bevy::prelude::{AddAsset, App, Plugin};
use conversation::Conversation;
use loader::ConversationLoader;
use prelude::{ChoicePickedEvent, ChoicesReachedEvent, NextDialogueEvent};

pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NextDialogueEvent>()
            .add_event::<ChoicesReachedEvent>()
            .add_event::<ChoicePickedEvent>()
            .add_asset::<Conversation>()
            .init_asset_loader::<ConversationLoader>();
    }
}

pub mod conversation;
pub mod dialogue_line;
pub mod errors;
mod events;
pub mod loader;
pub mod prelude;
pub mod talker;
