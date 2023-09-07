//! The trigger module.

use bevy::prelude::Component;

/// A trait for components that can trigger a talk.
pub trait TalkTriggerer {
    /// Trigger the talk.
    fn trigger(&self);
}

/// The player’s Selector or Proximity Selector sent an OnUse message to the GameObject, or the Dialogue System Trigger’s OnUse() method was called manually in a UnityEvent or script.
#[derive(Component)]
pub struct OnUseTrigger;

impl TalkTriggerer for OnUseTrigger {
    fn trigger(&self) {
        println!("OnUseTrigger");
    }
}

/// The component was enabled.
#[derive(Component)]
pub struct OnEnableTrigger;

impl TalkTriggerer for OnEnableTrigger {
    fn trigger(&self) {
        println!("OnEnableTrigger");
    }
}
