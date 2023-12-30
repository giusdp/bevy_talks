//! The Main types for a Talk.

use std::collections::HashSet;

use aery::prelude::*;
use bevy::{prelude::*, reflect::TypePath, utils::HashMap};
use indexmap::IndexMap;

use crate::{
    actors::ActorSlug,
    prelude::{Actor, BuildNodeId, BuildTalkError, TalkBuilder},
};

/// The relationship of the dialogue nodes.
/// It needs to be Poly because the choice nodes can have multiple branches.
#[derive(Relation)]
#[aery(Recursive, Poly)]
pub struct FollowedBy;

/// The relationship between dialogue nodes and actors.
/// It needs to be Poly because the nodes can have multiple actors (and vice-versa).
#[derive(Relation)]
#[aery(Recursive, Poly)]
pub struct PerformedBy;

/// The Talk component. It's used to identify the parent entity of dialogue entity graphs.
/// Build entities with Talk components via the [`TalkBuilder`] to correctly setup the dialogue graph.
#[derive(Component, Default)]
pub struct Talk {
    /// The text of the current talk node
    pub current_text: String,
    /// The current node kind
    pub current_kind: NodeKind, // TODO: add a Start node kind?
}

/// Marker component for the current node in a Talk.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct CurrentNode;

/// A unique identifier for an action in a Talk.
///
/// This type alias is used to define a unique identifier for an action in a Talk. Each action
/// in the Talk is assigned a unique ID, which is used to link the actions together in the
/// Talk graph.
pub(crate) type ActionId = usize;

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Clone, Eq, Hash, PartialEq)]
pub(crate) struct Action {
    /// The kind of action.
    pub(crate) kind: NodeKind,
    /// The actors involved in the action.
    pub(crate) actors: Vec<ActorSlug>,
    /// Any choices that the user can make during the action.
    pub(crate) choices: Vec<Choice>,
    /// The text of the action.
    pub(crate) text: String,
    /// The ID of the next action to perform.
    pub(crate) next: Option<ActionId>,
}
/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Default, Debug, Clone, Eq, Hash, PartialEq)]
pub(crate) struct Choice {
    /// The text of the choice.
    pub(crate) text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub(crate) next: ActionId,
}

/// A component that marks a node as the start of the dialogue graph.
#[derive(Component)]
pub struct StartTalk;

/// An enumeration of the different kinds of actions that can be performed in a Talk.
#[derive(Component, Debug, Default, Clone, Hash, Eq, PartialEq)]
pub enum NodeKind {
    /// A talk action, where a character speaks dialogue.
    #[default]
    Talk,
    /// A choice action, where the user is presented with a choice.
    Choice,
    /// An enter action, where a character enters a scene.
    Join,
    /// An exit action, where a character exits a scene.
    Leave,
}

/// The components that define a Talk node in the dialogue graph.
/// Use `TalkNodeBundle::new()` to create a new `TalkNodeBundle`.
#[derive(Bundle, Default)]
pub struct TalkNodeBundle {
    /// The kind of action that the node performs. This should be `NodeKind::Talk` as the TalkNodeBundle is used to create a talk node.
    pub kind: NodeKind,
    /// The text to be displayed by the talk node.
    pub text: TalkText,
}

impl TalkNodeBundle {
    /// Creates a new `TalkNodeBundle` with the specified `text`.
    /// The node kind is set to `NodeKind::Talk`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bevy_talks::prelude::*;
    ///
    /// let text = "Hello, world!".to_string();
    /// let actors = vec!["Alice".to_string(), "Bob".to_string()];
    /// let bundle = TalkNodeBundle::new(text.clone());
    ///
    /// assert_eq!(bundle.kind, NodeKind::Talk);
    /// assert_eq!(bundle.text.0, text);
    /// ```
    pub fn new(text: String) -> Self {
        Self {
            kind: NodeKind::Talk,
            text: TalkText(text),
        }
    }
}

/// The components that defines a `Choice` node in the dialogue graph.
/// Use `TalkNodeBundle::new()` to create a new `TalkNodeBundle`.
#[derive(Bundle, Default)]
pub struct ChoiceNodeBundle {
    /// Should be `NodeKind::Choice` for the choice node.
    pub kind: NodeKind,
    /// The choices of the node.
    pub choices: Choices,
}

impl ChoiceNodeBundle {
    /// Creates a new `ChoiceNodeBundle`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bevy_talks::prelude::*;
    ///
    /// let bundle = ChoiceNodeBundle::new(vec!["Choice 1".to_string(), "Choice 2".to_string()]);
    ///
    /// assert_eq!(bundle.kind, NodeKind::Choice);
    /// assert_eq!(bundle.choices.0[0], "Choice 1".to_string());
    /// ```
    pub fn new(cs: Vec<String>) -> Self {
        Self {
            kind: NodeKind::Choice,
            choices: Choices(cs),
        }
    }
}

