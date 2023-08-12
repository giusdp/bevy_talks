//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::prelude::{Commands, Component, Entity};
use petgraph::visit::EdgeRef;
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};

use crate::prelude::NextActionError;

/// A screenplay is a directed graph of actions.
/// The nodes of the graph are the actions, which are
/// bevy entities with specific [`bevy_talks`] components.
/// The Screenplay struct keeps track of the current action
/// and provides functions to move to the next action.
#[derive(Debug, Component)]
pub struct Screenplay {
    /// The graph that represents the screenplay.
    ///
    /// This field is a directed graph that represents the structure of the screenplay. Each node in the
    /// graph represents an action in the screenplay, and each edge represents a transition between
    /// actions.
    pub(crate) graph: DiGraph<ActionNode, ()>,
    /// The index of the current node in the screenplay graph.
    ///
    /// This field is used to keep track of the current node in the screenplay graph. It is updated
    /// whenever the [`next_action`] method is called.
    pub(crate) current_node: NodeIndex,
}

// Public API
impl Screenplay {
    /// Create a new [`ScreenplayBuilder`] with default values.
    pub fn builder() -> ScreenplayBuilder {
        ScreenplayBuilder::default()
    }

    /// Move to the next action. Returns an error if the current action
    /// has no next action.
    pub fn next_action(&mut self) -> Result<(), NextActionError> {
        if self.graph.node_weight(self.current_node).is_some() {
            // retrieve the next edge
            let edge_ref = self
                .graph
                .edges(self.current_node)
                .next()
                .ok_or(NextActionError::NoNextAction)?;

            self.current_node = edge_ref.target();
        }
        Ok(())
    }
}

/// The [`ScreenplayBuilder`] is used to construct a Screenplay.
/// It is a builder pattern implementation.
#[derive(Default)]
pub struct ScreenplayBuilder {
    /// The nodes of the screenplay.
    nodes: Vec<ActionNode>,
}

impl ScreenplayBuilder {
    /// Create a new [`ScreenplayBuilder`] with default values.
    pub fn new() -> ScreenplayBuilder {
        // Set the minimally required fields of Foo.
        ScreenplayBuilder { nodes: vec![] }
    }

    /// Add an action node to the screenplay.
    pub fn add_action_node(mut self, action: ActionNode) -> ScreenplayBuilder {
        self.nodes.push(action);
        self
    }

    /// Build the screenplay.
    pub fn build(self) -> Screenplay {
        if self.nodes.is_empty() {
            return Screenplay {
                graph: DiGraph::new(),
                current_node: 0.into(),
            };
        }

        // 1. Create the graph
        let mut graph: DiGraph<ActionNode, ()> = DiGraph::new();

        let first_action = self.nodes[0];
        let mut prev_node = graph.add_node(first_action);
        let current_node = prev_node;

        // 2. Add all actions as nodes and connect them linearly
        for action in self.nodes[1..].iter() {
            let curr_node = graph.add_node(*action);
            graph.add_edge(prev_node, curr_node, ());
            prev_node = curr_node;
        }

        Screenplay {
            graph,
            current_node,
        }
    }
}

/// An `ActionNode` is an entity that represents an action in a screenplay.
///
/// Action nodes are used to define the actions that characters perform in a screenplay. They can be
/// linked together to create a sequence of actions that make up a scene or an entire screenplay.
type ActionNode = Entity;

/// A component that indicates that the entity is a "talk".
/// It contains only the text to be displayed, without any
/// information about the speaker.
/// For example, it can be used to display text said by a narrator
/// and no speaker name is needed.
/// Use [`SpeakerTalkComp`] to have text and speaker.
#[derive(Component)]
pub struct TalkComp {
    /// The text to be displayed.
    pub text: String,
}

