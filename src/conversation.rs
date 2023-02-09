use bevy::{reflect::TypeUuid, utils::HashMap};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, visit::EdgeRef};
use serde::Deserialize;

use crate::{
    dialogue_line::{Choice, DialogueLine},
    errors::{ConversationError, ConvoCreationError},
    talker::Talker,
};

#[derive(Debug, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
pub struct Conversation {
    dialogue_graph: DiGraph<DialogueNode, ()>,
    current: NodeIndex,
    id_to_nodeidx: HashMap<i32, NodeIndex>,
}

impl Conversation {
    pub(crate) fn new(talk: RawTalk) -> Result<Self, ConvoCreationError> {
        if talk.lines.is_empty() {
            return Err(ConvoCreationError::NoLines);
        }

        let mut first_line = Option::<NodeIndex>::None;

        // build a map of talkers: name => talker for easy lookup
        let talker_map: HashMap<String, Talker> = talk
            .talkers
            .into_iter()
            .map(|t| (t.name.clone(), t))
            .collect();

        let mut dialogue_graph: DiGraph<DialogueNode, ()> = DiGraph::new();

        // Build a dialogue.id => (NodeIndex, DLineStripped) map so we can keep track of what we added
        // in the graph. If we add the same dialogue.id multiple times then it's a user error (they repeated ids).
        // Right now dialogue.id == NodeIndex in the graph so this is not really needed.
        // But I'd like to have uuids as ids in the future and not simple i32.
        let mut nodeidx_dialogue_map: HashMap<i32, (NodeIndex, DLineStripped)> = HashMap::new();

        // Start by adding all dialogues as nodes
        for dline in talk.lines {
            // -- Note: this is a bit verbose and I bet there is some functional magic to do this better
            // If line has a talker, retrieve it from the Talker struct map. Otherwise keep it None.
            let talker_opt = match dline.talker {
                Some(name) => {
                    if !talker_map.contains_key(&name) {
                        // if no Talker struct, then the line is invalid (it uses a non existent talker)
                        return Err(ConvoCreationError::TalkerNotFound(dline.id, name));
                    }
                    talker_map.get(&name).cloned()
                }
                None => None,
            };

            let dialogue_node = DialogueNode {
                text: dline.text,
                talker: talker_opt,
                choices: dline.choices.clone(),
            };

            let node_idx = dialogue_graph.add_node(dialogue_node);

            if let Some(true) = dline.start {
                if first_line.is_some() {
                    return Err(ConvoCreationError::MultipleStartingDialogues);
                }
                first_line = Some(node_idx);
            }

            let dlineid = dline.id;
            let dline_stripped = DLineStripped {
                id: dline.id,
                choices: dline.choices,
                next: dline.next,
            };
            if nodeidx_dialogue_map
                .insert(dline.id, (node_idx, dline_stripped))
                .is_some()
            {
                return Err(ConvoCreationError::RepeatedId(dlineid));
            }
        }

        if first_line.is_none() {
            return Err(ConvoCreationError::NoStartingDialogue);
        }

        // TODO: I forgot to handle the end: true case.
        // If a dialogue has end: true we stop adding edges that start from it.
        // Effectively we ignore next and choices

        // Note: Right now the next == None and choices == None case is not handled,
        // resulting in an end node, cause no edge are added to it.
        // Maybe we could think of it as pointing to the dialogue coming right after in the list?
        // Problem is I lost that ordering when I stripped the data for the map.
        // I'm also not convinced about having these subtle behaviours, perhaps we should just throw an error
        // if end is not Some(true) and next and choices are None.

        // Add edges to the graph (next has priority over choices)
        for (current_node_idx, current_dialogue) in nodeidx_dialogue_map.values() {
            // If the current dialogue has a next field, add an edge to the next dialogue
            if let Some(next_id) = current_dialogue.next {
                match nodeidx_dialogue_map.get(&next_id) {
                    Some((next_node_idx, _)) => {
                        dialogue_graph.add_edge(*current_node_idx, *next_node_idx, ())
                    }
                    None => {
                        return Err(ConvoCreationError::NextLineNotFound(
                            current_dialogue.id,
                            next_id,
                        ))
                    }
                };
            } else if let Some(choices) = &current_dialogue.choices {
                for choice in choices {
                    match nodeidx_dialogue_map.get(&choice.next) {
                        Some(_) => {
                            dialogue_graph.add_edge(*current_node_idx, *current_node_idx, ())
                        }
                        None => {
                            return Err(ConvoCreationError::NextLineNotFound(
                                current_dialogue.id,
                                choice.next,
                            ));
                        }
                    };
                }
            }
        }

        let id_to_nodeidx = nodeidx_dialogue_map
            .into_iter()
            .map(|(id, (nodeidx, _))| (id, nodeidx))
            .collect();

        Ok(Self {
            dialogue_graph,
            // there's an early return if first_line is None, so it's safe to unwrap here
            current: first_line.unwrap(),
            id_to_nodeidx,
        })
    }

