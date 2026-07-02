//! The ECS shell around [`step`](super::step): runner component, entity
//! events, and the systems that drive conversations.
//!
//! Spawning a [`DialogueRunner`] starts a conversation. The runner emits
//! [`SubtitleStarted`], [`ResponseMenuOpened`], and [`ConversationEnded`]
//! entity events; the game drives it with [`AdvanceConversation`] and
//! [`ChooseResponse`]. The library never renders anything.

use std::collections::HashMap;

use bevy::prelude::*;

use super::step::{
    ConversationRef, Response, Step, Subtitle, find_conversation, root_entry, step_from,
    subtitle_at,
};
use crate::data::{ActorId, ConversationId, DialogueDatabase, EntryId};

/// A running conversation. Spawn one to start talking.
#[derive(Component, Debug)]
pub struct DialogueRunner {
    /// The database the conversation lives in.
    pub database: Handle<DialogueDatabase>,
    /// Which conversation to run.
    pub conversation: ConversationRef,
    /// Where the conversation currently is.
    pub phase: Phase,
}

impl DialogueRunner {
    /// A runner that starts `conversation` as soon as the database is loaded.
    pub fn new(database: Handle<DialogueDatabase>, conversation: ConversationRef) -> Self {
        Self {
            database,
            conversation,
            phase: Phase::Starting,
        }
    }
}

/// Where a [`DialogueRunner`] currently is.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Waiting for the database asset; steps to the first line once loaded.
    Starting,
    /// A subtitle is on screen; waiting for [`AdvanceConversation`].
    Presenting {
        /// The entry being presented.
        at: (ConversationId, EntryId),
    },
    /// A menu is on screen; waiting for [`ChooseResponse`].
    AwaitingChoice {
        /// The offered responses.
        responses: Vec<Response>,
    },
    /// The conversation is over. The runner sticks around; despawning it is
    /// the game's call.
    Ended,
}

/// Binds database actors to world entities, so subtitle events can point at
/// the entities that speak and listen.
#[derive(Component, Debug, Default)]
pub struct Participants(pub HashMap<ActorId, Entity>);

/// Tells a presenting runner that the current line is done.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct AdvanceConversation {
    /// The runner to advance.
    pub entity: Entity,
}

/// Picks a response from the currently open menu, by index.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct ChooseResponse {
    /// The runner holding the menu.
    pub entity: Entity,
    /// Index into the offered responses.
    pub index: usize,
}

/// A new line should be presented.
#[derive(EntityEvent, Debug, Clone)]
pub struct SubtitleStarted {
    /// The runner presenting the line.
    pub entity: Entity,
    /// The line.
    pub subtitle: Subtitle,
    /// The bound speaker entity, if [`Participants`] maps the actor.
    pub speaker: Option<Entity>,
    /// The bound listener entity, if [`Participants`] maps the conversant.
    pub listener: Option<Entity>,
}

/// A response menu should be presented.
#[derive(EntityEvent, Debug, Clone)]
pub struct ResponseMenuOpened {
    /// The runner offering the menu.
    pub entity: Entity,
    /// The choices, in link order.
    pub responses: Vec<Response>,
}

/// The conversation reached a dead end.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct ConversationEnded {
    /// The runner that ended. Still alive, in [`Phase::Ended`].
    pub entity: Entity,
}

/// Starts runners in [`Phase::Starting`] once their database is available.
pub fn start_runners(
    mut runners: Query<(Entity, &mut DialogueRunner, Option<&Participants>)>,
    databases: Res<Assets<DialogueDatabase>>,
    mut commands: Commands,
) {
    for (entity, mut runner, participants) in &mut runners {
        if runner.phase != Phase::Starting {
            continue;
        }
        let Some(db) = databases.get(&runner.database) else {
            continue;
        };
        let Some(root) = find_conversation(db, &runner.conversation)
            .and_then(|c| root_entry(c).map(|e| (c.id, e.id)))
        else {
            warn!("conversation {:?} not found", runner.conversation);
            apply_step(Step::End, entity, &mut runner, participants, &mut commands);
            continue;
        };
        apply_step(
            step_from(db, root),
            entity,
            &mut runner,
            participants,
            &mut commands,
        );
    }
}

/// Steps a presenting runner to whatever follows the current line.
pub fn on_advance(
    advance: On<AdvanceConversation>,
    mut runners: Query<(&mut DialogueRunner, Option<&Participants>)>,
    databases: Res<Assets<DialogueDatabase>>,
    mut commands: Commands,
) {
    let entity = advance.entity;
    let Ok((mut runner, participants)) = runners.get_mut(entity) else {
        return;
    };
    let Phase::Presenting { at } = runner.phase else {
        warn!("AdvanceConversation while not presenting; ignored");
        return;
    };
    let Some(db) = databases.get(&runner.database) else {
        return;
    };
    apply_step(
        step_from(db, at),
        entity,
        &mut runner,
        participants,
        &mut commands,
    );
}

/// Presents the chosen player response as the next line.
pub fn on_choose(
    choose: On<ChooseResponse>,
    mut runners: Query<(&mut DialogueRunner, Option<&Participants>)>,
    databases: Res<Assets<DialogueDatabase>>,
    mut commands: Commands,
) {
    let entity = choose.entity;
    let Ok((mut runner, participants)) = runners.get_mut(entity) else {
        return;
    };
    let Phase::AwaitingChoice { responses } = &runner.phase else {
        warn!("ChooseResponse while no menu is open; ignored");
        return;
    };
    let Some(response) = responses.get(choose.index).cloned() else {
        warn!("ChooseResponse index {} out of bounds", choose.index);
        return;
    };
    let Some(db) = databases.get(&runner.database) else {
        return;
    };
    let step = match subtitle_at(db, (response.conversation, response.entry)) {
        Some(subtitle) => Step::Line(subtitle),
        None => Step::End,
    };
    apply_step(step, entity, &mut runner, participants, &mut commands);
}

