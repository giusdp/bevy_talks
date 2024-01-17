//! The Bevy Command to spawn Talk entity graphs

use aery::prelude::*;
use bevy::{ecs::system::Command, prelude::*, utils::hashbrown::HashMap};

use crate::prelude::{
    ActorSlug, Choice, ChoiceNode, CurrentNode, EndNode, FollowedBy, PerformedBy, StartNode,
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
    /// The parent entity will be the parent of the dialogue graph and will have a `Talk` component.
    pub(crate) fn new(p: Entity, b: TalkBuilder) -> Self {
        Self {
            parent: p,
            builder: b,
        }
    }
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        // spawn the start node with all the start events
        let start = &world.spawn((StartNode, CurrentNode)).id();

        // First pass: spawn all the node entities and add them to the map with their build node id
        let (ents, mut node_entities) = spawn_dialogue_entities(&self.builder.queue, world);
        let actor_ents = spawn_actor_entities(&self.builder.actors, world);

        // add the start entity and all the other entities to the parent
        let mut manager = world.entity_mut(self.parent);
        manager.add_child(*start);
        for e in ents {
            manager.add_child(e);
        }

        // Second pass: Extract all the components associated with the nodes
        let component_map = prepare_node_components(&self.builder.queue, &node_entities, world);

        // and insert them in the world
        component_map.into_iter().for_each(|(e, comps)| {
            let mut entity_mut = world.entity_mut(e);
            for (comp, comp_reflect) in comps {
                let comp_to_insert = &**comp;
                comp_reflect.insert(&mut entity_mut, comp_to_insert);
            }
        });

        // Third pass: connect the entities to form the graph
        form_graph(
            *start,
            &self.builder.queue,
            self.builder.connect_parent,
            &mut node_entities,
            world,
        );

        // Fourth pass: connect the actors to the nodes
        connect_nodes_with_actors(&self.builder.queue, node_entities, actor_ents, world);
    }
}

/// Extract the components from the build nodes and return a map of entity => components,
/// so they can be inserted in the world.
fn prepare_node_components<'a>(
    build_nodes: &'a VecDeque<BuildNode>,
    node_entities: &HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> HashMap<Entity, Vec<(&'a Box<dyn Reflect>, ReflectComponent)>> {
    let mut entity_components = HashMap::new();
    for build_node in build_nodes {
        let Some(entity) = node_entities.get(&build_node.id) else {
            panic!("Error retrieving node entity while adding components. It should not happen!")
        };

        // extract the components
        let reflect_comps = {
            let type_reg = world.resource::<AppTypeRegistry>().read();
            build_node
                .components
                .iter()
                .map(|component| {
                    (
                        component,
                        type_reg
                            .get_type_data::<ReflectComponent>((**component).type_id())
                            .unwrap_or_else(|| {
                                panic!(
                                "Component {:?} not registered. Cannot build dialogue graph! :(",
                                component
                            )
                            })
                            .clone(),
                    )
                })
                .collect::<Vec<_>>()
        };

        entity_components.insert(*entity, reflect_comps);

        // recursively insert the inner nodes
        if !build_node.choices.is_empty() {
            for (_, inner_builder) in build_node.choices.iter() {
                let inner_comps =
                    prepare_node_components(&inner_builder.queue, node_entities, world);
                entity_components.extend(inner_comps);
            }
        }
    }
    entity_components
}

/// Connect the nodes to the actors.
fn connect_nodes_with_actors(
    build_nodes: &VecDeque<BuildNode>,
    node_entities: HashMap<String, Entity>,
    all_actors: HashMap<String, Entity>,
    world: &mut World,
) {
    for node in build_nodes {
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
                    &inner_builder.queue,
                    node_entities.clone(),
                    all_actors.clone(),
                    world,
                );
            }
        }
    }
}

