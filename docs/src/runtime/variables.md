# Variables

Variables are the game state that dialogue reads and writes: has the player accepted the job, how much gold do they carry, what name did they pick. They also drive [conditions and scripts](./scripting.md) on entries.

There are two halves: definitions in the database, and a live store at runtime.

## Defining variables in the database

The database carries a list of variables with their starting values:

```rust,ignore
pub struct Variable {
    pub name: String,        // unique within the database
    pub initial: FieldValue, // what it starts as
    pub fields: Vec<Field>,  // custom data
}
```

In a `.dialogue.ron` file:

```ron
(
    version: "1",
    variables: [
        (name: "AcceptedJob", initial: Boolean(false)),
        (name: "Gold", initial: Number(10.0)),
        (name: "PlayerName", initial: Text("")),
    ],
    actors: [ /* … */ ],
    conversations: [ /* … */ ],
)
```

`variables` may be omitted entirely; old files load unchanged.

## The `Variables` resource

At runtime all values live in one resource, a plain map from name to `FieldValue`:

```rust,ignore
#[derive(Resource)]
pub struct Variables(pub HashMap<String, FieldValue>);
```

When a database asset loads, the plugin seeds the store: every variable the store doesn't know yet is inserted at its initial value. Values that already exist are left alone, so a hot-reloaded database never overwrites state the game has changed.

Read and write it like any resource:

```rust,ignore
fn accept_job(mut vars: ResMut<Variables>) {
    vars.set("AcceptedJob", true);
    vars.set("Gold", vars.number("Gold") + 50.0);
}

fn greet(vars: Res<Variables>) {
    if vars.truthy("AcceptedJob") {
        // ...
    }
}
```

`set` accepts anything convertible into a `FieldValue` (`bool`, `f32`, `&str`, `String`). Variables don't have to be declared in the database. `set` creates them on the fly; the database list is just the authored starting state.

## Reading values

A missing variable or a type mismatch returns the type's default instead of panicking.

| Accessor | Returns | On missing / wrong type |
|---|---|---|
| `get(name)` | `Option<&FieldValue>` | `None` |
| `truthy(name)` | `bool` | `false` |
| `number(name)` | `f32` | `0.0` |
| `text(name)` | `&str` | `""` |

Variables reset when the game closes. To keep them across sessions, see [Saving and Loading](./persistence.md).
