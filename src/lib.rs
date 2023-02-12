use bevy::prelude::{AddAsset, App, Plugin};
use loader::ConversationLoader;
use prelude::{ChoicePickedEvent, ChoicesReachedEvent, NextAction};
use screenplay::Screenplay;

pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NextAction>()
            .add_event::<ChoicesReachedEvent>()
            .add_event::<ChoicePickedEvent>()
            .add_asset::<Screenplay>()
            .init_asset_loader::<ConversationLoader>();
    }
}

pub mod errors;
pub mod events;
pub mod loader;
pub mod prelude;
pub mod screenplay;
pub mod types;
