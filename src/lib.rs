//! `bevy_talks` is a Bevy plugin that provides the basics to build and handle dialogues in games.

use aery::{prelude::*, tuple_traits::RelationEntries};
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
            .add_systems(
                PreUpdate,
                (
                    next_handler.pipe(error_logger),
                    choice_handler.pipe(error_logger),
                    refire_handler.pipe(error_logger),
                    set_has_started.after(next_handler),
                )
                    .in_set(TalksSet),
            );
    }
}

/// The `SystemSet` for the `TalksPlugin`.
#[derive(SystemSet, Debug, Default, Clone, PartialEq, Eq, Hash)]
struct TalksSet;

/// Logs errors from the other systems.
fn error_logger(In(result): In<Result<(), NextActionError>>) {
    if let Err(err) = result {
        error!("Error: {err}");
    }
}

/// Handles the `RefireNodeRequest` events. It will emit the events in the current node.
fn refire_handler(
    mut cmd: Commands,
    mut reqs: EventReader<RefireNodeRequest>,
    current_nodes: Query<(Entity, &Parent), With<CurrentNode>>,
    start: Query<Entity, With<StartNode>>,
    end: Query<Entity, With<EndNode>>,
    all_actors: Query<&Actor>,
    performers: Query<Relations<PerformedBy>>,
    emitters: Query<&dyn NodeEventEmitter>,
    type_registry: Res<AppTypeRegistry>,
    mut start_ev_writer: EventWriter<StartEvent>,
    mut end_ev_writer: EventWriter<EndEvent>,
) -> Result<(), NextActionError> {
    if let Some(event) = reqs.read().next() {
        for (current_node, talk_parent) in &current_nodes {
            let this_talk = talk_parent.get();
            // if this is the talk we want to advance
            if this_talk == event.talk {
                // send start event if we are at the start node
                maybe_emit_start_event(&start, current_node, &mut start_ev_writer, event.talk);

                // send end event if current node is an end node
                maybe_emit_end_event(&end, current_node, &mut end_ev_writer, event.talk);

                // grab the actors in the next node
                let actors_in_node = retrieve_actors(&performers, current_node, &all_actors);

                // emit the events in current node
                emit_events(
                    &mut cmd,
                    &emitters,
                    current_node,
                    &type_registry,
                    actors_in_node,
                );
                return Ok(());
            }
        }
        return Err(NextActionError::NoTalk);
    }
    Ok(())
}

/// Emits the start event if the current node is a start node.
#[inline]
pub(crate) fn maybe_emit_start_event(
    start: &Query<Entity, With<StartNode>>,
    current_node: Entity,
    start_ev_writer: &mut EventWriter<StartEvent>,
    requested_talk: Entity,
) {
    if start.get(current_node).is_ok() {
        start_ev_writer.send(StartEvent(requested_talk));
    }
}

/// Emit the end event if the current node is an end node.
#[inline]
pub(crate) fn maybe_emit_end_event(
    end: &Query<Entity, With<EndNode>>,
    next_node: Entity,
    end_ev_writer: &mut EventWriter<EndEvent>,
    requested_talk: Entity,
) {
    if end.get(next_node).is_ok() {
        end_ev_writer.send(EndEvent(requested_talk));
    }
}

/// Retrieves the actors connected to the given node.
#[inline]
pub(crate) fn retrieve_actors(
    performers: &Query<Relations<PerformedBy>>,
    next_node: Entity,
    all_actors: &Query<&Actor>,
) -> Vec<Actor> {
    let mut actors_in_node = Vec::<Actor>::new();
    if let Ok(actor_edges) = &performers.get(next_node) {
        for actor in actor_edges.targets(PerformedBy) {
            actors_in_node.push(all_actors.get(*actor).expect("Actor").clone());
        }
    }
    actors_in_node
}

/// Iterates over the `NodeEventEmitter` in the current node and emits the events.
#[inline]
pub(crate) fn emit_events(
    cmd: &mut Commands,
    emitters: &Query<&dyn NodeEventEmitter>,
    next_node: Entity,
    type_registry: &Res<AppTypeRegistry>,
    actors_in_node: Vec<Actor>,
) {
    if let Ok(emitters) = emitters.get(next_node) {
        let type_registry = type_registry.read();

        for emitter in &emitters {
            let emitted_event = emitter.make(&actors_in_node);

            let event_type_id = emitted_event.type_id();
            // The #[reflect] attribute we put on our event trait generated a new `ReflectEvent` struct,
            // which implements TypeData. This was added to MyType's TypeRegistration.
            let reflect_event = type_registry
                .get_type_data::<ReflectEvent>(event_type_id)
                .expect("Event not registered for event type")
                .clone();

            cmd.add(move |world: &mut World| {
                reflect_event.send(&*emitted_event, world);
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use bevy::ecs::{
        query::{ROQueryItem, WorldQuery},
        system::Command,
    };

    use indexmap::indexmap;

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

    /// Setup a talk with the given data, and send the first `NextActionRequest` event.
    /// Returns the app for further testing.
    #[track_caller]
    pub fn setup_and_next(talk_data: &TalkData) -> App {
        let mut app = talks_minimal_app();
        let builder = TalkBuilder::default().fill_with_talk_data(talk_data);
        BuildTalkCommand::new(app.world.spawn(Talk::default()).id(), builder).apply(&mut app.world);
        let (talk_ent, _) = single::<(Entity, With<Talk>)>(&mut app.world);
        let (edges, _) = single::<(Relations<FollowedBy>, With<CurrentNode>)>(&mut app.world);

        assert_eq!(edges.targets(FollowedBy).len(), 1);
        let start_following_ent = edges.targets(FollowedBy)[0];

        app.world.send_event(NextNodeRequest::new(talk_ent));
        app.update();

        let (next_e, _) = single::<(Entity, With<CurrentNode>)>(&mut app.world);
        assert_eq!(next_e, start_following_ent);
        app
    }

    #[test]
    fn refire_request_sends_events() {
        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), actors: vec!["actor_1".to_string()], ..default() }, // this will be a text node
        };
        let mut app = setup_and_next(&TalkData::new(script, vec![Actor::new("actor_1", "Actor")]));
        let evs = app.world.resource::<Events<TextNodeEvent>>();
        assert_eq!(evs.get_reader().read(evs).len(), 1);

        let (talk_ent, _) = single::<(Entity, With<Talk>)>(&mut app.world);
        app.world.send_event(RefireNodeRequest::new(talk_ent));
        app.update();

        let evs = app.world.resource::<Events<TextNodeEvent>>();
        assert_eq!(evs.get_reader().read(evs).len(), 2);
    }
}
