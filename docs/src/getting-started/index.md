# Getting Started

*In this tutorial we will nstall Bevy Talks and do a quick overview on how to build and spawn a dialogue in your game.*

## Content
<!-- toc -->

## 1. Installation

This plugin is compatible with Bevy 0.12 and is available on crates.io. To install it, add the following line to your `Cargo.toml` file:

```toml
bevy_talks = "0.4"
```

or just run:

```bash
cargo add bevy_talks
```


## 2. Open the editor

That we don't have yet... one day... one can dream...

Just go to the next section :( 

## 3. Create a talk

You have two ways to create a dialogue: via code or via a file.

If you want to do it via code, checkout the next chapter here: [Creating Talks with TalkBuilder](#builder).

Otherwise, let's create a `talk.ron` file in your `assets` folder, let's call it `hello.talk.ron`:

```ron
(
  actors: [],
  script: []
)
```

These files are made of two parts: the actors and the script. The actors are just a list of names and slugs (the identifier for an actor) and the script if a list of actions that can be performed (talk, choice, join and leave).

### 3.1 Talking

Let's add an actor (replace the `actors` field with this):

```ron
actors: [
    ( slug: "bob", name: "Bob" )
],
```

Now let's add a talk action:

```ron
script: [
    ( id: 1, action: Talk, text: Some("Hello!"),  actors: [ "bob" ], )
]
```

An action needs to have an `id` so it can be referenced by other actions. The `action` field is the type of action, in this case `Talk`. It is not mandatory, if missing defaults to `Talk`. 
The `text` field is the text that will be displayed in the dialogue box and needs to be wrapped in `Some` when present.
Finally, the `actors` field is a list of slugs of the actors performing the action. If missing, defaults to an empty list.

### 3.2 Joining

We could also add a `Join` action before Bob starts talking to model the fact that he enters the room:

```ron
script: [
    ( id: 1, action: Join, actors: [ "bob" ], next: Some(2) ),
    ( id: 2, action: Talk, text: Some("Hello!"), actors: [ "bob" ], )
]
```

We had to add a `next` field to the `Join` action to tell the plugin which action to go to next. If missing, defaults to `None` and the dialogue will end.

### 3.3 Leaving

Now let's send Bob away after his line:

```ron
script: [
    ( id: 1, action: Join, actors: [ "bob" ], next: Some(2) ),
    ( id: 2, text: Some("Hello!"), actors: [ "bob" ], next: Some(3)),
    ( id: 3, action: Leave, actors: [ "bob" ] )
]
```

Notice we can remove the `action` field from the `Talk` action, since it defaults to `Talk`.

### 3.4 Choices

The plugin also supports player choices. This results in branching because the talk continues with the action chosen by the player.


```ron
script: [
    ( id: 1, action: Join, actors: [ "bob" ], next: Some(2) ),
    ( id: 2, text: Some("Hello!"), actors: [ "bob" ], next: Some(3) ),
    ( id: 3, action: Choice, choices: Some([
        (text: "Hi Bob", next: 5), 
        (text: "I'm Alice's BF.", next: 4)
    ])),
    ( id: 4, action: Leave, actors: [ "bob" ] ),
    ( id: 5, text: Some(":)"), actors: [ "bob" ] ),
]
```

We added a `Choice` action with two choices. In each choice the `text` field is the text that you can display associated with a choice, and the `next` field is the id of the action to go to next if the player chooses that option.

We also don't really need the `action` field for the Choice action. If the choice vector is defined, it defaults to `Choice`.

Notice that we didn't add the `next` field to the last two actions. Any of the two choices will end the dialogue.

### 3.5 The Complete Talk

Here's the full talk.ron file:

```ron
(
    actors: [
        ( slug: "bob", name: "Bob" ),
    ],
    script: [
        ( id: 1, action: Join, actors: [ "bob" ], next: Some(2) ),
        ( id: 2, text: Some("Hello!"), actors: [ "bob" ], next: Some(3) ),
        ( id: 3, action: Choice, choices: Some([
            (text: "Hi Bob", next: 5), 
            (text: "I'm Alice's BF.", next: 4)
        ])),
        ( id: 4, action: Leave, actors: [ "bob" ] ),
        ( id: 5, text: Some(":)"), actors: [ "bob" ], next: Some(2) ),
    ]
)
```

#### 3.5.1 Loops

If you want to loop back, just use the next field:

```ron
script: [
    ( id: 1, action: Join, actors: [ "bob" ], next: Some(2) ),
    ( id: 2, text: Some("Hello!"), actors: [ "bob" ], next: Some(3) ),
    ...
    ( id: 5, text: Some(":)"), actors: [ "bob" ], next: Some(2) ),
]
```

## 4. Spawning the talk in your game

Now that we have a talk, let's add it to our game. To load the asset:

```rust
let h: Handle<TalkData> = asset_server.load("hello.talk.ron");
```

That creates a `TalkData` asset. We need to store that handle so we can retrieve the actual TalkData and use it to spawn the action entities in the world:

```rust
#[derive(Resource)]
struct MyTalkHandle(Handle<TalkData>);

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let h: Handle<TalkData> = server.load("hello.talk.ron");
    commands.insert_resource(MyTalkHandle(h));
}
```

Now that we have the system that loads the talk, we need to spawn it in the world. We can do that in another system:

```rust
fn spawn_talk(
    mut commands: Commands,
    talks: Res<Assets<TalkData>>,
    talk_handle: Res<MyTalkHandle>,
) {
    let my_talk = talks.get(&talk_handle.0).unwrap();
    let talk_builder = TalkBuilder::default().fill_with_talk_data(my_talk); // create a TalkBuilder with the TalkData

    let mut talk_commands = commands.talks(); // commands is extended with the TalkCommands
    talk_commands.spawn_talk(talk_builder, ()); // spawn the talk graph
}
```

Alright! Now we have just spawned a graph of entities where each action is an entity with their own components. The actions performed by actors are also connected to the actors entities (just Bob in our case). 

The entire graph is a child of a main entity with the `Talk` component. So we can just query for that entity and use the Talk component to get the data we need to display.

## 5. Displaying the talk

The plugin doesn't provide any UI right now, so you can use whatever you want to display the dialogue.
A quick way is to query for the Talk component and print the current node to the console:

```rust
/// Print the current talk node (if changed) to the console.
fn print(talk_comps: Query<Ref<Talk>>) { // with Ref<Talk> we get access to change detection
    for talk in &talk_comps {
        if !talk.is_changed() || talk.is_added() {
            continue;
        }

        let actors = &talk.current_actors;

        let mut speaker = "Narrator";
        if !talk.current_actors.is_empty() {
            speaker = &talk.current_actors[0];
        }

        match talk.current_kind {
            NodeKind::Talk => println!("{speaker}: {}", talk.current_text),
            NodeKind::Join => println!("--- {actors:?} enters the scene."),
            NodeKind::Leave => println!("--- {actors:?} exit the scene."),
            NodeKind::Choice => {
                println!("Choices:");
                for (i, choice) in talk.current_choices.iter().enumerate() {
                    println!("{}: {}", i + 1, choice.text);
                }
            }
            _ => (),
        };
    }
}
```

The Talk component has several fields that you can use to get the data of the current node of the dialogue graph.

Here we are using the `current_kind` field to check what kind of node we are in and then print the text, the actors or the choices.

If the current node has no actors (checked with `current_actors`) we default to "Narrator".

## 6. Interacting with the talk

We spawned and printed the talk, but we can't interact with it to move forward (or pick a choice). 

To do that, the plugin has a 2 events that you can use: `NextActionRequest` and `ChooseActionRequest`. They both need the entity with the `Talk` component you want to update, and for the `ChooseActionRequest` you also need to provide the entity of the next action to go to.

```rust
fn interact(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextActionRequest>,
    mut choose_action_events: EventWriter<ChooseActionRequest>,
    talks: Query<(Entity, &Talk)>,
) {
    let (talk_ent, talk) = talks.single(); // let's grab our talk entity

    if talk.current_kind == NodeKind::Choice { // if it's the choice node, let's check the input
        if input.just_pressed(KeyCode::Key1) {
            let next_ent = talk.current_choices[0].next; // choose first choice
            choose_action_events.send(ChooseActionRequest::new(talk_ent, next_ent));
        } else if input.just_pressed(KeyCode::Key2) {
            let next_ent = talk.current_choices[1].next; // choose second choice
            choose_action_events.send(ChooseActionRequest::new(talk_ent, next_ent));
        }
    }

    if input.just_pressed(KeyCode::Space) { // otherwise just try to move forward
        next_action_events.send(NextActionRequest(talk_ent));
    }
}
```

To grab the Talk entity for the events is pretty easy, just query for it.

For the ChooseActionRequest event we have access to the current choices in the Talk component. Each choice has a `next` (and a `text` used in the print system) field with the entity of the next action to go to. So we just grab that and send the event.

## That's it!

The tutorial was based on the ["full" example](https://github.com/giusdp/bevy_talks/blob/main/examples/full.rs) code in the examples folder. Also checkout the other examples, in particular the [ingame](https://github.com/giusdp/bevy_talks/blob/main/examples/ingame.rs) one where 2 dialogue graphs are spawned and set as children (actually the Talk parent entity) of 2 interactable entities.

## Next Steps

The plugin is being developed slowly but steady. Many many things are still missing or being experimented with. Hopefully in the following years as Bevy will shape up to a 1.0 this plugin will be the best dialogue system for it. :)