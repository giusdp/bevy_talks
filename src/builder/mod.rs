//! Programmatically build Talks
use bevy::prelude::*;
use bevy::utils::Uuid;
use std::collections::VecDeque;

use crate::prelude::{Actor, ActorError, ActorSlug, NodeKind, TalkData};

use self::command::BuildTalkCommand;

pub mod command;

/// The ID of the nodes in the builder. It is used to identify the dialogue graph nodes before
/// they are actually spawned in the world.
/// It is useful to connect manually the nodes at build time with the `connect_to` method.
pub type BuildNodeId = String;

/// A struct with the data to build a node.
#[derive(Default, Debug, Clone)]
pub(crate) struct BuildNode {
    /// The id of the node to build.
    pub(crate) id: BuildNodeId,
    /// The kind of the node to build.
    pub(crate) kind: NodeKind,
    /// The text of the node to build. If it's a choice node, it will be empty.
    pub(crate) text: String,
    /// The choices of the node to build. If it's a talk node, it will be empty.
    pub(crate) choices: Vec<(String, TalkBuilder)>,
    /// The ids to add extra connections.
    pub(crate) manual_connections: Vec<BuildNodeId>,
    /// The actors slugs that are performing the node action.
    pub(crate) actors: Vec<ActorSlug>,
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
#[derive(Default, Debug, Clone)]
pub struct TalkBuilder {
    /// The main queue of nodes that will be spawned.
    pub(crate) queue: VecDeque<BuildNode>,
    /// The queue of actors that will be spawned and connected to the nodes.
    pub(crate) actors: Vec<Actor>,
    /// It is set when `connect_to` is called on an empty builder.
    /// It signals the Command to connect the last node of the parent builder (in a choice node).
    pub(crate) connect_parent: Option<BuildNodeId>,
}

impl TalkBuilder {
    /// Parses the `Talk` asset into a [`TalkBuilder`] ready to spawn the dialogue graph.
    ///
    /// This function also validates the `Talk` asset (checks that the `next` and `choice.next` fields point to existing actions)
    /// and then fills the [`TalkBuilder`] with all the actions.
    ///
    /// # Errors
    ///
    /// If the `TalkData` asset is not valid, this function will return a [`BuildTalkError`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use bevy::prelude::*;
    /// use bevy_talks::prelude::*;
    ///
    /// #[derive(Resource)]
    /// struct ATalkHandle(Handle<TalkData>);
    ///
    /// fn spawn_system(mut commands: Commands, talk_handle: Res<ATalkHandle>, assets: Res<Assets<TalkData>>) {
    ///     let talk = assets.get(&talk_handle.0).unwrap();
    ///     let talk_builder = TalkBuilder::default().into_builder(talk);
    ///     commands.add(talk_builder.build());
    /// }
    /// ```
    ///
    pub fn into_builder(self, talk: &TalkData) -> TalkBuilder {
        talk.fill_builder(self)
    }

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
        BuildTalkCommand { builder: self }
    }

    /// Add a simple text node without any actor that will spawn an entity with `TalkText`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// TalkBuilder::default().say("Hello").say("World!");
    /// ```
    pub fn say(mut self, text: impl Into<String>) -> TalkBuilder {
        let id = Uuid::new_v4().to_string();
        let talk_node = BuildNode {
            id: id.clone(),
            text: text.into(),
            kind: NodeKind::Talk,
            ..default()
        };
        self.queue.push_back(talk_node);

        self
    }

    /// Add a choice node that branches the conversation in different paths.
    /// It will spawn a `ChoiceNode` entity.
    ///
    /// # WARNING
    /// If you don't add any choices (the vec is empty), a warning will be logged and it will be treated as talk node with an empty string.
    /// A choice node without choices would result in a dead end with all the successive nodes from the builder being unreachable.
    ///
    /// # NOTE
    /// With `choose` you are branching the graph into multiple paths. Adding another node on the same builder after a choose does NOT simply
    /// connect the choice node sequentially to the new node.
    /// Instead, it grabs all the leaf nodes of the branches (the last nodes) and connects THEM to the
    /// new node. This is because the choice node is a branch, so it can't be connected sequentially to the next node.
    ///
    /// This allows you to have a graph where all the branches converge into a single node.
    ///
    /// ```text
    ///                 +--> say +
    ///                 |        |
    ///  start --> choice         +--> say
    ///                 |        |
    ///                 +--> say +
    /// ```
    /// # Example
    ///
    /// To have dialogue graph like the above:
    ///
    /// ```rust,no_run
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// TalkBuilder::default().choose(vec![
    ///     ("Choice 1", TalkBuilder::default().say("Hello")),
    ///     ("Choice 2", TalkBuilder::default().say("World!")),
    /// ]).say("Hi");
    /// ```
    pub fn choose(mut self, choices: Vec<(impl Into<String>, TalkBuilder)>) -> TalkBuilder {
        if choices.is_empty() {
            warn!("You attempted to add a choice node without any choices. It will be treated as a talk node to avoid dead ends.");
        }

        let choices = choices
            .into_iter()
            .map(|(text, builder)| (text.into(), builder))
            .collect::<Vec<_>>();

        let choice_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            choices,
            kind: NodeKind::Choice,
            ..default()
        };

