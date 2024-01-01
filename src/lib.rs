#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
// Often exceeded by queries
#![allow(clippy::type_complexity)]
// Unhelpful for systems
#![allow(clippy::too_many_arguments)]

//! [`bevy_talks`] is a Bevy plugin that provides the basics to build and handle dialogues in games.

use aery::{prelude::*, tuple_traits::RelationEntries};
use bevy::prelude::*;
use prelude::*;
use ron_loader::loader::TalksLoader;

pub mod actors;
pub mod builder;
pub mod errors;
pub mod events;
pub mod prelude;
pub mod ron_loader;
pub mod talk;
pub mod talk_asset;
// pub mod talker;

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
        app.register_asset_loader(TalksLoader)
            .init_asset::<TalkData>()
            .add_event::<NextActionRequest>()
            .add_event::<ChooseActionRequest>()
            .add_systems(Update, next_handler.pipe(error_handler))
            .add_systems(Update, choice_handler.pipe(error_handler));
    }
}

/// Logs errors from the other systems.
fn error_handler(In(result): In<Result<(), NextActionError>>) {
    match result {
        Ok(_) => (),
        Err(err) => error!("Error: {err}"),
    }
}

/// Handles `ChooseActionRequest` events by updating the active Talk.
///
/// This function is a Bevy system that listens for `ChooseActionRequest` events.
/// It will move the current node of the given `Talk` to the one selected in the choose event.
fn choice_handler(
    mut commands: Commands,
    mut choose_requests: EventReader<ChooseActionRequest>,
    mut talks: Query<&mut Talk>,
    current_nodes: Query<(Entity, &Parent), With<CurrentNode>>,
    performers: Query<Relations<PerformedBy>>,
    actors: Query<&Actor>,
    node_kind_comps: Query<&NodeKind>,
    talk_comps: Query<&TalkText>,
    choices_comps: Query<&Choices>,
) -> Result<(), NextActionError> {
    let maybe_event = choose_requests.read().next();
    if maybe_event.is_none() {
        return Ok(());
    }
    let event_talk_ent = maybe_event.unwrap().talk;
    let event_choose_ent = maybe_event.unwrap().next;

    for (current_node, talk_parent) in &current_nodes {
        let talk_ent = talk_parent.get();
        // if this is the talk we want to advance
        if talk_ent == event_talk_ent {
            // move the current node component to the chosen one
            let next_node = move_current_node(&mut commands, current_node, event_choose_ent);
            let mut this_talk = talks.get_mut(talk_ent).unwrap();
            let next_kind = node_kind_comps.get(next_node).unwrap();
            reset_talk(&mut this_talk);
            set_node_kind(&mut this_talk, next_kind);
            set_text(next_node, &mut this_talk, next_kind, &talk_comps);
            set_actors(next_node, &mut this_talk, performers, actors);
            set_choices(next_node, next_kind, &mut this_talk, choices_comps)?;
            return Ok(());
        }
    }
    Err(NextActionError::NoTalk)
}

/// Handles `NextActionRequest` events by advancing the active Talk to the next action.
///
/// This function is a Bevy system that listens for `NextActionRequest` events.
/// It will move the current node of the given `Talk` to the next one.
fn next_handler(
    mut commands: Commands,
    mut next_requests: EventReader<NextActionRequest>,
    mut talks: Query<&mut Talk>,
    current_nodes: Query<(Entity, &Parent, Relations<FollowedBy>), With<CurrentNode>>,
    performers: Query<Relations<PerformedBy>>,
    actors: Query<&Actor>,
    node_kind_comps: Query<&NodeKind>,
    talk_comps: Query<&TalkText>,
    choices_comps: Query<&Choices>,
) -> Result<(), NextActionError> {
    let maybe_event = next_requests.read().next();
    if maybe_event.is_none() {
        return Ok(());
    }
    let event_talk_ent = maybe_event.unwrap().0;

    for (current_node, talk_parent, edges) in &current_nodes {
        let talk_ent = talk_parent.get();
        // if this is the talk we want to advance
        if talk_ent == event_talk_ent {
            let targets = edges.targets(FollowedBy);
            return match targets.len() {
                0 => Err(NextActionError::NoNextAction),
                1 => {
                    // move the current node component to the next one
                    let next_node = move_current_node(&mut commands, current_node, targets[0]);
                    let mut this_talk = talks.get_mut(talk_ent).unwrap();
                    let next_kind = node_kind_comps.get(next_node).unwrap();
                    reset_talk(&mut this_talk);
                    set_node_kind(&mut this_talk, next_kind);
                    set_text(next_node, &mut this_talk, next_kind, &talk_comps);
                    set_actors(next_node, &mut this_talk, performers, actors);
                    set_choices(next_node, next_kind, &mut this_talk, choices_comps)?;
                    Ok(())
                }
                2.. => Err(NextActionError::ChoicesNotHandled),
            };
        }
    }

    Err(NextActionError::NoTalk)
}

