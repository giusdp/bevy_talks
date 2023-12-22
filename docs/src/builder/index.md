# Talk Builder

You can build dialogue graphs programmatically using an implementation of the builder pattern: `TalkBuilder`. 

> [&#9432;] 
> The `TalkBuilder` is also what is used under the hood to build the graphs from the asset files.

If you need to generate procedurally dialogue graphs, or you just don't like the asset files, you can use this approach.

### Simple Usage

Depending on your needs, building a dialogue graph via code can be more or less verbose. 
A simple, linear, conversation such as:

```mermaid
graph LR
    A((Start)) --> B[Say]
    B --> C[Say]
```

can be built with just a few lines of code:

```rust
let talk_builder = Talk::builder().say("Hello").say(bob, "World");
let talk_root_entity = talk_builder.build(&app.world); // it needs a world to spawn the entities
```

The `build` method, for each `say`, spawns an entity and inserts the `TalkNodeBundle`. It also connects the entities lineraly with a relationship forming a dialogue graph with 2 nodes. 

You can checkout all the methods that the builder provides in the [API docs](https://docs.rs/bevy_talks/latest/bevy_talks/builder/struct.TalkBuilder.html).

### Build Branching Conversations

The builder normally just chains the nodes one after the other as you call the methods. If, instead, you need to connect a node to multiple other nodes (e.g. a choice node) you'll have to start branching.

The simplest example would be a conversation with just 1 choice node:

```mermaid
graph LR
    A((Start)) --> B[Say]
    B --> C[Choice]
    C --> D[Say]
    C --> E[Say]
```

```rust
let talk_builder = Talk::builder();

talk_builder.say("How are you?")
    .choice(vec![
    Choice {
        text: "I'm fine",
        branch: Task::builder().say("I'm glad to hear that").branch(),
    }, 
    Choice {
        text: "I'm not fine",
        branch: Task::builder().say("I'm sorry to hear that").branch(),
    }]);

let talk_root_entity = talk_builder.build(&app.world);
``` 

The choice node expects a vector of `Choice` structs. Each `Choice` struct has a `text` field and a `branch` field. 

The `text` field is the text to be displayed to the player. The `branch` field instead wants a "branch" of the conversation that is also built via a `TalkBuilder`.

In this case, instead of calling `build` at the end, we call `branch` to create the branch of the conversation and to avoid spawning already all the entity nodes.

### Multiple Branches

To make the example a bit more complex, let's say that we have another choice in a branch:

```mermaid
graph LR
    A((Start)) --> B[Say]
    B --> C[Choice]
    C --> D[Say]
    C --> E[Say]
    E --> F[Choice]
    F --> G[Say]
    F --> H[Say]
```

```rust
let talk_builder = Talk::builder();

let happy_branch = Talk::builder().say("I'm glad to hear that").branch();

talk_builder.say("How are you?")
    .choice(vec![
        Choice {
            text: "I'm fine",
            branch: happy_branch,
        }, 
        Choice {
            text: "I'm not fine",
            branch: Talk::builder()
                .say("Why?")
                .choice(vec![
                    Choice {
                        text: "Jk, I'm fine",
                        branch: happy_branch,
                    }, 
                    Choice {
                        text: "Cause I want a cool editor!",
                        branch: Talk::builder().say("I know!").branch(),
                    }]
                .branch(),
        }]
    );

let talk_root_entity = talk_builder.build(&app.world);
```

As you can see, it's easy to keep branching the conversation. You can also reuse branches as we did with the `happy_branch` variable. The problem with this approach is that it can get quite verbose and hard to read. 

It is recommended to use the asset files for more complex conversations, but this can be useful if you want to quickly give some lines of texts to an item, or an NPC, or you are generating the conversation procedurally.


### Connecting Nodes Manually

You can connect nodes manually with the `connect_to` method. But you will need to have the node to connect to. 

If for some reason we need a loop like this:

```mermaid
graph LR
    A((Start)) --> B[Say]
    B --> C[Say]
    C --> B
```

```rust
let talk_builder = Talk::builder();
let node_a = talk_builder.say("Hello").node();
talk_builder.say("World").connect_to(node_a).build(&app.world);
```

The `node` method returns an identifier of the node, and we can use it to do manual connections. 
If you want a one node loop, you can connect the node to itself.

You can also chain multiple `connect_to` calls to connect multiple nodes to the same node.

### Branching and Manual Connections

Suppose we want to build this conversation:

```mermaid
graph LR
    A((Start)) --> B[Say]
    B --> C[Say]
    C --> D[Choice]
    D --> E[Say]
    D --> F[Say]
    F --> B
```

Situations like this are somewhat common. You are talking to an NPC where only one choice lets you continue 
and the others are just some flavour text or some extra lore. 

```rust
let talk_builder = Talk::builder();

let convo_start = talk_builder.say("Hello").node();

convo_start
    .say("Hey")
    .choice(vec![
        Choice {
            text: "Good Choice",
            branch: Talk::builder()
                .say("End of the conversation")
                .branch(),
        },
        Choice {
            text: "Wrong Choice",
            branch: Talk::builder()
                .say("Go Back")
                .connect_to(convo_start)
                .branch(),
        }
    )
    .build(&app.world);
 ```