//! bevy_talks is a Bevy plugin that provides
//! the basics to build and handle dialogues in games.
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
// Often exceeded by queries
#![allow(clippy::type_complexity)]
// Unhelpful for systems
#![allow(clippy::too_many_arguments)]

use bevy::prelude::{info, App, Commands, Entity, Plugin, Query, Update};
use prelude::ScreenplayNextAction;
use screenplay::Screenplay;

pub mod errors;
// pub mod loader;
pub mod prelude;
pub mod screenplay;
pub mod types;

/// The plugin that provides the basics to build and handle dialogues in games.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, next_action_request_handler);
    }
}

fn next_action_request_handler(
    mut commands: Commands,
    mut screenplays: Query<(Entity, &mut Screenplay, &ScreenplayNextAction)>,
) {
    for (e, mut sp, _) in screenplays.iter_mut() {
        info!("Requested next action for {:?} !", sp);
        let _ = sp.next_action();
        commands.entity(e).remove::<ScreenplayNextAction>();
    }
}

#[cfg(test)]
mod test {
    // use crate::{simulation::SimulationPlugin, world_gen::GenerationConfig};
    use bevy::prelude::*;

    use crate::{
        screenplay::{Screenplay, ScreenplayBuilder},
        types::ScreenplayNextAction,
        TalksPlugin,
    };

    /// Just [`MinimalPlugins`].
    pub fn minimal_app() -> App {
        let mut app = App::new();

        app.add_plugins(MinimalPlugins).add_plugins(TalksPlugin);

        app
    }

    #[test]
    fn next_action_request() {
        let mut app = minimal_app();

        let sp = ScreenplayBuilder::new()
            .add_action_node(Entity::PLACEHOLDER)
            .add_action_node(Entity::PLACEHOLDER)
            .build();

        let e = app.world.spawn((sp, ScreenplayNextAction)).id();

        app.update();

        let post_sp = app.world.get_mut::<Screenplay>(e).unwrap();
        let cn = post_sp.current_node.index();
        assert_eq!(cn, 1);

        // check that ScreenplayNextAction was removed
        assert!(!app.world.entity(e).contains::<ScreenplayNextAction>());
    }
}
