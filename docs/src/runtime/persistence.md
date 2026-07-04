# Saving and Loading

Variables and visit counts live in resources, so they vanish when the game closes. The persistence layer turns them into a snapshot your game can store and apply back.

The library does not own save files. It has no opinion on where saves live, how they are encrypted, or when to write them. 
It gives you a serializable `DialogueSave` and your game puts it wherever it keeps its saves.

## Visit tracking

Before saving, it helps to know what gets saved. Besides variables, the runner records how often each entry has been reached in the `Visits` resource:

```rust,ignore
pub struct VisitCount {
    pub offered: u32,   // times the entry appeared in a response menu
    pub displayed: u32, // times the entry was presented as a line
}
```

Presenting a line bumps its `displayed` count; opening a menu bumps `offered` for every choice in it. 
You can read it from your own systems, including [dialogue systems](./scripting.md#calling-into-your-game) called from conditions, for things like "only say this once":

```rust,ignore
fn already_greeted(visits: Res<Visits>) -> bool {
    visits.displayed((ConversationId(1), EntryId(2))) > 0
}
```

## The snapshot

`DialogueSave` captures variables and visits in one serializable struct:

```rust,ignore
// Saving: record the state and stringify it.
fn save_game(variables: Res<Variables>, visits: Res<Visits>) {
    let snapshot = DialogueSave::record(&variables, &visits);
    let text = save_to_ron(&snapshot).unwrap();
    // write `text` into your save file
}

// Loading: parse and apply.
fn load_game(mut variables: ResMut<Variables>, mut visits: ResMut<Visits>) {
    let text = /* read from your save file */;
    let snapshot = save_from_ron(&text).unwrap();
    snapshot.apply(&mut variables, &mut visits);
}
```

`save_to_ron`/`save_from_ron` are conveniences. `DialogueSave` derives serde, so you can also embed it directly in your game's own save struct and serialize everything together.

## Applying merges

`apply` overwrites the values present in the snapshot and leaves everything else alone. Combined with database seeding (which only fills in variables the store doesn't know), this makes loading order-independent and update-safe:

- Load the save before or after the database loads; the result is the same.
- Ship a game update that adds new variables or entries; old saves keep working. The new variables get their initial values, the saved ones keep their saved values.

Apply a save before anything reads `Variables`, or those reads see initial values for one moment.

## Resuming a conversation

Runners are entities your game owns, so they are not part of `DialogueSave`. If you want to save mid-conversation, store the runner's position in your own save data:

```rust,ignore
// Saving: where is the conversation?
let at: Option<(ConversationId, EntryId)> = runner.save_point();

// Loading: pick it back up.
commands.spawn(DialogueRunner::resume(database_handle, at));
```

`save_point` returns the entry currently on screen, both while a line is presented and while a menu is open (the menu's entry is the line the choices hang off). On load, the resumed runner re-presents that entry: your UI gets a fresh `SubtitleStarted` showing the line the player was reading when they saved. It does not count as a new visit. Advancing continues normally; if a menu was open at save time, it reopens on the first advance.

Most games don't save mid-conversation. If yours doesn't, skip all of this: an ended runner returns `None` and there is nothing to resume.
