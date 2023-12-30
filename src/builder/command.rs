//! The Bevy Command to spawn Talk entity graphs
use aery::prelude::*;
use bevy::{ecs::system::Command, prelude::*, utils::hashbrown::HashMap};

use crate::prelude::{
    ActorSlug, ChoiceNodeBundle, CurrentNode, FollowedBy, PerformedBy, StartTalk, Talk,
    TalkNodeBundle,
};

use super::*;

/// The command that spawns a dialogue graph in the world.
/// You can create this command via the `build` method of the [`TalkBuilder`] struct.
pub struct BuildTalkCommand {
    /// The builder that contains the queue of nodes to spawn.
    pub(crate) builder: TalkBuilder,
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        // spawn the start node
        let start = &world.spawn((StartTalk, CurrentNode)).id();

        let mut node_entities: HashMap<BuildNodeId, Entity> = HashMap::new();

        // First pass: spawn all the node entities and add them to the map with their build node id
        let ents = spawn_dialogue_entities(&self.builder, &mut node_entities, world);
        let actor_ents: HashMap<ActorSlug, Entity> = spawn_actor_entities(&self.builder, world);

        let mut manager = world.spawn(Talk::default());
        manager.add_child(*start);
        for e in ents {
            manager.add_child(e);
        }

        // Second pass: connect them to form the graph
        form_graph(*start, &self.builder, &mut node_entities, world);

