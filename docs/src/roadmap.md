# Roadmap

What exists today is the authoring and playback core: the database asset and
format, the editor, and a runtime that plays branching, multi-actor
conversations.

Planned, roughly in order:

- **Conditions and scripts**: gate links and entries on game state, and run effects when lines are delivered. This includes choosing an evaluation
  approach (embedded scripting language vs. typed expressions).
- **Variables**: a variable table in the database with a mutable runtime store, the state that conditions read and scripts write.
- **Link priorities**: control which branch wins when several are valid.
- **Visit tracking**: per-entry "was offered / was displayed" state, for "only say this once" logic.
- **Localization**: the `Localization` field variant exists; the runtime needs language selection and text resolution.
- **Cutscenes**: sequences of commands (camera cuts, animations, audio) that play alongside a line's delivery.