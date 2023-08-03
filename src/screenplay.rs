use bevy::{
    a11y::accesskit::Action,
    prelude::{Commands, Component, Entity},
};
use petgraph::prelude::DiGraph;

#[derive(Component)]
pub struct Screenplay {
    pub(crate) graph: DiGraph<ActionNode, ()>,
}

// Public API
impl Screenplay {
    // This method will help users to discover the builder
    pub fn builder() -> ScreenplayBuilder {
        ScreenplayBuilder::default()
    }
}

#[derive(Default)]
pub struct ScreenplayBuilder {
    // Probably lots of optional fields.
    nodes: Vec<ActionNode>,
}

impl ScreenplayBuilder {
    pub fn new(/* ... */) -> ScreenplayBuilder {
        // Set the minimally required fields of Foo.
        ScreenplayBuilder { nodes: vec![] }
    }

    pub fn add_action_node(mut self, action: ActionNode) -> ScreenplayBuilder {
        self.nodes.push(action);
        self
    }

    // If we can get away with not consuming the Builder here, that is an
    // advantage. It means we can use the Screenplay as a template for constructing
    // many Foos.
    pub fn build(self) -> Screenplay {
        Screenplay {
            graph: DiGraph::new(),
        }
    }
}

type ActionNode = Entity;

#[derive(Component)]
struct TalkComp {
    pub text: String,
}

fn new_talk(commands: &mut Commands, text: String) -> ActionNode {
    let c = commands.spawn(TalkComp { text });
    c.id()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn builder_test() {
        let sp_from_builder: Screenplay = ScreenplayBuilder::new().build();
        assert_eq!(sp_from_builder.graph.node_count(), 0);
    }

    // #[test]
    // fn new_with_one_action() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
    //             start: Some(true),
    //             ..default() // end: None,
    //         })],
    //     };
    //     let play = build_screenplay(raw_sp).unwrap();

    //     assert_eq!(play.graph.node_count(), 1);
    //     assert_eq!(play.graph.edge_count(), 0);
    //     assert_eq!(play.current, NodeIndex::new(0));
    // }

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
}

// impl Screenplay {
//     pub(crate) fn new(
//         graph: DiGraph<ActionNode, ()>,
//         current: NodeIndex,
//         id_to_nodeidx: HashMap<ActionId, NodeIndex>,
//     ) -> Self {
//         Self {
//             graph,
//             current,
//             id_to_nodeidx,
//         }
//     }

//     pub fn text(&self) -> &str {
//         match &self.graph[self.current].text {
//             Some(t) => t,
//             None => "",
//         }
//     }

//     pub fn next_action(&mut self) -> Result<(), NextRequestError> {
//         let cnode = self.graph.node_weight(self.current);
//         if let Some(current_act) = cnode {
//             // if it's a player action, return an error
//             if current_act.kind == ActionKind::PlayerChoice {
//                 return Err(NextRequestError::ChoicesNotHandled);
//             }

//             // retrieve the next edge
//             let edge_ref = self
//                 .graph
//                 .edges(self.current)
//                 .next()
//                 .ok_or(NextRequestError::NoNextAction)?;

//             // what's this NodeId? Is it the NodeIndex? I'm not sure. Let's assign it anyway
//             self.current = edge_ref.target();
//         }
//         Ok(())
//     }

//     pub fn jump_to(&mut self, id: i32) -> Result<(), ChoicesError> {
//         let idx = self
//             .id_to_nodeidx
//             .get(&id)
//             .ok_or(ChoicesError::WrongId(id))?;

//         self.current = *idx;
//         Ok(())
//     }

//     /// Returns the choices for the current dialogue. If there are no choices, returns an error.
//     pub fn choices(&self) -> Result<Vec<Choice>, ChoicesError> {
//         if let Some(cur_act) = self.graph.node_weight(self.current) {
//             return match &cur_act.choices {
//                 Some(choices) => Ok(choices.clone()),
//                 None => Err(ChoicesError::NotAChoiceAction),
//             };
//         }
//         Ok(vec![])
//     }

//     /// Returns the first actor for the current action.
//     pub fn first_actor(&self) -> Option<Actor> {
//         let cnode = self.graph.node_weight(self.current)?;
//         match &cnode.actors {
//             Some(actors) => actors.first().cloned(),
//             None => None,
//         }
//     }

//     /// Returns the actors for the current action.
//     pub fn actors(&self) -> Option<Vec<Actor>> {
//         let cnode = self.graph.node_weight(self.current)?;
//         cnode.actors.clone()
//     }

//     /// Returns true if the current action is a player choice.
//     pub fn at_player_action(&self) -> bool {
//         self.graph[self.current].kind == ActionKind::PlayerChoice
//     }

//     /// Returns true if the current action is an actor action.
//     pub fn at_actor_action(&self) -> bool {
//         !self.at_player_action()
//     }

//     /// Returns the kind of the current action.
//     pub fn action_kind(&self) -> ActionKind {
//         self.graph[self.current].kind
//     }

//     pub(crate) fn sound_effect(&self) -> Option<String> {
//         self.graph[self.current].sound_effect.clone()
//     }
// }

// #[derive(Debug, Default)]
// pub(crate) struct ActionNode {
//     pub(crate) kind: ActionKind,
//     pub(crate) text: Option<String>,
//     pub(crate) actors: Option<Vec<Actor>>,
//     pub(crate) choices: Option<Vec<Choice>>,
//     pub(crate) sound_effect: Option<String>,
// }

