# Bevy Talks

[![][img_bevy]][bevycrate]
[![][img_license]][license]
[![][img_tracking]][tracking]
[![][img_version]][crates]
[![][img_doc]][doc]
[![][img_downloads]][crates]

> [!WARNING]
> `bevy_talks` is in development. The API is young and will change. Feedback is very welcome.

A dialogue system for [Bevy][bevy]. Author conversations as branching graphs, load them as assets, and play them back through events.

- **A dialogue database asset**: actors and conversations in `.dialogue.ron` files. Every actor, conversation, and entry carries an extensible fields bag for your custom data.
- **Conversations as graphs**: entries (lines, player choices, group nodes) connected by links. NPC lines flow automatically; player entries become choice menus.
- **An event-driven runtime**: spawn a `DialogueRunner`, observe `SubtitleStarted`, `ResponseMenuOpened`, and `ConversationEnded`, and drive it with `AdvanceConversation` / `ChooseResponse`.
- **A variable store**: a `Variables` resource seeded from the database, the shared state that dialogue and gameplay read and write.
- **A visual editor**: a Bevy app with a node canvas and inspector for authoring databases without writing RON by hand.

📖 The full manual lives in the [book](https://giusdp.github.io/bevy_talks/).

## Quick look

```rust,no_run
use bevy::prelude::*;
use bevy_talks::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .add_systems(Startup, start)
        .add_observer(on_line)
        .run();
}

fn start(mut commands: Commands, assets: Res<AssetServer>) {
    let database: Handle<DialogueDatabase> = assets.load("test.dialogue.ron");
    commands.spawn(DialogueRunner::new(
        database,
        ConversationRef::Title("Test".to_owned()),
    ));
}

fn on_line(line: On<SubtitleStarted>, mut commands: Commands) {
    println!("{}", line.subtitle.text);
    // Show the line in your UI; when it's done:
    commands.trigger(AdvanceConversation { entity: line.entity });
}
```

Try it in your terminal:

```sh
cargo run --example terminal
```

## The editor

The `tools/editor` workspace crate is a visual editor for `.dialogue.ron` files: a node canvas for the conversation graph and an inspector for entries, actors, and their fields.

```sh
cargo run -p editor
```

## Bevy version support

| `bevy_talks` | `bevy` |
| :--          | :--    |
| `main`       | `0.19` |
| `0.6.0`      | `0.19` |
| `0.5.0`      | `0.12` |
| `0.4.0`      | `0.12` |
| `0.3.1`      | `0.12` |
| `0.3.0`      | `0.11` |
| `0.2.0`      | `0.11` |
| `0.1.1`      | `0.11` |

## License

Dual-licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](/LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](/LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

[bevy]: https://bevyengine.org/

[img_bevy]: https://img.shields.io/badge/Bevy-0.19-blue
[img_version]: https://img.shields.io/crates/v/bevy_talks.svg
[img_doc]: https://docs.rs/bevy_talks/badge.svg
[img_license]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[img_downloads]: https://img.shields.io/crates/d/bevy_talks.svg
[img_tracking]: https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue

[bevycrate]: https://crates.io/crates/bevy/0.19.0
[crates]: https://crates.io/crates/bevy_talks
[doc]: https://docs.rs/bevy_talks/
[license]: https://github.com/giusdp/bevy_talks#license
[tracking]: https://bevyengine.org/learn/book/plugin-development/#main-branch-tracking
