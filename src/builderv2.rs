//! The talk builder module.
use aery::prelude::*;
use bevy::ecs::system::Command;
use bevy::prelude::*;
use std::collections::VecDeque;

use crate::prelude::TalkText;

/// A component that marks a node as the start of the dialogue graph.
#[derive(Component)]
pub struct TalkStart;

#[derive(Relation)]
#[aery(Recursive)]
struct FollowedBy;

/// The command that spawns a dialogue graph in the world.
/// You can create this command via the `build` method of the [`TalkBuilder`] struct.
pub struct BuildTalkCommand {
    build_node_queue: VecDeque<BuildNode>,
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        let mut parent = world.spawn(TalkStart).id();

        for build_node in self.build_node_queue {
            let successor = world.spawn_empty().id();
            world
                .entity_mut(parent)
                .set::<FollowedBy>(successor)
                .insert(TalkText(build_node.text));
            parent = successor;
        }
    }
}

/// A struct with the data to build a node.
struct BuildNode {
    /// The text of the node to build.
    text: String,
}

/// An implementation of the builder pattern for the dialogue graph.
/// You can define dialogue graphs programmatically using this builder and
/// then spawn all the node entities appropriately connected.
///
/// You can instantiate a new builder with `Talk::builder()` or `TalkBuilder::default()`.
///
/// # Usage
///
/// To build an entity dialogue graph you will define it with the `TalkBuilder` methods
/// and finally call `build` to generate the `BuildTalkCommand`.
///
/// This [`Command`] is what will actually spawn all the entities, you will have to `add` it to the commands queue.
///
/// ```rust,no_run
/// use bevy::app::App;
/// use bevy::ecs::system::CommandQueue;
/// use bevy::prelude::Commands;
/// use bevy_talks::prelude::TalkBuilder;
///
/// fn some_startup_system(mut commands: Commands) {
///     let build_talk_cmd = TalkBuilder::default().say("Hello").build();
///     commands.add(build_talk_cmd);
/// }
/// ```
#[derive(Default)]
pub struct TalkBuilder {
    /// The main queue of nodes that will be spawned.
    main: VecDeque<BuildNode>,
}

impl TalkBuilder {
    /// Add a simple text node without any actor that will spawn a `TalkNode` entity.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// TalkBuilder::default().say("Hello").say("World!");
    /// ```
    pub fn say(mut self, text: &str) -> Self {
        let talk_node = BuildNode {
            text: text.to_string(),
        };
        self.main.push_back(talk_node);
        self
    }
    /*
        pub fn choice() {}

        pub fn connect_to() {}

        pub fn branch() {}

        pub fn node() {}
    */

    /// Generate a `BuildTalkCommand` that will spawn all the dialogue nodes
    /// and connect them to each other to form a dialogue graph.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use bevy::prelude::Commands;
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// fn some_system(mut commands: Commands) {
    ///     let build_talk_cmd = TalkBuilder::default().say("Hello").build();
    ///     commands.add(build_talk_cmd);
    /// }
    /// ```
    pub fn build(self) -> BuildTalkCommand {
        BuildTalkCommand {
            build_node_queue: self.main,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn talk_builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    #[test]
    fn build_returns_command_with_queue() {
        let builder = TalkBuilder::default().say("Hello").say("World!");
        let build_talk_cmd = builder.build();
        assert_eq!(build_talk_cmd.build_node_queue.len(), 2);
        assert_eq!(build_talk_cmd.build_node_queue[0].text, "Hello");
        assert_eq!(build_talk_cmd.build_node_queue[1].text, "World!");
    }

    #[test]
    fn say_adds_to_the_queue() {
        let builder = TalkBuilder::default().say("Hello");
        assert_eq!(builder.main.len(), 1);
    }

    #[rstest]
    fn say_adds_a_talk_node(talk_builder: TalkBuilder) {
        let added_node = talk_builder.say("Hello").main.pop_front().unwrap();
        assert_eq!(added_node.text, "Hello");
    }

    #[rstest]
    fn say_pushes_back_nodes(talk_builder: TalkBuilder) {
        let mut builder = talk_builder.say("Hello").say("World!");
        assert_eq!(builder.main.len(), 2);

        let first_node = builder.main.pop_front().unwrap();
        assert_eq!(first_node.text, "Hello");

        let second_node = builder.main.pop_front().unwrap();
        assert_eq!(second_node.text, "World!");
    }

    #[rstest]
    #[case(0, 1)]
    #[case(1, 2)]
    #[case(2, 3)]
    #[case(3, 4)]
    #[case(4, 5)]
    fn command_spawns_entities_with_say(
        mut talk_builder: TalkBuilder,
        #[case] node_number: u32,
        #[case] expected: usize,
    ) {
        let mut app = App::new();
        for _ in 0..node_number {
            talk_builder = talk_builder.say("Hello");
        }
        let build_talk_cmd = talk_builder.build();

        build_talk_cmd.apply(&mut app.world);

        // there is always the root "start" node so we need to add 1
        assert_eq!(
            app.world.query::<Entity>().iter(&app.world).count(),
            expected
        );
    }

    #[rstest]
    #[case(0, 0)]
    #[case(1, 2)]
    #[case(2, 3)]
    #[case(3, 4)]
    #[case(4, 5)]
    fn command_spawn_linear_related_nodes(
        mut talk_builder: TalkBuilder,
        #[case] node_number: u32,
        #[case] expected_related_nodes: usize,
    ) {
        let mut app = App::new();
        for _ in 0..node_number {
            talk_builder = talk_builder.say("Hello");
        }
        let build_talk_cmd = talk_builder.build();

        build_talk_cmd.apply(&mut app.world);

        // there should be 1 root node in all cases (besides when 0 nodes)
        // For the 1 node case, there is still a root cause of the special start node
        assert_eq!(
            app.world
                .query::<(Entity, Root<FollowedBy>)>()
                .iter(&app.world)
                .count(),
            if node_number > 0 { 1 } else { 0 }
        );

        // check relations (e1, e2)
        assert_eq!(
            app.world
                .query::<(Entity, Relations<FollowedBy>)>()
                .iter(&app.world)
                .count(),
            expected_related_nodes
        );

        // check there is 1 leaf node
        assert_eq!(
            app.world
                .query::<(Entity, Leaf<FollowedBy>)>()
                .iter(&app.world)
                .count(),
            if node_number > 0 { 1 } else { 0 }
        );
    }

    #[rstest]
    #[case(1, vec!["Hello"])]
    #[case(2, vec!["Hello", "World!"])]
    #[case(3, vec!["Hi", "Hello", "World!"])]
    fn say_spawns_talk_nodes(
        mut talk_builder: TalkBuilder,
        #[case] node_number: u32,
        #[case] expected_texts: Vec<&str>,
    ) {
        use crate::prelude::TalkText;

        let mut app = App::new();

        for i in 0..node_number {
            talk_builder = talk_builder.say(expected_texts[i as usize]);
        }

        let build_talk_cmd = talk_builder.build();
        build_talk_cmd.apply(&mut app.world);

        let mut query = app.world.query::<&TalkText>();

        // check length
        assert_eq!(query.iter(&app.world).count(), node_number as usize);

        // check texts
        for t in query.iter(&app.world) {
            let found = expected_texts.iter().any(|&s| s == t.0);
            assert!(found);
        }
    }
}