// #[cfg(test)]
// mod test {
//     use bevy::prelude::default;

//     use crate::{
//         loader::build_screenplay,
//         types::{
//             ActorAction, ActorActionKind, ActorOrPlayerActionJSON, PlayerAction, RawScreenplay,
//         },
//     };

//     use super::*;

//     // 'current_text' tests
//     #[test]
//     fn current_text() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 id: 1,
//                 text: Some("Hello".to_string()),
//                 next: None,
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.text(), "Hello");
//     }

//     // 'next_line' tests
//     #[test]
//     fn next_no_next_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 id: 1,
//                 text: Some("Hello".to_string()),
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let mut play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(
//             play.next_action().err(),
//             Some(NextRequestError::NoNextAction)
//         );
//     }

//     #[test]
//     fn next_choices_not_handled_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 1,
//                     choices: vec![Choice {
//                         text: "Whatup".to_string(),
//                         next: 2,
//                     }],
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
//             ],
//         };

//         let mut play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(
//             play.next_action().err(),
//             Some(NextRequestError::ChoicesNotHandled)
//         );
//     }

//     #[test]
//     fn next_action() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     next: Some(2),
//                     start: Some(true),
//                     text: Some("Hello".to_string()),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 2,
//                     text: Some("Whatup".to_string()),
//                     ..default()
//                 }),
//             ],
//         };

//         let mut play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.text(), "Hello");
//         assert!(play.next_action().is_ok());
//         assert_eq!(play.text(), "Whatup");
//     }

//     // 'choices' tests
//     #[test]
//     fn choices_no_choices_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 id: 1,
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.choices().err(), Some(ChoicesError::NotAChoiceAction));
//     }

//     #[test]
//     fn choices() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 1,
//                     choices: vec![
//                         Choice {
//                             text: "Choice 1".to_string(),
//                             next: 2,
//                         },
//                         Choice {
//                             text: "Choice 2".to_string(),
//                             next: 3,
//                         },
//                     ],
//                     start: Some(true),
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).unwrap();

//         assert_eq!(play.choices().unwrap()[0].next, 2);
//         assert_eq!(play.choices().unwrap()[1].next, 3);
//         assert_eq!(play.choices().unwrap()[0].text, "Choice 1");
//         assert_eq!(play.choices().unwrap()[1].text, "Choice 2");
//     }

//     // 'jump_to' tests
//     #[test]
//     fn jump_to_no_action_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 id: 1,
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let mut play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.jump_to(2).err(), Some(ChoicesError::WrongId(2)));
//     }

//     #[test]
//     fn jump_to() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 1,
//                     choices: vec![
//                         Choice {
//                             text: "Choice 1".to_string(),
//                             next: 2,
//                         },
//                         Choice {
//                             text: "Choice 2".to_string(),
//                             next: 3,
//                         },
//                     ],
//                     start: Some(true),
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 2,
//                     text: Some("I'm number 2".to_string()),
//                     next: Some(3),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
//             ],
//         };

//         let mut play = build_screenplay(raw_sp).unwrap();
//         assert!(play.jump_to(2).is_ok());
//         assert_eq!(play.text(), "I'm number 2");
//     }

//     // 'current_first_actor' tests
//     #[test]
//     fn first_actor_none() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert!(play.first_actor().is_none());
//     }

//     #[test]
//     fn first_actor() {
//         let mut actors = HashMap::new();
//         actors.insert(
//             "bob".to_string(),
//             Actor {
//                 name: "Bob".to_string(),
//                 asset: "bob.png".to_string(),
//             },
//         );

//         let raw_sp = RawScreenplay {
//             actors,
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 actors: vec!["bob".to_string()],
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert!(play.first_actor().is_some());
//     }

//     #[test]
//     fn current_actors() {
//         let mut actors = HashMap::new();
//         actors.insert(
//             "bob".to_string(),
//             Actor {
//                 name: "Bob".to_string(),
//                 asset: "bob.png".to_string(),
//             },
//         );
//         actors.insert(
//             "alice".to_string(),
//             Actor {
//                 name: "alice".to_string(),
//                 asset: "alice".to_string(),
//             },
//         );

//         let raw_sp = RawScreenplay {
//             actors,
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 actors: vec!["bob".to_string(), "alice".to_string()],
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.actors().unwrap().len(), 2);
//     }

//     #[test]
//     fn at_player_action() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 1,
//                     choices: vec![Choice {
//                         text: "Whatup".to_string(),
//                         next: 2,
//                     }],
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert!(play.at_player_action());
//     }

//     #[test]
//     fn action_kind_player() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 1,
//                     choices: vec![Choice {
//                         text: "Whatup".to_string(),
//                         next: 2,
//                     }],
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.action_kind(), ActionKind::PlayerChoice);
//     }

//     #[test]
//     fn action_kind_actor() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 1,
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 2,
//                     action: ActorActionKind::Enter,
//                     ..default()
//                 }),
//             ],
//         };

//         let mut play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.action_kind(), ActionKind::ActorTalk);
//         play.next_action().unwrap();
//         assert_eq!(play.action_kind(), ActionKind::ActorEnter);
//     }
// }

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
