# Rhai Reference

Conditions, scripts, and sequences all share one engine, so beyond what's listed here they are plain [Rhai](https://rhai.rs/book/): arithmetic, strings, `if`, `let`, loops, functions.

## The variable store

It's a simple map that is available everywhere. See [Variables](./variables.md). You can access it and modify it in the following ways:

| Syntax | What it does |
|---|---|
| `vars["Name"]` | Reads a variable. Unknown names are an error, so typos surface as warnings. |
| `vars["Name"] = value` | Writes a variable, creating it if needed. Holds bools, numbers, and text. |
| `vars.has("Name")` | Whether the variable exists. |

Numbers are floats on the script side, but Rhai mixes integers and floats freely: `vars["Gold"] >= 10` works.

## Built-in cues

Available in [sequences](./cues.md), you can schedule cues instead of running on the spot.

| Call | What it schedules |
|---|---|
| `wait(seconds)` | A simple wait to make a step in your cutscene to take as long as you need. |
| `emit("message")` | Fires a named message. Other cues can start `.after` it. |

| Constant | Value |
|---|---|
| `line_end` | The estimated reading time of the line being presented, from `SequencerSettings`. |

An entry with no sequence plays the default one, `wait(line_end)` unless you change it in `SequencerSettings`.

## Timing methods

There are some built-in methods you can use for timing:

| Method | Effect |
|---|---|
| `.at(seconds)` | Starts the cue that many seconds into the sequence. |
| `.after("message")` | Holds the cue until that message fires. |
| `.emits("message")` | Fires a message when the cue finishes. |
| `.required()` | Still runs when the line is skipped, for cleanup and state that must land. |

An example: 
```rhai
shake_camera(0.5);
play_anim("draw").at(1.5).emits("drawn");
play_sound("gasp").after("drawn");
reset_camera().required();
wait(line_end)
```

## Your game's functions

You register your own Bevy systems so dialogue runners can call them by name. There are two ways to register one, because there are two different things a call can mean:

| Registration | It's a... | The call in Rhai | What that means |
|---|---|---|---|
| `app.add_dialogue_system("has_item", system)` | **function** | `has_item("sword")` | Runs right away and gives back an answer. |
| `app.add_sequencer_command("play_anim", system)` | **stage direction** | `play_anim("draw").at(1.5)` | Doesn't run yet when a runner reads the sequence. It's staged for later, with a `.at`/`.after`/`.emits`/`.required` on when. |

Both kinds are callable everywhere: conditions, scripts, and sequences. That's why a sequence can mix them, `if has_item("lute") { play_anim() }`: checking `has_item` happens immediately, while `play_anim()` gets scheduled to play out. 

Both take up to four arguments of `bool`, `i64`, `f64`, `f32`, `String`, or `Dynamic`.

## Where each field runs

| Entry field | Mode | Runs | Good for |
|---|---|---|---|
| `condition` | Expression only — one expression returning a bool | When the runner checks whether the entry can be reached | Gating branches: `vars["Gold"] >= 10 && has_item("sword")` |
| `script` | Full Rhai | Once per presentation; never on resume | Changing the game: `vars["Gold"] -= 10; guard_bribed()` |
| `sequence` | Full Rhai, calls schedule cues | Every presentation, including resume | Showing the game: `play_anim("bow"); wait(line_end)` |

Broken logic never blocks dialogue: a condition that fails to compile or errors at runtime passes with a warning, a failing script is reported and skipped.
