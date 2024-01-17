//! Programmatically build Talks

use bevy::prelude::*;
use bevy::utils::Uuid;
use std::collections::VecDeque;

use crate::prelude::{Actor, ActorSlug, TalkData};
use crate::{JoinNode, LeaveNode, TextNode};

pub mod build_command;
pub mod commands;

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
/// This `Command` is what will actually spawn all the entities, you will have to `add` it to the commands queue.
///
/// ```rust,no_run
/// use bevy::app::App;
/// use bevy::ecs::system::CommandQueue;
/// use bevy::prelude::Commands;
/// use bevy_talks::prelude::{TalkBuilder, TalkCommandsExt};
///
/// fn some_startup_system(mut commands: Commands) {
///     let builder = TalkBuilder::default().say("Hello");
///     commands.spawn_talk(builder);
/// }
/// ```
#[derive(Default)]
pub struct TalkBuilder {
    /// The main queue of nodes that will be spawned.
    pub(crate) queue: VecDeque<BuildNode>,
    /// The queue of actors that will be spawned and connected to the nodes.
    pub(crate) actors: Vec<Actor>,
    /// It is set when `connect_to` is called on an empty builder.
    /// It signals the Command to connect the last node of the parent builder (in a choice node).
    pub(crate) connect_parent: Option<BuildNodeId>,
}

/// The ID of the nodes in the builder. It is used to identify the dialogue graph nodes before
/// they are actually spawned in the world.
/// It is useful to connect manually the nodes at build time with the `connect_to` method.
pub type BuildNodeId = String;

/// A struct with the data to build a node.
#[derive(Default)]
pub(crate) struct BuildNode {
    /// The id of the node to build.
    pub(crate) id: BuildNodeId,
    /// The choices of the node to build.
    /// NOTE: due to the limitation of current entity relationship system (with aery) we need to store the choices
    /// until the entities are spawned cause edges cannot hold any data, so we can't already create the
    /// choice node components.
    pub(crate) choices: Vec<(String, TalkBuilder)>,
    /// The ids to add extra connections.
    pub(crate) manual_connections: Vec<BuildNodeId>,
    /// The actors slugs that are performing the node action.
    pub(crate) actors: Vec<ActorSlug>,
    /// The components to add to the node entity. These will be `TextNode`, JoinNode`, `LeaveNode` + custom components.
    /// `ChoiceNode` components are added later when the entities are spawned.
    pub(crate) components: Vec<Box<dyn Reflect>>,
}

impl TalkBuilder {
    /// Parses the `Talk` asset into a [`TalkBuilder`] ready to spawn the dialogue graph.
    ///
    /// This function also validates the `Talk` asset (checks that the `next` and `choice.next` fields point to existing actions)
    /// and then fills the [`TalkBuilder`] with all the actions.
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
    /// fn spawn_system(talk_handle: Res<ATalkHandle>, assets: Res<Assets<TalkData>>) {
    ///     let talk = assets.get(&talk_handle.0).unwrap();
    ///     let talk_builder = TalkBuilder::default().fill_with_talk_data(talk);
    /// }
    /// ```
    ///
    pub fn fill_with_talk_data(self, talk: &TalkData) -> Self {
        talk.fill_builder(self)
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
    pub fn say(mut self, text: impl Into<String>) -> Self {
        let talk_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            components: vec![Box::new(TextNode(text.into()))],
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
    pub fn choose(mut self, choices: Vec<(impl Into<String>, Self)>) -> Self {
        assert!(!choices.is_empty(), "You can't choose node without choices");

        let choices = choices
            .into_iter()
            .map(|(t, b)| (t.into(), b))
            .collect::<Vec<(String, TalkBuilder)>>();

        let choice_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            choices,
            ..default()
        };

        self.queue.push_back(choice_node);
        self
    }

    /// Add a Join node to the dialogue graph.
    pub fn join(mut self, actor_slugs: &[ActorSlug]) -> Self {
        let join_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            actors: actor_slugs.to_vec(),
            components: vec![Box::new(JoinNode)],
            ..default()
        };
        self.queue.push_back(join_node);
        self
    }

