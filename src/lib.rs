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

/// TODO: refactor in multiple systems
fn next_action_request_handler(
    mut next_requests: EventReader<ScreenplayNextActionRequest>,
    mut sp_comps: Query<(Entity, &mut Screenplay)>,
    mut sp_res: ResMut<ActiveScreenplay>,
) {
    for _ev in next_requests.iter() {
        if let Some(e) = sp_res.e {
            if let Ok((_, mut sp)) = sp_comps.get_mut(e) {
                match sp.next_action() {
                    Ok(()) => {
                        info!("Next action for Active Screenplay set!");
                        sp_res.changed = true;
                    }
                    Err(err) => {
                        error!("Next action in active screenplay could not be set: {}", err);
                    }
                }
            }
        } else {
            error!("Next Action Request received but no active screenplay set!");
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
        app.world.get_resource_mut::<ActiveScreenplay>().unwrap().e = Some(e);

        app.update();

        app.world.send_event(ScreenplayNextActionRequest);

        app.update();

        let sp_spawned = app.world.get::<Screenplay>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 1);
        assert_eq!(
            app.world
                .get_resource::<ActiveScreenplay>()
                .unwrap()
                .changed,
            true
        );
    }
}
