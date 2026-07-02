# Getting Started

## Install

Add the plugin to your app:

```rust,no_run
use bevy::prelude::*;
use bevy_talks::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .run();
}
```

`TalksPlugin` registers the `DialogueDatabase` asset, its `.dialogue.ron` loader, and the conversation runtime.

## Author a database

Put a `.dialogue.ron` file in your assets folder — either written by hand (see [the format](./concepts/format.md)) or authored in [the editor](./editor.md). A minimal one:

```ron
(
    version: "1",
    actors: [
        (id: 0, name: "Player", is_player: true),
        (id: 1, name: "Actor 1", is_player: false),
    ],
    conversations: [
        (
            id: 1,
            title: "Greeting",
            actor: 0,
            conversant: 1,
            entries: [
                (
                    id: 1, actor: 1, conversant: 0,
                    menu_text: "", dialogue_text: "",
                    is_root: true, is_group: false,
                    links: [(dest_conversation: 1, dest_entry: 2)],
                ),
                (
                    id: 2, actor: 1, conversant: 0,
                    menu_text: "", dialogue_text: "Welcome back.",
                    is_root: false, is_group: false,
                    links: [],
                ),
            ],
        ),
    ],
)
```

## Play it

Spawning a `DialogueRunner` starts the conversation. 
Observe the events it emits and drive it with `AdvanceConversation` and `ChooseResponse`:

```rust,ignore
fn start(mut commands: Commands, assets: Res<AssetServer>) {
    let db: Handle<DialogueDatabase> = assets.load("my_game.dialogue.ron");
    commands.spawn(DialogueRunner::new(
        db,
        ConversationRef::Title("Greeting".to_owned()),
    ));
}

fn show_line(line: On<SubtitleStarted>) {
    println!("{}", line.subtitle.text);
    // present the line, then when the player continues:
    // commands.trigger(AdvanceConversation { entity: line.entity });
}

fn show_menu(menu: On<ResponseMenuOpened>) {
    for (i, response) in menu.responses.iter().enumerate() {
        println!("{}) {}", i + 1, response.text);
    }
    // when the player picks one:
    // commands.trigger(ChooseResponse { entity: menu.entity, index });
}

fn done(_: On<ConversationEnded>) {
    println!("bye!");
}
```

Register the observers with `app.add_observer(...)`, or scope them to one runner with `.observe(...)` at the spawn site.

## A complete example

The repository ships a playable terminal example:

```sh
cargo run --example terminal
```

It plays `assets/test.dialogue.ron` in your shell.