        // Third pass: connect the actors to the nodes
        for node in self.builder.queue.iter() {
            if !node.actors.is_empty() {
                let node_ent = node_entities.get(&node.id).unwrap();
                for actor in node.actors.iter() {
                    let actor_ent = actor_ents.get(actor).unwrap();
                    world.entity_mut(*node_ent).set::<PerformedBy>(*actor_ent);
                }
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
    // TODO Perf: this is probably not the most efficient way to do this. Looks pretty slow.

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
    build_node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> Vec<Entity> {
    let mut entities: Vec<Entity> = Vec::with_capacity(talk_builder.queue.len());
    for n in talk_builder.queue.iter() {
        let e = world.spawn_empty().id();
        entities.push(e);
        build_node_entities.insert(n.id.clone(), e);

        for (_, inner_builder) in n.choices.iter() {
            spawn_dialogue_entities(inner_builder, build_node_entities, world);
        }
    }
    entities
}

/// A recursive function that spawns all the nodes in the queue and connects them to each other.
///
/// # Returns
///
/// A vector of leaf nodes spawned from the given builder. It is used internally during the recursion to connect
/// a the leaves from the branches created from a choice node to the successive node in the queue.
fn form_graph(
    root: Entity,
    talk_builder: &TalkBuilder,
    node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> Vec<Entity> {
    let mut parent = root;

    // Connect parent entity (choice node) to the given node.
    if let Some(connect_node_id) = &talk_builder.connect_parent {
        let entity_to_connect_to = node_entities.get(connect_node_id);

        if let Some(e) = entity_to_connect_to {
            use aery::prelude::*;
            world.entity_mut(parent).set::<FollowedBy>(*e);
        } else {
            error!("Attempted to connect a choice node to some specific node that is not (yet) present in the builder.");
        }
    }

    let mut leaves: Vec<Entity> = vec![];
    let mut previous_node_was_choice = false;

    let mut peekable_queue = talk_builder.queue.iter().peekable();

    // for each node in the queue, spawn it and connect it to the previous one
    while let Some(build_node) = peekable_queue.next() {
        // retrieve the child node
        let child = *node_entities
            .get(&build_node.id)
            .expect("Error! Dialogue node entity not found. Cannot build dialogue graph! :(");

        // if the choices are empty, it's a talk node
        match build_node.kind {
            NodeKind::Talk => {
                world
                    .entity_mut(child)
                    .insert(TalkNodeBundle::new(build_node.text.clone()));
                connect_to_previous(world, parent, &mut leaves, previous_node_was_choice, child);
                previous_node_was_choice = false;
            }
            NodeKind::Choice => {
                connect_to_previous(world, parent, &mut leaves, previous_node_was_choice, child);

                // We have to spawn the branches from the inner builders
                // and connect them to the choice node
                let mut choices_texts = Vec::with_capacity(build_node.choices.len());
                for (choice_text, inner_builder) in build_node.choices.clone() {
                    choices_texts.push(choice_text);
                    // recursively spawn the branches
                    let branch_leaves = form_graph(child, &inner_builder, node_entities, world);
                    leaves.extend(branch_leaves);
                }
                // insert the ChoicesTexts component
                world
                    .entity_mut(child)
                    .insert(ChoiceNodeBundle::new(choices_texts));

                previous_node_was_choice = true;
            }
            NodeKind::Join => {
                world.entity_mut(child).insert(NodeKind::Join);
                connect_to_previous(world, parent, &mut leaves, previous_node_was_choice, child);
                previous_node_was_choice = false;
            }
            NodeKind::Leave => {
                world.entity_mut(child).insert(NodeKind::Leave);
                connect_to_previous(world, parent, &mut leaves, previous_node_was_choice, child);
                previous_node_was_choice = false;
            }
        }

        // Let's add the extra connections here
        process_manual_connections(node_entities, &build_node.manual_connections, child, world);

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

        let mut map = HashMap::new();
        spawn_dialogue_entities(&builder, &mut map, &mut app.world);

        assert_eq!(map.len(), 5);
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
        let mut build_node_entities = HashMap::default();
        let root = world.spawn_empty().id();
        let talk_builder = TalkBuilder::default().say("Hello There");
        spawn_dialogue_entities(&talk_builder, &mut build_node_entities, &mut world);

        let leaves = form_graph(root, &talk_builder, &mut build_node_entities, &mut world);

        // Assert that the relationships are built correctly
        assert_eq!(leaves.len(), 1);
        assert_eq!(
            world.query::<Relations<FollowedBy>>().iter(&world).count(),
            2
        );
    }

    #[test]
    fn test_add_relationships() {
        let mut world = World::default();
        let mut build_node_entities = HashMap::default();
        let root = world.spawn_empty().id();

        let talk_builder = TalkBuilder::default()
            .choose(vec![
                ("Choice Text".to_string(), TalkBuilder::default().say("t")),
                ("Choice Text 2".to_string(), TalkBuilder::default().say("t")),
            ])
            .say("something");

        spawn_dialogue_entities(&talk_builder, &mut build_node_entities, &mut world);

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

    use crate::prelude::TalkText;

    use super::*;

    #[fixture]
    fn talk_builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    #[rstest]
    #[case(vec!["Hello"])]
    #[case(vec!["Hello", "World!"])]
    #[case(vec!["Hi", "Hello", "World!"])]
    #[case(vec!["Hi", "Hello", "World!", "The End"])]
    fn linear_say_graph_creation(mut talk_builder: TalkBuilder, #[case] text_nodes: Vec<&str>) {
        use crate::prelude::TalkText;

        let mut app = App::new();
        let node_number = text_nodes.len();

        for t in text_nodes.iter() {
            talk_builder = talk_builder.say(*t);
        }

        talk_builder.build().apply(&mut app.world);

        let mut query = app.world.query::<&TalkText>();

        // check number of nodes with the text component
        assert_eq!(query.iter(&app.world).count(), node_number);

        // check texts
        for t in query.iter(&app.world) {
            assert!(text_nodes.iter().any(|&s| s == t.0));
        }

        // need to add 1 cause of the start node
        assert_relationship_nodes(node_number, node_number + 1, 1, &mut app);
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
        use crate::prelude::Choices;

        let mut app = App::new();

        for _ in 0..choice_node_number {
            talk_builder = talk_builder.choose(vec![
                ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                ("Choice2".to_string(), TalkBuilder::default().say("World!")),
            ]);
        }

        talk_builder.build().apply(&mut app.world);

        let mut query = app.world.query::<&Choices>();

        // check length
        assert_eq!(query.iter(&app.world).count(), choice_node_number);

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
    fn interleaved_choice_and_say_graph_creation(
        mut talk_builder: TalkBuilder,
        #[case] choice_number: usize,
        #[case] say_number: usize,
        #[case] expected_nodes: usize,
        #[case] expected_leaves: usize,
    ) {
        let mut app = App::new();

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

        talk_builder.build().apply(&mut app.world);

        assert_relationship_nodes(choice_number, expected_nodes, expected_leaves, &mut app);
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

        // TODO: I should assert on the actual structure of the graph instead of simple number of nodes, leaf and roots.
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

        assert_relationship_nodes(6, 6, 1, &mut app);
    }

    #[rstest]
    fn error_actor_say_with_no_existent_actor(talk_builder: TalkBuilder) {
        assert!(talk_builder.actor_say("actor", "Hello").is_err());
    }

    #[rstest]
    fn actor_say_creates_node_with_actor_relationship(mut talk_builder: TalkBuilder) {
        let mut app = App::new();

        talk_builder = talk_builder
            .add_actor(Actor::new("actor", "Actor"))
            .actor_say("actor", "Hello")
            .unwrap();

        talk_builder.build().apply(&mut app.world);

        let mut query = app.world.query::<Relations<PerformedBy>>();

        // check number of nodes in the performed by relationship
        assert_eq!(query.iter(&app.world).count(), 2);

        let mut r_query = app.world.query::<(&TalkText, Relations<PerformedBy>)>();

        let (actor_ent, _) = app
            .world
            .query::<(Entity, With<Actor>)>()
            .single(&app.world);

        // check that the only existing actor is in the relationship
        for (t, edges) in r_query.iter(&app.world) {
            assert_eq!(t.0, "Hello");
            assert_eq!(edges.targets(PerformedBy).len(), 1);
            for e in edges.targets(PerformedBy) {
                assert_eq!(actor_ent, *e);
            }
        }

        // need to add 1 cause of the start node
        assert_relationship_nodes(1, 2, 1, &mut app);
    }

    #[track_caller]
    fn assert_relationship_nodes(
        node_number: usize,
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