/// The text component to be displayed from a Talk Node.
#[derive(Component, Default, Debug)]
pub struct TalkText(pub String);

/// The choices texts component to be displayed from a Choice Node.
#[derive(Component, Default, Debug)]
pub struct Choices(pub Vec<String>);

/// The asset representation of a Talk.
#[derive(Asset, Debug, Default, Clone, TypePath)]
pub struct TalkData {
    /// The list of actions that make up the Talk.
    pub(crate) script: IndexMap<ActionId, Action>,
    /// The list of actors that appear in the Talk.
    pub(crate) actors: Vec<Actor>,
}

impl TalkData {
    /// Take a builder and fill it with the talk actions
    pub(crate) fn fill_builder(
        &self,
        mut builder: TalkBuilder,
    ) -> Result<TalkBuilder, BuildTalkError> {
        if self.script.is_empty() {
            return Err(BuildTalkError::EmptyTalk);
        }

        // Check all the nexts and choice.next (they should point to existing actions)
        validate_all_nexts(&self.script)?;

        builder = builder.add_actors(self.actors.clone());

        let mut visited = HashMap::with_capacity(self.script.len());

        let start_id = self.script.keys().next().unwrap();

        prepare_builder(*start_id, &self.script, builder, &mut visited)
    }
}

/// Build the builder
fn prepare_builder(
    starting_action_id: usize,
    actions: &IndexMap<ActionId, Action>,
    mut builder: TalkBuilder,
    visited: &mut HashMap<usize, BuildNodeId>,
) -> Result<TalkBuilder, BuildTalkError> {
    // get the first action
    let mut the_action = &actions[&starting_action_id];
    let mut the_id = starting_action_id;

    let mut done = false;
    while !done {
        match the_action.kind {
            NodeKind::Talk => builder = builder.say(&the_action.text),
            NodeKind::Choice => {
                let mut choice_vec = Vec::with_capacity(the_action.choices.len());

                for c in the_action.choices.iter() {
                    let text = c.text.clone();
                    let next = c.next;
                    let mut inner_builder = TalkBuilder::default();

                    // if already visited, just connect to it instead of recursively building
                    if visited.get(&next).is_some() {
                        inner_builder = inner_builder.connect_to(visited[&next].clone());
                    } else {
                        inner_builder = prepare_builder(next, actions, inner_builder, visited)?;
                    }
                    choice_vec.push((text, inner_builder));
                }

                builder = builder.choose(choice_vec);
                visited.insert(the_id, builder.last_node_id());
                break; // no other nodes to visit from a choice (nexts are not used in this case)
            }
            NodeKind::Join => builder = builder.join(),
            NodeKind::Leave => builder = builder.leave(),
        }

        visited.insert(the_id, builder.last_node_id());
        if let Some(next) = the_action.next {
            // just connect if already processed
            if visited.get(&next).is_some() {
                builder = builder.connect_to(visited[&next].clone());
                done = true; // no need to continue
            }
            // move to the next action
            the_action = &actions[&next];
            the_id = next;
        } else {
            done = true; // reached an end node
        }
    }

    Ok(builder)
}

