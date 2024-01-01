//! Talk Asset structs and types.

use crate::{
    builder::{BuildNodeId, TalkBuilder},
    prelude::{Actor, ActorSlug, NodeKind},
};
use bevy::{prelude::*, reflect::TypePath, utils::HashMap};
use indexmap::IndexMap;

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
    pub(crate) choices: Vec<ChoiceData>,
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
pub(crate) struct ChoiceData {
    /// The text of the choice.
    pub(crate) text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub(crate) next: ActionId,
}

/// The asset representation of a Talk. It is assumed to represent a well formed Talk,
/// because the loader should have already validated it while loading.
///
#[derive(Asset, Debug, Default, Clone, TypePath)]
pub struct TalkData {
    /// The list of actions that make up the Talk.
    pub(crate) script: IndexMap<ActionId, Action>,
    /// The list of actors that appear in the Talk.
    pub(crate) actors: Vec<Actor>,
}

impl TalkData {
    /// Take a builder and fill it with the talk actions
    pub(crate) fn fill_builder(&self, mut builder: TalkBuilder) -> TalkBuilder {
        builder = builder.add_actors(self.actors.clone());

        if self.script.is_empty() {
            return builder;
        }

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
) -> TalkBuilder {
    // get the first action
    let mut the_action = &actions[&starting_action_id];
    let mut the_id = starting_action_id;

    let mut done = false;
    while !done {
        match the_action.kind {
            NodeKind::Talk => {
                builder = match the_action.actors.len() {
                    0 => builder.say(&the_action.text),
                    1 => builder.actor_say(&the_action.actors[0], &the_action.text),
                    2.. => builder.actors_say(&the_action.actors, &the_action.text),
                }
            }
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
                        inner_builder = prepare_builder(next, actions, inner_builder, visited);
                    }
                    choice_vec.push((text, inner_builder));
                }

                builder = builder.choose(choice_vec);
                visited.insert(the_id, builder.last_node_id());
                break; // no other nodes to visit from a choice (nexts are not used in this case)
            }
            NodeKind::Join => builder = builder.join(&the_action.actors),
            NodeKind::Leave => builder = builder.leave(&the_action.actors),
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

    builder
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
        talk.fill_builder(builder).build().apply(&mut app.world);

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
        talk.fill_builder(builder).build().apply(&mut app.world);

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
                    ChoiceData { text: "Choice 1".to_string(), next: 1, },
                    ChoiceData { text: "Choice 2".to_string(), next: 2, },
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
        talk.fill_builder(builder).build().apply(&mut app.world);

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
                    ChoiceData { text: "Choice 1".to_string(), next: 3, },
                    ChoiceData { text: "Choice 2".to_string(), next: 4, },
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
        talk.fill_builder(builder).build().apply(&mut app.world);

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
                    ChoiceData { text: "First Choice 1".to_string(), next: 1, },
                    ChoiceData { text: "First Choice 2".to_string(), next: 2, },
                ],
                kind: NodeKind::Choice,
                ..default()
            },
            1 => Action { text: "First Text".to_string(), next: Some(3), ..default() },
            2 => Action { text: "Last Text".to_string(), next: None, ..default() },
            3 =>
            Action {
                choices: vec![
                    ChoiceData { text: "Second Choice 1".to_string(), next: 2, },
                    ChoiceData { text: "Second Choice 2".to_string(), next: 4, },
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
        talk.fill_builder(builder).build().apply(&mut app.world);

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

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    #[case(200)]
    fn linear_talk_nodes_with_actors(builder: TalkBuilder, #[case] nodes: usize) {
        let actors = vec![
            Actor::new("actor1", "Actor 1"),
            Actor::new("actor2", "Actor 2"),
            Actor::new("actor3", "Actor 3"),
        ];

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
                    actors: vec![actors[index % 3].slug.clone()],
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
        let talk = TalkData { script, actors };

        let mut app = App::new();
        talk.fill_builder(builder).build().apply(&mut app.world);

        assert_eq!(app.world.query::<&StartTalk>().iter(&app.world).count(), 1);
        assert_eq!(
            app.world.query::<&TalkText>().iter(&app.world).count(),
            nodes
        );
        assert_eq!(app.world.query::<&Actor>().iter(&app.world).count(), 3);

        assert_on_talk_nodes(app, map);
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

            for (i, c) in t.0.iter().enumerate() {
                assert_eq!(c.text, expected_texts[i]);
            }
        }
    }
}
