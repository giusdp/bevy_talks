//! Asset loader for screenplays with json format.
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::{Reflect, TypeUuid},
    utils::BoxedFuture,
};
use jsonschema::JSONSchema;
use serde_json::{json, Value};

use crate::{prelude::ScreenplayJSONError, raw_screenplay_json::RawScreenplayJSON};

/// The raw screenplay asset. It contains the json data loaded from the asset.
#[derive(Debug, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawScreenplay(RawScreenplayJSON);

/// Load screenplays from json assets.
#[derive(Default)]
pub struct ScreenplayLoader;

impl AssetLoader for ScreenplayLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let script = serde_json::from_slice(bytes)?;
            let res = build_raw_screenplay(script)?;
            load_context.set_default_asset(LoadedAsset::new(res));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// Validates a JSON value representing a screenplay.
///
/// This function takes a JSON value representing a screenplay and validates its structure. The
/// function checks that the JSON value contains the required fields for a screenplay, and that
/// the values of those fields are of the correct type.
///
/// # Errors
///
/// This function returns a `ScreenplayJSONError` if the JSON value is not a valid screenplay.
fn validate_screenplay_json(script: &Value) -> Result<(), ScreenplayJSONError> {
    let schema = json_schema();
    let compiled = JSONSchema::compile(&schema).expect("A valid schema");
    let result = compiled.validate(script);
    if let Err(errors) = result {
        let error_strings = errors.map(|e| e.to_string()).collect();
        return Err(ScreenplayJSONError::JSONValidation(error_strings));
    }

    // TODO: add more validation (unique ids, empty json, etc)

    Ok(())
}

/// Builds a `RawScreenplay` from a JSON value.
///
/// This function takes a JSON value representing a screenplay and returns a `RawScreenplay` object
/// that can be used to build a `Screenplay` object. The function validates the structure of the
/// JSON value and converts it to a `RawScreenplay` object.
///
/// # Errors
///
/// This function returns a `ScreenplayParsingError` if the JSON value is not a valid screenplay.
fn build_raw_screenplay(script: Value) -> Result<RawScreenplay, ScreenplayJSONError> {
    validate_screenplay_json(&script)?;
    let raw_sp_json = serde_json::from_value::<RawScreenplayJSON>(script)
        .map_err(|e| ScreenplayJSONError::BadParse(e.to_string()))?;
    Ok(RawScreenplay(raw_sp_json))
}

/// Returns the JSON schema for a screenplay.
///
/// The schema is used to validate the structure of the screenplay JSON file.
fn json_schema() -> Value {
    json!({
      "$schema": "http://json-schema.org/draft-07/schema#",
      "title": "Generated schema for Root",
      "type": "object",
      "properties": {
        "actors": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "actor_id": {
                "type": "string"
              },
              "character_name": {
                "type": "string"
              },
              "asset": {
                "type": "string"
              }
            },
            "required": [
              "actor_id",
              "character_name",
              "asset"
            ]
          }
        },
        "script": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "id": {
                "type": "number"
              },
              "action": {
                "type": "string"
              },
              "text": {
                "type": "string"
              },
              "actors": {
                "type": "array",
                "items": {
                  "type": "string"
                }
              },
              "start": {
                "type": "boolean"
              },
              "sound_effect": {
                "type": "string"
              },
              "choices": {
                "type": "array",
                "items": {
                  "type": "object",
                  "properties": {
                    "text": {
                      "type": "string"
                    },
                    "next": {
                      "type": "number"
                    }
                  },
                  "required": [
                    "text",
                    "next"
                  ]
                }
              },
              "next": {
                "type": "number"
              }
            },
            "required": [
              "id",
              "action",
            ]
          }
        }
      },
      "required": [
        "actors",
        "script"
      ]
    })
}
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

// /// A minimal representation of a convo node for validation purposes
// #[derive(Debug)]
// struct StrippedNodeAction {
//     node_idx: NodeIndex,
//     next_action_id: Option<ActionId>,
//     choices: Option<Vec<ActionId>>,
// }

// fn build_id_to_next_map(
//     script: &Vec<ActorOrPlayerActionJSON>,
// ) -> Result<HashMap<ActionId, ActionId>, ScreenplayParsingError> {
//     let mut id_to_next_map: HashMap<ActionId, ActionId> = HashMap::with_capacity(script.len() - 1);
//     for (i, a) in script.iter().enumerate() {
//         match a.next() {
//             Some(n) => {
//                 if id_to_next_map.insert(a.id(), n).is_some() {
//                     return Err(ScreenplayParsingError::RepeatedId(a.id()));
//                 }
//             }
//             None => {
//                 // if next not defined:
//                 // either player action (with choices) or actor action pointing to the one below it
//                 // NOTE: we are not adding the last action (if next: None) as it can't have a next
//                 if i + 1 < script.len() {
//                     id_to_next_map.insert(a.id(), script[i + 1].id());
//                 }
//             }
//         };
//     }
//     Ok(id_to_next_map)
// }

