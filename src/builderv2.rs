//! The talk builder module.
use aery::prelude::*;
use bevy::ecs::system::Command;
use bevy::prelude::*;
use std::{collections::VecDeque, marker::PhantomData};

use crate::prelude::{ChoicesTexts, TalkText};

// region: - States -

/// Type state to identify a TalkBuilder without any nodes added.
#[derive(Default)]
pub struct Empty;

/// Type state to identify a TalkBuilder ready to build.
#[derive(Default)]
pub struct NonEmpty;
// endregion: - States -

/// A component that marks a node as the start of the dialogue graph.
#[derive(Component)]
pub struct TalkStart;

/// The relationship of the dialogue nodes. It needs to be Poly because the choice nodes
/// can have multiple branches.
#[derive(Relation)]
#[aery(Recursive, Poly)]
struct FollowedBy;

/// The command that spawns a dialogue graph in the world.
/// You can create this command via the `build` method of the [`TalkBuilder`] struct.
pub struct BuildTalkCommand {
    /// The builder that contains the queue of nodes to spawn.
    builder: TalkBuilder<NonEmpty>,
}

/// A recursive function that spawns all the nodes in the queue and connects them to each other.
///
/// # Arguments
///
/// * `root` - The root node of the graph. This is the node that will be connected to the first node in the queue.
/// * `talk_builder` - The builder that contains the queue of nodes to spawn.
/// * `world` - The world where the nodes will be spawned.
///
/// # Returns
///
/// A vector of leaf nodes spawned from the given builder. It is used internally during the recursion to connect
/// a the leaves from the branches created from a choice node to the successive node in the queue.
fn apply_build_cmd(
    root: Entity,
    talk_builder: TalkBuilder<NonEmpty>,
    world: &mut World,
) -> Vec<Entity> {
    let mut parent = root;
    let mut leaves: Vec<Entity> = vec![];
    let mut previous_node_was_choice = false;

    let mut peekable_queue = talk_builder.queue.into_iter().peekable();

    // for each node in the queue, spawn it and connect it to the previous one
    while let Some(build_node) = peekable_queue.next() {
        // spawn the child node
        let child = world.spawn_empty().id();

        // if the choices are empty, it's a talk node
        match build_node.choices.is_empty() {
            true => {
                // insert the TalkText component
                world.entity_mut(child).insert(TalkText(build_node.text));
                connect_to_previous(world, parent, &mut leaves, previous_node_was_choice, child);
                previous_node_was_choice = false;
            }
            false => {
                // otherwise it's a choice node.
                connect_to_previous(world, parent, &mut leaves, previous_node_was_choice, child);

                // We have to spawn the branches from the inner builders
                // and connect them to the choice node
                let mut choices_texts = Vec::with_capacity(build_node.choices.len());
                for (choice_text, inner_builder) in build_node.choices {
                    choices_texts.push(choice_text);
                    // recursively spawn the branches
                    let branch_leaves = apply_build_cmd(child, inner_builder, world);
                    leaves.extend(branch_leaves);
                }
                // insert the ChoicesTexts component
                world.entity_mut(child).insert(ChoicesTexts(choices_texts));

                previous_node_was_choice = true;
            }
        }

        // if this is the last node, it's a leaf
        if peekable_queue.peek().is_none() {
            leaves.push(child);
        }
        // set the new parent for the next iteration
        parent = child;
    }

    leaves
}

fn connect_to_previous(
    world: &mut World,
    parent: Entity,
    leaves: &mut Vec<Entity>,
    previous_node_was_choice: bool,
    child: Entity,
) {
    if previous_node_was_choice {
        // We have to connect the previous leaf nodes to the new node
        // we need drain cause we need to also clear the leaves vec for the next choice nodes
        for leaf in leaves.drain(..) {
            world.entity_mut(leaf).set::<FollowedBy>(child);
        }
    } else {
        // otherwise simply connect the parent to the child
        world.entity_mut(parent).set::<FollowedBy>(child);
    }
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        // spawn the start node
        let start = world.spawn(TalkStart).id();

        // spawn the rest of the nodes
        apply_build_cmd(start, self.builder, world);
    }
}

/// A struct with the data to build a node.
#[derive(Default)]
struct BuildNode {
    id: BuildNodeId,
    /// The text of the node to build. If it's a choice node, it will be empty.
    text: String,
    /// The choices of the node to build. If it's a talk node, it will be empty.
    choices: Vec<(String, TalkBuilder<NonEmpty>)>,
}

/// The ID of the nodes in the builder to identify the dialogue graph nodes before they are actually spawned in the world.
/// It is used to connect manually the nodes at build time with the `connect_to` method.
pub type BuildNodeId = usize;

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
pub struct TalkBuilder<T> {
    /// The main queue of nodes that will be spawned.
    queue: VecDeque<BuildNode>,
    marker: PhantomData<T>,
}
impl Default for TalkBuilder<Empty> {
    fn default() -> TalkBuilder<Empty> {
        TalkBuilder {
            queue: VecDeque::default(),
            marker: PhantomData,
        }
    }
}