/// Reset the current Talk values.
fn reset_talk(talk: &mut Mut<'_, Talk>) {
    talk.current_text = "".to_string();
    talk.current_kind = NodeKind::Talk;
    talk.current_actors = Vec::new();
    talk.current_choices = Vec::new();
}

/// Update the current node kind
fn set_node_kind(talk: &mut Mut<'_, Talk>, next_kind: &NodeKind) {
    talk.current_kind = next_kind.clone();
}

/// Moves the current node component from the current node to the next one.
fn move_current_node(commands: &mut Commands<'_, '_>, current: Entity, next: Entity) -> Entity {
    commands.entity(current).remove::<CurrentNode>();
    commands.entity(next).insert(CurrentNode);
    next
}

/// Updates the current text of the active Talk based on the next node kind.
fn set_text(
    next_node: Entity,
    talk: &mut Mut<'_, Talk>,
    next_kind: &NodeKind,
    talk_comps: &Query<'_, '_, &TalkText>,
) {
    if next_kind == &NodeKind::Talk {
        let next_text = talk_comps.get(next_node).unwrap().0.clone();
        talk.current_text = next_text;
    }
}

/// Updates the current actors of the given Talk.
fn set_actors(
    next_node: Entity,
    talk: &mut Mut<'_, Talk>,
    performers: Query<Relations<PerformedBy>>,
    actors: Query<&Actor>,
) {
    let mut actor_names = Vec::<String>::new();
    for edges in &performers.get(next_node) {
        for performer_ent in edges.targets(PerformedBy) {
            let actor = actors.get(*performer_ent).unwrap();
            actor_names.push(actor.name.clone());
        }
    }
    talk.current_actors = actor_names;
}

/// Gets the choices from the next choice node and stores them in the given Talk.
fn set_choices(
    next_node: Entity,
    next_kind: &NodeKind,
    talk: &mut Mut<'_, Talk>,
    choices_comps: Query<&Choices>,
) -> Result<(), NextActionError> {
    if next_kind == &NodeKind::Choice {
        let choices = choices_comps
            .get(next_node)
            .map_err(|_| NextActionError::BadChoice)?;

        talk.current_choices = choices.0.clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::prelude::Action;
    use bevy::ecs::system::Command;
    use indexmap::indexmap;

    use super::*;

    /// A minimal Bevy app with the Talks plugin.
    pub fn minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));
        app
    }

    #[test]
    fn test_next_handler_with_talk_nodes() {
        let mut app = minimal_app();

        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), next: Some(2), ..default() },
            2 => Action { text: "Hello 2".to_string(), ..default() },
        };
        let mut talk_asset = TalkData::default();
        talk_asset.script = script;

        let builder = TalkBuilder::default().fill_from_talk_data(&talk_asset);

        BuildTalkCommand::new(app.world.spawn_empty().id(), builder).apply(&mut app.world);

        let (e, t) = app.world.query::<(Entity, &Talk)>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Start);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        // let sp_spawned = app.world.get::<Talk>(e).unwrap();
        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "Hello".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "Hello 2".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);
    }

    #[test]
    fn test_next_handler_with_join_and_leave_nodes() {
        let mut app = minimal_app();

        let script = indexmap! {
            0 => Action { kind: NodeKind::Join, next: Some(1), ..default() },
            1 => Action { text: "Hello".to_string(), next: Some(2), ..default() },
            2 => Action { kind: NodeKind::Leave, ..default() },
        };

        let mut talk_asset = TalkData::default();
        talk_asset.script = script;

        let builder = TalkBuilder::default().fill_from_talk_data(&talk_asset);

        BuildTalkCommand::new(app.world.spawn_empty().id(), builder).apply(&mut app.world);

        let (e, t) = app.world.query::<(Entity, &Talk)>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Start);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        // let sp_spawned = app.world.get::<Talk>(e).unwrap();
        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Join);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "Hello".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Leave);
    }

    #[test]
    fn test_choice_handler() {
        let mut app = minimal_app();

        let script = indexmap! {
            1 => Action {  choices: vec![
                ChoiceData {text: "Choice 1".to_string(), next: 2},
                ChoiceData {text: "Choice 2".to_string(), next: 3}
                ], kind: NodeKind::Choice, ..default() },
            2 => Action { kind: NodeKind::Leave, ..default() },
            3 => Action { text: "test".to_string(), ..default() },
        };

        let mut talk_asset = TalkData::default();
        talk_asset.script = script;

        let builder = TalkBuilder::default().fill_from_talk_data(&talk_asset);

        BuildTalkCommand::new(app.world.spawn_empty().id(), builder).apply(&mut app.world);
        let (e, _) = app.world.query::<(Entity, &Talk)>().single(&app.world);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Choice);
        assert_eq!(t.current_choices.len(), 2);
        assert_eq!(t.current_choices[0].text, "Choice 1");

        // check that next action does not work when there are choices
        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_choices.len(), 2);
        assert_eq!(t.current_kind, NodeKind::Choice);

        app.world
            .send_event(ChooseActionRequest::new(e, t.current_choices[0].next));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Leave);
    }
}
