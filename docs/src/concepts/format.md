# The dialogue.ron Format

Databases are stored as [RON](https://github.com/ron-rs/ron) files with the `.dialogue.ron` extension. 
The file is a direct serialization of `DialogueDatabase`.

```ron
(
    version: "1",
    variables: [
        (name: "AcceptedJob", initial: Boolean(false)),
    ],
    actors: [
        (
            id: 0,
            name: "Player",
            is_player: true,
            fields: [],
        ),
    ],
    conversations: [
        (
            id: 1,
            title: "Greeting",
            actor: 0,
            conversant: 1,
            entries: [
                (
                    id: 1,
                    actor: 1,
                    conversant: 0,
                    menu_text: "",
                    dialogue_text: "Hello",
                    is_root: true,
                    is_group: false,
                    links: [
                        (dest_conversation: 1, dest_entry: 2),
                    ],
                    fields: [
                        (title: "mood", value: Text("cheerful")),
                        (title: "canvas_x", value: Number(25.0)),
                    ],
                ),
            ],
            fields: [],
        ),
    ],
)
```

Notes:

- `fields` and `variables` may be omitted, as may an entry's `condition` and `script` (see [Conditions and Scripts](../runtime/scripting.md)).
- Field values are tagged enum variants: `Text("…")`, `Number(1.5)`, `Boolean(true)`, `Localization("…")`, `Actor(2)`.
- Loading is **lenient**: files that parse are accepted even if their content has problems. See [Validation](./validation.md).