// fn extract_actors(
//     aaction: &ActorAction,
//     actors_map: &HashMap<String, Actor>,
// ) -> Result<Vec<Actor>, ScreenplayParsingError> {
//     // Retrieve the actors from the actors map. In case one is not found, return an error.
//     let mut actors = Vec::with_capacity(1);
//     for actor_key in aaction.actors.iter() {
//         let retrieved_actor = actors_map
//             .get(actor_key)
//             .ok_or_else(|| {
//                 ScreenplayParsingError::ActorNotFound(aaction.id, actor_key.to_string())
//             })?
//             .to_owned();
//         actors.push(retrieved_actor);
//     }
//     Ok(actors)
// }

// fn check_start_flag(
//     start_flag: Option<bool>,
//     already_have_start: bool,
// ) -> Result<bool, ScreenplayParsingError> {
//     if let Some(true) = start_flag {
//         if already_have_start {
//             return Err(ScreenplayParsingError::MultipleStartingAction);
//         }
//         return Ok(true);
//     }
//     Ok(false)
// }

// fn add_action_node(
//     graph: &mut DiGraph<ActionNode, ()>,
//     action: ActorOrPlayerActionJSON,
//     actors_map: &HashMap<String, Actor>,
// ) -> Result<NodeIndex, ScreenplayParsingError> {
//     let mut node = ActionNode { ..default() };
//     match action {
//         ActorOrPlayerActionJSON::Actor(actor_action) => {
//             node.actors = Some(extract_actors(&actor_action, actors_map)?);
//             node.text = actor_action.text;
//             node.kind = actor_action.action.into();
//             node.sound_effect = actor_action.sound_effect;
//         }
//         ActorOrPlayerActionJSON::Player(player_action) => {
//             node.choices = Some(player_action.choices);
//             node.kind = ActionKind::PlayerChoice;
//         }
//     }
//     let node_idx = graph.add_node(node);
//     Ok(node_idx)
// }

// fn validate_nexts(
//     nodeidx_dialogue_map: &HashMap<i32, StrippedNodeAction>,
// ) -> Result<(), ScreenplayParsingError> {
//     for (id, stripped_node) in nodeidx_dialogue_map {
//         if let Some(next_id) = stripped_node.next_action_id {
//             if !nodeidx_dialogue_map.contains_key(&next_id) {
//                 return Err(ScreenplayParsingError::NextActionNotFound(*id, next_id));
//             }
//         } else if let Some(vc) = &stripped_node.choices {
//             for c in vc {
//                 if !nodeidx_dialogue_map.contains_key(c) {
//                     return Err(ScreenplayParsingError::NextActionNotFound(*id, *c));
//                 }
//             }
//         }
//     }
//     Ok(())
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::types::{ActorAction, ActorOrPlayerActionJSON, Choice, PlayerAction};
//     use bevy::prelude::default;

//     fn an_actors_map(name: String) -> HashMap<String, Actor> {
//         let mut actors = HashMap::new();
//         actors.insert(
//             name,
//             Actor {
//                 name: "Bob".to_string(),
//                 asset: "bob.png".to_string(),
//             },
//         );
//         actors
//     }

//     // 'new' tests
//     #[test]
//     fn no_script_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: default(),
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::EmptyScript));
//     }