        self.queue.push_back(choice_node);
        self
    }

    /// Add a Join node to the dialogue graph.
    pub fn join(mut self, actor_slugs: &[ActorSlug]) -> TalkBuilder {
        let join_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            kind: NodeKind::Join,
            actors: actor_slugs.to_vec(),
            ..default()
        };
        self.queue.push_back(join_node);
        self
    }

    /// Add a Leave node to the dialogue graph.
    pub fn leave(mut self, actor_slugs: &[ActorSlug]) -> TalkBuilder {
        let leave_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            kind: NodeKind::Leave,
            actors: actor_slugs.to_vec(),
            ..default()
        };
        self.queue.push_back(leave_node);
        self
    }

    /// Create a relationship manually from the latest node to the node identified by the given id.
    ///
    /// # Note
    /// If you call this method on an empty builder (a newly created one) it will try to connect
    /// the parent builder last node, if any. This is the case when you do it inside
    /// the construction of a choice node:
    ///
    /// ```rust,no_run
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// let mut builder = TalkBuilder::default().say("hello");
    /// let hello_id = builder.last_node_id();
    /// builder = builder.choose(vec![
    ///     ("Choice 1".to_string(), TalkBuilder::default().say("Hello")),
    ///     ("Choice 2".to_string(), TalkBuilder::default().connect_to(hello_id))
    /// ]);
    /// ```
    ///
    /// For the "Choice 2" branch we just passed an empty builder calling `connect_to`. It will not find any previous node to use
    /// so it will fall back to the parent node which is the choice node itself.
    ///
    /// If you call `connect_to` from an empty builder with not parent builder it will just do nothing.
    ///
    /// # Example
    ///
    /// If you want to form a loop (for example `start --> say <---> say`):
    /// ```rust
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// let mut builder = TalkBuilder::default().say("hello");
    /// let hello_id = builder.last_node_id();
    /// builder = builder.say("how are you?");
    /// builder = builder.connect_to(hello_id);
    /// ```
    pub fn connect_to(mut self, node_id: BuildNodeId) -> TalkBuilder {
        match self.queue.back_mut() {
            None => self.connect_parent = Some(node_id),
            Some(node) => node.manual_connections.push(node_id),
        };

        self
    }

    /// Get a unique id (uuids v4) for the latest node added to the builder.
    /// You can use the returned id with `connect_to` to manually pair nodes.
    ///
    /// # Panics
    /// If you call this method on an empty builder it will panic.
    ///
    /// # Example
    /// ```rust
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// let builder = TalkBuilder::default().say("hello");
    ///
    /// println!("{}", builder.last_node_id());
    /// ```
    pub fn last_node_id(&self) -> BuildNodeId {
        match self.queue.back() {
            None => panic!("You can't get the last node id of an empty builder"),
            Some(node) => node.id.clone(),
        }
    }

    /// Add an actor to the builder to be spawned (if not already present in the world, checked with the slug identifier).
    pub fn add_actor(mut self, actor: Actor) -> TalkBuilder {
        self.actors.push(actor);
        self
    }

    /// Add multiple actors to the builder to be spawned (if not already present in the world, checked with the slug identifier).
    pub fn add_actors(mut self, actors: Vec<Actor>) -> TalkBuilder {
        self.actors.extend(actors);
        self
    }

    /// Add a talk node with an actor. It will spawn an entity with `TalkText` connected with the actor entity identified by the slug.
    pub fn actor_say(
        mut self,
        actor_slug: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<TalkBuilder, ActorError> {
        let actor_slug = actor_slug.into();
        if !self.actors.iter().any(|a| a.slug == actor_slug) {
            return Err(ActorError::Invalid(actor_slug));
        }
        let id = Uuid::new_v4().to_string();
        let talk_node = BuildNode {
            id: id.clone(),
            text: text.into(),
            kind: NodeKind::Talk,
            actors: vec![actor_slug],
            ..default()
        };
        self.queue.push_back(talk_node);
        Ok(self)
    }

    /// Add a talk node with multiple actors.
    /// It will spawn an entity with `TalkText` connected with the actor entities identified by the slugs.
    pub fn actors_say(
        mut self,
        actor_slugs: &[ActorSlug],
        text: impl Into<String>,
    ) -> Result<TalkBuilder, ActorError> {
        for slug in actor_slugs.iter() {
            if !self.actors.iter().any(|a| a.slug == *slug) {
                return Err(ActorError::Invalid(slug.clone()));
            }
        }
        let id = Uuid::new_v4().to_string();
        let talk_node = BuildNode {
            id: id.clone(),
            text: text.into(),
            kind: NodeKind::Talk,
            actors: actor_slugs.to_vec(),
            ..default()
        };
        self.queue.push_back(talk_node);
        Ok(self)
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

    #[rstest]
    fn build_returns_command_with_queue(talk_builder: TalkBuilder) {
        let build_talk_cmd = talk_builder.say("Hello").say("World!").build();
        assert_eq!(build_talk_cmd.builder.queue.len(), 2);
        assert_eq!(build_talk_cmd.builder.queue[0].text, "Hello");
        assert_eq!(build_talk_cmd.builder.queue[1].text, "World!");
        assert_eq!(build_talk_cmd.builder.queue[0].kind, NodeKind::Talk);
        assert_eq!(build_talk_cmd.builder.queue[1].kind, NodeKind::Talk);
    }

    #[rstest]
    #[case(vec!["Hello"])]
    #[case(vec!["Hello", "World!"])]
    fn say_pushes_back_nodes(mut talk_builder: TalkBuilder, #[case] expected_texts: Vec<&str>) {
        for t in expected_texts.iter() {
            talk_builder = talk_builder.say(*t);
        }

        assert_eq!(talk_builder.queue.len(), expected_texts.len());

        for t in expected_texts {
            let text = talk_builder.queue.pop_front().unwrap().text;
            assert_eq!(text, t);
        }
    }

    #[rstest]
    fn choose_adds_a_choice_node(talk_builder: TalkBuilder) {
        let added_node = talk_builder
            .choose(vec![(
                "Hello".to_string(),
                TalkBuilder::default().say("hello").to_owned(),
            )])
            .queue
            .pop_front()
            .unwrap();
        assert_eq!(added_node.text, "");
        assert_eq!(added_node.choices.len(), 1);
        assert_eq!(added_node.kind, NodeKind::Choice);
    }

    #[rstest]
    fn connect_to_adds_entry_to_last_node(talk_builder: TalkBuilder) {
        let mut builder = talk_builder.say("hello");
        let hello_id = builder.last_node_id();
        builder = builder.say("how are you?").connect_to(hello_id);

        assert_eq!(builder.queue.len(), 2);

        let previous_node = builder.queue.pop_back().unwrap();
        assert_eq!(previous_node.manual_connections.len(), 1);
    }

    #[rstest]
    fn connect_to_in_empty_builder_sets_connect_parent(talk_builder: TalkBuilder) {
        let id = "some id".to_string();
        let builder = talk_builder.connect_to(id.clone());
        assert_eq!(builder.connect_parent, Some(id));
    }

    #[test]
    #[should_panic]
    fn last_node_id_panics_on_empty() {
        TalkBuilder::default().last_node_id();
    }

    #[rstest]
    fn test_last_node_id(talk_builder: TalkBuilder) {
        let builder = talk_builder.say("hello");
        let id = builder.last_node_id();
        assert_eq!(id, builder.queue[0].id);
    }

    #[rstest]
    fn test_join(talk_builder: TalkBuilder) {
        let actors = vec!["actor1".to_string(), "actor2".to_string()];
        let builder = talk_builder.join(&actors);
        assert_eq!(builder.queue.len(), 1);
        assert_eq!(builder.queue[0].kind, NodeKind::Join);
    }

    #[rstest]
    fn test_leave(talk_builder: TalkBuilder) {
        let actors = vec!["actor1".to_string(), "actor2".to_string()];
        let builder = talk_builder.leave(&actors);
        assert_eq!(builder.queue.len(), 1);
        assert_eq!(builder.queue[0].kind, NodeKind::Leave);
    }

    #[rstest]
    fn test_add_actor(talk_builder: TalkBuilder) {
        let actor = Actor {
            slug: "slug".to_string(),
            name: "Actor".to_string(),
        };
        let builder = talk_builder.add_actor(actor.clone());
        assert_eq!(builder.actors.len(), 1);
        assert_eq!(builder.actors[0], actor);
    }

    #[rstest]
    fn test_actor_say_success(talk_builder: TalkBuilder) {
        let builder = talk_builder.add_actor(Actor {
            slug: "slug".to_string(),
            name: "Actor".to_string(),
        });
        let builder = builder.actor_say("slug", "hello").unwrap();
        assert_eq!(builder.queue.len(), 1);
        assert_eq!(builder.queue[0].kind, NodeKind::Talk);
        assert_eq!(builder.queue[0].text, "hello");
        assert_eq!(builder.queue[0].actors[0], "slug");
    }
}