    /// Add a Leave node to the dialogue graph.
    pub fn leave(mut self, actor_slugs: &[ActorSlug]) -> Self {
        let leave_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            actors: actor_slugs.to_vec(),
            components: vec![Box::new(LeaveNode)],
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
    pub fn connect_to(mut self, node_id: BuildNodeId) -> Self {
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
    /// # Note
    /// Adding actors to nested builders (when branching) has no effect. Add them to the root builder instead.
    pub fn add_actor(mut self, actor: Actor) -> Self {
        self.actors.push(actor);
        self
    }

    /// Add multiple actors to the builder to be spawned (if not already present in the world, checked with the slug identifier).
    /// # Note
    /// Adding actors to nested builders (when branching) has no effect. Add them to the root builder instead.
    pub fn add_actors(mut self, actors: Vec<Actor>) -> Self {
        self.actors.extend(actors);
        self
    }

    /// Add a talk node with an actor. It will spawn an entity with `TalkText` connected with the actor entity identified by the slug.
    pub fn actor_say(mut self, actor_slug: impl Into<String>, text: impl Into<String>) -> Self {
        let talk_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            actors: vec![actor_slug.into()],
            components: vec![Box::new(TextNode(text.into()))],
            ..default()
        };
        self.queue.push_back(talk_node);
        self
    }

    /// Add a talk node with multiple actors.
    /// It will spawn an entity with `TalkText` connected with the actor entities identified by the slugs.
    pub fn actors_say(mut self, actor_slugs: &[ActorSlug], text: impl Into<String>) -> Self {
        let talk_node = BuildNode {
            id: Uuid::new_v4().to_string(),
            components: vec![Box::new(TextNode(text.into()))],
            actors: actor_slugs.to_vec(),
            ..default()
        };
        self.queue.push_back(talk_node);
        self
    }

    /// Add components attached to the latest added node.
    /// If you add a `NodeEventEmitter` component the node will automatically emit the relative event when reached.
    ///
    /// # Note
    /// Remember to register the types! For `NodeEventEmitter` components you can use app.register_node_event::<MyComp, MyCompEvent>()
    /// to setup everything at once. If it is a normal component, just use `app.world.register_type::<MyComp>()`.
    ///
    /// # Panics
    /// If you call this method on an empty builder it will panic.
    pub fn add_component<C: Component + Reflect>(mut self, comp: C) -> Self {
        match self.queue.back_mut() {
            None => panic!("You can't add a custom component to an empty builder"),
            Some(node) => node.components.push(Box::new(comp)),
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn talk_builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    #[rstest]
    #[case(vec!["Hello"])]
    #[case(vec!["Hello", "World!"])]
    fn say_pushes_back_nodes(mut talk_builder: TalkBuilder, #[case] expected_texts: Vec<&str>) {
        for t in expected_texts.iter() {
            talk_builder = talk_builder.say(*t);
        }

        assert_eq!(talk_builder.queue.len(), expected_texts.len());
        assert_eq!(talk_builder.queue.pop_front().unwrap().components.len(), 1);
        if expected_texts.len() > 1 {
            assert_eq!(talk_builder.queue.pop_front().unwrap().components.len(), 1);
        }
    }

    #[rstest]
    fn choose_adds_a_choice_node(talk_builder: TalkBuilder) {
        let added_node = talk_builder
            .choose(vec![(
                "Hello".to_string(),
                TalkBuilder::default().say("hello"),
            )])
            .queue
            .pop_front()
            .unwrap();
        assert_eq!(added_node.choices.len(), 1);
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
        assert_eq!(builder.queue[0].components.len(), 1);
    }

    #[rstest]
    fn test_leave(talk_builder: TalkBuilder) {
        let actors = vec!["actor1".to_string(), "actor2".to_string()];
        let builder = talk_builder.leave(&actors);
        assert_eq!(builder.queue.len(), 1);
        assert_eq!(builder.queue[0].components.len(), 1);
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
        let builder = builder.actor_say("slug", "hello");
        assert_eq!(builder.queue.len(), 1);
        assert_eq!(builder.queue[0].actors[0], "slug");
    }

    #[derive(Component, Reflect)]
    struct MyComp;

    #[rstest]
    fn add_component_on_last_node(talk_builder: TalkBuilder) {
        let builder = talk_builder.say("hello").add_component(MyComp);
        assert_eq!(builder.queue.len(), 1);
        assert_eq!(builder.queue[0].components.len(), 2);
    }

    #[rstest]
    #[should_panic]
    fn add_component_on_empty_panics(talk_builder: TalkBuilder) {
        talk_builder.add_component(MyComp);
    }
}
