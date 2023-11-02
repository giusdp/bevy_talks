//! Asset loader for Talks from "talks.ron" files.

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    log::error,
    utils::BoxedFuture,
};
use serde_ron::de::from_bytes;
use thiserror::Error;

use crate::prelude::{RawAction, RawActor, RawTalk};

use super::types::RonTalk;

/// Load Talks from json assets.
pub struct TalksLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum RonLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonError(#[from] serde_ron::error::SpannedError),
}

impl AssetLoader for TalksLoader {
    type Asset = RawTalk;
    type Settings = ();
    type Error = RonLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let ron_talk = from_bytes::<RonTalk>(&bytes)?;

            // build a RawTalk from the RonTalk by loading the Actor assets

            // 1. Build the actors vec
            let actors = ron_talk.actors;
            let mut talk_actors = Vec::<RawActor>::with_capacity(actors.len());
            // let mut asset_deps = vec![];
            for actor in actors {
                let mut talk_actor = RawActor {
                    id: actor.id,
                    name: actor.name,
                    asset: None,
                };
                // TODO: load the actor asset ? Maybe it's better to just focus on text for now
                // if let Some(actor_asset) = actor.asset {
                //     let path = Path::new(actor_asset.as_str()).to_owned();
                //     let asset_path = AssetPath::new(path, None);
                //     asset_deps.push(asset_path.clone());
                //     let handle: Handle<Image> = load_context.get_handle(asset_path.clone());
                //     talk_actor.asset = Some(handle);
                // }
                talk_actors.push(talk_actor);
            }

            // build the raw_actions vec
            let mut raw_actions = Vec::<RawAction>::with_capacity(ron_talk.script.len());
            for action in ron_talk.script {
                raw_actions.push(action.into());
            }

            let raw_talk = RawTalk {
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

#[cfg(test)]
mod tests {
    use bevy::prelude::{AssetServer, Assets, Handle};

    use crate::{prelude::RawTalk, tests::minimal_app};

    #[test]
    fn test_parse_raw_talk() {
        let mut app = minimal_app();
        let asset_server = app.world.get_resource::<AssetServer>();
        assert!(asset_server.is_some());

        let asset_server = asset_server.unwrap();
        let talk_handle: Handle<RawTalk> = asset_server.load("talks/simple.talk.ron");
        app.update();
        
        let talk_assets = app.world.get_resource::<Assets<RawTalk>>();
        assert!(talk_assets.is_some());

        let talk_assets = talk_assets.unwrap();
        let talk = talk_assets.get(&talk_handle);
        assert!(talk.is_some());

        let talk = talk.unwrap();
        assert_eq!(talk.actors.len(), 2);
        assert_eq!(talk.script.len(), 13);
    }
}
