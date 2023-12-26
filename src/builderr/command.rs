use aery::prelude::*;
use bevy::{ecs::system::Command, prelude::*, utils::hashbrown::HashMap};

use crate::prelude::{ChoicesTexts, TalkStart, TalkText};

use super::{
    builder::{BuildNodeId, TalkBuilder},
    FollowedBy,
};

/// The command that spawns a dialogue graph in the world.
/// You can create this command via the `build` method of the [`TalkBuilder`] struct.
pub struct BuildTalkCommand {
    /// The builder that contains the queue of nodes to spawn.
    pub(crate) builder: TalkBuilder,
}

impl Command for BuildTalkCommand {
    fn apply(self, world: &mut World) {
        // spawn the start node
        let start = world.spawn(TalkStart).id();

        let mut build_node_entities = HashMap::new();

        // First pass: spawn all the node entities and add them to the map with their build node id
        spawn_dialogue_entities(&self.builder, &mut build_node_entities, world);

        // Second pass: connect them to form the graph
        form_graph(start, self.builder, &mut build_node_entities, world);
    }
}

/// A recursive function that spawns all the nodes from a talk builder and adds them in the given hashmap.
/// It is used as the first pass of the building, so we have all the entities spawned and the `build_node_entities` map filled.
fn spawn_dialogue_entities(
    talk_builder: &TalkBuilder,
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
fn form_graph(
    root: Entity,
    talk_builder: TalkBuilder,
    build_node_entities: &mut HashMap<BuildNodeId, Entity>,
    world: &mut World,
) -> Vec<Entity> {
    let mut parent = root;

    // Connect parent entity (choice node) to the given node.
    if let Some(connect_node_id) = &talk_builder.connect_parent {
        let entity_to_connect_to = build_node_entities.get(connect_node_id);

        if let Some(e) = entity_to_connect_to {
            use aery::prelude::*;
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
                        form_graph(child, inner_builder, build_node_entities, world);
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

#[cfg(test)]
mod tests {
    use std::vec;

    use bevy::{prelude::*, utils::HashMap};
    use rstest::rstest;

    use super::*;
    use crate::prelude::TalkBuilder;

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

        let leaves = form_graph(root, talk_builder, &mut build_node_entities, &mut world);

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

        form_graph(root, talk_builder, &mut build_node_entities, &mut world);

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
    use bevy::prelude::*;
    use rstest::{fixture, rstest};

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
        use crate::prelude::ChoicesTexts;

        let mut app = App::new();

        for _ in 0..choice_node_number {
            talk_builder = talk_builder.choose(vec![
                ("Choice1".to_string(), TalkBuilder::default().say("Hello")),
                ("Choice2".to_string(), TalkBuilder::default().say("World!")),
            ]);
        }

        talk_builder.build().apply(&mut app.world);

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
