//! Events the plugin emits.
use bevy::prelude::*;

use crate::prelude::{Actor, Choice, ChoiceNode, JoinNode, LeaveNode, TextNode};

use super::{NodeEventEmitter, ReflectEvent};

// TODO: start and end events
/// Start event sent when a talk is started.
/// It happens when a next action request is sent to the talk that is still in the start node.
/// Contains the talk parent entity.
#[derive(Event)]
pub struct StartEvent(pub Entity);

/// End event sent when a talk reaches an end node.
/// Contains the talk parent entity.
#[derive(Event)]
pub struct EndEvent(pub Entity);

/// Emitted when a text node is reached.
#[derive(Event, Reflect, Default, Clone)]
#[reflect(Event)]
pub struct TextNodeEvent {
    /// The text from the node.
    pub text: String,
    /// The actor names from the node.
    pub actors: Vec<String>,
}

impl NodeEventEmitter for TextNode {
    fn make(&self, actors: &[Actor]) -> Box<dyn Reflect> {
        Box::from(TextNodeEvent {
            text: self.0.clone(),
            actors: actors.iter().map(|a| a.name.clone()).collect(),
        })
    }
}

/// Emitted when a choice node is reached.
#[derive(Event, Reflect, Default, Clone)]
#[reflect(Event)]
pub struct ChoiceNodeEvent {
    /// The choices from the node.
    pub choices: Vec<Choice>,
}

impl NodeEventEmitter for ChoiceNode {
    fn make(&self, _actors: &[Actor]) -> Box<dyn Reflect> {
        Box::from(ChoiceNodeEvent {
            choices: self.0.clone(),
        })
    }
}

/// Emitted when a join node is reached.
#[derive(Event, Reflect, Default, Clone)]
#[reflect(Event)]
pub struct JoinNodeEvent {
    /// The actor names from the node.
    pub actors: Vec<String>,
}

impl NodeEventEmitter for JoinNode {
    fn make(&self, actors: &[Actor]) -> Box<dyn Reflect> {
        Box::from(JoinNodeEvent {
            actors: actors.iter().map(|a| a.name.clone()).collect(),
        })
    }
}

/// Emitted when a leave node is reached.
#[derive(Event, Reflect, Default, Clone)]
#[reflect(Event)]
pub struct LeaveNodeEvent {
    /// The actor names from the node.
    pub actors: Vec<String>,
}

impl NodeEventEmitter for LeaveNode {
    fn make(&self, actors: &[Actor]) -> Box<dyn Reflect> {
        Box::from(LeaveNodeEvent {
            actors: actors.iter().map(|a| a.name.clone()).collect(),
        })
    }
}
