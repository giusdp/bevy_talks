//! The talk builder module.
use aery::prelude::*;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy::{ecs::system::Command, utils::Uuid};
use std::{collections::VecDeque, marker::PhantomData};

use crate::prelude::{ChoicesTexts, TalkText};

// region: - States -

/// Type state to identify a `TalkBuilder` without any nodes added.
#[derive(Default, Clone)]
pub struct Empty;

/// Type state to identify a `TalkBuilder` ready to build.
#[derive(Default, Clone)]
pub struct NonEmpty;
// endregion: - States -

/// A component that marks a node as the start of the dialogue graph.
#[derive(Component)]
pub struct TalkStart;

/// The relationship of the dialogue nodes. It needs to be Poly because the choice nodes
/// can have multiple branches.
#[derive(Relation)]
#[aery(Recursive, Poly)]
pub struct FollowedBy;

/// The command that spawns a dialogue graph in the world.
/// You can create this command via the `build` method of the [`TalkBuilder`] struct.
pub struct BuildTalkCommand {
    /// The builder that contains the queue of nodes to spawn.
    builder: TalkBuilder<NonEmpty>,
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        // spawn the start node
        let start = world.spawn(TalkStart).id();

        let mut build_node_entities = HashMap::new();

        // First pass: spawn all the node entities and add them to the map with their build node id
        spawn_dialogue_entities(&self.builder, &mut build_node_entities, world);

        // Second pass: connect them to form the graph
        add_relationships(start, self.builder, &mut build_node_entities, world);
    }
}

/// A recursive function that spawns all the nodes from a talk builder and adds them in the given hashmap.
/// It is used as the first pass of the building, so we have all the entities spawned and the `build_node_entities` map filled.
fn spawn_dialogue_entities(
    talk_builder: &TalkBuilder<NonEmpty>,
    build_node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) {
    for n in talk_builder.queue.iter() {
        let e = world.spawn_empty().id();
        build_node_entities.insert(n.id.clone(), e);

        for (_, inner_builder) in n.choices.iter() {
            spawn_dialogue_entities(inner_builder, build_node_entities, world);
        }
    }
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
fn add_relationships(
    root: Entity,
    talk_builder: TalkBuilder<NonEmpty>,
    build_node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> Vec<Entity> {
    let mut parent = root;

    // Connect parent entity (choice node) to the given node.
    if let Some(connect_node_id) = talk_builder.connect_parent {
        let entity_to_connect_to = build_node_entities.get(&connect_node_id);

        if let Some(e) = entity_to_connect_to {
            world.entity_mut(parent).set::<FollowedBy>(*e);
        } else {
            error!("Attempted to connect a choice node to some specific node that is not (yet) present in the builder.");
        }
    }

    let mut leaves: Vec<Entity> = vec![];
    let mut previous_node_was_choice = false;

    let mut peekable_queue = talk_builder.queue.into_iter().peekable();

    // for each node in the queue, spawn it and connect it to the previous one
    while let Some(build_node) = peekable_queue.next() {
        // retrieve the child node
        let child = *build_node_entities
            .get(&build_node.id)
            .expect("Error! Dialogue node entity not found. Cannot build dialogue graph! :(");

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
                    let branch_leaves =
                        add_relationships(child, inner_builder, build_node_entities, world);
                    leaves.extend(branch_leaves);
                }
                // insert the ChoicesTexts component
                world.entity_mut(child).insert(ChoicesTexts(choices_texts));

                previous_node_was_choice = true;
            }
        }

        // Let's add the extra connections here
        process_manual_connections(
            build_node_entities,
            &build_node.manual_connections,
            child,
            world,
        );

        // if this is the last node, it's a leaf
        if peekable_queue.peek().is_none() {
            leaves.push(child);
        }
        // set the new parent for the next iteration
        parent = child;
    }

    leaves
}

