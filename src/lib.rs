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

use bevy::prelude::*;
use prelude::{ActiveScreenplay, ScreenplayNextActionRequest};
use screenplay::Screenplay;

pub mod errors;
pub mod prelude;
pub mod screenplay;
pub mod types;

/// The plugin that provides the basics to build and handle dialogues in games.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveScreenplay>()
            .add_event::<ScreenplayNextActionRequest>()
            .add_systems(Update, next_action_request_handler);
    }
}

fn next_action_request_handler(
    mut next_requests: EventReader<ScreenplayNextActionRequest>,
    mut sp_comps: Query<(Entity, &mut Screenplay)>,
    sp_res: Res<ActiveScreenplay>,
) {
    for _ev in next_requests.iter() {
        if let Some(e) = sp_res.0 {
            let a = sp_comps.get_mut(e);

            if let Ok((_, mut sp)) = a {
                info!("Requested next action for {:?} !", e);
                let _ = sp.next_action();
            }
        } else {
            info!("No active screenplay!");
        }
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::*;

    use crate::{
        screenplay::{Screenplay, ScreenplayBuilder},
        types::{ActiveScreenplay, ScreenplayNextActionRequest},
        TalksPlugin,
    };

    /// Just [`MinimalPlugins`].
    pub fn minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(TalksPlugin);
        app
    }

    #[test]
    fn next_action_request_handler() {
        let mut app = minimal_app();

        let sp = ScreenplayBuilder::new()
            .add_action_node(Entity::PLACEHOLDER)
            .add_action_node(Entity::PLACEHOLDER)
            .build();

        let e = app.world.spawn(sp).id();
        app.world.get_resource_mut::<ActiveScreenplay>().unwrap().0 = Some(e);

        app.update();

        app.world.send_event(ScreenplayNextActionRequest);

        app.update();

        let sp_spawned = app.world.get::<Screenplay>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 1);
    }
}
