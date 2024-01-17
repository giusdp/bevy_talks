//! Events to interact with the dialogue graph.
use bevy::prelude::*;
use bevy::reflect::{FromType, Reflect};
use bevy_trait_query::RegisterExt;

use crate::prelude::{Actor, ChoiceNode, JoinNode, LeaveNode, TextNode};
use crate::TalksSet;

use self::{node_events::*, requests::*};

pub mod node_events;
pub mod requests;

/// All the built-in events for `bevy_talks`.
pub(crate) struct TalksEventsPlugin;

impl Plugin for TalksEventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NextNodeRequest>()
            .add_event::<ChooseNodeRequest>()
            .add_event::<RefireNodeRequest>()
            .add_event::<StartEvent>()
            .add_event::<EndEvent>()
            .register_node_event::<TextNode, TextNodeEvent>()
            .register_node_event::<ChoiceNode, ChoiceNodeEvent>()
            .register_node_event::<JoinNode, JoinNodeEvent>()
            .register_node_event::<LeaveNode, LeaveNodeEvent>();
    }
}

/// Extension trait for [`App`] to register dialogue node events.
pub trait AppExt {
    /// Registers a node event for a component.
    fn register_node_event<
        C: Component + NodeEventEmitter + bevy::reflect::GetTypeRegistration,
        T: Event + bevy::reflect::GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self;
}

impl AppExt for App {
    fn register_node_event<
        C: Component + NodeEventEmitter + bevy::reflect::GetTypeRegistration,
        E: Event + bevy::reflect::GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self {
        if !self.world.contains_resource::<Events<E>>() {
            self.add_event::<E>();
        }
        self.add_event::<EmissionTrigger<E>>();
        self.add_systems(PreUpdate, relay_node_event::<E>.after(TalksSet));
        self.register_type::<C>();
        self.register_type::<E>();
        self.register_component_as::<dyn NodeEventEmitter, C>();
        info!("Registered node emitter: {}", std::any::type_name::<C>());

        self
    }
}

/// A struct used to operate on reflected [`Event`] of a type.
///
/// A [`ReflectEvent`] for type `T` can be obtained via
/// [`bevy::reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectEvent(ReflectEventFns);

/// The raw function pointers needed to make up a [`ReflectEvent`].
///
/// This is used when creating custom implementations of [`ReflectEvent`] with
/// [`ReflectEvent::new()`].
#[derive(Clone)]
pub struct ReflectEventFns {
    /// Function pointer implementing [`ReflectEvent::send()`].
    pub send: fn(&dyn Reflect, &mut World),
}

impl ReflectEventFns {
    /// Get the default set of [`ReflectEventFns`] for a specific component type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Event + Reflect + Clone>() -> Self {
        <ReflectEvent as FromType<T>>::from_type().0
    }
}

impl ReflectEvent {
    /// Sends reflected [`Event`] to world using [`send()`](ReflectEvent::send).
    pub fn send(&self, event: &dyn Reflect, world: &mut World) {
        (self.0.send)(event, world)
    }

    /// Create a custom implementation of [`ReflectEvent`].
    pub fn new(fns: ReflectEventFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectEvent`.
    pub fn fn_pointers(&self) -> &ReflectEventFns {
        &self.0
    }
}

impl<E: Event + Reflect + Clone> FromType<E> for ReflectEvent {
    fn from_type() -> Self {
        ReflectEvent(ReflectEventFns {
            send: |event, world| {
                if let Some(ev) = event.downcast_ref::<E>() {
                    world.send_event(ev.clone());
                }
            },
        })
    }
}

/// Trait to implement on dialogue node components to make them emit an event when reached.
#[bevy_trait_query::queryable]
pub trait NodeEventEmitter {
    /// Creates an event to be emitted when a node is reached.
    fn make(&self, actors: &[Actor]) -> Box<dyn Reflect>;
}

/// Internal event used to trigger the emission of a node event.
#[derive(Event)]
pub(crate) struct EmissionTrigger<T: Event> {
    /// The event to be emitted.
    pub(crate) event: T,
}

/// System that relays node events to their respective event channels.
fn relay_node_event<T: Event>(mut t: ResMut<Events<EmissionTrigger<T>>>, mut w: EventWriter<T>) {
    t.drain().for_each(|EmissionTrigger { event }| {
        w.send(event);
    });
}

#[cfg(test)]
mod tests {
    use crate::tests::talks_minimal_app;

    #[test]
    fn node_events_registered() {
        use super::*;

        let app = talks_minimal_app();
        assert!(app.world.contains_resource::<Events<TextNodeEvent>>());
        assert!(app.world.contains_resource::<Events<ChoiceNodeEvent>>());
        assert!(app.world.contains_resource::<Events<JoinNodeEvent>>());
        assert!(app.world.contains_resource::<Events<LeaveNodeEvent>>());
    }
}