impl<E> TalkBuilder<E> {
    /// Add a simple text node without any actor that will spawn a `TalkNode` entity.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use bevy_talks::prelude::TalkBuilder;
    ///
    /// TalkBuilder::default().say("Hello").say("World!");
    /// ```
    pub fn say(mut self, text: &str) -> TalkBuilder<NonEmpty> {
        let id = self.generate_id();
        let talk_node = BuildNode {
            id,
            text: text.to_string(),
            ..default()
        };
        self.queue.push_back(talk_node);

        TalkBuilder {
            queue: self.queue,
            marker: PhantomData,
        }
    }

    /// Utility function to generate a build node ID.
    /// If the queue is empty returns 0 otherwise the latest node id + 1.
    fn generate_id(&self) -> BuildNodeId {
        self.queue.back().map_or(0, |b| b.id + 1)
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
    ///     ("Choice 1".to_string(), TalkBuilder::default().say("Hello")),
    ///     ("Choice 2".to_string(), TalkBuilder::default().say("World!")),
    /// ]).say("Hi");
    /// ```
    pub fn choose(
        mut self,
        choices: Vec<(String, TalkBuilder<NonEmpty>)>,
    ) -> TalkBuilder<NonEmpty> {
        if choices.is_empty() {
            warn!("You attempted to add a choice node without any choices. It will be treated as a talk node to avoid dead ends.");
        }

        let choice_node = BuildNode {
            id: self.generate_id(),
            text: "".to_string(),
            choices,
        };

        self.queue.push_back(choice_node);
        TalkBuilder {
            queue: self.queue,
            marker: PhantomData,
        }
    }
}

impl TalkBuilder<NonEmpty> {
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

    /// Get a unique id for the latest node added to the builder.
    /// These IDs are incremented sequentially as you add nodes.
    /// You can use the returned id with `connect_to` to manually pair nodes.
    /// You could also guess the id number of a node if you are confident enough when connecting manually.
    ///
    /// # Example
    /// ```rust
    ///
    /// use bevy_talks::builderv2::TalkBuilder;
    ///
    /// let builder = TalkBuilder::default().say("hello");
    ///
    /// assert_eq!(builder.node(), 0);
    /// ```
    pub fn node(&self) -> BuildNodeId {
        self.queue.back().map_or(0, |b| b.id)
    }
    /*

        pub fn connect_to() {}

    */
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn talk_builder() -> TalkBuilder<Empty> {
        TalkBuilder::default()
    }

    #[rstest]
    fn build_returns_command_with_queue(talk_builder: TalkBuilder<Empty>) {
        let builder = talk_builder.say("Hello").say("World!");
        let build_talk_cmd = builder.build();
        assert_eq!(build_talk_cmd.builder.queue.len(), 2);
        assert_eq!(build_talk_cmd.builder.queue[0].text, "Hello");
        assert_eq!(build_talk_cmd.builder.queue[1].text, "World!");
    }

    #[rstest]
    #[case(vec!["Hello"])]
    #[case(vec!["Hello", "World!"])]
    fn say_pushes_back_nodes(talk_builder: TalkBuilder<Empty>, #[case] expected_texts: Vec<&str>) {
        let mut tbuilder = talk_builder.say(expected_texts[0]);
        if expected_texts.len() > 1 {
            for t in expected_texts.iter().skip(1) {
                tbuilder = tbuilder.say(*t);
            }
        }

        assert_eq!(tbuilder.queue.len(), expected_texts.len());

        for t in expected_texts {
            let text = tbuilder.queue.pop_front().unwrap().text;
            assert_eq!(text, t);
        }
    }

    #[rstest]
    #[case(1, 2, 2)]
    #[case(2, 3, 3)]
    #[case(20, 21, 21)]
    #[case(100, 101, 101)]
    fn command_spawns_entities_with_say(
        talk_builder: TalkBuilder<Empty>,
        #[case] node_number: u32,
        #[case] expected: usize,
        #[case] expected_nodes_in_relation: usize,
    ) {
        let mut app = App::new();

        let mut tbuilder = talk_builder.say("Hello");
        for _ in 1..node_number {
            tbuilder = tbuilder.say("Hello");
        }

        let build_talk_cmd = tbuilder.build();

        build_talk_cmd.apply(&mut app.world);

        // there is always the root "start" entity
        assert_eq!(
            app.world.query::<Entity>().iter(&app.world).count(),
            expected
        );

        assert_relationship_nodes(
            node_number,
            expected_nodes_in_relation,
            if node_number > 0 { 1 } else { 0 },
            &mut app,
        );
    }

    #[rstest]
    #[case(1, vec!["Hello"])]
    #[case(2, vec!["Hello", "World!"])]
    #[case(3, vec!["Hi", "Hello", "World!"])]
    fn command_with_only_talk_nodes(
        talk_builder: TalkBuilder<Empty>,
        #[case] node_number: usize,
        #[case] expected_texts: Vec<&str>,
    ) {
        use crate::prelude::TalkText;

        let mut app = App::new();
        let mut tbuilder = talk_builder.say(expected_texts[0]);

        for i in 1..node_number {
            tbuilder = tbuilder.say(expected_texts[i]);
        }

        let build_talk_cmd = tbuilder.build();
        build_talk_cmd.apply(&mut app.world);

        let mut query = app.world.query::<&TalkText>();

        // check length
        assert_eq!(query.iter(&app.world).count(), node_number as usize);

        // check texts
        for t in query.iter(&app.world) {
            assert!(expected_texts.iter().any(|&s| s == t.0));
        }
    }

