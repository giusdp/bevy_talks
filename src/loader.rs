//! Asset loader for screenplays with json format.
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
};
use jsonschema::JSONSchema;
use serde_json::{json, Value};

use crate::prelude::{JsonError, RawScreenplay};

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
            let res = build_raw(script)?;
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
fn validate(script: &Value) -> Result<(), JsonError> {
    let schema = json_schema();
    let compiled = JSONSchema::compile(&schema).expect("A valid schema");
    let result = compiled.validate(script);
    if let Err(errors) = result {
        let error_strings = errors.map(|e| e.to_string()).collect();
        return Err(JsonError::Validation(error_strings));
    }

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
/// This function returns a `JsonError` if the JSON value is not a valid screenplay.
fn build_raw(script: Value) -> Result<RawScreenplay, JsonError> {
    validate(&script)?;
    serde_json::from_value::<RawScreenplay>(script).map_err(|e| JsonError::BadParse(e.to_string()))
}

/// Returns the JSON schema for a screenplay.
///
/// The schema is used to validate the structure of the screenplay JSON file.
fn json_schema() -> Value {
    json!({
      "$schema": "http://json-schema.org/draft-07/schema#",
      "title": "Schema for bevy_talk jsons",
      "type": "object",
      "properties": {
        "actors": {
          "type": "object",
          "patternProperties": {
            "^[A-Za-z0-9]$": {
              "type": "object",
              "properties": {
                "name": {
                  "type": "string"
                },
                "asset": {
                  "type": "string"
                }
              },
              "required": [
                "name"
              ]
            }
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
              "id"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_empty_raw_ok() {
        let j = json!({
          "actors":{},
          "script":[]
        });

        assert!(build_raw(j).is_ok());
    }

    #[test]
    fn build_raw_err_when_invalid_json() {
        assert!(build_raw(json!({})).is_err());
    }

    #[test]
    fn build_raw_success() {
        let j = json!(
          {
            "actors": {
                "bob": { "name": "Bob", "asset": "bob.png" }
              },
            "script": [
              {
                "id": 1,
                "action": "talk",
                "text": "Hello",
                "actors": [],
              }
            ]
          }
        );

        assert!(build_raw(j).is_ok())
    }

    #[test]
    fn empty_json_fails_validate() {
        assert!(validate(&json!({})).is_err());
    }

    #[test]
    fn missing_required_fails_validate() {
        let j = json!({
          "actors": {},
          "script": [
            {
              "text": "Hello",
              "actors": [],
            }
          ]
        });

        assert!(validate(&j).is_err())
    }

    #[test]
    fn correct_json_passes_validate() {
        let j = json!(
          {
            "actors": {
              "bob": { "name": "Bob", "asset": "bob.png" }
            },
            "script": [
              {
                "id": 1,
                "action": "talk",
                "text": "Hello",
                "actors": [],
              }
            ]
          }
        );

        assert!(validate(&j).is_ok());
    }
}