/// Applies a [`Step`] to a runner: updates its phase and emits the event.
fn apply_step(
    step: Step,
    entity: Entity,
    runner: &mut DialogueRunner,
    participants: Option<&Participants>,
    commands: &mut Commands,
) {
    match step {
        Step::Line(subtitle) => {
            runner.phase = Phase::Presenting {
                at: (subtitle.conversation, subtitle.entry),
            };
            let bound = |actor: ActorId| participants.and_then(|p| p.0.get(&actor).copied());
            let speaker = bound(subtitle.actor);
            let listener = bound(subtitle.conversant);
            commands.trigger(SubtitleStarted {
                entity,
                subtitle,
                speaker,
                listener,
            });
        }
        Step::Menu(responses) => {
            commands.trigger(ResponseMenuOpened {
                entity,
                responses: responses.clone(),
            });
            runner.phase = Phase::AwaitingChoice { responses };
        }
        Step::End => {
            runner.phase = Phase::Ended;
            commands.trigger(ConversationEnded { entity });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TalksPlugin;
    use crate::data::{Actor, Conversation, DialogueEntry, Link};
    use rstest::{fixture, rstest};

    /// Player asks one question, NPC answers, conversation ends.
    fn db() -> DialogueDatabase {
        let entry = |id: i32, actor: i32, menu: &str, text: &str, links: Vec<i32>| DialogueEntry {
            id: EntryId(id),
            actor: ActorId(actor),
            conversant: ActorId(1 - actor),
            menu_text: menu.to_owned(),
            dialogue_text: text.to_owned(),
            is_root: id == 1,
            is_group: false,
            links: links
                .into_iter()
                .map(|to| Link {
                    dest_conversation: ConversationId(1),
                    dest_entry: EntryId(to),
                })
                .collect(),
            fields: vec![],
        };
        DialogueDatabase {
            version: "1".to_owned(),
            actors: vec![
                Actor {
                    id: ActorId(0),
                    name: "Player".to_owned(),
                    is_player: true,
                    fields: vec![],
                },
                Actor {
                    id: ActorId(1),
                    name: "Feri".to_owned(),
                    is_player: false,
                    fields: vec![],
                },
            ],
            conversations: vec![Conversation {
                id: ConversationId(1),
                title: "Test".to_owned(),
                actor: ActorId(0),
                conversant: ActorId(1),
                entries: vec![
                    entry(1, 1, "", "", vec![2]),
                    entry(2, 1, "", "Hello", vec![3]),
                    entry(3, 0, "Ask", "What is this?", vec![4]),
                    entry(4, 1, "", "It's Bevy Talks!", vec![]),
                ],
                fields: vec![],
            }],
        }
    }

    /// Everything the runner emitted, in order.
    #[derive(Resource, Default)]
    struct Emitted(Vec<String>);

    #[fixture]
    fn test_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));
        app.init_resource::<Emitted>();
        app.add_observer(|line: On<SubtitleStarted>, mut emitted: ResMut<Emitted>| {
            emitted.0.push(format!("line: {}", line.subtitle.text));
        });
        app.add_observer(
            |menu: On<ResponseMenuOpened>, mut emitted: ResMut<Emitted>| {
                let labels: Vec<&str> = menu.responses.iter().map(|r| r.text.as_str()).collect();
                emitted.0.push(format!("menu: {}", labels.join(", ")));
            },
        );
        app.add_observer(|_: On<ConversationEnded>, mut emitted: ResMut<Emitted>| {
            emitted.0.push("ended".to_owned());
        });

        let handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueDatabase>>()
            .add(db());
        let runner = app
            .world_mut()
            .spawn(DialogueRunner::new(
                handle,
                ConversationRef::Title("Test".to_owned()),
            ))
            .id();
        (app, runner)
    }

    #[rstest]
    fn plays_a_conversation_end_to_end(test_app: (App, Entity)) {
        let (mut app, runner) = test_app;

        app.update();
        app.world_mut()
            .trigger(AdvanceConversation { entity: runner });
        app.update();
        app.world_mut().trigger(ChooseResponse {
            entity: runner,
            index: 0,
        });
        app.update();
        app.world_mut()
            .trigger(AdvanceConversation { entity: runner });
        app.update();
        app.world_mut()
            .trigger(AdvanceConversation { entity: runner });
        app.update();

        let emitted = &app.world().resource::<Emitted>().0;
        assert_eq!(
            emitted,
            &[
                "line: Hello",
                "menu: Ask",
                "line: What is this?",
                "line: It's Bevy Talks!",
                "ended",
            ]
        );
        let phase = &app.world().get::<DialogueRunner>(runner).unwrap().phase;
        assert_eq!(*phase, Phase::Ended);
    }

    #[rstest]
    fn out_of_bounds_choice_is_ignored(test_app: (App, Entity)) {
        let (mut app, runner) = test_app;
        app.update();
        app.world_mut()
            .trigger(AdvanceConversation { entity: runner });
        app.update();
        app.world_mut().trigger(ChooseResponse {
            entity: runner,
            index: 7,
        });
        app.update();

        let phase = &app.world().get::<DialogueRunner>(runner).unwrap().phase;
        assert!(matches!(phase, Phase::AwaitingChoice { .. }));
    }
}
