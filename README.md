# Bevy Talks

<!-- [![crates.io](https://img.shields.io/crates/v/bevy_talks.svg)](https://crates.io/crates/bevy_talks)
[![docs](https://docs.rs/bevy_talks/badge.svg)](https://docs.rs/bevy_talks)
[![license](https://img.shields.io/crates/l/bevy_talks)](https://github.com/giusdp/bevy_talks#license)
[![crates.io](https://img.shields.io/crates/d/bevy_talks.svg)](https://crates.io/crates/bevy_talks) -->

This [Bevy][bevy] plugin provides an opinionated way to create dialogues and conversations in your game. 
It is inspiried by [Ren'Py][renpy] and its scripting system although it does not use a scripting language, instead 
it uses json files. With a json file you can define the actors and the script of the conversation. In the script you can
specify actions that your actors can do (like enter the scene, exit the scene, change their expression, etc.) and you 
choices that the player can make. 

The plugin will parse this json file and build a conversation graph. TODO

### Example
Here's an example of a conversation:

```json

{
    "actors": {
        "bob": { "name": "Bob", "asset": "bob.png" },
        "alice": { "name": "Alice", "asset": "alice.png" }
    },
    "script": [
        { "id": 1, "text": "Bob and Alice enter the room.", "start": true },
        { "id": 2, "action": "enter", "actors": [ "bob", "alice" ] },
        { "id": 3, "actor": "bob", "text": "Hello, Alice!" },
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

The idea is to have a graphical editor that to create conversations, and it will generate the json files. That is a future work
 so for now we have to write the json files by hand.

Compatibility of `bevy_talks` versions:
| `bevy_talks` | `bevy` |
| :--                 |  :--   |
| `main`              | `0.9`  |

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