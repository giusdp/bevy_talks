# Bevy Talks

[![][img_bevy]][bevycrate] 
[![][img_license]][license] 
[![][img_tracking]][tracking] 
[![][img_version]][crates]
<!-- [![][img_doc]][doc]  -->
<!-- [![][img_downloads]][crates] -->


This [Bevy][bevy] plugin provides a way to create dialogues and conversations in your game, via *Talk*s components. 
A *Talk* is a directed graph where each node is an *action* that an actor can perform, 
such as saying a line, joining/leaving the scene, or even a choice the player can make.

The most common action is text being displayed on the screen, and a simple *Talk* is
just a sequence of texts forming a conversation between actors.

You can have multiple entities each with their own *Talk*. Or you can make a VN-like game with one single Talk in the game.

The heart of the Talk is a directed graph where each node is an `TalkNode` struct:

```rust
struct TalkNode {
     /// Talk, Join, Leave, Choice
    kind: TalkNodeKind,
    /// Text to display on the screen.
    text: String,
    /// The actors involved in the action.
    actors: Vec<Actor>,
    /// The choices available for the player
    choices: Vec<Choice>,
}
```
The `Actor` struct is a simple struct that contains the name of the actor and the asset to display on the screen.

```rust
struct Actor {
    /// The name of the character that the actor plays.
    name: String,
    /// An optional asset for the actor.
    asset: Option<Handle<Image>>,
}
```

The Choice struct is a simple struct that contains the text of the choice and the index of the node to jump to.

```rust
struct Choice {
    /// The text of the choice.
    pub text: String,
    /// The ID of the next action to jump to if the choice is selected.
    pub next: NodeIndex,
}
```

### Build Talks from talk.ron files

The plugin can parse ron files to create `RawTalk` assets, which can then be used to build a `Talk` component. 
The files must have the extension: `talk.ron`.

Here's an example:

```rust,ignore
(
    actors: [
        ( id: "bob", name: "Bob" ),
        ( id: "alice", name: "Alice" )
    ],
    script: [
        ( id: 1, action: Talk, text: Some("Bob and Alice enter the room.") ),
        ( id: 2, action: Join, actors: [ "bob", "alice" ] ),
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
let handle: Handle<RawTalk> = asset_server.load("simple.talk.ron");
```

Then you can use `Talk::build` function with the `RawTalk` asset. 
You can retrieve the `RawTalk` from the assets collection `raws: Res<Assets<RawTalk>>`.

```rust
let raw_sp = raws.get(&simple_sp_asset.handle).unwrap();
Talk::build(&raw_sp)
```

### Usage


The plugin provides a `TalkerBundle` to give an entity the required components to handle its own dialogues.
```rust
struct TalkerBundle {
    talk: Talk,
    /// The dialogue line component for a Talk.
    talk_text: CurrentText,
    /// The actor component that represents a character in a Talk.
    current_actors: CurrentActors,
    /// The Talk Node Kind component that represents the kind of action in a Talk.
    kind: CurrentNodeKind,
    /// The component that represents the current choices in a Talk.
    current_choices: CurrentChoices,
}
```

With these components you can query the current text/actor/choices for the current action in a talk. 
Together with the Change Detection System, you can react to changes in the `Talk` component to update your UI.

```rust
fn print_text(talks: Query<(Ref<CurrentText> &CurrentNodeKind)>) {
    for (text, kind) in talks.iter() {
        if kind == TalkNodeKind::Talk && text.is_changed() {
            println!("{}", text.text());
        }
    }
}
```

To interact with Talks you can send 3 different events. One to initialize the Talk (it populates the components with the first node), and two to advance the Talk to the next node or to jump to a specific node):

```rust
struct InitTalkRequest(pub Entity);
```

To move forward to the next action:

```rust
NextActionRequest(pub Entity);
```

To jump to a specific action (used with choices):

```rust
JumpToActionRequest(pub Entity, pub NodeIndex);
```

You pass the entity with the `Talk` component for the first 2 events.
The third required the entity and the index that identifies the node to jump to.

Check out the example in the `examples` folder to see how to use the plugin.

- [simple.rs](examples/simple.rs) shows how to use the plugin to create a simple, linear conversation. 
- [choices.rs](examples/choices.rs) shows how to use the plugin to create a conversation with choices (jumps in the graph).
- [full.rs](examples/full.rs) shows a Talk where all the action kinds are used.
- [ingame.rs](examples/ingame.rs) shows how to use the plugin with more than one `Talker` entity you can interact with.

### Roadmap

- [x] A `TalkerBundle` to give an entity the required components to access and track the dialogues
- [ ] Dialogue UIs 
- [ ] Interaction/Trigger system (to activate/advance dialogues)
- [ ] Graphical editor to create the asset files
- [ ] Voice lines/sound support
- [ ] Support other asset formats (?)
- [ ] More examples
- [ ] Extensive documentation/manual wiki


### Bevy Version Support


Compatibility of `bevy_talks` versions:
| `bevy_talks` | `bevy` |
| :--                 |  :--   |
| `main`              | `0.11`  |
| `0.3.0`              | `0.11`  |
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
[img_version]: https://img.shields.io/crates/v/bevy_talks.svg
[img_doc]: https://docs.rs/bevy_talks/badge.svg
[img_license]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[img_downloads]:https://img.shields.io/crates/d/bevy_talks.svg
[img_tracking]: https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue

[bevycrate]: https://crates.io/crates/bevy/0.11.0
[crates]: https://crates.io/crates/bevy_talks
[doc]: https://docs.rs/bevy_talks/
[license]: https://github.com/giusdp/bevy_talks#license
[tracking]: https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking
