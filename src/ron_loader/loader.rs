//! The ron Asset Loader.

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    log::error,
    utils::{hashbrown::HashSet, BoxedFuture},
};
use indexmap::IndexMap;
use serde_ron::de::from_bytes;
use thiserror::Error;

use crate::prelude::{Action, ActionId, Actor, ActorSlug, TalkData};

use super::types::RonTalk;

/// Load Talks from json assets.
pub struct TalksLoader;

/// The error type for the RON Talks loader.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum RonLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonError(#[from] serde_ron::error::SpannedError),
    /// Multiple actions have same id error
    #[error("multiple actions have same id: {0}")]
    DuplicateActionId(ActionId),
    /// The actor slug is duplicated
    #[error("the actor slug {0} is duplicated")]
    DuplicateActorSlug(ActorSlug),
    /// An action has the next field pointing to a non-existent action
    #[error("the action {0} is pointing to id {1} which was not found")]
    InvalidNextAction(ActionId, ActionId),
}

impl AssetLoader for TalksLoader {
    type Asset = TalkData;
    type Settings = ();
    type Error = RonLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let ron_talk = from_bytes::<RonTalk>(&bytes)?;

            // build a TalkData Asset from the RonTalk

            // 1. Build the actors vec
            let actors = ron_talk.actors;
            let mut talk_actors = Vec::<Actor>::with_capacity(actors.len());

            let mut slug_set = HashSet::<ActorSlug>::with_capacity(actors.len());

            // let mut asset_deps = vec![];
            for actor in actors {
                let slug = actor.slug.clone();

                if !slug_set.insert(slug.clone()) {
                    return Err(RonLoaderError::DuplicateActorSlug(slug));
                }
                let talk_actor = Actor::new(slug.clone(), actor.name);
                talk_actors.push(talk_actor)
            }

            // 2. build the raw_actions vec
            let mut raw_actions =
                IndexMap::<ActionId, Action>::with_capacity(ron_talk.script.len());
            for action in ron_talk.script {
                let id = action.id;
                if raw_actions.insert(id, action.into()).is_some() {
                    return Err(RonLoaderError::DuplicateActionId(id));
                }
            }

            validate_all_nexts(&raw_actions)?; // check if all nexts point to real actions

            let raw_talk = TalkData {
                actors: talk_actors,
                script: raw_actions,
            };

            Ok(raw_talk)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["talk.ron"]
    }
}

/// Check if all `next` fields and `Choice` `next` fields in a `Vec<RawAction>` point to real actions.
/// If the action has choices, the `next` field is not checked.
///
/// Returns a `TalkError::InvalidNextAction` error if any of the `next` fields or `Choice` `next` fields in the `RawAction`s do not point to real actions.
fn validate_all_nexts(actions: &IndexMap<ActionId, Action>) -> Result<(), RonLoaderError> {
    let id_set = actions.keys().cloned().collect::<HashSet<_>>();
    for (id, action) in actions {
        if !action.choices.is_empty() {
            for choice in action.choices.iter() {
                if !id_set.contains(&choice.next) {
                    return Err(RonLoaderError::InvalidNextAction(*id, choice.next));
                }
            }
        } else if let Some(next_id) = &action.next {
            if !id_set.contains(next_id) {
                return Err(RonLoaderError::InvalidNextAction(*id, *next_id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;

    use bevy::prelude::*;

    use crate::{prelude::*, tests::minimal_app};

    use super::*;

    // TODO: test for the RonLoaderErrors

    #[test]
    fn test_parse_raw_talk() {
        let mut app = minimal_app();
        let asset_server = app.world.get_resource::<AssetServer>();
        assert!(asset_server.is_some());

        let asset_server = asset_server.unwrap();
        let talk_handle: Handle<TalkData> = asset_server.load("talks/simple.talk.ron");
        app.update();
        app.update();

        let talk_assets = app.world.get_resource::<Assets<TalkData>>();
        assert!(talk_assets.is_some());

        let talk_assets = talk_assets.unwrap();
        let talk = talk_assets.get(&talk_handle);
        assert!(talk.is_some());

        let talk = talk.unwrap();
        assert_eq!(talk.actors.len(), 2);
        assert_eq!(talk.script.len(), 13);
    }

    #[test]
    fn error_invalid_next_action() {
        let talk = TalkData {
            script: indexmap! {0 => Action {
                next: Some(2),
                ..default()
            }},
            ..default()
        };
        let res = validate_all_nexts(&talk.script);
        assert!(res.is_err());
    }

    #[test]
    fn error_not_found_in_choice() {
        let talk = TalkData {
            actors: default(),
            script: indexmap! {
                0 => Action {
                    choices: vec![Choice { next: 2, ..default()}],
                    ..default()
                },
                1 => Action {
                    ..default()
                },
            },
        };
        let res = validate_all_nexts(&talk.script);
        assert!(res.is_err());
    }
}
