# Bevy Screenplay

[![][img_bevy]][bevycrate] 
[![][img_license]][license] 
[![][img_tracking]][tracking] 
[![][img_version]][crates]
<!-- [![][img_doc]][doc]  -->
<!-- [![][img_downloads]][crates] -->


This [Bevy][bevy] plugin provides a way to create dialogues and conversations in your game, via *screenplay*s. 
A *screenplay* is a directed graph where each node is an *action* that can happen.
An action is usually something that an *actor* can do, such as saying a line, entering/exiting the scene, or even a choice 
the player can make, etc.

As the most most common action is text being displayed on the screen, the most basic
*screenplay* is a sequence of texts forming a conversation between actors, which results in a linear graph of actions.

A *screenplay* is a Bevy component that can be attached to an entity. This way you can have multiple entities 
each with its own *screenplay*, so each entity has its own dialog. Or, you to make a VN-like game, you can 
have one single screenplay in the game.

The heart of the screenplay is a directed graph where each node is an `ActionNode`:

```rust
struct ActionNode {
    /// The kind of action.
    kind: ActionKind,
    /// The text of the action.
    text: Option<String>,
    /// The actors involved in the action.
    actors: Vec<Actor>,
    /// The choices available after the action.
    choices: Option<Vec<Choice>>,
    /// The sound effect associated with the action.
    sound_effect: Option<String>,
}
```

This struct defines an action that can happen. 
- `kind` field defines the kind of action (Talk, Enter, Exit, Choice). 
- `text` field is the text that to display on the screen.
- `actors` field is the list of actors involved in the action.
- `choices` field is the list of choices available to present to the player.
- `sound_effect` field is an extra field to specify a sound effect to play when the action is reached.

The `Actor` struct is a simple struct that contains the name of the actor and the asset to display on the screen.

```rust
struct Actor {
    /// The name of the character that the actor plays.
    name: String,
    /// An optional asset that represents the actor's appearance or voice.
    asset: Option<String>,
}
```

If an action has one or more actors defined, they can be accessed to get the names (and the assets) to be 
displayed together with the text.

### Build Screenplay from screenplay.json files

The plugin can parse json files to create `RawScreenplay` assets, which can then be used to build a `Screenplay` component. 
The files must have the extension: `screenplay.json`.

Here's an example:

```json

{
    "actors": {
        "bob": { "name": "Bob", "asset": "bob.png" },
        "alice": { "name": "Alice", "asset": "alice.png" }
    },
    "script": [
        { "id": 1, "action": "talk", "text": "Bob and Alice enter the room." },
        { "id": 2, "action": "enter", "actors": [ "bob", "alice" ] },
        { "id": 3, "actors": ["bob"], "text": "Hello, Alice!" },
        {
            "id": 4,
            "choices": [
                { "text": "Alice says hello back.", "next": 5 },
                { "text": "Alice ignores Bob.", "next": 6 },
            ]
        },
        { "id": 5, "text": "Bob smiles." },
        { "id": 6, "text": "Bob starts crying." },
        { "id": 7, "text": "The end." }
    ]
}
```

Note the last 3 actions have no `actors` nor `action` fields. This is because the `talk` action is the default action, so it can be omitted.
And if there are no actors, the `actors` field can be omitted too.

The plugin adds an `AssetLoader` for these json files, so it's as easy as: 

```rust
let handle: Handle<RawScreenplay> = server.load("simple.json");
```

Then you can use the `ScreenplayBuilder` to build a `Screenplay` component from the `RawScreenplay` asset. 
You also need to pass the `RawScreenplay` assets collection `raws: Res<Assets<RawScreenplay>>`.

```rust
ScreenplayBuilder::new().with_raw_screenplay(handle.clone()).build(&raws)
```

Or if you have hold of the `RawScreenplay` directly, you can use `raw_build` directly:

```rust
ScreenplayBuilder::raw_build(&raw_screenplay);
```

### Usage

Once you have a `Screenplay` component attached to an entity, you can use the usual queires to access it.
The component offers a public API to interact with graph and the current action.

- `next_action` moves the screenplay to the next action.
- `actors` returns the list of actors involved in the current action.
- `choices` returns the list of choices available in the current action.
- `text` returns the text of the current action.
- `action_kind` returns the kind of the current action.
- `jump_to` jumps to a specific action by id (usually used to jump to the action pointed by a choice).

You can check out the example in the `examples` folder to see how to use the plugin.

- [simple.rs](examples/simple.rs) shows how to use the plugin to create a simple, linear conversation. 
- [choices.rs](examples/choices.rs) shows how to use the plugin to create a conversation with choices.
- [full.rs](examples/full.rs) shows a screenplay where all the action kinds are used.

### Other Things

A future work is to have a graphical editor to create these files, but for now we have to write them by hand.
Any contributions are welcome!

Compatibility of `bevy_screenplay` versions:
| `bevy_screenplay` | `bevy` |
| :--                 |  :--   |
| `main`              | `0.11`  |

## License

Dual-licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](/LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](/LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

[bevy]: https://bevyengine.org/
[renpy]: https://www.renpy.org/

[img_bevy]: https://img.shields.io/badge/Bevy-0.11-blue
[img_version]: https://img.shields.io/crates/v/bevy_screenplay.svg
[img_doc]: https://docs.rs/bevy_screenplay/badge.svg
[img_license]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[img_downloads]:https://img.shields.io/crates/d/bevy_screenplay.svg
[img_tracking]: https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue

[bevycrate]: https://crates.io/crates/bevy/0.11.0
[crates]: https://crates.io/crates/bevy_screenplay
[doc]: https://docs.rs/bevy_screenplay/
[license]: https://github.com/giusdp/bevy_screenplay#license
[tracking]: https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking