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
            .add_event::<StartEvent>()
            .add_event::<EndEvent>()
            .register_node_event::<TextNode, TextNodeEvent>()
            .register_node_event::<ChoiceNode, ChoiceNodeEvent>()
            .register_node_event::<JoinNode, JoinNodeEvent>()
            .register_node_event::<LeaveNode, LeaveNodeEvent>();
    }
}

trait AppExt {
    fn register_node_event<
        C: Component + NodeEventEmitter,
        T: Event + bevy::reflect::GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self;
}

impl AppExt for App {
    fn register_node_event<
        C: Component + NodeEventEmitter,
        T: Event + bevy::reflect::GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self {
        if !self.world.contains_resource::<Events<T>>() {
            self.add_event::<T>();
        }
        self.add_event::<EmissionTrigger<T>>();
        self.add_systems(PreUpdate, relay_node_event::<T>.after(TalksSet));
        self.register_type::<T>();
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
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectEvent`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectEvent`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Event)]` attribute.
/// > After adding the component to the [`TypeRegistry`][bevy::reflect::TypeRegistry],
/// > its [`ReflectEvent`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectEvent`] may be useful if you need to create new component types
/// at runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectEvent`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected components of that type will be inserted into the Bevy
/// world.
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
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Component)]` component
    /// to generate a [`ReflectEvent`] implementation automatically.
    ///
    /// See [`ReflectEventFns`] for more information.
    pub fn new(fns: ReflectEventFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectComponent`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectComponent>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectEvent`] and keeping it
    /// between frames, cloning a `ReflectComponent` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectComponent`,
    /// use `fn_pointers` to get the underlying [`ReflectComponentFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectComponent>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
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

#[derive(Event)]
pub(crate) struct EmissionTrigger<T: Event> {
    pub(crate) event: T,
}

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