/// Spawn the actor entities in the world and return a map of actor slug => entity.
/// If the actor is already present in the world (identified via the slug), it will not be spawned again.
fn spawn_actor_entities(actors: &[Actor], world: &mut World) -> HashMap<ActorSlug, Entity> {
    let mut actor_ents = HashMap::with_capacity(actors.len());

    // find the already existing actors in the world
    let already_spawned_actors = world
        .query::<(Entity, &Actor)>()
        .iter(world)
        .map(|(e, a)| (a.slug.clone(), (e, a.clone())))
        .collect::<HashMap<String, (Entity, Actor)>>();

    debug!("Already spawned actors: {:?}", already_spawned_actors);

    for a in actors.iter() {
        if already_spawned_actors.contains_key(&a.slug) {
            actor_ents.insert(a.slug.clone(), already_spawned_actors[&a.slug].0);
        } else {
            actor_ents.insert(a.slug.clone(), world.spawn(a.clone()).id());
        }
    }

    // add the remaining actors from the already spawned ones to the map
    for (slug, (e, _)) in already_spawned_actors.iter() {
        if !actor_ents.contains_key(slug) {
            actor_ents.insert(slug.clone(), *e);
        }
    }

    actor_ents
}

/// A recursive function that spawns all the nodes from a talk builder and adds them in the given hashmap.
/// It is used as the first pass of the building, so we have all the entities spawned and the `build_node_entities` map filled.
fn spawn_dialogue_entities(
    build_nodes: &VecDeque<BuildNode>,
    world: &mut World,
) -> (Vec<Entity>, HashMap<BuildNodeId, Entity>) {
    let mut entities: Vec<Entity> = Vec::with_capacity(build_nodes.len());
    let mut build_node_entities = HashMap::new();
    for n in build_nodes.iter() {
        let e = world.spawn_empty().id();
        entities.push(e);
        build_node_entities.insert(n.id.clone(), e);

        for (_, inner_builder) in n.choices.iter() {
            let (inner_ents, inner_bne) = spawn_dialogue_entities(&inner_builder.queue, world);
            entities.extend(inner_ents);
            build_node_entities.extend(inner_bne);
        }
    }
    (entities, build_node_entities)
}

/// A recursive function that connects the entity nodes in the queue with `aery` relations.
/// This also adds the `ChoiceNode` component!
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
    build_nodes: &VecDeque<BuildNode>,
    connect_parent: Option<BuildNodeId>,
    node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> (Entity, Vec<Entity>) {
    let mut parent = root;

    let mut first_child_set = false;
    let mut first_child_ent = root;

    // Connect parent entity (choice node) to the given node.
    if let Some(connect_node_id) = &connect_parent {
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

    if !build_nodes.is_empty() && !first_child_set {
        first_child_ent = *node_entities
            .get(&build_nodes[0].id)
            .expect("First entity from the builder");
    }

    let mut peekable_queue = build_nodes.iter().peekable();

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

        previous_node_was_choice = false;
        if !build_node.choices.is_empty() {
            // We have to process the branches from the inner builders
            // and connect them to the choice node
            let mut choices: Vec<Choice> = Vec::with_capacity(build_node.choices.len());
            for (choice_text, inner_builder) in build_node.choices.iter() {
                // recursively spawn the branches
                let (branch_root, branch_leaves) = form_graph(
                    this_ent,
                    &inner_builder.queue,
                    inner_builder.connect_parent.clone(),
                    node_entities,
                    world,
                );
                choices.push(Choice::new(choice_text, branch_root));
                leaves.extend(branch_leaves);
            }

            // insert the ChoiceNode component here
            world.entity_mut(this_ent).insert(ChoiceNode(choices));

            previous_node_was_choice = true;
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

    use crate::tests::{count, single, talks_minimal_app};

    use super::*;

    #[test]
    fn test_spawn_dialogue_entities() {
        let mut app = App::new();

        let builder = TalkBuilder::default()
            .say("Hello")
            .choose(vec![
                ("Choice 1".to_string(), TalkBuilder::default().say("Hi")),
                ("Choice 2".to_string(), TalkBuilder::default().say("World!")),
            ])
            .say("something");

        let (ents, map) = spawn_dialogue_entities(&builder.queue, &mut app.world);

        assert_eq!(map.len(), 5);
        assert_eq!(ents.len(), 5);
        assert_eq!(app.world.iter_entities().count(), 5);
    }

    #[test]
    fn spawn_actor_entities_without_already_spawned() {
        let mut app = App::new();

        let builder = TalkBuilder::default()
            .add_actor(Actor::new("my_actor", "Actor"))
            .add_actor(Actor::new("actor_0", "Actor2"))
            .say("Hello");

        let actor_ents = spawn_actor_entities(&builder.actors, &mut app.world);
        app.update();

        assert_eq!(actor_ents.len(), 2);
        assert_eq!(app.world.iter_entities().count(), 2);
    }

    #[test]
    fn spawn_actor_entities_with_prespawned() {
        let mut app = App::new();

        for i in 0..3 {
            app.world
                .spawn(Actor::new(format!("actor_{}", i), format!("Actor {}", i)));
        }

        let builder = TalkBuilder::default()
            .add_actor(Actor::new("my_actor", "Actor"))
            .add_actor(Actor::new("actor_0", "Actor2"))
            .say("Hello")
            .say("something");
        app.update();

        let actor_ents = spawn_actor_entities(&builder.actors, &mut app.world);
        app.update();

        assert_eq!(actor_ents.len(), 4);
        assert_eq!(app.world.iter_entities().count(), 4);
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

        let (_, node_entities) = spawn_dialogue_entities(&builder.queue, &mut app.world);
        let actor_ents = spawn_actor_entities(&builder.actors, &mut app.world);
        connect_nodes_with_actors(&builder.queue, node_entities, actor_ents, &mut app.world);

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
        assert_eq!(single::<(Entity, Leaf<FollowedBy>)>(&mut world).0, root_ent);
        assert_eq!(single::<(Entity, Root<FollowedBy>)>(&mut world).0, leaf_ent);
        if previous_node_was_choice {
            assert_eq!(count::<(Entity, Branch<FollowedBy>)>(&mut world), 2);
        }
    }

    #[test]
    fn test_add_relationships_simple() {
        let mut world = World::default();
        let root = world.spawn_empty().id();
        let builder = TalkBuilder::default().say("Hello There");
        let (_, mut build_node_entities) = spawn_dialogue_entities(&builder.queue, &mut world);

        let (ent, leaves) = form_graph(
            root,
            &builder.queue,
            builder.connect_parent,
            &mut build_node_entities,
            &mut world,
        );

        // Assert that the relationships are built correctly
        assert_ne!(ent, root);
        assert_eq!(leaves.len(), 1);
        assert_eq!(count::<Relations<FollowedBy>>(&mut world), 2);
    }
    #[test]
    fn test_add_relationships() {
        let mut world = World::default();
        let root = world.spawn_empty().id();

        let builder = TalkBuilder::default()
            .choose(vec![
                ("Choice Text".to_string(), TalkBuilder::default().say("t")),
                ("Choice Text 2".to_string(), TalkBuilder::default().say("t")),
            ])
            .say("something");

        let (_, mut build_node_entities) = spawn_dialogue_entities(&builder.queue, &mut world);

        form_graph(
            root,
            &builder.queue,
            builder.connect_parent,
            &mut build_node_entities,
            &mut world,
        );

        // Assert that the relationships are built correctly
        assert_eq!(count::<Relations<FollowedBy>>(&mut world), 5);
        assert_eq!(count::<(Entity, Leaf<FollowedBy>)>(&mut world), 1);
        assert_eq!(count::<(Entity, Branch<FollowedBy>)>(&mut world), 3);
        assert_eq!(count::<(Entity, Root<FollowedBy>)>(&mut world), 1);
    }

    #[test]
    fn prepare_node_components_with_choice() {
        let mut app = talks_minimal_app();
        let builder = TalkBuilder::default()
            .say("Hello There")
            .choose(vec![
                ("Choice Text".to_string(), TalkBuilder::default().say("a")),
                ("Choice Text 2".to_string(), TalkBuilder::default().say("b")),
            ])
            .say("something");

        let (_, build_node_entities) = spawn_dialogue_entities(&builder.queue, &mut app.world);

        let comps = prepare_node_components(&builder.queue, &build_node_entities, &mut app.world);

        // Assert that the map has all the entities
        assert_eq!(comps.len(), 5);
    }

    #[test]
    fn prepare_node_components_linear_graph() {
        let mut app = talks_minimal_app();

        let builder = TalkBuilder::default()
            .say("Hello There")
            .say("something")
            .say("something");

        let (_, build_node_entities) = spawn_dialogue_entities(&builder.queue, &mut app.world);

        let comps = prepare_node_components(&builder.queue, &build_node_entities, &mut app.world);

        // Assert that the map has all the entities
        assert_eq!(comps.len(), 3);
        for (_, comp) in comps.iter() {
            assert_eq!(comp.len(), 1);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use aery::tuple_traits::RelationEntries;
    use bevy::prelude::*;
    use rstest::{fixture, rstest};

    use crate::{
        prelude::TextNode,
        tests::{get_comp, talks_minimal_app},
    };

    use super::*;

    #[fixture]
    fn talk_builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    #[track_caller]
    fn build(builder: TalkBuilder) -> World {
        let mut app = talks_minimal_app();
        BuildTalkCommand::new(app.world.spawn_empty().id(), builder).apply(&mut app.world);
        app.update();
        app.world
    }

    #[rstest]
    #[should_panic]
    fn test_panic_on_wrong_actor(talk_builder: TalkBuilder) {
        build(talk_builder.actor_say("actor", "Hello"));
    }

    #[rstest]
    #[case(vec!["Hello"])]
    #[case(vec!["Hello", "World!"])]
    #[case(vec!["Hi", "Hello", "World!"])]
    #[case(vec!["Hi", "Hello", "World!", "The End"])]
    fn linear_say_graph_creation(mut talk_builder: TalkBuilder, #[case] text_nodes: Vec<&str>) {
        let node_number = text_nodes.len();
        for t in text_nodes.iter() {
            talk_builder = talk_builder.say(*t);
        }
        let mut world = build(talk_builder);
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
        for _ in 0..choice_node_number {
            talk_builder = talk_builder.choose(vec![
                ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                ("Choice2".to_string(), TalkBuilder::default().say("World!")),
            ]);
        }
        let mut world = build(talk_builder);
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
        let mut world = build(talk_builder);
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

        let mut world = build(builder);
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
        let mut world = build(builder);
        assert_relationship_nodes(6, 6, 1, &mut world);
    }

    #[rstest]
    fn actor_say_creates_node_with_actor_relationship(mut talk_builder: TalkBuilder) {
        talk_builder = talk_builder
            .add_actor(Actor::new("actor", "Actor"))
            .actor_say("actor", "Hello");
        let mut world = build(talk_builder);

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

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct TestComp;
    #[test]
    fn node_with_components() {
        let mut app = talks_minimal_app();
        app.register_type::<TestComp>();
        app.update();
        let builder = TalkBuilder::default()
            .say("Hello There")
            .add_component(TestComp);

        BuildTalkCommand::new(app.world.spawn_empty().id(), builder).apply(&mut app.world);
        app.update();

        // check that the start node has a NodeEvents component with the event
        let (ent, _) = app
            .world
            .query::<(Entity, With<TextNode>)>()
            .single(&app.world);
        get_comp::<TestComp>(ent, &mut app.world);
    }
}
