# Actors and Participants

## Every line already knows its actors

When you author an entry in the editor, you pick two actors for it: who
speaks the line and who is being spoken to. That information is stored in
the database, so the runtime never has to guess who is talking.

When the runner presents a line, the `SubtitleStarted` event carries both as
`ActorId`s:

```rust,ignore
fn show_line(line: On<SubtitleStarted>) {
    let speaker_id = line.subtitle.actor;       // who says the line
    let listener_id = line.subtitle.conversant; // who it's said to
    // look the name up in the database:
    // db.actors.iter().find(|a| a.id == speaker_id).map(|a| &a.name)
}
```

If all your UI needs is a name above a text box, **you are done**. Look the name up by id and render.

## The problem `Participants` component solves

An `ActorId` is just a number in a data file. Suppose a line is spoken by `ActorId(1)`, "Ferry Keeper". 
The runtime has no idea that the ferry keeper in your *scene* is that entity standing on the pier. 
If you want to spawn a speech bubble above her head or point the camera at her, you need an
`Entity`, not an id.

`Participants` is how you tell the dialogue runner. It's an optional component, a plain map from `ActorId` to `Entity`,
added next to the `DialogueRunner` when you spawn it:

```rust,ignore
use std::collections::HashMap;

commands.spawn((
    DialogueRunner::new(db, ConversationRef::Title("Greeting".to_owned())),
    // "in this conversation, actor 0 is the player entity,
    //  and actor 1 is the ferry keeper entity"
    Participants(HashMap::from([
        (ActorId(0), player_entity),
        (ActorId(1), ferry_keeper_entity),
    ])),
));
```

With the map in place, the runner does the lookup for you on every line:

```rust,ignore
fn show_line(line: On<SubtitleStarted>) {
    if let Some(speaker_entity) = line.speaker {
        // `speaker_entity` is the actual Entity, we can spawn the bubble above it
    }
}
```

### The rules:

- `line.subtitle.actor` / `line.subtitle.conversant`: the `ActorId`s. **Always present**, straight from the database.
- `line.speaker` / `line.listener`: `Option<Entity>`. 
    - `Some` only when a `Participants` map on the runner contains that actor's id. 
    - `None` in every other case: no `Participants` component, or the id isn't in the map.
- You don't have to map every actor in the conversation. Only the ones you want resolved. Unmapped actors simply come through as `None`.

### Why the indirection

Because the conversation data only ever references actor *ids*, the same
graph can play against different casts. A generic "Merchant" conversation
can run in three towns: each runner maps `ActorId(1)` to a different
merchant entity, and the dialogue itself never changes.