    #[rstest]
    fn choose_adds_a_choice_node(talk_builder: TalkBuilder<Empty>) {
        let added_node = talk_builder
            .choose(vec![(
                "Hello".to_string(),
                TalkBuilder::default().say("hello"),
            )])
            .queue
            .pop_front()
            .unwrap();
        assert_eq!(added_node.text, "");
        assert_eq!(added_node.choices.len(), 1);
    }

    #[rstest]
    #[case(1, 4)]
    #[case(2, 7)]
    #[case(3, 10)]
    fn command_with_choice_nodes(
        talk_builder: TalkBuilder<Empty>,
        #[case] choice_node_number: u32,
        #[case] expected_nodes_in_relation: usize,
    ) {
        use crate::prelude::ChoicesTexts;

        let mut app = App::new();
        let mut tbuilder = talk_builder.choose(vec![
            ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
            ("Choice2".to_string(), TalkBuilder::default().say("World!")),
        ]);

        for _ in 1..choice_node_number {
            tbuilder = tbuilder.choose(vec![
                ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                ("Choice2".to_string(), TalkBuilder::default().say("World!")),
            ]);
        }

        let build_talk_cmd = tbuilder.build();
        build_talk_cmd.apply(&mut app.world);

        let mut query = app.world.query::<&ChoicesTexts>();

        // check length
        assert_eq!(query.iter(&app.world).count(), choice_node_number as usize);

        // check texts
        for t in query.iter(&app.world) {
            assert_eq!(t.0[0], "Choice1");
            assert_eq!(t.0[1], "Choice2");
        }

        assert_relationship_nodes(choice_node_number, expected_nodes_in_relation, 2, &mut app);
    }

    #[rstest]
    #[case(1, 1, 5, 1)]
    #[case(2, 2, 9, 1)]
    #[case(3, 2, 12, 2)]
    fn command_with_interleaved_choice_and_say_nodes(
        talk_builder: TalkBuilder<Empty>,
        #[case] choice_node_number: u32,
        #[case] say_node_number: u32,
        #[case] expected_related_nodes: usize,
        #[case] expected_leaf_nodes: usize,
    ) {
        let mut app = App::new();

        let mut tbuilder = talk_builder.choose(vec![
            ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
            ("Choice2".to_string(), TalkBuilder::default().say("Hi!")),
        ]);

        tbuilder = tbuilder.say("Hello");

        let max_range = if choice_node_number > say_node_number {
            choice_node_number
        } else {
            say_node_number
        };
        for i in 1..max_range {
            if i < choice_node_number {
                tbuilder = tbuilder.choose(vec![
                    ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                    ("Choice2".to_string(), TalkBuilder::default().say("Hi!")),
                ]);
            }
            if i < say_node_number {
                tbuilder = tbuilder.say("Hello");
            }
        }

        let build_talk_cmd = tbuilder.build();
        build_talk_cmd.apply(&mut app.world);

        assert_relationship_nodes(
            choice_node_number,
            expected_related_nodes,
            expected_leaf_nodes,
            &mut app,
        );
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    fn node_returns_latest_id(talk_builder: TalkBuilder<Empty>, #[case] node_number: usize) {
        let mut tbuilder = talk_builder.say("hello");
        let mut node_id = tbuilder.node();
        assert_eq!(node_id, 0);

        for i in 1..node_number {
            tbuilder = tbuilder.say("hello there");
            node_id = tbuilder.node();
            assert_eq!(node_id, i);
        }
    }

    #[track_caller]
    fn assert_relationship_nodes(
        node_number: u32,
        expected_nodes_in_relation: usize,
        expected_leaf_nodes: usize,
        app: &mut App,
    ) {
        // some assertions on the relationship. We are collecting the vec for debug purposes.

        // there should be 1 root node in all cases (besides when 0 nodes)
        // For the 1 node case, there is still a root cause of the special start node
        // We have to use Leaf tho cause in aery Root and Leaf are swapped
        let root_nodes: Vec<_> = app
            .world
            .query::<(Entity, Leaf<FollowedBy>)>()
            .iter(&app.world)
            .collect();
        assert_eq!(root_nodes.len(), if node_number > 0 { 1 } else { 0 });

        // check relations (e1, e2)
        let related_nodes: Vec<_> = app
            .world
            .query::<(Entity, Relations<FollowedBy>)>()
            .iter(&app.world)
            .collect();
        assert_eq!(related_nodes.len(), expected_nodes_in_relation);

        // check leaf nodes
        let leaf_nodes: Vec<_> = app
            .world
            .query::<(Entity, Root<FollowedBy>)>()
            .iter(&app.world)
            .collect();
        assert_eq!(leaf_nodes.len(), expected_leaf_nodes);
    }
}