/// Spawn a new entity with a [`TalkComp`] component attached.
pub fn new_talk(commands: &mut Commands, text: String) -> ActionNode {
    let c = commands.spawn(TalkComp { text });
    c.id()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_empty_screenplay() {
        let sp: Screenplay = ScreenplayBuilder::new().build();
        assert_eq!(sp.graph.node_count(), 0);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn build_one_action_screenplay() {
        let sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert_eq!(sp.graph.node_count(), 1);
        assert_eq!(sp.graph.edge_count(), 0);
        assert!(sp.current_node.index() == 0);
    }

    #[test]
    fn build_two_action_screenplay() {
        let sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert_eq!(sp.graph.node_count(), 2);
        assert_eq!(sp.graph.edge_count(), 1);
        assert!(sp.current_node.index() == 0);
    }

    #[test]
    fn build_three_action_screenplay() {
        let sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .add_action_node(ActionNode::PLACEHOLDER)
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert_eq!(sp.graph.node_count(), 3);
        assert_eq!(sp.graph.edge_count(), 2);
        assert!(sp.current_node.index() == 0);
    }

    // #[test]
    // fn new_with_two_actor_action_nodes() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 1,
    //                 next: Some(2),
    //                 start: Some(true),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
    //         ],
    //     };

    //     let play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(play.graph.node_count(), 2);
    //     assert_eq!(play.graph.edge_count(), 1);
    // }

    // #[test]
    // fn new_with_self_loop() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
    //             id: 1,
    //             next: Some(1),
    //             start: Some(true),
    //             ..default()
    //         })],
    //     };

    //     let play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(play.graph.node_count(), 1);
    //     assert_eq!(play.graph.edge_count(), 1);
    // }

    // #[test]
    // fn new_with_branching() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![
    //             ActorOrPlayerActionJSON::Player(PlayerAction {
    //                 choices: vec![
    //                     Choice {
    //                         text: "Choice 1".to_string(),
    //                         next: 2,
    //                     },
    //                     Choice {
    //                         text: "Choice 2".to_string(),
    //                         next: 3,
    //                     },
    //                 ],
    //                 start: Some(true),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 2,
    //                 text: Some("Hello".to_string()),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
    //         ],
    //     };

    //     let play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(play.graph.node_count(), 3);
    //     assert_eq!(play.graph.edge_count(), 4);
    //     assert_eq!(play.current, NodeIndex::new(0));
    // }

    // #[test]
    // fn new_with_actors() {
    //     let mut actors_map = an_actors_map("bob".to_string());
    //     actors_map.insert(
    //         "alice".to_string(),
    //         Actor {
    //             name: "Alice".to_string(),
    //             asset: "alice.png".to_string(),
    //         },
    //     );

    //     let raw_sp = RawScreenplay {
    //         actors: actors_map,
    //         script: vec![
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 1,
    //                 text: Some("Hello".to_string()),
    //                 actors: vec!["bob".to_string()],
    //                 next: Some(2),
    //                 start: Some(true),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 2,
    //                 text: Some("Whatup".to_string()),
    //                 actors: vec!["alice".to_string()],
    //                 ..default()
    //             }),
    //         ],
    //     };
    //     let play = build_screenplay(raw_sp).unwrap();

    //     assert_eq!(play.graph.node_count(), 2);
    //     assert_eq!(play.graph.edge_count(), 1);
    //     assert_eq!(play.current, NodeIndex::new(0));
    // }

    #[test]
    fn next_no_next_err() {
        let mut sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert_eq!(sp.next_action().err(), Some(NextActionError::NoNextAction));
    }

    #[test]
    fn next_action() {
        let mut sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert!(sp.next_action().is_ok());
    }
}

// BUILDER STUFF --------------------------------------------------------------

// pub(crate) fn build_screenplay(
//     raw_script: RawScreenplay,
// ) -> Result<Screenplay, ScreenplayParsingError> {
//     if raw_script.script.is_empty() {
//         return Err(ScreenplayParsingError::EmptyScript);
//     }
//     let mut graph: DiGraph<ActionNode, ()> = DiGraph::new();

//     let mut start_action = Option::<NodeIndex>::None;

//     // 1. Build auxiliary maps (I'm bad at naming maps)

//     // ActionId => next_id map, so we can fill the next when it's None
//     // (it means point to the next action) and throw duplicate id error
//     let id_to_next_map = build_id_to_next_map(&raw_script.script)?;

//     // ActionId => (NodeIndex, next_id) map so we can keep track of what we added in the graph.
//     // Right now ActionId == NodeIndex so not really needed, but I'd like to have uuids as ids in the future
//     let mut id_to_nodeids_map: HashMap<ActionId, StrippedNodeAction> =
//         HashMap::with_capacity(raw_script.script.len());

//     // 2. Add all actions as nodes with some validation
//     for action in raw_script.script {
//         let this_action_id = action.id();
//         let start_flag = action.start();

//         // Grab the nexts in the choices for later validation
//         let choices_nexts = action
//             .choices()
//             .map(|vc| vc.iter().map(|c| c.next).collect());

//         // 2.a add the node to the graph
//         let node_idx = add_action_node(&mut graph, action, &raw_script.actors)?;

//         // 2.b check if this is the starting action
//         if check_start_flag(start_flag, start_action.is_some())? {
//             start_action = Some(node_idx);
//         }

//         // 2.c add (idx, next_id) as we build the graph

//         if id_to_nodeids_map
//             .insert(
//                 this_action_id,
//                 StrippedNodeAction {
//                     node_idx,
//                     next_action_id: id_to_next_map.get(&this_action_id).copied(),
//                     choices: choices_nexts,
//                 },
//             )
//             .is_some()
//         {
//             return Err(ScreenplayParsingError::RepeatedId(this_action_id));
//         };
//     }

//     // 3 Validate all the nexts (they should point to existing actions)
//     validate_nexts(&id_to_nodeids_map)?;

//     // 4 Add edges to the graph
//     for (action_id, this_action) in &id_to_nodeids_map {
//         // 4.a With the next field, add a single edge
//         if let Some(next_id) = this_action.next_action_id {
//             let next_node_action = id_to_nodeids_map.get(&next_id).ok_or(
//                 ScreenplayParsingError::NextActionNotFound(*action_id, next_id),
//             )?;
//             graph.add_edge(this_action.node_idx, next_node_action.node_idx, ());
//         } else if let Some(choices) = &this_action.choices {
//             // 4.b With the choices, add an edge for each choice
//             for choice in choices {
//                 let chosen_action = id_to_nodeids_map.get(choice).ok_or(
//                     ScreenplayParsingError::NextActionNotFound(*action_id, *choice),
//                 )?;

//                 info!(
//                     "ASKJDASJDMASKLJDM {} -> {:?}",
//                     action_id, chosen_action.node_idx
//                 );
//                 graph.add_edge(this_action.node_idx, chosen_action.node_idx, ());
//             }
//         }
//     }

//     // 5. We can drop the next/choices now and just keep action_id => NodeIndex
//     let id_to_nodeidx = id_to_nodeids_map
//         .into_iter()
//         .map(|(id, node_act)| (id, node_act.node_idx))
//         .collect();

//     Ok(Screenplay::new(
//         graph,
//         start_action.ok_or(ScreenplayParsingError::NoStartingAction)?,
//         id_to_nodeidx,
//     ))
// }
