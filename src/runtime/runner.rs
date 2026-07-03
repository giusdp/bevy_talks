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
use super::variables::Variables;
use super::visits::Visits;
use crate::data::{ActorId, ConversationId, DialogueDatabase, EntryId};
use crate::scripting::{check_condition, ensure_compiled, run_script};

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

    /// A runner that resumes a saved conversation at `at`, re-presenting that
    /// entry once the database is loaded. `at` comes from [`save_point`](Self::save_point).
    pub fn resume(database: Handle<DialogueDatabase>, at: (ConversationId, EntryId)) -> Self {
        Self {
            database,
            conversation: ConversationRef::Id(at.0),
            phase: Phase::Resuming { at },
        }
    }

    /// Where this conversation currently is, for saving. `None` when the
    /// runner hasn't started or has ended; feed it back to [`resume`](Self::resume).
    pub fn save_point(&self) -> Option<(ConversationId, EntryId)> {
        match &self.phase {
            Phase::Presenting { at }
            | Phase::AwaitingChoice { at, .. }
            | Phase::Resuming { at }
            | Phase::Advancing { from: at }
            | Phase::Choosing { to: at } => Some(*at),
            Phase::Starting | Phase::Ended => None,
        }
    }
}

/// Where a [`DialogueRunner`] currently is.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Waiting for the database asset; steps to the first line once loaded.
    Starting,
    /// Waiting for the database asset; re-presents the saved entry once
    /// loaded, without counting a new visit or re-running its script.
    Resuming {
        /// The entry to resume at.
        at: (ConversationId, EntryId),
    },
    /// A subtitle is on screen; waiting for [`AdvanceConversation`].
    Presenting {
        /// The entry being presented.
        at: (ConversationId, EntryId),
    },
    /// The current line is done; [`drive_runners`] steps past it next Update.
    Advancing {
        /// The entry being stepped past.
        from: (ConversationId, EntryId),
    },
    /// A menu is on screen; waiting for [`ChooseResponse`].
    AwaitingChoice {
        /// The entry whose links produced the menu.
        at: (ConversationId, EntryId),
        /// The offered responses.
        responses: Vec<Response>,
    },
    /// A response was picked; [`drive_runners`] presents it next Update.
    Choosing {
        /// The chosen entry.
        to: (ConversationId, EntryId),
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

/// Marks a presenting runner as ready to step past its current line.
pub fn on_advance(advance: On<AdvanceConversation>, mut runners: Query<&mut DialogueRunner>) {
    let Ok(mut runner) = runners.get_mut(advance.entity) else {
        return;
    };
    let Phase::Presenting { at } = runner.phase else {
        warn!("AdvanceConversation while not presenting; ignored");
        return;
    };
    runner.phase = Phase::Advancing { from: at };
}

/// Marks a choosing runner's picked response, by menu index.
pub fn on_choose(choose: On<ChooseResponse>, mut runners: Query<&mut DialogueRunner>) {
    let Ok(mut runner) = runners.get_mut(choose.entity) else {
        return;
    };
    let Phase::AwaitingChoice { responses, .. } = &runner.phase else {
        warn!("ChooseResponse while no menu is open; ignored");
        return;
    };
    let Some(response) = responses.get(choose.index) else {
        warn!("ChooseResponse index {} out of bounds", choose.index);
        return;
    };
    runner.phase = Phase::Choosing {
        to: (response.conversation, response.entry),
    };
}

/// Drives every runner with pending work: starting, resuming, advancing past
/// a finished line, or presenting a chosen response.
///
/// Exclusive because conditions and scripts may reach anything in the world.
pub fn drive_runners(world: &mut World) {
    let pending: Vec<_> = world
        .query::<(Entity, &DialogueRunner)>()
        .iter(world)
        .filter(|(_, runner)| {
            matches!(
                runner.phase,
                Phase::Starting
                    | Phase::Resuming { .. }
                    | Phase::Advancing { .. }
                    | Phase::Choosing { .. }
            )
        })
        .map(|(entity, runner)| {
            (
                entity,
                runner.database.clone(),
                runner.conversation.clone(),
                runner.phase.clone(),
            )
        })
        .collect();
    for (entity, database, conversation, phase) in pending {
        drive_runner(world, entity, &database, conversation, phase);
    }
}

/// Steps one runner; does nothing while its database asset isn't loaded.
fn drive_runner(
    world: &mut World,
    entity: Entity,
    database: &Handle<DialogueDatabase>,
    conversation: ConversationRef,
    phase: Phase,
) {
    // The database is cloned out of Assets so conditions, scripts, and
    // observers keep unrestricted world access while we traverse it.
    let Some(db) = world
        .resource::<Assets<DialogueDatabase>>()
        .get(database)
        .cloned()
    else {
        return;
    };
    // Seeding and compilation normally follow asset events, which land one
    // frame after the asset exists. A conversation starting on that first
    // frame must not race them, so first contact does both (idempotently).
    if matches!(phase, Phase::Starting | Phase::Resuming { .. }) {
        world.resource_mut::<Variables>().seed(&db);
        ensure_compiled(world, database.id(), &db);
    }
    match phase {
        Phase::Starting => {
            match find_conversation(&db, &conversation)
                .and_then(|c| root_entry(c).map(|e| (c.id, e.id)))
            {
                Some(root) => advance_from(world, entity, &db, root),
                None => {
                    warn!("conversation {conversation:?} not found");
                    let nowhere = (ConversationId::default(), EntryId::default());
                    apply_step(world, entity, Step::End, nowhere, true);
                }
            }
        }
        Phase::Resuming { at } => {
            let step = match subtitle_at(&db, at) {
                Some(subtitle) => Step::Line(subtitle),
                None => {
                    warn!("resume point {at:?} not found");
                    Step::End
                }
            };
            apply_step(world, entity, step, at, false);
        }
        Phase::Advancing { from } => advance_from(world, entity, &db, from),
        Phase::Choosing { to } => {
            let step = match subtitle_at(&db, to) {
                Some(subtitle) => Step::Line(subtitle),
                None => Step::End,
            };
            apply_step(world, entity, step, to, true);
        }
        _ => {}
    }
}

/// Steps past `from` to whatever follows, gating links by their conditions.
fn advance_from(
    world: &mut World,
    entity: Entity,
    db: &DialogueDatabase,
    from: (ConversationId, EntryId),
) {
    let step = step_from(db, from, &mut |key| check_condition(world, key));
    apply_step(world, entity, step, from, true);
}

/// Applies a [`Step`] to a runner: updates its phase, records visits, runs
/// the presented entry's script, and emits the event. `from` is the entry the
/// step was taken from, kept as the menu's position. `fresh: false` means the
/// line was already seen (resume): no visit is counted, no script runs.
fn apply_step(
    world: &mut World,
    entity: Entity,
    step: Step,
    from: (ConversationId, EntryId),
    fresh: bool,
) {
    match step {
        Step::Line(subtitle) => {
            let at = (subtitle.conversation, subtitle.entry);
            set_phase(world, entity, Phase::Presenting { at });
            if fresh {
                world.resource_mut::<Visits>().record_displayed(at);
                run_script(world, at);
            }
            let bound = |world: &World, actor: ActorId| {
                world
                    .get::<Participants>(entity)
                    .and_then(|p| p.0.get(&actor).copied())
            };
            let speaker = bound(world, subtitle.actor);
            let listener = bound(world, subtitle.conversant);
            world.trigger(SubtitleStarted {
                entity,
                subtitle,
                speaker,
                listener,
            });
        }
        Step::Menu(responses) => {
            if fresh {
                let mut visits = world.resource_mut::<Visits>();
                for response in &responses {
                    visits.record_offered((response.conversation, response.entry));
                }
            }
            set_phase(
                world,
                entity,
                Phase::AwaitingChoice {
                    at: from,
                    responses: responses.clone(),
                },
            );
            world.trigger(ResponseMenuOpened { entity, responses });
        }
        Step::End => {
            set_phase(world, entity, Phase::Ended);
            world.trigger(ConversationEnded { entity });
        }
    }
}

/// Sets a runner's phase, if the runner still exists.
fn set_phase(world: &mut World, entity: Entity, phase: Phase) {
    if let Some(mut runner) = world.get_mut::<DialogueRunner>(entity) {
        runner.phase = phase;
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
            condition: String::new(),
            script: String::new(),
            sequence: String::new(),
        };
        DialogueDatabase {
            version: "1".to_owned(),
            variables: vec![],
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
        app_with(db())
    }

    /// An app with event-logging observers and one runner on `db`.
    fn app_with(db: DialogueDatabase) -> (App, Entity) {
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
            .add(db);
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
    fn tracks_visits_and_save_point(test_app: (App, Entity)) {
        let (mut app, runner) = test_app;
        app.update(); // presents "Hello" (entry 2)
        assert_eq!(
            app.world()
                .get::<DialogueRunner>(runner)
                .unwrap()
                .save_point(),
            Some((ConversationId(1), EntryId(2)))
        );

        app.world_mut()
            .trigger(AdvanceConversation { entity: runner });
        app.update(); // opens the menu offering entry 3

        let visits = app.world().resource::<Visits>();
        assert_eq!(visits.displayed((ConversationId(1), EntryId(2))), 1);
        assert_eq!(visits.offered((ConversationId(1), EntryId(3))), 1);
        // A menu's save point is the entry whose links produced it.
        assert_eq!(
            app.world()
                .get::<DialogueRunner>(runner)
                .unwrap()
                .save_point(),
            Some((ConversationId(1), EntryId(2)))
        );
    }

    #[rstest]
    fn resume_re_presents_the_saved_line_without_counting_a_visit(test_app: (App, Entity)) {
        let (mut app, runner) = test_app;
        let handle = app
            .world()
            .get::<DialogueRunner>(runner)
            .unwrap()
            .database
            .clone();
        let resumed = app
            .world_mut()
            .spawn(DialogueRunner::resume(
                handle,
                (ConversationId(1), EntryId(4)),
            ))
            .id();
        app.update();

        let emitted = &app.world().resource::<Emitted>().0;
        assert!(emitted.contains(&"line: It's Bevy Talks!".to_owned()));
        assert_eq!(
            app.world()
                .resource::<Visits>()
                .displayed((ConversationId(1), EntryId(4))),
            0
        );
        let phase = &app.world().get::<DialogueRunner>(resumed).unwrap().phase;
        assert!(matches!(phase, Phase::Presenting { .. }));
    }

    #[rstest]
    fn conditions_gate_menus_and_scripts_run_on_presentation() {
        use crate::data::{FieldValue, Variable};

        // "Hello" greets via script; the "Ask" response needs Rich, which
        // starts false; a second response "Leave" is always available.
        let mut db = db();
        db.variables.push(Variable {
            name: "Rich".to_owned(),
            initial: FieldValue::Boolean(false),
            fields: vec![],
        });
        let conversation = &mut db.conversations[0];
        conversation.entries[1].script = r#"vars["Greeted"] = true"#.to_owned();
        conversation.entries[1].links.push(Link {
            dest_conversation: ConversationId(1),
            dest_entry: EntryId(4),
        });
        conversation.entries[2].condition = r#"vars["Rich"]"#.to_owned();
        conversation.entries[3].actor = ActorId(0);
        conversation.entries[3].conversant = ActorId(1);
        conversation.entries[3].menu_text = "Leave".to_owned();

        let (mut app, runner) = app_with(db);
        app.update(); // presents "Hello" and runs its script
        assert!(app.world().resource::<Variables>().truthy("Greeted"));

        app.world_mut()
            .trigger(AdvanceConversation { entity: runner });
        app.update(); // opens the menu; "Ask" is gated out

        let emitted = &app.world().resource::<Emitted>().0;
        assert_eq!(emitted, &["line: Hello", "menu: Leave"]);
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
