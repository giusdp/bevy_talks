//! Dialogue graph traversal systems.

use crate::prelude::*;
use aery::{prelude::*, tuple_traits::RelationEntries};
use bevy::prelude::*;

pub(crate) fn set_has_started(mut talks: Query<&mut Talk>, mut start_evs: EventReader<StartEvent>) {
    for event in start_evs.read() {
        let mut talk = talks.get_mut(event.0).expect("Talk");
        talk.has_started = true;
    }
}

pub(crate) fn next_handler(
    mut cmd: Commands,
    mut reqs: EventReader<NextNodeRequest>,
    current_nodes: Query<(Entity, &Parent, Relations<FollowedBy>), With<CurrentNode>>,
    start: Query<Entity, (With<StartNode>, With<CurrentNode>)>,
    end: Query<Entity, With<EndNode>>,
    all_actors: Query<&Actor>,
    performers: Query<Relations<PerformedBy>>,
    emitters: Query<&dyn NodeEventEmitter>,
    type_registry: Res<AppTypeRegistry>,
    mut start_ev_writer: EventWriter<StartEvent>,
    mut end_ev_writer: EventWriter<EndEvent>,
) -> Result<(), NextActionError> {
    for event in reqs.read() {
        for (current_node, talk_parent, edges) in &current_nodes {
            let this_talk = talk_parent.get();
            // if this is the talk we want to advance
            if this_talk == event.talk {
                // send start event if we are at the start node
                maybe_emit_start_event(&start, current_node, &mut start_ev_writer, event.talk);

                let followings = edges.targets(FollowedBy);

                let next_node = validate_next_node(&followings)?;

                // send end event if next node is an end node
                maybe_emit_end_event(&end, next_node, &mut end_ev_writer, event.talk);

                // grab the actors in the next node
                let actors_in_node = retrieve_actors(&performers, next_node, &all_actors);
                // move CurrentNode component to next node
                move_current(&mut cmd, current_node, next_node);
                // emit the events in the next node
                emit_events(
                    &mut cmd,
                    &emitters,
                    next_node,
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

/// Handles `ChooseActionRequest` events by updating the given Talk graph.
///
/// This function is a Bevy system that listens for `ChooseActionRequest` events.
/// It will move the current node of the given `Talk` to the one selected in the choose event.
pub(crate) fn choice_handler(
    mut cmd: Commands,
    mut reqs: EventReader<ChooseNodeRequest>,
    current_nodes: Query<(Entity, &Parent, Relations<FollowedBy>), With<CurrentNode>>,
    start: Query<Entity, (With<StartNode>, With<CurrentNode>)>,
    end: Query<Entity, With<EndNode>>,
    all_actors: Query<&Actor>,
    performers: Query<Relations<PerformedBy>>,
    emitters: Query<&dyn NodeEventEmitter>,
    type_registry: Res<AppTypeRegistry>,
    mut start_ev_writer: EventWriter<StartEvent>,
    mut end_ev_writer: EventWriter<EndEvent>,
) -> Result<(), NextActionError> {
    for event in reqs.read() {
        for (current_node, talk_parent, edges) in &current_nodes {
            let this_talk = talk_parent.get();
            // if this is the talk we want to advance
            if this_talk == event.talk {
                // send start event if we are at the start node
                maybe_emit_start_event(&start, current_node, &mut start_ev_writer, event.talk);

                let followings = edges.targets(FollowedBy);

                let next_node = validate_chosen_node(&followings, event.next)?;

                // send end event if next node is an end node
                maybe_emit_end_event(&end, next_node, &mut end_ev_writer, event.talk);

                // grab the actors in the next node
                let actors_in_node = retrieve_actors(&performers, next_node, &all_actors);
                // move CurrentNode component to next node
                move_current(&mut cmd, current_node, next_node);
                // emit the events in the next node
                emit_events(
                    &mut cmd,
                    &emitters,
                    next_node,
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

#[inline]
fn maybe_emit_start_event(
    start: &Query<Entity, (With<StartNode>, With<CurrentNode>)>,
    current_node: Entity,
    start_ev_writer: &mut EventWriter<StartEvent>,
    requested_talk: Entity,
) {
    if let Ok(_) = start.get(current_node) {
        start_ev_writer.send(StartEvent(requested_talk));
    }
}

#[inline]
fn maybe_emit_end_event(
    end: &Query<Entity, With<EndNode>>,
    next_node: Entity,
    end_ev_writer: &mut EventWriter<EndEvent>,
    requested_talk: Entity,
) {
    if let Ok(_) = end.get(next_node) {
        end_ev_writer.send(EndEvent(requested_talk));
    }
}

#[inline]
fn move_current(cmd: &mut Commands<'_, '_>, current_node: Entity, next_node: Entity) {
    cmd.entity(current_node).remove::<CurrentNode>();
    cmd.entity(next_node).insert(CurrentNode);
}

#[inline]
fn retrieve_actors(
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

#[inline]
fn emit_events(
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

#[inline]
fn validate_next_node(followings: &[Entity]) -> Result<Entity, NextActionError> {
    if followings.len() > 1 {
        return Err(NextActionError::ChoicesNotHandled);
    } else if followings.is_empty() {
        return Err(NextActionError::NoNextAction);
    }

    Ok(followings[0])
}

fn validate_chosen_node(
    followings: &[Entity],
    chosen_node: Entity,
) -> Result<Entity, NextActionError> {
    if !followings.contains(&chosen_node) {
        return Err(NextActionError::BadChoice);
    }

    Ok(chosen_node)
}

#[cfg(test)]
mod tests {

    use crate::{
        prelude::Action,
        tests::{single, talks_minimal_app},
    };
    use bevy::ecs::system::Command;
    use indexmap::indexmap;

    use super::*;

    /// Setup a talk with the given data, and send the first `NextActionRequest` event.
    /// Returns the app for further testing.
    #[track_caller]
    fn setup_and_next(talk_data: &TalkData) -> App {
        let mut app = talks_minimal_app();
        let builder = TalkBuilder::default().fill_with_talk_data(talk_data);
        BuildTalkCommand::new(app.world.spawn_empty().id(), builder).apply(&mut app.world);
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
    fn next_request_moves_current_node_marker() {
        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), ..default() },
        };
        setup_and_next(&TalkData::new(script, vec![]));
    }

    #[test]
    fn text_event_from_text_node() {
        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), ..default() }, // this will be a text node
        };
        let app = setup_and_next(&TalkData::new(script, vec![]));
        let evs = app.world.resource::<Events<TextNodeEvent>>();
        assert!(evs.len() > 0);
    }

    #[test]
    fn text_event_with_actors_from_text_node() {
        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), actors: vec!["actor_1".to_string()], ..default() }, // this will be a text node
        };
        let app = setup_and_next(&TalkData::new(script, vec![Actor::new("actor_1", "Actor")]));
        let evs = app.world.resource::<Events<TextNodeEvent>>();
        assert!(evs.get_reader().read(evs).next().unwrap().actors.len() > 0);
    }

    #[test]
    fn join_event_from_join_node() {
        let script = indexmap! {
            0 => Action { kind: NodeKind::Join, actors: vec!["actor_1".to_string()], ..default() }, // this will be a join node
        };
        let app = setup_and_next(&TalkData::new(script, vec![Actor::new("actor_1", "Actor")]));
        let evs = app.world.resource::<Events<JoinNodeEvent>>();
        assert!(evs.len() > 0);
        assert!(evs.get_reader().read(evs).next().unwrap().actors.len() > 0);
    }

    #[test]
    fn leave_event_from_leave_node() {
        let script = indexmap! {
            0 => Action { kind: NodeKind::Leave, actors: vec!["actor_1".to_string()], ..default() }, // this will be a leave node
        };
        let app = setup_and_next(&TalkData::new(script, vec![Actor::new("actor_1", "Actor")]));
        let evs = app.world.resource::<Events<LeaveNodeEvent>>();
        assert!(evs.len() > 0);
        assert!(evs.get_reader().read(evs).next().unwrap().actors.len() > 0);
    }

    #[test]
    fn start_event_when_moving_from_start_node() {
        let script = indexmap! {
            1 => Action { text: "Hello".to_string(), ..default() },
        };
        let app = setup_and_next(&TalkData::new(script, vec![]));
        let evs = app.world.resource::<Events<StartEvent>>();
        assert!(evs.len() > 0);
    }

    #[test]
    fn end_event_when_reached_end_node() {
        let script = indexmap! {
           1 => Action { text: "Hello".to_string(), ..default() }, // this will be a text end node (no next)
        };
        let app = setup_and_next(&TalkData::new(script, vec![]));
        let evs = app.world.resource::<Events<EndEvent>>();
        assert!(evs.len() > 0);
    }

    #[test]
    fn choice_event_from_choice_node() {
        let script = indexmap! {
            1 => Action { choices: vec![
                ChoiceData {text: "Choice 1".to_string(), next: 2},
                ], kind: NodeKind::Choice, ..default() },
            2 => Action { text: "test".to_string(), ..default() },
        };
        let app = setup_and_next(&TalkData::new(script, vec![]));
        let evs = app.world.resource::<Events<ChoiceNodeEvent>>();
        assert!(evs.len() > 0);
    }

    #[test]
    fn test_choice_handler() {
        let script = indexmap! {
            1 => Action {  choices: vec![
                ChoiceData {text: "Choice 1".to_string(), next: 2},
                ChoiceData {text: "Choice 2".to_string(), next: 3}
                ], kind: NodeKind::Choice, ..default() },
            2 => Action { kind: NodeKind::Leave, ..default() },
            3 => Action { text: "test".to_string(), ..default() },
        };
        let mut app = setup_and_next(&TalkData::new(script, vec![]));
        let (t, _) = app.world.query::<(Entity, With<Talk>)>().single(&app.world);

        // check that next action does not work when there are choices
        app.world.send_event(NextNodeRequest::new(t));
        app.update();

        let (choice_node, _) = app
            .world
            .query::<(&ChoiceNode, With<CurrentNode>)>()
            .single(&app.world);

        app.world
            .send_event(ChooseNodeRequest::new(t, choice_node.0[0].next));
        app.update();

        assert!(app
            .world
            .query::<(&LeaveNode, With<CurrentNode>)>()
            .get_single(&app.world)
            .is_ok())
    }

    #[test]
    fn has_started_becomes_true() {
        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), ..default() }, // this will be a text node
        };
        let mut app = setup_and_next(&TalkData::new(script, vec![]));

        let talk = single::<&Talk>(&mut app.world);
        assert!(talk.has_started);
    }
}