/// Connect the node to the given nodes.
fn process_manual_connections(
    build_node_entities: &HashMap<BuildNodeId, Entity>,
    manual_connections: &[BuildNodeId],
    child: Entity,
    world: &mut World,
) {
    if !manual_connections.is_empty() {
        for input_id in manual_connections {
            // get the entity node from the map
            let entity_to_connect_to = build_node_entities.get(input_id);

            // if the node is not present, log a warning and skip it
            if entity_to_connect_to.is_none() {
                warn!("You attempted to connect a dialogue node with that is not (yet) present in the builder. Skipping.");
                continue;
            }

            // connect it
            world
                .entity_mut(child)
                .set::<FollowedBy>(*entity_to_connect_to.unwrap());
        }
    }
}

/// Connect the previous nodes to the new node.
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

/// The ID of the nodes in the builder. It is used to identify the dialogue graph nodes before
/// they are actually spawned in the world.
/// It is useful to connect manually the nodes at build time with the `connect_to` method.
pub type BuildNodeId = String;

/// A struct with the data to build a node.
#[derive(Default, Clone)]
struct BuildNode {
    /// The id of the node to build.
    id: BuildNodeId,
    /// The text of the node to build. If it's a choice node, it will be empty.
    text: String,
    /// The choices of the node to build. If it's a talk node, it will be empty.
    choices: Vec<(String, TalkBuilder<NonEmpty>)>,
    /// The ids to add extra connections.
    manual_connections: Vec<BuildNodeId>,
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
#[derive(Clone)]
pub struct TalkBuilder<T> {
    /// The main queue of nodes that will be spawned.
    queue: VecDeque<BuildNode>,
    /// It is set when `connect_to` is called on an empty builder.
    /// It signals the Command to connect the last node of the parent builder (in a choice node).
    connect_parent: Option<BuildNodeId>,
    /// The marker to identify the state of the builder.
    marker: PhantomData<T>,
}
impl Default for TalkBuilder<Empty> {
    fn default() -> TalkBuilder<Empty> {
        TalkBuilder {
            queue: VecDeque::default(),
            connect_parent: None,
            marker: PhantomData,
        }
    }
}

impl<T> TalkBuilder<T> {
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
        let id = Uuid::new_v4().to_string();
        let talk_node = BuildNode {
            id: id.clone(),
            text: text.to_string(),
            ..default()
        };
        self.queue.push_back(talk_node);

        TalkBuilder {
            queue: self.queue,
            connect_parent: self.connect_parent,
            marker: PhantomData,
        }
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
            id: Uuid::new_v4().to_string(),
            choices,
            ..default()
        };