/// Check if all `next` fields and `Choice` `next` fields in a `Vec<RawAction>` point to real actions.
/// If the action has choices, the `next` field is not checked.
///
/// Returns a `TalkError::InvalidNextAction` error if any of the `next` fields or `Choice` `next` fields in the `RawAction`s do not point to real actions.
fn validate_all_nexts(actions: &IndexMap<ActionId, Action>) -> Result<(), BuildTalkError> {
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
    use crate::{prelude::*, FollowedBy};

    use aery::{edges::Root, operations::utils::Relations, tuple_traits::RelationEntries};
    use bevy::{ecs::system::Command, prelude::*, utils::hashbrown::HashMap};
    use indexmap::{indexmap, IndexMap};
    use rstest::{fixture, rstest};

    #[fixture]
    fn builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    #[rstest]
    fn error_into_builder_empty(builder: TalkBuilder) {
        let res = TalkData::default().fill_builder(builder);
        assert!(res.is_err());
        assert_eq!(res.err(), Some(BuildTalkError::EmptyTalk));
    }

    #[rstest]
    fn error_invalid_next_action(builder: TalkBuilder) {
        let talk = TalkData {
            script: indexmap! {0 => Action {
                next: Some(2),
                ..default()
            }},
            ..default()
        };
        let res = talk.fill_builder(builder).err();
        assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
    }

    #[rstest]
    fn error_not_found_in_choice(builder: TalkBuilder) {
        let talk = TalkData {
            actors: default(),
            script: indexmap! {
                0 => Action {
                    choices: vec![Choice { next: 2, ..default()}],
                    ..default()
                },
                1 => Action {
                    ..default()
                },
            },
        };
        let res = talk.fill_builder(builder).err();
        assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    #[case(200)]
    #[case(1000)]
    fn linear_talk_nodes(builder: TalkBuilder, #[case] nodes: usize) {
        let mut script = IndexMap::with_capacity(nodes);
        let mut map = HashMap::with_capacity(nodes);
        for index in 0..nodes {
            script.insert(
                index,
                Action {
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
        let talk = TalkData {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.fill_builder(builder)
            .unwrap()
            .build()
            .apply(&mut app.world);

        assert_eq!(app.world.query::<&StartTalk>().iter(&app.world).count(), 1);
        assert_eq!(
            app.world.query::<&TalkText>().iter(&app.world).count(),
            nodes
        );

        assert_on_talk_nodes(app, map);
    }

    #[rstest]
    fn talk_nodes_with_loop(builder: TalkBuilder) {
        let script = indexmap! {
            1 => Action { text: "1".to_string(), next: Some(10), ..default() },
            2 => Action { text: "2".to_string(), next: Some(10), ..default() },
            10 => Action { text: "10".to_string(), next: Some(2), ..default() },
        };

        let talk = TalkData {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.fill_builder(builder)
            .unwrap()
            .build()
            .apply(&mut app.world);

        assert_eq!(app.world.query::<&StartTalk>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 3);

        let mut map = HashMap::new();
        map.insert(1, (Some(2), "1"));
        map.insert(3, (Some(2), "2"));
        map.insert(2, (Some(3), "10"));
        assert_on_talk_nodes(app, map);
    }

    #[rstest]
    fn choice_pointing_to_talks(builder: TalkBuilder) {
        let script = indexmap! {
            0 =>
            Action {
                choices: vec![
                    Choice { text: "Choice 1".to_string(), next: 1, },
                    Choice { text: "Choice 2".to_string(), next: 2, },
                ],
                kind: NodeKind::Choice,
                ..default()
            },
            1 => Action { text: "Hello".to_string(), next: Some(2), ..default() },
            2 => Action { text: "Fin".to_string(), ..default() },
        };

        let talk = TalkData {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.fill_builder(builder)
            .unwrap()
            .build()
            .apply(&mut app.world);

        assert_eq!(app.world.query::<&StartTalk>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 2);
        assert_eq!(app.world.query::<&Choices>().iter(&app.world).count(), 1);
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

    #[rstest]
    fn connect_back_from_branch_book_example(builder: TalkBuilder) {
        // From the Branching and Manual Connections builder section
        let script = indexmap! {
            0 => Action { text: "First Text".to_string(), next: Some(1), ..default() },
            1 => Action { text: "Second Text".to_string(), next: Some(2), ..default() },
            2 =>
            Action {
                choices: vec![
                    Choice { text: "Choice 1".to_string(), next: 3, },
                    Choice { text: "Choice 2".to_string(), next: 4, },
                ],
                kind: NodeKind::Choice,
                ..default()
            },
            3 => Action { text: "Third Text (End)".to_string(), ..default() },
            4 => Action { text: "Fourth Text".to_string(), next: Some(0), ..default() },
        };
        let talk = TalkData {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.fill_builder(builder)
            .unwrap()
            .build()
            .apply(&mut app.world);

        assert_eq!(app.world.query::<&StartTalk>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 4);
        assert_eq!(app.world.query::<&Choices>().iter(&app.world).count(), 1);
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

    #[rstest]
    fn connect_forward_from_book_example(builder: TalkBuilder) {
        // From the Connecting To The Same Node builder section
        let script = indexmap! {
            0 =>
            Action {
                choices: vec![
                    Choice { text: "First Choice 1".to_string(), next: 1, },
                    Choice { text: "First Choice 2".to_string(), next: 2, },
                ],
                kind: NodeKind::Choice,
                ..default()
            },
            1 => Action { text: "First Text".to_string(), next: Some(3), ..default() },
            2 => Action { text: "Last Text".to_string(), next: None, ..default() },
            3 =>
            Action {
                choices: vec![
                    Choice { text: "Second Choice 1".to_string(), next: 2, },
                    Choice { text: "Second Choice 2".to_string(), next: 4, },
                ],
                kind: NodeKind::Choice,
                ..default()
            },
            4 => Action { text: "Second Text".to_string(), next: Some(2), ..default() },
        };
        let talk = TalkData {
            script,
            ..default()
        };

        let mut app = App::new();
        talk.fill_builder(builder)
            .unwrap()
            .build()
            .apply(&mut app.world);

        assert_eq!(app.world.query::<&StartTalk>().iter(&app.world).count(), 1);
        assert_eq!(app.world.query::<&TalkText>().iter(&app.world).count(), 3);
        assert_eq!(app.world.query::<&Choices>().iter(&app.world).count(), 2);
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
            .query::<(Entity, &Choices, Relations<FollowedBy>)>()
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
