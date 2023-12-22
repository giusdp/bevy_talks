//! The talk builder module.
use bevy::ecs::system::Command;
use bevy::prelude::*;
use std::collections::VecDeque;

/// A component that marks a node as the start of the dialogue graph.
#[derive(Component)]
pub struct TalkStart;

/// Our custom command
pub struct BuildTalkCommand();

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        let _root_entity = world.spawn(TalkStart);
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
    pub fn build(self) -> BuildTalkCommand {
        BuildTalkCommand {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn say_adds_to_the_queue() {
        let builder = TalkBuilder::default().say("Hello");
        assert_eq!(builder.main.len(), 1);
    }

    #[test]
    fn say_adds_a_talk_node() {
        let mut builder = TalkBuilder::default().say("Hello");
        let added_node = builder.main.pop_front().unwrap();
        assert_eq!(added_node.text, "Hello");
    }

    #[test]
    fn say_pushes_back_nodes() {
        let mut builder = TalkBuilder::default().say("Hello").say("World!");
        assert_eq!(builder.main.len(), 2);

        let first_node = builder.main.pop_front().unwrap();
        assert_eq!(first_node.text, "Hello");

        let second_node = builder.main.pop_front().unwrap();
        assert_eq!(second_node.text, "World!");
    }

    #[test]
    fn spawn_empty_dialogue() {
        let mut app = App::new();
        let build_talk_cmd = TalkBuilder::default().build();

        build_talk_cmd.apply(&mut app.world);

        assert!(app
            .world
            .query::<&TalkStart>()
            .get_single(&app.world)
            .is_ok());
    }
}
