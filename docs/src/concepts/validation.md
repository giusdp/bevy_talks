# Validation

`bevy_talks` never refuses to load a database because its *content* is wrong.
Instead, validation is a separate, explicit step that reports problems:

```rust,ignore
use bevy_talks::prelude::validate;

for issue in validate(&db) {
    warn!("{issue}");
}
```

An empty result means the database is clean. The reported `Issue`s:

| Issue | Meaning |
|---|---|
| `DuplicateActor` | two actors share an id |
| `DuplicateConversation` | two conversations share an id |
| `DuplicateEntry` | two entries in one conversation share an id |
| `NoRoot` | a conversation has no root entry |
| `MultipleRoots` | a conversation has more than one root entry |
| `DanglingLink` | a link points at a missing destination |

The editor runs validation continuously and shows the result in its status bar. 
