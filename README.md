# Bevy Talks

[![][img_bevy]][bevycrate] 
[![][img_license]][license] 
[![][img_tracking]][tracking] 
<!-- [![][img_version]][crates] -->
<!-- [![][img_doc]][doc]  -->
<!-- [![][img_downloads]][crates] -->


This [Bevy][bevy] plugin provides an opinionated way to create dialogues and conversations in your game, or
*screenplay* as we called it. 
A *screenplay* is a directed graph where each node is an *action* that can happen.
An action is user-defined and can be anything, like a character entering the scene, a character saying something,
a choice that the player can make, moving the conversation to another node, playing a sound, etc.

You can imagine the most common action is text being displayed on the screen, and the most basic
*screenplay* is a sequence of text being displayed on the screen, represented by a linear graph of actions.

Actions are composable via Bevy components. By default an action is just an entity with no components attached.
This library provides a set of built-in components that you can use to create your own actions.

### Built-in Action Components

- `Text`: a line of text that can be displayed on the screen. It only has a `text`. It can be useful to display
  a line of text that is not spoken by any character. 
- `Talk`: a line of conversation that can be displayed on the screen. It has a `text` field and a `speaker` field.
<!-- 
It is inspiried by [Ren'Py][renpy] and its scripting system although it does not use a scripting language, instead 
it uses json files. With a json file you can define the actors and the script of the conversation. In the script you can
specify actions that your actors can do (like enter the scene, exit the scene, change their expression, etc.) and you 
choices that the player can make. 

The plugin will parse this json file and build a conversation graph. TODO -->

### Usage
TODO
<!-- Here's an example of a conversation:

```json

{
    "actors": {
        "bob": { "name": "Bob", "asset": "bob.png" },
        "alice": { "name": "Alice", "asset": "alice.png" }
    },
    "script": [
        { "id": 1, "action": "talk", "actors": [] , "text": "Bob and Alice enter the room.", "start": true },
        { "id": 2, "action": "enter", "actors": [ "bob", "alice" ] },
        { "id": 3, "actors": ["bob"], "text": "Hello, Alice!" },
        {
            "id": 4,
            "choices": [
                { "text": "Alice says hello back.", "next": 5 },
                { "text": "Alice ignores Bob.", "next": 6 },
            ]
        },
        { "id": 5, "action": "talk", "actors": [], "text": "Bob smiles." },
        { "id": 6, "action": "talk", "actors": [], "text": "Bob starts crying." },
        { "id": 7, "action": "talk", "actors": [], "text": "The end." }
    ]
}
```

A future work is to have a graphical editor to create these files, but for now we have to write them by hand. -->

Compatibility of `bevy_talks` versions:
| `bevy_talks` | `bevy` |
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