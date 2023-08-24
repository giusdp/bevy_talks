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

### Build Screenplay from screenplay.ron files

The plugin can parse ron files to create `RawScreenplay` assets, which can then be used to build a `Screenplay` component. 
The files must have the extension: `screenplay.ron`.

Here's an example:

```rust,ignore
(
    actors: [
        ( id: "bob", name: "Bob", asset: Some("bob.png") ),
        ( id: "alice", name: "Alice", asset: Some("alice.png") )
    ],
    script: [
        ( id: 1, action: Talk, text: Some("Bob and Alice enter the room.") ),
        ( id: 2, action: Enter, actors: [ "bob", "alice" ] ),
        ( id: 3, actors: ["bob"], text: Some("Hello, Alice!") ), // with missing action field, it defaults to Talk
        (
            id: 4,
            choices: Some([
                ( text: "Alice says hello back.", next: 5 ),
                ( text: "Alice ignores Bob.", next: 6 ),
            ])
        ),
        ( id: 5, text: Some("Bob smiles.") ), // with missing actors field, it defaults to an empty vector
        ( id: 6, text: Some("Bob starts crying.") ),
        ( id: 7, text: Some("The end.") )
    ]
)
```

The plugin adds an `AssetLoader` for these ron files, so it's as easy as: 

```rust
let handle: Handle<RawScreenplay> = server.load("simple.screenplay.ron");
```

Then you can use the `ScreenplayBuilder` to build a `Screenplay` component from the `RawScreenplay` asset. 
You can retrieve the `RawScreenplay` from the assets collection `raws: Res<Assets<RawScreenplay>>`.

```rust
let raw_sp = raws.get(&simple_sp_asset.handle).unwrap();
ScreenplayBuilder::new().build(&raw_sp);
```

### Usage

Once you have a `Screenplay` component attached to an entity, you can use the usual Bevy `Query` to access it to 
retrive information about the current action (text, choices, actors involved, the kind of action). 

To move to the next action (or to jump to a specific action), you can send 2 different events and take advantage of 
the Change Detection System to react after a change in the `Screenplay` component.

The event to go to the next action:


```rust
NextActionRequest(pub Entity);
```

The event to jump to a specific action:

```rust
JumpToActionRequest(pub Entity, pub ActionId);
```

You pass the entity with the `Screenplay` component for the former event, and the entity with the `Screenplay` component
and the id of the action to jump to for the latter event.

The plugin will internally call the `next_action` and `jump_to` methods of the `Screenplay` component, respectively.

On your side you can use the Changed api to react to the change after these events are sent. With something like:

```rust
fn print_text(screenplays: Query<&Screenplay, Changed<Screenplay>>) {
    for sp in screenplays.iter() {
        println!("{}", sp.text());
    }
}
```

The component offers a public API to retrieve info from the graph:

- `actors` returns the list of actors involved in the current action.
- `choices` returns the list of choices available in the current action.
- `text` returns the text of the current action.
- `action_kind` returns the kind of the current action.

Check out the example in the `examples` folder to see how to use the plugin.

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
| `0.2.0`              | `0.11`  |
| `0.1.1`              | `0.11`  |

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
