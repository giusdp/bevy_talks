# Conditions and Scripts

Entries can carry logic. A **condition** decides whether the entry can be reached, a **script** runs when the entry is presented. Both are written in [Rhai](https://rhai.rs/book/), a small scripting language embedded in the library, and both are optional. A third kind of entry logic, the sequence, schedules what happens on screen while the line plays; it has [its own page](./cues.md).

```ron
(
    id: 3,
    menu_text: "Bribe the guard",
    dialogue_text: "Perhaps this changes your mind.",
    condition: "vars[\"Gold\"] >= 10",
    script: "vars[\"Gold\"] -= 10; guard_bribed()",
    // ...
)
```

In the editor both live in the Logic section of the entry inspector.

## The `vars` binding

Conditions and scripts see the [variable store](./variables.md) as `vars`:

```rhai
vars["Gold"] >= 10          // read
vars["AcceptedJob"] = true  // write, creates the variable if needed
vars.has("MetBoris")        // existence check
```

Reading a variable that doesn't exist is an error, so typos surface as warnings instead of silently comparing against a default. Numbers are floats on the script side, but Rhai mixes integers and floats freely: `vars["Gold"] >= 10` works.

## Conditions

A condition is a single expression that returns a bool. When the runner follows links, every destination is checked: entries whose condition fails are dropped. A choice disappears from the menu, an NPC branch is skipped, and gating a group node cuts off everything behind it. An empty condition always passes.

Conditions are expressions only. Statements like `vars["x"] = 1` don't compile there, which catches the classic `=` versus `==` mistake at load time.

## Scripts

A script runs when its entry is presented as a line, after the visit is recorded and before `SubtitleStarted` fires, so anything the script writes is visible to your observers. Scripts are full Rhai: statements, `if`, `let`, function calls.

Scripts run once per presentation. Resuming a saved conversation re-presents the line without re-running its script, matching how resume skips visit counting.

## Calling into your game

The game can expose Bevy systems to dialogue logic:

```rust,ignore
app.add_dialogue_system("has_item", |In(name): In<String>, inventory: Res<Inventory>| {
    inventory.contains(&name)
});

app.add_dialogue_system("give_item", |In(name): In<String>, mut inventory: ResMut<Inventory>| {
    inventory.add(&name);
});
```

A condition can now say `has_item("sword")` and a script `give_item("sword")`. The function runs as a one-shot system in the middle of evaluation, with everything a system can do: queries, resources, commands.

The system's `In` input is the argument list: a single value or a tuple, up to four arguments, of `bool`, `i64`, `f64`, `f32`, `String`, or `Dynamic`. Integer arguments coerce to float parameters. The return value, if any, becomes the call's result in the script; systems returning nothing are fine for fire-and-forget calls.

Register systems before the app runs; the script engine picks up changes to the registered set automatically.

## When logic breaks

Broken logic never blocks dialogue. A condition that fails to compile, errors at runtime, or returns something that isn't a bool passes with a warning in the log.
A failing script is reported and skipped. Warnings name the conversation and entry, so a typo in a condition shows up as a log line, not as a quest that can't be started.

Compilation of the script code happens once when the database loads, so syntax errors in any entry are reported up front, before the entry is ever reached.

## Try it

The shop example plays a merchant whose greeting, stock, and prices are driven by everything on this page: conditions calling a `gold()` system, scripts spending it, and a `give_item` system filling a game resource.

```sh
cargo run --example shop
```
