//! The raw data structures used to build a Talk.
//!
use std::collections::HashSet;

use bevy::{
    prelude::{Asset, Handle, Image},
    reflect::TypePath,
    utils::HashMap,
};
use indexmap::IndexMap;

use crate::prelude::{Actor, BuildNodeId, BuildTalkError, TalkBuilder, TalkNodeKind};

/// A unique identifier for an action in a Talk.
///
/// This type alias is used to define a unique identifier for an action in a Talk. Each action
/// in the Talk is assigned a unique ID, which is used to link the actions together in the
/// Talk graph.
pub(crate) type ActionId = usize;

/// A unique identifier for an actor in a Talk.
///
/// An `ActorId` is a `String` that uniquely identifies an actor in a Talk. It is used to
/// associate actions with the actors that perform them.
///
pub(crate) type ActorId = String;

/// A struct that represents a Raw Talk.
#[derive(Asset, Debug, Default, Clone, TypePath)]
pub struct RawTalk {
    /// The list of actions that make up the Talk.
    pub script: IndexMap<ActionId, RawAction>,
    /// The list of actors that appear in the Talk.
    pub actors: Vec<RawActor>,
}

impl RawTalk {
    /// Parse the asset into a builder that lets you spawn the dialogue graph.
    pub fn into_builder(self) -> Result<TalkBuilder, BuildTalkError> {
        if self.script.is_empty() {
            return Err(BuildTalkError::EmptyTalk);
        }

        self.validation_pass()?;

        let mut visited = HashMap::with_capacity(self.script.len());

        let builder = TalkBuilder::default();

        let start_id = self.script.keys().next().unwrap();

        build_pass(*start_id, &self.script, builder, &mut visited)
    }

    /// Validate the asset.
    pub(crate) fn validation_pass(&self) -> Result<(), BuildTalkError> {
        // // Check that there are no duplicate ids
        // check_duplicate_action_ids(&self.script)?;

        // Check all the nexts and choice.next (they should point to existing actions)
        validate_all_nexts(&self.script)?;
        Ok(())
    }
}

fn build_pass(
    starting_action_id: usize,
    actions: &IndexMap<ActionId, RawAction>,
    mut builder: TalkBuilder,
    visited: &mut HashMap<usize, BuildNodeId>,
) -> Result<TalkBuilder, BuildTalkError> {
    // get the first action
    let mut the_action = &actions[&starting_action_id];
    let mut the_id = starting_action_id;

    let mut done = false;
    while !done {
        match the_action.kind {
            TalkNodeKind::Talk => {
                builder = builder.say(&the_action.text);
                visited.insert(the_id, builder.last_node_id());

                if let Some(next) = the_action.next {
                    // just connect if already processed
                    if visited.get(&next).is_some() {
                        builder = builder.connect_to(visited[&next].clone());
                        done = true; // no need to continue
                    }
                    // move to the next actino
                    the_action = &actions[&next];
                    the_id = next;
                } else {
                    done = true; // reached an end node
                }
            }
            TalkNodeKind::Choice => {
                let mut choice_vec = Vec::with_capacity(the_action.choices.len());

                for c in the_action.choices.iter() {
                    let text = c.text.clone();
                    let next = c.next;
                    let mut inner_builder = TalkBuilder::default();

                    // if already visited, just connect to it instead of recursively building
                    if visited.get(&next).is_some() {
                        inner_builder = inner_builder.connect_to(visited[&next].clone());
                    } else {
                        inner_builder = build_pass(next, actions, inner_builder, visited)?;
                    }

                    choice_vec.push((text, inner_builder));
                }

                builder = builder.choose(choice_vec);
                visited.insert(the_id, builder.last_node_id());
                done = true; // no other nodes to visit from a choice (nexts are not used in this case)
            }
            TalkNodeKind::Join => todo!(),
            TalkNodeKind::Leave => todo!(),
        }
    }

    Ok(builder)
}

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Clone, Eq, Hash, PartialEq)]
pub struct RawAction {
    /// The kind of action.
    pub kind: TalkNodeKind,
    /// The actors involved in the action.
    pub actors: Vec<ActorId>,
    /// Any choices that the user can make during the action.
    pub choices: Vec<RawChoice>,
    /// The text of the action.
    pub text: String,
    /// The ID of the next action to perform.
    pub next: Option<ActionId>,
}

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Clone, Default)]
pub struct RawActor {
    /// The ID of the actor.
    pub id: ActorId,
    /// The name of the character that the actor plays.
    pub name: String,
    /// An optional asset that represents the actor's appearance or voice.
    pub asset: Option<Handle<Image>>,
}

impl From<RawActor> for Actor {
    fn from(val: RawActor) -> Self {
        Actor {
            name: val.name,
            asset: val.asset,
        }
    }
}

/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Default, Debug, Clone, Eq, Hash, PartialEq)]
pub struct RawChoice {
    /// The text of the choice.
    pub text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub next: ActionId,
}

/// Check if all `next` fields and `Choice` `next` fields in a `Vec<RawAction>` point to real actions.
/// If the action has choices, the `next` field is not checked.
///
/// Returns a `TalkError::InvalidNextAction` error if any of the `next` fields or `Choice` `next` fields in the `RawAction`s do not point to real actions.
fn validate_all_nexts(actions: &IndexMap<ActionId, RawAction>) -> Result<(), BuildTalkError> {
    let id_set = actions.keys().cloned().collect::<HashSet<_>>();
    for (id, action) in actions {
        if !action.choices.is_empty() {
            for choice in action.choices.iter() {
                if !id_set.contains(&choice.next) {
                    return Err(BuildTalkError::InvalidNextAction(*id, choice.next));
                }
            }
        } else if let Some(next_id) = &action.next {
            if !id_set.contains(next_id) {
                return Err(BuildTalkError::InvalidNextAction(*id, *next_id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{builderr::FollowedBy, prelude::*};

    use aery::{edges::Root, operations::utils::Relations, tuple_traits::RelationEntries};
    use bevy::{ecs::system::Command, prelude::*, utils::hashbrown::HashMap};
    use indexmap::{indexmap, IndexMap};
    use rstest::rstest;

    #[test]
    fn error_into_builder_empty() {
        let res = RawTalk::default().into_builder();
        assert!(res.is_err());
        assert_eq!(res.err(), Some(BuildTalkError::EmptyTalk));
    }

    #[test]
    fn error_invalid_next_action() {
        let talk = RawTalk {
            script: indexmap! {0 => RawAction {
                next: Some(2),
                ..default()
            }},
            ..default()
        };
        let res = talk.into_builder().err();
        assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
    }

    #[test]
    fn error_not_found_in_choice() {
        let talk = RawTalk {
            actors: default(),
            script: indexmap! {
                0 => RawAction {
                    choices: vec![RawChoice { next: 2, ..default()}],
                    ..default()
                },
                1 => RawAction {
                    ..default()
                },
            },
        };
        let res = talk.into_builder().err();
        assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    #[case(200)]
    #[case(1000)]
    fn linear_talk_nodes(#[case] nodes: usize) {
        let mut script = IndexMap::with_capacity(nodes);
        let mut map = HashMap::with_capacity(nodes);
        for index in 0..nodes {
            script.insert(
                index,
                RawAction {
                    text: "Hello".to_string(),
                    next: if nodes > 1 && index < nodes - 1 {
                        Some(index + 1)
                    } else {
                        None
                    },
                    ..default()
                },
            );
            let target = if nodes > 1 && index < nodes - 1 {
                Some((index + 2) as u32)
            } else {
                None
            };
            map.insert(index + 1, (target, "Hello"));
        }
        let talk = RawTalk {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.into_builder().unwrap().build().apply(&mut app.world);

        assert_eq!(app.world.query::<&TalkStart>().iter(&app.world).count(), 1);
        assert_eq!(
            app.world.query::<&TalkText>().iter(&app.world).count(),
            nodes
        );

        assert_on_talk_nodes(app, map);
    }

    #[test]
    fn talk_nodes_with_loop() {
        let script = indexmap! {
            1 => RawAction { text: "1".to_string(), next: Some(10), ..default() },
            2 => RawAction { text: "2".to_string(), next: Some(10), ..default() },
            10 => RawAction { text: "10".to_string(), next: Some(2), ..default() },
        };

        let talk = RawTalk {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.into_builder().unwrap().build().apply(&mut app.world);

        assert_eq!(app.world.query::<&TalkStart>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 3);

        let mut map = HashMap::new();
        map.insert(1, (Some(2), "1"));
        map.insert(3, (Some(2), "2"));
        map.insert(2, (Some(3), "10"));
        assert_on_talk_nodes(app, map);
    }

    #[test]
    fn choice_pointing_to_talks() {
        let script = indexmap! {
            0 =>
            RawAction {
                choices: vec![
                    RawChoice { text: "Choice 1".to_string(), next: 1, },
                    RawChoice { text: "Choice 2".to_string(), next: 2, },
                ],
                kind: TalkNodeKind::Choice,
                ..default()
            },
            1 => RawAction { text: "Hello".to_string(), next: Some(2), ..default() },
            2 => RawAction { text: "Fin".to_string(), ..default() },
        };

        let talk = RawTalk {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.into_builder().unwrap().build().apply(&mut app.world);

        assert_eq!(app.world.query::<&TalkStart>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 2);
        assert_eq!(
            app.world.query::<&ChoicesTexts>().iter(&app.world).count(),
            1
        );
        assert_eq!(
            app.world
                .query::<Root<FollowedBy>>()
                .iter(&app.world)
                .count(),
            1
        );
        let mut map: HashMap<usize, (Vec<u32>, Vec<&str>)> = HashMap::new();
        map.insert(1, (vec![2, 3], vec!["Choice 1", "Choice 2"]));
        assert_on_choice_nodes(&mut app, map);
    }

    #[test]
    fn connect_back_from_branch_book_example() {
        // From the Branching and Manual Connections builder section
        let script = indexmap! {
            0 => RawAction { text: "First Text".to_string(), next: Some(1), ..default() },
            1 => RawAction { text: "Second Text".to_string(), next: Some(2), ..default() },
            2 =>
            RawAction {
                choices: vec![
                    RawChoice { text: "Choice 1".to_string(), next: 3, },
                    RawChoice { text: "Choice 2".to_string(), next: 4, },
                ],
                kind: TalkNodeKind::Choice,
                ..default()
            },
            3 => RawAction { text: "Third Text (End)".to_string(), ..default() },
            4 => RawAction { text: "Fourth Text".to_string(), next: Some(0), ..default() },
        };
        let talk = RawTalk {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.into_builder().unwrap().build().apply(&mut app.world);

        assert_eq!(app.world.query::<&TalkStart>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 4);
        assert_eq!(
            app.world.query::<&ChoicesTexts>().iter(&app.world).count(),
            1
        );
        assert_eq!(
            app.world
                .query::<Root<FollowedBy>>()
                .iter(&app.world)
                .count(),
            1
        );
        let mut choice_map = HashMap::new();
        choice_map.insert(3, (vec![4, 5], vec!["Choice 1", "Choice 2"]));
        let mut talk_map = HashMap::new();
        talk_map.insert(1, (Some(2), "First Text"));
        talk_map.insert(2, (Some(3), "Second Text"));
        talk_map.insert(4, (None, "Third Text (End)"));
        talk_map.insert(5, (Some(1), "Fourth Text"));
        assert_on_choice_nodes(&mut app, choice_map);
        assert_on_talk_nodes(app, talk_map);
    }

    #[test]
    fn connect_forward_from_book_example() {
        // From the Connecting To The Same Node builder section
        let script = indexmap! {
            0 =>
            RawAction {
                choices: vec![
                    RawChoice { text: "First Choice 1".to_string(), next: 1, },
                    RawChoice { text: "First Choice 2".to_string(), next: 2, },
                ],
                kind: TalkNodeKind::Choice,
                ..default()
            },
            1 => RawAction { text: "First Text".to_string(), next: Some(3), ..default() },
            2 => RawAction { text: "Last Text".to_string(), next: None, ..default() },
            3 =>
            RawAction {
                choices: vec![
                    RawChoice { text: "Second Choice 1".to_string(), next: 2, },
                    RawChoice { text: "Second Choice 2".to_string(), next: 4, },
                ],
                kind: TalkNodeKind::Choice,
                ..default()
            },
            4 => RawAction { text: "Second Text".to_string(), next: Some(2), ..default() },
        };
        let talk = RawTalk {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.into_builder().unwrap().build().apply(&mut app.world);

        assert_eq!(app.world.query::<&TalkStart>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 3);
        assert_eq!(
            app.world.query::<&ChoicesTexts>().iter(&app.world).count(),
            2
        );
        assert_eq!(
            app.world
                .query::<Root<FollowedBy>>()
                .iter(&app.world)
                .count(),
            1
        );
        let mut choice_map = HashMap::new();
        choice_map.insert(1, (vec![2, 4], vec!["First Choice 1", "First Choice 2"]));
        choice_map.insert(3, (vec![4, 5], vec!["Second Choice 1", "Second Choice 2"]));
        let mut talk_map = HashMap::new();
        talk_map.insert(2, (Some(3), "First Text"));
        talk_map.insert(4, (None, "Last Text"));
        talk_map.insert(5, (Some(4), "Second Text"));
        assert_on_choice_nodes(&mut app, choice_map);
        assert_on_talk_nodes(app, talk_map);
    }

    /// Asserts that the talk nodes are correct. It wants a map to check the targets of the edges.
    /// The map is a map of entity index to (target entity index, text).
    #[track_caller]
    fn assert_on_talk_nodes(mut app: App, map: HashMap<usize, (Option<u32>, &str)>) {
        for (e, t, edges) in app
            .world
            .query::<(Entity, &TalkText, Relations<FollowedBy>)>()
            .iter(&app.world)
        {
            let eid = e.index() as usize;
            let expected_text = map[&eid].1;
            let maybe_target = map[&eid].0;
            let expected_count = if maybe_target.is_some() { 1 } else { 0 };

            if let Some(expected_target) = maybe_target {
                assert_eq!(
                    edges.targets(FollowedBy).iter().next().unwrap().index(),
                    expected_target
                );
            }

            assert_eq!(edges.targets(FollowedBy).iter().count(), expected_count);
            assert_eq!(t.0, expected_text);
        }
    }

    /// Asserts that the choice nodes are correct. It wants a map to check the targets of the edges.
    /// The map is a map of entity index to (entity targets, choice texts).
    #[track_caller]
    fn assert_on_choice_nodes(app: &mut App, map: HashMap<usize, (Vec<u32>, Vec<&str>)>) {
        for (e, t, edges) in app
            .world
            .query::<(Entity, &ChoicesTexts, Relations<FollowedBy>)>()
            .iter(&app.world)
        {
            let eid = e.index() as usize;
            let expected_texts = map[&eid].1.clone();
            let expected_count = expected_texts.len();
            for target in map[&eid].0.clone() {
                assert!(edges
                    .targets(FollowedBy)
                    .iter()
                    .any(|e| e.index() == target));
            }

            assert_eq!(edges.targets(FollowedBy).iter().count(), expected_count);

            for (i, text) in t.0.iter().enumerate() {
                assert_eq!(text, expected_texts[i]);
            }
        }
    }
}
