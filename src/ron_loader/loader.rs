//! Asset loader for Talks from "talks.ron" files.

use std::path::Path;

use bevy::asset::AssetPath;
use bevy::log::error;
use bevy::prelude::{Handle, Image};
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
};

use crate::prelude::{RawAction, RawActor, RawTalk};

use super::types::RonTalk;

/// Load Talks from json assets.
#[derive(Default)]
pub struct TalkLoader;

impl AssetLoader for TalkLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let raw_sp = parse_ron_talk(bytes);
            if let Err(e) = &raw_sp {
                error!("Error parsing Talk: {e:}");
            }

            let raw_sp = raw_sp?;

            // for each actor, load the asset
            let actors = raw_sp.actors;

            // build a RawTalk from the RonTalk by loading the Actor assets

            // build the actors vec
            let mut talk_actors = Vec::<RawActor>::with_capacity(actors.len());
            let mut asset_deps = vec![];
            for actor in actors {
                let mut talk_actor = RawActor {
                    id: actor.id,
                    name: actor.name,
                    asset: None,
                };
                if let Some(actor_asset) = actor.asset {
                    let path = Path::new(actor_asset.as_str()).to_owned();
                    let asset_path = AssetPath::new(path, None);
                    asset_deps.push(asset_path.clone());
                    let handle: Handle<Image> = load_context.get_handle(asset_path.clone());
                    talk_actor.asset = Some(handle);
                }
                talk_actors.push(talk_actor);
            }

            // build the raw_actions vec
            let mut raw_actions = Vec::<RawAction>::with_capacity(raw_sp.script.len());
            for action in raw_sp.script {
                raw_actions.push(action.into());
            }

            let raw_talk = RawTalk {
                actors: talk_actors,
                script: raw_actions,
            };

            let asset = LoadedAsset::new(raw_talk).with_dependencies(asset_deps);
            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["talk.ron"]
    }
}

/// Parse a Talk from a byte slice.
fn parse_ron_talk(bytes: &[u8]) -> Result<RonTalk, bevy::asset::Error> {
    let script_str = std::str::from_utf8(bytes)?;
    let raw_sp: RonTalk = ron::from_str(script_str)?;
    Ok(raw_sp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_raw_talk() {
        let bytes = b"(
            script: [
                (
                    id: 1,
                    text: Some(\"Text 1\"),
                    actors: [\"actor1\"],
                    next: Some(2)
                ),
                (
                    id: 2,
                    text: Some(\"Text 2\"),
                    actors: [\"actor2\"]
                ),
            ],
            actors: [ ( id: \"actor1\", name: \"Actor 1\" ), ( id: \"actor2\", name: \"Actor 2\" ) ],
        )";
        let result = parse_ron_talk(bytes);
        println!("{:?}", result);
        assert!(result.is_ok());

        let raw_sp = result.unwrap();
        assert_eq!(raw_sp.script.len(), 2);
        assert_eq!(raw_sp.actors.len(), 2);
    }
}
