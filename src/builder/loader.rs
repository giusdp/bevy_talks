//! Asset loader for Talks from "talks.ron" files.

use bevy::log::error;
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
};

use crate::prelude::RawTalk;

/// Load Talks from json assets.
#[derive(Default)]
pub struct TalkLoader;

impl AssetLoader for TalkLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        let raw_sp = parse_ron_talk(bytes);
        if let Err(e) = &raw_sp {
            error!("Error parsing Talk: {e:}");
        }
        Box::pin(async move {
            load_context.set_default_asset(LoadedAsset::new(raw_sp?));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["talk.ron"]
    }
}

/// Parse a Talk from a byte slice.
fn parse_ron_talk(bytes: &[u8]) -> Result<RawTalk, bevy::asset::Error> {
    let script_str = std::str::from_utf8(bytes)?;
    let raw_sp: RawTalk = ron::from_str(script_str)?;
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
