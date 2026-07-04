# Sequences and Cutscenes

A **sequence** is [Rhai](https://rhai.rs/book/) code that runs when the entry is presented, but instead of doing things on the spot, it schedules **cues**: timed instructions played out while the line is on screen. Camera moves, animations, sounds, pauses.

```ron
(
    id: 5,
    dialogue_text: "You dare come back here?",
    sequence: "shake_camera(0.5); play_anim(\"point\").at(0.8); wait(line_end)",
    // ...
)
```

In the editor the sequence lives in the Logic section of the entry inspector, next to the condition and the script.

## The timing methods

Every scheduled cue returns a handle, and the timing methods chain on it:

```rhai
wait(2.0)                              // a cue that lasts two seconds
emit("looked")                         // fires a message, instantly
play_anim("draw").at(1.5)             // starts 1.5 seconds in
play_sound("gasp").after("looked")    // starts when that message fires
zoom("closeup").emits("zoomed")       // fires a message when done
reset_camera().required()             // still runs if the line is skipped
```

`at` delays a cue's start. `after` holds it until a message fires. `emits` fires a message when the cue finishes, which is how cues chain off each other without counting seconds. `required` marks cleanup that must happen even when the player skips the line.

`wait` and `emit` are built in. Everything else is a command your game registers.

A sequence is full Rhai, so it can branch on game state:

```rhai
if vars["Scared"] { play_anim("cower") } else { play_anim("smirk") }
wait(line_end)
```

## line_end and the default sequence

`line_end` is the estimated reading time of the line: its character count divided by a reading speed, with a floor so short lines don't flash by. An entry with no sequence plays a default one instead. So every presented line plays a sequence and every line has a clock, even when nobody authored one.

Both knobs live in the `SequencerSettings` resource:

| Field | Default | What it does |
|---|---|---|
| `chars_per_second` | `30.0` | Reading speed `line_end` is estimated with. Lower it for slower, more deliberate pacing. |
| `min_seconds` | `1.0` | The floor for `line_end`: even a "Hi." stays up this long. |
| `default_sequence` | `"wait(line_end)"` | The sequence played by entries that don't author one. |

`TalksPlugin` inserts the defaults; to customize, overwrite the resource:

```rust,ignore
app.insert_resource(SequencerSettings {
    chars_per_second: 15.0,
    min_seconds: 2.0,
    default_sequence: r#"wait(line_end); emit("line_done")"#.to_owned(),
});
```

The default sequence is full Rhai like any authored one, so it can call your registered commands — handy for a blip sound or a portrait animation on every unauthored line. It is also the fallback: an authored sequence that fails to evaluate logs a warning and plays the default instead. Setting `default_sequence` to an empty string makes unauthored lines play no cues at all, so their `LineFinished` fires immediately.

## Registering commands

Commands are Bevy systems, registered like [dialogue systems](./scripting.md#calling-into-your-game):

```rust,ignore
app.add_sequencer_command("play_anim", play_anim);

fn play_anim(In((cue, clip)): In<(Entity, String)>, /* any system params */) -> CueLife {
    // start the clip...
    CueLife::For(Duration::from_secs_f32(1.2))
}
```

The system's `In` input is the cue entity paired with the arguments (a value or a tuple, up to four of `bool`, `i64`, `f64`, `f32`, `String`, or `Dynamic`). It returns how long the cue lives:

- `CueLife::Instant`: done the moment it ran.
- `CueLife::For(duration)`: done after that long.
- `CueLife::Until`: open-ended. The game finishes it by triggering `FinishCue` on the cue entity, when the audio ends, the tween completes, the character arrives.

## Pacing the conversation

When a line's last cue finishes, `LineFinished` fires on the runner. Advancing stays your call, so nothing moves until you trigger `AdvanceConversation`. A game that wants sequences to pace the dialogue wires the two together with one observer:

```rust,ignore
app.add_observer(|line: On<LineFinished>, mut commands: Commands| {
    commands.trigger(AdvanceConversation { entity: line.entity });
});
```

With that in place a conversation plays itself: each line stays up for its reading time, or for as long as its cues take, then flows on. Menus still wait for a choice. Manual advance, auto-play, and switching between the two are covered in [Manual and Auto Advance](./pacing.md).

## Skipping

Trigger `SkipLine` on the runner to fast-forward the line's sequence without advancing the conversation. The sequence ends immediately but lands on the same state it would have reached by playing out: `required` cues that hadn't started yet still run (marked with a `Skipped` component, so their handlers can jump straight to the end result), every running cue gets a `CueSkipped` event to snap its effects to their final state, and `LineFinished` fires.

Advancing or choosing while a sequence is still playing cuts it short the same way, except `LineFinished` does not fire, since the line was replaced rather than finished.

## Script or sequence?

If it changes the game, it is a script. If it shows the game, it is a sequence.

The two fields differ in when they re-run. A script runs once per visit and never on resume. A sequence replays every time the line is presented, including when a saved conversation resumes, because the re-presented line still needs its cues and its clock. The whole sequence body re-runs to rebuild the cue list, so writing game state there means paying for it again on every load. Keep `vars` writes and calls like `spend(10)` in the script; a sequence should be safe to run twice.

## Try it

The cutscene example plays a stormy inn scene that runs on its own: sound effects landing mid-line, a song chained with messages, a required cue that survives skipping, and the one-observer auto-advance. Enter skips a line.

```sh
cargo run --example cutscene
```
