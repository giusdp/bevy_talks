//! Events used in the plugin.

use bevy::prelude::{Entity, Event};

/// Event to request the next action in a `Talk`. It requires an entity with the `Talk` component you want to update.
///
/// This event is typically used wired to an input from the player, e.g. a mouse click to advance the current dialogue.
/// It can fail (and logs an error) in case there is no next action or in case the current action is a choice action.
#[derive(Event)]
pub struct NextActionRequest(pub Entity);

/// An event to jump to some specific node in a graph. It requires an entity with the `Talk` component you want to update.
///
/// It is typically used when you want to go to a target node from a choice node.
/// The `ActionId` to jump to is the one defined in the next field for the Choice choosen by the player.
#[derive(Event)]
pub struct ChooseActionRequest {
    /// The entity with the `Talk` component you want to update.
    pub talk: Entity,
    /// The next entity to go to.
    pub next: Entity,
}

impl ChooseActionRequest {
    /// Creates a new `ChooseActionRequest`.
    pub fn new(talk: Entity, next: Entity) -> Self {
        Self { talk, next }
    }
}

// TODO: more events to talk to the library... (reset talk?)
// TODO: events in the other direction: from the library to the game (e.g. text event when reaching a text action node...)

// region: EXPERIMENTS

// trait DialogueNode<E: Event>: Component<Storage = TableStorage> {
//     fn emit_event(&self, writer: EventWriter<E>);
// }

// // #[derive(DialogueNode)]
// #[derive(Component)]
// pub struct MyNode {
//     pub my_field: bool,
// }

// impl DialogueNode<MyEvent> for MyNode {
//     fn emit_event(&self, mut writer: EventWriter<MyEvent>) {
//         writer.send(MyEvent);
//     }
// }
// #[derive(Event)]
// pub struct MyEvent;

// /// An event to request the next action in a [`Talk`]. It requires an entity with the [`Talk`] component you want to update.
// #[derive(Event)]
// pub struct TextEvent {
//     text: String,
// }

// fn test(mut q: Query<&DialogueNode<>, mut writer: EventWriter<TextEvent>) {
//     for c in &q {
//         c.emit_event(writer);
//     }
// }

// endregion

// NOTE: An invariant of the dialogue graph is that there is always only one current node
