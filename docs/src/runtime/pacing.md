# Manual and Auto Advance

Who decides when the next line comes? Some games wait for a click on every line, others also offer an auto mode.

The key idea: a presented line is not instant, it *plays* for a while. A line with a [sequence](./cues.md) plays for as long as its effects take.

Three events are involved. Two are commands your game sends to the dialogue runner:

- `AdvanceConversation` ends the current line and moves on.
- `SkipLine` fast-forwards the current line without leaving it. Fires `LineFinished` at the end.

The third goes the other way, from the runner to your game:

- `LineFinished` reports that the line has finished playing. It exists so your game knows the line has nothing left and can advance the conversation.

## Manual

You can just ignore `LineFinished` entirely and advance on input:

```rust,ignore
// on click / key press, while the runner is presenting:
commands.trigger(AdvanceConversation { entity: runner });
```

This is the shop example. 

## Manual with fast-forward

What most dialogue-heavy games do: the first press finishes the line, the second moves on. Track whether the line has played out with a marker:

```rust,ignore
#[derive(Component)]
struct LineDone;

app.add_observer(|line: On<LineFinished>, mut commands: Commands| {
    commands.entity(line.entity).insert(LineDone);
});
app.add_observer(|line: On<SubtitleStarted>, mut commands: Commands| {
    commands.entity(line.entity).remove::<LineDone>();
});
```

Then input branches on the marker:

```rust,ignore
fn on_press(runner: Entity, done: bool, commands: &mut Commands) {
    if done {
        commands.trigger(AdvanceConversation { entity: runner });
    } else {
        commands.trigger(SkipLine { entity: runner });
    }
}
```

Press once to jump to the end, press again to continue.

## Auto-play

One observer, and the lines pace themselves:

```rust,ignore
app.add_observer(|line: On<LineFinished>, mut commands: Commands| {
    commands.trigger(AdvanceConversation { entity: line.entity });
});
```

Every line stays up for its reading time, or for as long as its authored effects take, then flows on. 
This is the cutscene example. Input still works: `SkipLine` fires `LineFinished`, which this observer turns into an advance, so pressing skip in auto mode jumps to the next line.

To give readers more time without touching every entry, stretch the default reading-time clock in `SequencerSettings`:

```rust,ignore
settings.default_sequence = "wait(line_end + 0.75)".to_owned();
```

## The toggle

Both modes can be the same two observers with a switch in front:

```rust,ignore
#[derive(Resource, Default)]
struct AutoPlay(bool);

app.add_observer(
    |line: On<LineFinished>, auto: Res<AutoPlay>, mut commands: Commands| {
        if auto.0 {
            commands.trigger(AdvanceConversation { entity: line.entity });
        } else {
            commands.entity(line.entity).insert(LineDone);
        }
    },
);
```

Input keeps the fast-forward branching from above. 
One catch when the player turns auto on: the current line may have finished long ago, sitting there waiting for a press that will never come. Catch it up when the flag flips:

```rust,ignore
fn enable_auto(
    mut auto: ResMut<AutoPlay>,
    waiting: Query<Entity, With<LineDone>>,
    mut commands: Commands,
) {
    auto.0 = true;
    for runner in &waiting {
        commands.trigger(AdvanceConversation { entity: runner });
    }
}
```

Turning auto off needs nothing: the next `LineFinished` simply inserts the marker and waits.
