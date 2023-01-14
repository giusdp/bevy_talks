use crate::Dialogue;
use bevy::{prelude::Component, ui::Node, utils::HashMap};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, visit::IntoNodeIdentifiers};
use thiserror::Error;

#[derive(Debug, Clone, Component)]
pub struct Conversation {
    graph: DiGraph<Box<Dialogue>, ()>,
    current: NodeIndex,
}

#[derive(Error, Debug)]
pub enum ConversationError {
    #[error("an empty dialogue vector was used to build the conversation")]
    NoDialogues,
    #[error("the dialogue with id {0} is pointing to id {1} which was not found")]
    NextDialogueNotFound(i32, i32),
    #[error("the dialogue tree cannot be built: {0}")]
    InvalidDialogueTree(String),
    #[error("the dialogue with id {0} has the same id as another dialogue")]
    RepeatedId(i32),
}

impl Conversation {
    pub fn new(ds: Vec<Dialogue>) -> Result<Self, ConversationError> {
        if ds.is_empty() {
            return Err(ConversationError::NoDialogues);
        }

        let mut graph: DiGraph<Box<Dialogue>, ()> = DiGraph::new();

        let mut node_indices = vec![];

        // Build a dialogue.id => (NodeIndex, Dialogue) map so we can analize dialogues while keeping the index in the graph
        // This might be useful if we want to have alphanumeric IDs instead of simple i32
        let mut nodeidx_dialogue_map: HashMap<i32, (NodeIndex, Box<Dialogue>)> = HashMap::new();

        // Start by adding all dialogues as nodes
        for d in ds {
            let id = d.id;
            let boxed_dialogue = Box::new(d);
            let node_idx = graph.add_node(boxed_dialogue.clone());
            node_indices.push(node_idx);
            if let Some(_) = nodeidx_dialogue_map.insert(id, (node_idx, boxed_dialogue)) {
                return Err(ConversationError::RepeatedId(id));
            }
        }

        // Add edges to the graph
        for (current_node_idx, current_dialogue) in nodeidx_dialogue_map.values() {
            // If the current dialogue has a next field, add an edge to the next dialogue
            if let Some(next_id) = current_dialogue.next {
                match nodeidx_dialogue_map.get(&next_id) {
                    Some((next_node_idx, _)) => {
                        graph.add_edge(*current_node_idx, *next_node_idx, ())
                    }
                    None => {
                        return Err(ConversationError::NextDialogueNotFound(
                            current_dialogue.id,
                            next_id,
                        ))
                    }
                };
            } else if let Some(choices) = &current_dialogue.choices {
                for choice in choices {
                    match nodeidx_dialogue_map.get(&choice.next) {
                        Some(_) => graph.add_edge(*current_node_idx, *current_node_idx, ()),
                        None => {
                            return Err(ConversationError::NextDialogueNotFound(
                                current_dialogue.id,
                                choice.next,
                            ));
                        }
                    };
                }
            }
        }

        Ok(Self {
            graph,
            current: NodeIndex::new(0),
        })
    }

    // pub fn current_text(&self) -> &str {
    //     self.graph.current_text()
    // }

    // pub fn current_dialogue(&self) -> &Dialogue {
    //     self.graph.current_dialogue()
    // }

    // pub fn next(&mut self) {
    //     if let Some(next) = self.tree.next() {
    //         self.graph = next.clone();
    //     }
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Choice, Talker};

    #[test]
    fn new_error_with_empty_vec() {
        let ds = vec![];
        let graph = Conversation::new(ds);
        assert!(graph.is_err());
    }

    #[test]
    fn new_error_repeated_id() {
        let ds = vec![
            Dialogue {
                id: 1,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Bob".to_string(),
                    asset: "bob.png".to_string(),
                },
                choices: None,
                next: None,
            },
            Dialogue {
                id: 1,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Alice".to_string(),
                    asset: "alice.png".to_string(),
                },
                choices: None,
                next: None,
            },
        ];

        let graph = Conversation::new(ds);
        assert!(graph.is_err());
    }

    fn new_with_one_dialogue() {
        let ds = vec![Dialogue {
            id: 1,
            text: "Hello".to_string(),
            talker: Talker {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            },
            choices: None,
            next: None,
        }];

        let conv = Conversation::new(ds).unwrap();
        assert_eq!(conv.graph.node_count(), 1);
        assert_eq!(conv.graph.edge_count(), 0);
    }

    #[test]
    fn new_2_linear_nodes_builds_graph() {
        let ds = vec![
            Dialogue {
                id: 1,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Bob".to_string(),
                    asset: "bob.png".to_string(),
                },
                choices: None,
                next: Some(2),
            },
            Dialogue {
                id: 2,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Alice".to_string(),
                    asset: "alice.png".to_string(),
                },
                choices: None,
                next: None,
            },
        ];

        let convo = Conversation::new(ds).unwrap();

        assert_eq!(convo.graph.node_count(), 2);
        assert_eq!(convo.graph.edge_count(), 1);
        assert_eq!(convo.current, NodeIndex::new(0));
    }

    #[test]
    fn new_error_when_next_not_found() {
        let ds = vec![Dialogue {
            id: 1,
            text: "Hello".to_string(),
            talker: Talker {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            },
            choices: None,
            next: Some(2),
        }];

        let convo = Conversation::new(ds);
        assert!(convo.is_err());
    }

    #[test]
    fn new_success_when_next_is_itself() {
        let ds = vec![Dialogue {
            id: 1,
            text: "Hello".to_string(),
            talker: Talker {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            },
            choices: None,
            next: Some(1),
        }];

        let convo = Conversation::new(ds);
        assert!(convo.is_ok());
        let convo = convo.unwrap();
        assert_eq!(convo.graph.node_count(), 1);
        assert_eq!(convo.graph.edge_count(), 1);
    }

    #[test]
    fn new_error_with_bad_branching() {
        let ds = vec![
            Dialogue {
                id: 1,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Bob".to_string(),
                    asset: "bob.png".to_string(),
                },
                choices: Some(vec![Choice {
                    text: "Choice 1".to_string(),
                    next: 3,
                }]),
                next: None,
            },
            Dialogue {
                id: 2,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Alice".to_string(),
                    asset: "alice.png".to_string(),
                },
                choices: None,
                next: None,
            },
        ];

        let convo = Conversation::new(ds);
        assert!(convo.is_err());
    }

    #[test]
    fn new_builds_with_branching() {
        let ds = vec![
            Dialogue {
                id: 1,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Bob".to_string(),
                    asset: "bob.png".to_string(),
                },
                choices: Some(vec![
                    Choice {
                        text: "Choice 1".to_string(),
                        next: 2,
                    },
                    Choice {
                        text: "Choice 2".to_string(),
                        next: 3,
                    },
                ]),
                next: None,
            },
            Dialogue {
                id: 2,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Alice".to_string(),
                    asset: "alice.png".to_string(),
                },
                choices: None,
                next: None,
            },
            Dialogue {
                id: 3,
                text: "Hello".to_string(),
                talker: Talker {
                    name: "Alice".to_string(),
                    asset: "alice.png".to_string(),
                },
                choices: None,
                next: None,
            },
        ];

        let convo = Conversation::new(ds).unwrap();

        assert_eq!(convo.graph.node_count(), 3);
        assert_eq!(convo.graph.edge_count(), 2);
    }
}