        self.queue.push_back(choice_node);
        TalkBuilder {
            queue: self.queue,
            connect_parent: self.connect_parent,
            marker: PhantomData,
        }
    }

    /// Create a relationship manually from the latest node to the node identified by the given id.
    ///
    /// # Note
    /// If you call this method on an empty builder (a newly created one) it will try to connect
    /// the parent builder last node, if any. This is the case when you do it inside
    /// the construction of a choice node:
    ///
    /// ```rust,no_run
    /// use bevy_talks::builderv2::TalkBuilder;
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
    /// use bevy_talks::builderv2::TalkBuilder;
    ///
    /// let mut builder = TalkBuilder::default().say("hello");
    /// let hello_id = builder.last_node_id();
    /// builder = builder.say("how are you?");
    /// builder = builder.connect_to(hello_id);
    /// ```
    pub fn connect_to<E>(mut self, node_id: BuildNodeId) -> TalkBuilder<E> {
        match self.queue.back_mut() {
            None => self.connect_parent = Some(node_id),
            Some(node) => node.manual_connections.push(node_id),
        };

        TalkBuilder {
            queue: self.queue,
            connect_parent: self.connect_parent,
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

    /// Get a unique id (uuids v4) for the latest node added to the builder.
    /// You can use the returned id with `connect_to` to manually pair nodes.
    /// You could also guess the id number of a node if you are confident enough when connecting manually.
    ///
    /// # Example
    /// ```rust
    /// use bevy_talks::builderv2::TalkBuilder;
    ///
    /// let builder = TalkBuilder::default().say("hello");
    ///
    /// println!("{}", builder.last_node_id());
    /// ```
    pub fn last_node_id(&self) -> BuildNodeId {
        self.queue.back().unwrap().id.clone()
    }
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
        assert_eq!(query.iter(&app.world).count(), node_number);

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

    #[test]
    fn spawn_dialogue_entities_test() {
        let mut app = App::new();

        let builder = TalkBuilder::default()
            .say("Hello")
            .choose(vec![
                ("Choice 1".to_string(), TalkBuilder::default().say("Hi")),
                ("Choice 2".to_string(), TalkBuilder::default().say("World!")),
            ])
            .say("something");

        let mut map = HashMap::new();
        spawn_dialogue_entities(&builder, &mut map, &mut app.world);

        assert_eq!(map.len(), 5);
        assert_eq!(app.world.iter_entities().count(), 5);
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
    #[case(2, 4)]
    #[case(3, 5)]
    fn connect_to_adds_relationship(
        talk_builder: TalkBuilder<Empty>,
        #[case] node_number: u32,
        #[case] expected_related: usize,
    ) {
        let mut app = App::new();
        let mut tbuilder = talk_builder.say("hello");

        let first_node = tbuilder.last_node_id();

        tbuilder = tbuilder.say("hello there");

        for _ in 1..node_number {
            tbuilder = tbuilder.say("hello there");
        }

        tbuilder = tbuilder.connect_to(first_node);

        let build_talk_cmd = tbuilder.build();
        build_talk_cmd.apply(&mut app.world);

        assert_relationship_nodes(node_number, expected_related, 0, &mut app)
    }

    #[test]
    fn connect_back_from_branch_book_example() {
        // From the Branching and Manual Connections builder section
        let talk_builder = TalkBuilder::default().say("Hello");

        // grab latest node
        let convo_start = talk_builder.last_node_id();

        let cmd = talk_builder
            .say("Hey")
            .choose(vec![
                (
                    "Good Choice".to_string(),
                    TalkBuilder::default().say("End of the conversation"),
                ),
                (
                    "Wrong Choice".to_string(),
                    TalkBuilder::default()
                        .say("Go Back")
                        .connect_to(convo_start),
                ),
            ])
            .build();

        let mut app = App::new();
        cmd.apply(&mut app.world);

        // TODO: I would like to assert on the actual structure of the graph but I can't do much with aery here.
        assert_relationship_nodes(6, 6, 1, &mut app);
    }

    #[test]
    fn connect_forward_from_book_example() {
        // From the Connecting To The Same Node builder section
        let end_branch_builder = TalkBuilder::default().say("The End"); // Create the end immediately
        let end_node_id = end_branch_builder.last_node_id(); // <- grab the end node

        // Create the good path
        let good_branch = TalkBuilder::default().say("something").choose(vec![
            (
                "Bad Choice".to_string(),
                TalkBuilder::default().connect_to(end_node_id.clone()),
            ),
            (
                "Another Good Choice".to_string(),
                TalkBuilder::default()
                    .say("Before the end...")
                    .connect_to(end_node_id),
            ),
        ]);

        let build_cmd = TalkBuilder::default()
            .choose(vec![
                ("Good Choice".to_string(), good_branch),
                // NB the builder is passed here. If we never add it and keep using connect_to
                // the end node would never be created
                ("Bad Choice".to_string(), end_branch_builder),
            ])
            .build();

        let mut app = App::new();
        build_cmd.apply(&mut app.world);

        // TODO: I would like to assert on the actual structure of the graph but I can't do much with aery here.
        assert_relationship_nodes(6, 6, 1, &mut app);
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