//     #[test]
//     fn actor_not_found_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 text: Some("Hello".to_string()),
//                 actors: vec!["Bob".to_string()],
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(
//             play,
//             Some(ScreenplayParsingError::ActorNotFound(0, "Bob".to_string()))
//         );
//     }

//     #[test]
//     fn actor_not_found_with_mismath_err() {
//         let raw_sp = RawScreenplay {
//             actors: an_actors_map("Bob".to_string()),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 actors: vec!["Alice".to_string()],
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(
//             play,
//             Some(ScreenplayParsingError::ActorNotFound(
//                 0,
//                 "Alice".to_string()
//             ))
//         );
//     }

//     #[test]
//     fn no_start_err() {
//         let raw_sp = RawScreenplay {
//             actors: an_actors_map("Alice".to_string()),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 actors: vec!["Alice".to_string()],

//                 ..default()
//             })],
//         };
//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::NoStartingAction));
//     }

//     #[test]
//     fn multiple_start_actor_action_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     start: Some(true),
//                     ..default()
//                 }),
//             ],
//         };
//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::MultipleStartingAction));
//     }

//     #[test]
//     fn multiple_start_mixed_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 3,
//                     start: Some(true),
//                     ..default()
//                 }),
//             ],
//         };
//         let play = build_screenplay(raw_sp).err();

//         assert_eq!(play, Some(ScreenplayParsingError::MultipleStartingAction));
//     }

//     #[test]
//     fn multiple_start_player_action_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     start: Some(true),
//                     ..default()
//                 }),
//             ],
//         };
//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::MultipleStartingAction));
//     }

//     #[test]
//     fn repeated_id_actor_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 1,
//                     text: Some("Hello".to_string()),
//                     next: Some(1),
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 1,
//                     text: Some("Whatup".to_string()),
//                     next: Some(2),
//                     ..default()
//                 }),
//             ],
//         };
//         let play = build_screenplay(raw_sp).err();

//         assert_eq!(play, Some(ScreenplayParsingError::RepeatedId(1)));
//     }

//     #[test]
//     fn repeated_id_mixed_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 1,
//                     text: Some("Hello".to_string()),
//                     next: Some(1),
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Player(PlayerAction { id: 1, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::RepeatedId(1)));
//     }

//     #[test]
//     fn repeated_id_player_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
//                     id: 1,
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Player(PlayerAction { id: 1, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::RepeatedId(1)));
//     }

//     #[test]
//     fn next_actor_action_not_found_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 next: Some(2),
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::NextActionNotFound(0, 2)));
//     }

//     #[test]
//     fn next_not_found_in_choice_err() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Player(PlayerAction {
//                 choices: vec![Choice {
//                     text: "Whatup".to_string(),
//                     next: 2,
//                 }],
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).err();
//         assert_eq!(play, Some(ScreenplayParsingError::NextActionNotFound(0, 2)));
//     }

//     #[test]
//     fn new_with_one_action() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 start: Some(true),
//                 ..default() // end: None,
//             })],
//         };
//         let play = build_screenplay(raw_sp).unwrap();

//         assert_eq!(play.graph.node_count(), 1);
//         assert_eq!(play.graph.edge_count(), 0);
//         assert_eq!(play.current, NodeIndex::new(0));
//     }

//     #[test]
//     fn new_with_two_actor_action_nodes() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 1,
//                     next: Some(2),
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.graph.node_count(), 2);
//         assert_eq!(play.graph.edge_count(), 1);
//     }

//     #[test]
//     fn new_with_self_loop() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
//                 id: 1,
//                 next: Some(1),
//                 start: Some(true),
//                 ..default()
//             })],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.graph.node_count(), 1);
//         assert_eq!(play.graph.edge_count(), 1);
//     }

//     #[test]
//     fn new_with_branching() {
//         let raw_sp = RawScreenplay {
//             actors: default(),
//             script: vec![
//                 ActorOrPlayerActionJSON::Player(PlayerAction {
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
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 2,
//                     text: Some("Hello".to_string()),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
//             ],
//         };

//         let play = build_screenplay(raw_sp).unwrap();
//         assert_eq!(play.graph.node_count(), 3);
//         assert_eq!(play.graph.edge_count(), 4);
//         assert_eq!(play.current, NodeIndex::new(0));
//     }

//     #[test]
//     fn new_with_actors() {
//         let mut actors_map = an_actors_map("bob".to_string());
//         actors_map.insert(
//             "alice".to_string(),
//             Actor {
//                 name: "Alice".to_string(),
//                 asset: "alice.png".to_string(),
//             },
//         );

//         let raw_sp = RawScreenplay {
//             actors: actors_map,
//             script: vec![
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 1,
//                     text: Some("Hello".to_string()),
//                     actors: vec!["bob".to_string()],
//                     next: Some(2),
//                     start: Some(true),
//                     ..default()
//                 }),
//                 ActorOrPlayerActionJSON::Actor(ActorAction {
//                     id: 2,
//                     text: Some("Whatup".to_string()),
//                     actors: vec!["alice".to_string()],
//                     ..default()
//                 }),
//             ],
//         };
//         let play = build_screenplay(raw_sp).unwrap();

//         assert_eq!(play.graph.node_count(), 2);
//         assert_eq!(play.graph.edge_count(), 1);
//         assert_eq!(play.current, NodeIndex::new(0));
//     }
// }