    pub fn current_text(&self) -> &str {
        &self.dialogue_graph[self.current].text
    }

    pub fn next_line(&mut self) -> Result<(), ConversationError> {
        let dnode = self.dialogue_graph.node_weight(self.current);

        // if for some reason the current node is not in the graph, return an error
        let cur_dial = dnode.ok_or(ConversationError::InvalidDialogue)?;

        // if the current dialogue has choices, return an error
        if cur_dial.choices.is_some() {
            return Err(ConversationError::ChoicesNotHandled);
        }

        let edge_ref = self
            .dialogue_graph
            .edges(self.current)
            .next()
            .ok_or(ConversationError::NoNextDialogue)?;

        // TODO: wait, what is this NodeId? Is it the NodeIndex? I'm not sure
        self.current = edge_ref.target();
        Ok(())
    }

    pub fn jump_to(&mut self, id: i32) -> Result<(), ConversationError> {
        let idx = self
            .id_to_nodeidx
            .get(&id)
            .ok_or(ConversationError::WrongJump(id))?;

        self.current = *idx;
        Ok(())
    }

    /// Returns the choices for the current dialogue. If there are no choices, returns an error.
    pub fn choices(&self) -> Result<Vec<Choice>, ConversationError> {
        let dnode = self.dialogue_graph.node_weight(self.current);
        // if for some reason the current node is not in the graph, return an error
        let cur_dial = dnode.ok_or(ConversationError::InvalidDialogue)?;

        if let Some(choices) = &cur_dial.choices {
            Ok(choices.clone())
        } else {
            Err(ConversationError::NoChoices)
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawTalk {
    talkers: Vec<Talker>,
    lines: Vec<DialogueLine>,
}

#[derive(Debug)]
struct DialogueNode {
    text: String,
    talker: Option<Talker>,
    choices: Option<Vec<Choice>>,
}

/// A stripped down version of DialogueLine that only contains the data we need to build the graph edges.
#[derive(Debug)]
struct DLineStripped {
    id: i32,
    next: Option<i32>,
    choices: Option<Vec<Choice>>,
}

#[cfg(test)]
mod test {
    use super::*;

    // 'new' tests
    #[test]
    fn no_lines_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(convo, Some(ConvoCreationError::NoLines));
    }

    #[test]
    fn talker_not_found_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: Some("Bob".to_string()),
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(
            convo,
            Some(ConvoCreationError::TalkerNotFound(1, "Bob".to_string()))
        );
    }

    #[test]
    fn talker_not_found_with_mismath_err() {
        let raw_talk = RawTalk {
            talkers: vec![Talker {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            }],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: Some("Alice".to_string()),
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(
            convo,
            Some(ConvoCreationError::TalkerNotFound(1, "Alice".to_string()))
        );
    }

    #[test]
    fn next_line_not_found_err() {
        let raw_talk = RawTalk {
            talkers: vec![Talker {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            }],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: Some("Bob".to_string()),
                choices: None,
                next: Some(2),
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(convo, Some(ConvoCreationError::NextLineNotFound(1, 2)));
    }

    #[test]
    fn repeated_id_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(1),
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 1,
                    text: "Whatup".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(2),
                    start: None,
                    end: None,
                },
            ],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(convo, Some(ConvoCreationError::RepeatedId(1)));
    }

