//! Types used by the ron loader.

use serde::Deserialize;

use crate::prelude::{Action, ActionId, ActorSlug, ChoiceData, NodeKind};

/// The ron talk asset type.
///
/// It contains a list of actors that appear in the Talk, and a list of actions that make up the Talk.
#[derive(Deserialize, Debug)]
pub(crate) struct RonTalk {
    /// The list of actors that appear in the Talk.
    pub(crate) actors: Vec<RonActor>,
    /// The list of actions that make up the Talk.
    pub(crate) script: Vec<RonAction>,
}

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct RonAction {
    /// The ID of the action.
    pub(crate) id: ActionId,
    /// The kind of action.
    #[serde(default)]
    pub(crate) action: NodeKind,
    /// The actors involved in the action.
    #[serde(default)]
    pub(crate) actors: Vec<ActorSlug>,
    /// Any choices that the user can make during the action.
    pub(crate) choices: Option<Vec<RonChoice>>,
    /// The text of the action.
    pub(crate) text: Option<String>,
    /// The ID of the next action to perform.
    pub(crate) next: Option<ActionId>,
}

impl From<RonAction> for Action {
    fn from(val: RonAction) -> Self {
        let mut action_kind = val.action;
        if action_kind == NodeKind::Talk && val.choices.is_some() {
            action_kind = NodeKind::Choice;
        }
        Action {
            kind: action_kind,
            actors: val.actors,
            choices: val
                .choices
                .map_or(vec![], |c| c.into_iter().map(|c| c.into()).collect()),
            text: val.text.unwrap_or_default(),
            next: val.next,
        }
    }
}

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct RonActor {
    /// A string identifying uniquely the actor.
    pub(crate) slug: ActorSlug,
    /// The name of the character that the actor plays.
    pub(crate) name: String,
    // An optional asset that represents the actor's appearance or voice.
    // pub(crate) asset: Option<String>,
}
/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RonChoice {
    /// The text of the choice.
    pub(crate) text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub(crate) next: ActionId,
}

impl From<RonChoice> for ChoiceData {
    fn from(val: RonChoice) -> Self {
        ChoiceData {
            text: val.text,
            next: val.next,
        }
    }
}
