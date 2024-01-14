//! The Bevy Command to spawn Talk entity graphs
use aery::prelude::*;
use bevy::{ecs::system::Command, prelude::*, utils::hashbrown::HashMap};

use crate::prelude::{
    ActorSlug, Choice, ChoiceNode, CurrentNode, EndNode, FollowedBy, JoinNode, LeaveNode,
    PerformedBy, StartNode, TextNode,
};

use super::*;

/// The command that spawns a dialogue graph in the world.
/// You can create this command via the `build` method of the [`TalkBuilder`] struct.
pub struct BuildTalkCommand {
    /// The entity parent of a dialogue graph
    pub(crate) parent: Entity,
    /// The builder that contains the queue of nodes to spawn.
    pub(crate) builder: TalkBuilder,
}

impl BuildTalkCommand {
    /// Create a new `BuildTalkCommand` with a parent entity and a builder.
    /// The parent entity will be the parent of the dialogue graph and will have a [`Talk`] component.
    pub(crate) fn new(p: Entity, b: TalkBuilder) -> Self {
        Self {
            parent: p,
            builder: b,
        }
    }
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        // spawn the start node
        let start = &world.spawn((StartNode, CurrentNode)).id();

        // First pass: spawn all the node entities and add them to the map with their build node id
        let (ents, mut node_entities) = spawn_dialogue_entities(&self.builder, world);
        let actor_ents: HashMap<ActorSlug, Entity> = spawn_actor_entities(&self.builder, world);

        let mut manager = world.entity_mut(self.parent);
        manager.add_child(*start);
        for e in ents {
            manager.add_child(e);
        }

        // Second pass: connect them to form the graph
        form_graph(*start, &self.builder, &mut node_entities, world);

        // Third pass: connect the actors to the nodes
        connect_nodes_with_actors(&self.builder, node_entities, actor_ents, world);
    }
}
/// Connect the nodes to the actors.
fn connect_nodes_with_actors(
    talk_builder: &TalkBuilder,
    node_entities: HashMap<String, Entity>,
    all_actors: HashMap<String, Entity>,
    world: &mut World,
) {
    for node in talk_builder.queue.iter() {
        if !node.actors.is_empty() {
            let node_ent = node_entities.get(&node.id).unwrap();

            for actor in node.actors.iter() {
                let actor_ent = all_actors.get(actor).unwrap_or_else(|| {
                    panic!(
                        "Error! Actor {} not found while building talk from builder.",
                        actor
                    )
                });
                world.entity_mut(*node_ent).set::<PerformedBy>(*actor_ent);
            }
        }

        // recursively connect the inner nodes
        if !node.choices.is_empty() {
            for (_, inner_builder) in node.choices.iter() {
                connect_nodes_with_actors(
                    inner_builder,
                    node_entities.clone(),
                    all_actors.clone(),
                    world,
                );
            }
        }
    }
}

/// Spawn the actor entities in the world and return a map with the entities and the actors.
/// If the actor is already present in the world (identified via the slug), it will not be spawned again.
fn spawn_actor_entities(
    talk_builder: &TalkBuilder,
    world: &mut World,
) -> HashMap<ActorSlug, Entity> {
    // TODO: this is probably not the most efficient way to do this. Looks pretty slow.

    let mut actor_ents: HashMap<Entity, Actor> = HashMap::with_capacity(talk_builder.actors.len());
    let mut actors_to_spawn = talk_builder.actors.clone();

    // find the already existing actors in the world
    let already_spawned_actors = world
        .query::<(Entity, &Actor)>()
        .iter(world)
        .map(|(e, a)| (a.slug.clone(), (e, a.clone())))
        .collect::<HashMap<String, (Entity, Actor)>>();

    actors_to_spawn.retain(|a| !already_spawned_actors.contains_key(&a.slug));

    for (_, (e, actor)) in already_spawned_actors {
        actor_ents.insert(e, actor);
    }

    for a in actors_to_spawn {
        let e = world.spawn(a.clone()).id();
        actor_ents.insert(e, a);
    }

    actor_ents
        .into_iter()
        .map(|(e, a)| (a.slug, e))
        .collect::<HashMap<_, _>>()
}

/// A recursive function that spawns all the nodes from a talk builder and adds them in the given hashmap.
/// It is used as the first pass of the building, so we have all the entities spawned and the `build_node_entities` map filled.
fn spawn_dialogue_entities(
    talk_builder: &TalkBuilder,
    world: &mut World,
) -> (Vec<Entity>, HashMap<BuildNodeId, Entity>) {
    let mut entities: Vec<Entity> = Vec::with_capacity(talk_builder.queue.len());
    let mut build_node_entities = HashMap::new();
    for n in talk_builder.queue.iter() {
        let e = world.spawn_empty().id();
        entities.push(e);
        build_node_entities.insert(n.id.clone(), e);

        for (_, inner_builder) in n.choices.iter() {
            let (inner_ents, inner_bne) = spawn_dialogue_entities(inner_builder, world);
            entities.extend(inner_ents);
            build_node_entities.extend(inner_bne);
        }
    }
    (entities, build_node_entities)
}

/// A recursive function that spawns all the nodes in the queue and connects them to each other.
///
/// # Returns
///
/// A tuple with the first child node and the the vector of leaf nodes spawned from the given builder.
/// It is used internally during the recursion to connect the last nodes from the branches
/// of a choice node to the successive node in the queue.
///
/// NB: The returned fist node is only needed because we have to store the `Entity` in the [`Choice`] struct
/// as a workaround of the zero size edges. If we could add data to the edges we could simply
/// add the choice text to the edge and use the edge directly to perform the choice.
fn form_graph(
    root: Entity,
    talk_builder: &TalkBuilder,
    node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> (Entity, Vec<Entity>) {
    let mut parent = root;

    let mut first_child_set = false;
    let mut first_child_ent = root;

    // Connect parent entity (choice node) to the given node.
    if let Some(connect_node_id) = &talk_builder.connect_parent {
        let entity_to_connect_to = node_entities.get(connect_node_id);
        first_child_ent = *entity_to_connect_to.unwrap();
        first_child_set = true;
        if let Some(e) = entity_to_connect_to {
            world.entity_mut(parent).set::<FollowedBy>(*e);
            first_child_ent = *e;
        } else {
            error!("Attempted to connect a choice node to some specific node that is not (yet) present in the builder.");
        }
    }

    let mut leaves: Vec<Entity> = vec![];
    let mut previous_node_was_choice = false;

    if !talk_builder.queue.is_empty() && !first_child_set {
        first_child_ent = *node_entities
            .get(&talk_builder.queue[0].id)
            .expect("First entity from the builder");
    }

    let mut peekable_queue = talk_builder.queue.iter().peekable();

    // for each node in the queue, spawn it and connect it to the previous one
    while let Some(build_node) = peekable_queue.next() {
        // retrieve the child node
        let this_ent = *node_entities
            .get(&build_node.id)
            .expect("Error! Dialogue node entity not found. Cannot build dialogue graph! :(");

        connect_to_previous(
            world,
            parent,
            &mut leaves,
            previous_node_was_choice,
            this_ent,
        );

        match build_node.kind {
            NodeKind::Talk => {
                world
                    .entity_mut(this_ent)
                    .insert(TextNode(build_node.text.clone()));
                previous_node_was_choice = false;
            }
            NodeKind::Choice => {
                // We have to spawn the branches from the inner builders
                // and connect them to the choice node
                let mut choices: Vec<Choice> = Vec::with_capacity(build_node.choices.len());
                for (choice_text, inner_builder) in build_node.choices.iter() {
                    // recursively spawn the branches
                    let (branch_root, branch_leaves) =
                        form_graph(this_ent, inner_builder, node_entities, world);
                    choices.push(Choice::new(choice_text, branch_root));
                    leaves.extend(branch_leaves);
                }

                // insert the ChoicesTexts component
                world.entity_mut(this_ent).insert(ChoiceNode(choices));

                previous_node_was_choice = true;
            }
            NodeKind::Join => {
                world.entity_mut(this_ent).insert(JoinNode);
                previous_node_was_choice = false;
            }
            NodeKind::Leave => {
                world.entity_mut(this_ent).insert(LeaveNode);
                previous_node_was_choice = false;
            }
            _ => (), // ignore other kinds for now
        }

        // Let's add the extra connections here
        process_manual_connections(
            node_entities,
            &build_node.manual_connections,
            this_ent,
            world,
        );

        // if this is the last node, it's a leaf
        if peekable_queue.peek().is_none() {
            leaves.push(this_ent);

            // if it was not manually connected add EndNode component
            if build_node.manual_connections.is_empty() {
                world.entity_mut(this_ent).insert(EndNode);
            }
        }
        // set the new parent for the next iteration
        parent = this_ent;
    }

    (first_child_ent, leaves)
}

/// Connect the node to the given nodes.
fn process_manual_connections(
    build_node_entities: &HashMap<BuildNodeId, Entity>,
    manual_connections: &[BuildNodeId],
    child: Entity,
    world: &mut World,
) {
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

#[cfg(test)]
mod tests {
    use std::vec;

    use bevy::{prelude::*, utils::HashMap};
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_spawn_dialogue_entities() {
        let mut app = App::new();

        let builder = TalkBuilder::default()
            .say("Hello")
            .choose(vec![
                (
                    "Choice 1".to_string(),
                    TalkBuilder::default().say("Hi").to_owned(),
                ),
                (
                    "Choice 2".to_string(),
                    TalkBuilder::default().say("World!").to_owned(),
                ),
            ])
            .say("something");

        let (ents, map) = spawn_dialogue_entities(&builder, &mut app.world);

        assert_eq!(map.len(), 5);
        assert_eq!(ents.len(), 5);
        assert_eq!(app.world.iter_entities().count(), 5);
    }

    #[rstest]
    #[case(0)]
    #[case(5)]
    fn test_spawn_actor_entities(#[case] already_spawned: usize) {
        let mut app = App::new();

        for i in 0..already_spawned {
            app.world
                .spawn(Actor::new(format!("actor_{}", i), format!("Actor {}", i)));
        }

        let builder = TalkBuilder::default()
            .add_actor(Actor::new("my_actor", "Actor"))
            .add_actor(Actor::new("actor_0", "Actor2"))
            .say("Hello")
            .say("something");

        app.update();
        let actor_ents = spawn_actor_entities(&builder, &mut app.world);
        app.update();

        let expected = if already_spawned > 0 {
            already_spawned + 1
        } else {
            2
        };

        assert_eq!(actor_ents.len(), expected);
        assert_eq!(app.world.iter_entities().count(), expected);
    }

    #[test]
    fn test_connect_nodes_with_actors() {
        let mut app = App::new();

        let builder = TalkBuilder::default()
            .add_actor(Actor::new("my_actor", "Actor"))
            .add_actor(Actor::new("actor_0", "Actor2"))
            .actor_say("my_actor", "Hello")
            .choose(vec![
                (
                    "Choice 1",
                    TalkBuilder::default().actor_say("actor_0", "Hi"),
                ),
                ("Choice 2", TalkBuilder::default().say("World!")),
            ]);

        let (_, node_entities) = spawn_dialogue_entities(&builder, &mut app.world);
        let actor_ents = spawn_actor_entities(&builder, &mut app.world);
        connect_nodes_with_actors(&builder, node_entities, actor_ents, &mut app.world);

        let nodes_with_actors = app
            .world
            .query::<(Relations<PerformedBy>, Without<Actor>)>()
            .iter(&app.world)
            .count();

        assert_eq!(nodes_with_actors, 2);
    }

    #[test]
    fn test_process_manual_connections() {
        let mut world = World::default();
        let mut build_node_entities = HashMap::default();
        let fist_ent = world.spawn_empty().id();
        let manual_connections = vec!["1".to_string(), "2".to_string()];
        build_node_entities.insert("1".to_string(), world.spawn_empty().id());
        build_node_entities.insert("2".to_string(), world.spawn_empty().id());

        process_manual_connections(
            &build_node_entities,
            &manual_connections,
            fist_ent,
            &mut world,
        );

        // Assert that the connections are made correctly
        let (root_ent, _) = world.query::<(Entity, Leaf<FollowedBy>)>().single(&world);
        assert_eq!(root_ent, fist_ent);

        for (leaf, _) in world.query::<(Entity, Root<FollowedBy>)>().iter(&world) {
            assert!(leaf == build_node_entities["1"] || leaf == build_node_entities["2"]);
        }
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    fn test_connect_to_previous(#[case] previous_node_was_choice: bool) {
        let mut world = World::default();
        let root_ent = world.spawn_empty().id();
        let leaf_ent = world.spawn_empty().id();

        let mut leaves = vec![];
        if previous_node_was_choice {
            leaves = vec![world.spawn_empty().id(), world.spawn_empty().id()];
            for leaf in leaves.iter() {
                world.entity_mut(root_ent).set::<FollowedBy>(*leaf);
            }
        }

        connect_to_previous(
            &mut world,
            root_ent,
            &mut leaves,
            previous_node_was_choice,
            leaf_ent,
        );

        // Assert that the connections are made correctly
        assert_eq!(
            world.query::<(Entity, Leaf<FollowedBy>)>().single(&world).0,
            root_ent
        );

        assert_eq!(
            world.query::<(Entity, Root<FollowedBy>)>().single(&world).0,
            leaf_ent
        );

        if previous_node_was_choice {
            assert_eq!(
                world
                    .query::<(Entity, Branch<FollowedBy>)>()
                    .iter(&world)
                    .count(),
                2
            );
        }
    }

    #[test]
    fn test_add_relationships_simple() {
        let mut world = World::default();
        let root = world.spawn_empty().id();
        let talk_builder = TalkBuilder::default().say("Hello There");
        let (_, mut build_node_entities) = spawn_dialogue_entities(&talk_builder, &mut world);

        let (ent, leaves) = form_graph(root, &talk_builder, &mut build_node_entities, &mut world);

        // Assert that the relationships are built correctly
        assert_ne!(ent, root);
        assert_eq!(leaves.len(), 1);
        assert_eq!(
            world.query::<Relations<FollowedBy>>().iter(&world).count(),
            2
        );
    }
    #[test]
    fn test_add_relationships() {
        let mut world = World::default();
        let root = world.spawn_empty().id();

        let talk_builder = TalkBuilder::default()
            .choose(vec![
                ("Choice Text".to_string(), TalkBuilder::default().say("t")),
                ("Choice Text 2".to_string(), TalkBuilder::default().say("t")),
            ])
            .say("something");

        let (_, mut build_node_entities) = spawn_dialogue_entities(&talk_builder, &mut world);

        form_graph(root, &talk_builder, &mut build_node_entities, &mut world);

        // Assert that the relationships are built correctly
        assert_eq!(
            world.query::<Relations<FollowedBy>>().iter(&world).count(),
            5
        );
        assert_eq!(
            world
                .query::<(Entity, Leaf<FollowedBy>)>()
                .iter(&world)
                .count(),
            1
        );
        assert_eq!(
            world
                .query::<(Entity, Branch<FollowedBy>)>()
                .iter(&world)
                .count(),
            3
        );
        assert_eq!(
            world
                .query::<(Entity, Root<FollowedBy>)>()
                .iter(&world)
                .count(),
            1
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use aery::tuple_traits::RelationEntries;
    use bevy::prelude::*;
    use rstest::{fixture, rstest};

    use crate::prelude::TextNode;

    use super::*;

    #[fixture]
    fn talk_builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    #[rstest]
    #[should_panic]
    fn test_panic_on_wrong_actor(mut talk_builder: TalkBuilder) {
        let mut world = World::default();
        talk_builder = talk_builder.actor_say("actor", "Hello");
        BuildTalkCommand::new(world.spawn_empty().id(), talk_builder).apply(&mut world);
    }

    #[rstest]
    #[case(vec!["Hello"])]
    #[case(vec!["Hello", "World!"])]
    #[case(vec!["Hi", "Hello", "World!"])]
    #[case(vec!["Hi", "Hello", "World!", "The End"])]
    fn linear_say_graph_creation(mut talk_builder: TalkBuilder, #[case] text_nodes: Vec<&str>) {
        let mut world = World::default();
        let node_number = text_nodes.len();

        for t in text_nodes.iter() {
            talk_builder = talk_builder.say(*t);
        }

        BuildTalkCommand::new(world.spawn_empty().id(), talk_builder).apply(&mut world);

        let mut query = world.query::<&TextNode>();

        // check number of nodes with the text component
        assert_eq!(query.iter(&world).count(), node_number);

        // check texts
        for t in query.iter(&world) {
            assert!(text_nodes.iter().any(|&s| s == t.0));
        }

        // need to add 1 cause of the start node
        assert_relationship_nodes(node_number, node_number + 1, 1, &mut world);
    }

    #[rstest]
    #[case(1, 4)]
    #[case(2, 7)]
    #[case(3, 10)]
    fn branching_graph_creation(
        mut talk_builder: TalkBuilder,
        #[case] choice_node_number: usize,
        #[case] expected_nodes_in_relation: usize,
    ) {
        let mut world = World::default();

        for _ in 0..choice_node_number {
            talk_builder = talk_builder.choose(vec![
                ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                ("Choice2".to_string(), TalkBuilder::default().say("World!")),
            ]);
        }

        BuildTalkCommand::new(world.spawn_empty().id(), talk_builder).apply(&mut world);

        let mut query = world.query::<&ChoiceNode>();

        // check length
        assert_eq!(query.iter(&world).count(), choice_node_number);

        // check texts
        for t in query.iter(&world) {
            assert_eq!(t.0[0].text, "Choice1");
            assert_eq!(t.0[1].text, "Choice2");
        }

        assert_relationship_nodes(
            choice_node_number,
            expected_nodes_in_relation,
            2,
            &mut world,
        );
    }

    #[rstest]
    #[case(1, 1, 5, 1)]
    #[case(2, 2, 9, 1)]
    #[case(3, 2, 12, 2)]
    fn interleaved_choice_and_say_graph_creation(
        mut talk_builder: TalkBuilder,
        #[case] choice_number: usize,
        #[case] say_number: usize,
        #[case] expected_nodes: usize,
        #[case] expected_leaves: usize,
    ) {
        let mut world = World::default();

        let max_range = if choice_number > say_number {
            choice_number
        } else {
            say_number
        };
        for i in 0..max_range {
            if i < choice_number {
                talk_builder = talk_builder.choose(vec![
                    ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                    ("Choice2".to_string(), TalkBuilder::default().say("Hi!")),
                ]);
            }
            if i < say_number {
                talk_builder = talk_builder.say("Hello");
            }
        }

        BuildTalkCommand::new(world.spawn_empty().id(), talk_builder).apply(&mut world);

        assert_relationship_nodes(choice_number, expected_nodes, expected_leaves, &mut world);
    }

    #[test]
    fn connect_back_from_branch_book_example() {
        // From the Branching and Manual Connections builder section
        let mut builder = TalkBuilder::default().say("Hello");

        // grab latest node
        let convo_start = builder.last_node_id();

        builder = builder.say("Hey").choose(vec![
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
        ]);

        let mut world = World::default();
        BuildTalkCommand::new(world.spawn_empty().id(), builder).apply(&mut world);

        // TODO: I should assert on the actual structure of the graph instead of simple number of nodes, leaf and roots.
        assert_relationship_nodes(6, 6, 1, &mut world);
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

        let builder = TalkBuilder::default().choose(vec![
            ("Good Choice".to_string(), good_branch),
            // If we never pass the actual biulder the end node would never be created
            ("Bad Choice".to_string(), end_branch_builder),
        ]);
        let mut world = World::default();
        BuildTalkCommand::new(world.spawn_empty().id(), builder).apply(&mut world);

        assert_relationship_nodes(6, 6, 1, &mut world);
    }

    #[rstest]
    fn actor_say_creates_node_with_actor_relationship(mut talk_builder: TalkBuilder) {
        let mut world = World::default();

        talk_builder = talk_builder
            .add_actor(Actor::new("actor", "Actor"))
            .actor_say("actor", "Hello");
        BuildTalkCommand::new(world.spawn_empty().id(), talk_builder).apply(&mut world);

        let mut query = world.query::<Relations<PerformedBy>>();

        // check number of nodes in the performed by relationship
        assert_eq!(query.iter(&world).count(), 2);

        let mut r_query = world.query::<(&TextNode, Relations<PerformedBy>)>();

        let (actor_ent, _) = world.query::<(Entity, With<Actor>)>().single(&world);

        // check that the only existing actor is in the relationship
        for (t, edges) in r_query.iter(&world) {
            assert_eq!(t.0, "Hello");
            assert_eq!(edges.targets(PerformedBy).len(), 1);
            for e in edges.targets(PerformedBy) {
                assert_eq!(actor_ent, *e);
            }
        }

        // need to add 1 cause of the start node
        assert_relationship_nodes(1, 2, 1, &mut world);
    }

    #[track_caller]
    fn assert_relationship_nodes(
        node_number: usize,
        expected_nodes_in_relation: usize,
        expected_leaf_nodes: usize,
        world: &mut World,
    ) {
        // some assertions on the relationship. We are collecting the vec for debug purposes.

        // there should be 1 root node in all cases (besides when 0 nodes)
        // For the 1 node case, there is still a root cause of the special start node
        // We have to use Leaf tho cause in aery Root and Leaf are swapped
        let root_nodes: Vec<_> = world
            .query::<(Entity, Leaf<FollowedBy>)>()
            .iter(&world)
            .collect();
        assert_eq!(root_nodes.len(), if node_number > 0 { 1 } else { 0 });

        // check relations (e1, e2)
        let related_nodes: Vec<_> = world
            .query::<(Entity, Relations<FollowedBy>)>()
            .iter(&world)
            .collect();
        assert_eq!(related_nodes.len(), expected_nodes_in_relation);

        // check leaf nodes
        let leaf_nodes: Vec<_> = world
            .query::<(Entity, Root<FollowedBy>)>()
            .iter(&world)
            .collect();
        assert_eq!(leaf_nodes.len(), expected_leaf_nodes);
    }
}
