//! `bevy_talks` is a Bevy plugin that provides the basics to build and handle dialogues in games.

use aery::prelude::*;
use bevy::prelude::*;

use prelude::*;
use ron_loader::loader::TalksLoader;
use traverse::{choice_handler, next_handler, set_has_started};

pub mod actors;
pub mod builder;
pub mod errors;
pub mod events;
pub mod prelude;
pub mod ron_loader;
pub mod talk;
pub mod talk_asset;
mod traverse;

/// The plugin that provides the basics to build and handle dialogues in games.
///
/// # Note
/// If you are using [Aery](https://crates.io/crates/aery), add it to the App before this plugin, or just add this plugin.
/// This plugin will add Aery if it's not in the app, since it is a unique plugin, having multiple will panic.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<Aery>() {
            app.add_plugins(Aery);
        }

        app.add_plugins(TalksEventsPlugin)
            .register_asset_loader(TalksLoader)
            .init_asset::<TalkData>()
            .configure_sets(PreUpdate, TalksSet)
            .add_systems(PreUpdate, next_handler.pipe(error_logger).in_set(TalksSet))
            .add_systems(
                PreUpdate,
                set_has_started.after(next_handler).in_set(TalksSet),
            )
            .add_systems(
                PreUpdate,
                choice_handler.pipe(error_logger).in_set(TalksSet),
            );
    }
}

#[derive(SystemSet, Debug, Default, Clone, PartialEq, Eq, Hash)]
struct TalksSet;

/// Logs errors from the other systems.
fn error_logger(In(result): In<Result<(), NextActionError>>) {
    if let Err(err) = result {
        error!("Error: {err}");
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::query::{ROQueryItem, WorldQuery};

    use super::*;

    /// A minimal Bevy app with the Talks plugin.
    pub fn talks_minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), TalksPlugin));
        app
    }

    #[inline]
    pub fn single<Q: WorldQuery>(world: &mut World) -> ROQueryItem<Q> {
        world.query::<Q>().single(world)
    }
}