    #[test]
    fn no_starting_dialogue_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: None,
                start: None,
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(convo, Some(ConvoCreationError::NoStartingDialogue));
    }

    #[test]
    fn multiple_starting_dialogues_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "Whatup".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: Some(true),
                    end: None,
                },
            ],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(convo, Some(ConvoCreationError::MultipleStartingDialogues));
    }

    #[test]
    fn next_not_found_in_choice_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: Some(vec![Choice {
                    text: "Whatup".to_string(),
                    next: 2,
                }]),
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).err();
        assert_eq!(convo, Some(ConvoCreationError::NextLineNotFound(1, 2)));
    }

    #[test]
    fn new_with_one_dialogue() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.dialogue_graph.node_count(), 1);
        assert_eq!(convo.dialogue_graph.edge_count(), 0);
        assert_eq!(convo.current, NodeIndex::new(0));
    }

    #[test]
    fn new_with_two_linear_nodes() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(2),
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "Whatup".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: None,
                    end: None,
                },
            ],
        };

        let convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.dialogue_graph.node_count(), 2);
        assert_eq!(convo.dialogue_graph.edge_count(), 1);
    }

    #[test]
    fn new_with_self_loop() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: Some(1),
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.dialogue_graph.node_count(), 1);
        assert_eq!(convo.dialogue_graph.edge_count(), 1);
    }

    #[test]
    fn new_with_branching() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
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
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(3),
                    start: None,
                    end: None,
                },
                DialogueLine {
                    id: 3,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: None,
                    end: None,
                },
            ],
        };

        let convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.dialogue_graph.node_count(), 3);
        assert_eq!(convo.dialogue_graph.edge_count(), 3);
        assert_eq!(convo.current, NodeIndex::new(0));
    }

    // 'current_text' tests
    #[test]
    fn current_text() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.current_text(), "Hello");
    }

    // 'next_line' tests
    #[test]
    fn next_no_next_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let mut convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(
            convo.next_line().err(),
            Some(ConversationError::NoNextDialogue)
        );
    }

    #[test]
    fn next_choices_not_handled_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: Some(vec![Choice {
                        text: "Whatup".to_string(),
                        next: 2,
                    }]),
                    next: None,
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "Whatup to you".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: None,
                    end: None,
                },
            ],
        };

        let mut convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(
            convo.next_line().err(),
            Some(ConversationError::ChoicesNotHandled)
        );
    }

    #[test]
    fn next_line() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(2),
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "Whatup".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: None,
                    end: None,
                },
            ],
        };

        let mut convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.current_text(), "Hello");
        assert!(convo.next_line().is_ok());
        assert_eq!(convo.current_text(), "Whatup");
    }

    // 'choices' tests
    #[test]
    fn choices_no_choices_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.choices().err(), Some(ConversationError::NoChoices));
    }

    #[test]
    fn choices() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
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
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(3),
                    start: None,
                    end: None,
                },
                DialogueLine {
                    id: 3,
                    text: "Hello".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: None,
                    end: None,
                },
            ],
        };

        let convo = Conversation::new(raw_talk).unwrap();

        assert_eq!(convo.choices().unwrap()[0].next, 2);
        assert_eq!(convo.choices().unwrap()[1].next, 3);
        assert_eq!(convo.choices().unwrap()[0].text, "Choice 1");
        assert_eq!(convo.choices().unwrap()[1].text, "Choice 2");
    }

    // 'jump_to' tests
    #[test]
    fn jump_to_no_line_err() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![DialogueLine {
                id: 1,
                text: "Hello".to_string(),
                talker: None,
                choices: None,
                next: None,
                start: Some(true),
                end: None,
            }],
        };

        let mut convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(
            convo.jump_to(2).err(),
            Some(ConversationError::WrongJump(2))
        );
    }
    #[test]
    fn jump_to() {
        let raw_talk = RawTalk {
            talkers: vec![],
            lines: vec![
                DialogueLine {
                    id: 1,
                    text: "Hello".to_string(),
                    talker: None,
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
                    start: Some(true),
                    end: None,
                },
                DialogueLine {
                    id: 2,
                    text: "I'm number 2".to_string(),
                    talker: None,
                    choices: None,
                    next: Some(3),
                    start: None,
                    end: None,
                },
                DialogueLine {
                    id: 3,
                    text: "I;m number 3".to_string(),
                    talker: None,
                    choices: None,
                    next: None,
                    start: None,
                    end: None,
                },
            ],
        };

        let mut convo = Conversation::new(raw_talk).unwrap();
        assert_eq!(convo.current_text(), "Hello");
        assert!(convo.jump_to(2).is_ok());
        assert_eq!(convo.current_text(), "I'm number 2");
    }
}
