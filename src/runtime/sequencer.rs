//! The sequence driver: plays an entry's cue list out over time.
//!
//! Presenting a line starts a [`PlayingSequence`] child of the runner. Each
//! frame the driver starts the cues whose time has come or whose awaited
//! message has fired, and finishes the ones whose life ended. Started cues
//! are child entities of the sequence; a [`CueLife::Until`] cue ends when the
//! game triggers [`FinishCue`] on it. When no cues remain, [`LineFinished`]
//! fires on the runner.
//!
//! Entries without a sequence play the default one from
//! [`SequencerSettings`], `wait(line_end)` unless configured: every line is a
//! sequence, and a game that wants sequences to pace the dialogue observes
//! [`LineFinished`] and triggers
//! [`AdvanceConversation`](crate::runtime::AdvanceConversation).

use std::collections::HashSet;
use std::time::Duration;

use bevy::prelude::*;

use crate::data::{ConversationId, EntryId};
use crate::scripting::cues::{CueLife, CueRecord, eval_cues};
use crate::scripting::{CompiledScripts, ScriptEngine, SequencerCommands};

/// Tuning for sequences and the `line_end` reading-time estimate.
#[derive(Resource)]
pub struct SequencerSettings {
    /// Reading speed `line_end` is estimated with.
    pub chars_per_second: f32,
    /// The floor for `line_end`.
    pub min_seconds: f32,
    /// Sequence played by entries that don't author one.
    pub default_sequence: String,
}

impl Default for SequencerSettings {
    fn default() -> Self {
        Self {
            chars_per_second: 30.0,
            min_seconds: 1.0,
            default_sequence: "wait(line_end)".to_owned(),
        }
    }
}

/// A sequence being played for a presented line. Child of the runner.
#[derive(Component)]
pub struct PlayingSequence {
    /// The runner presenting the line.
    runner: Entity,
    /// Cues not started yet.
    pending: Vec<CueRecord>,
    /// Started cues still alive.
    active: Vec<Entity>,
    /// Seconds since the sequence started.
    elapsed: f32,
    /// Messages fired so far.
    fired: HashSet<String>,
}

/// A started cue. Child of its [`PlayingSequence`].
#[derive(Component)]
pub struct Cue {
    /// The command that started this cue.
    pub name: String,
    /// Message fired when this cue finishes.
    emits: Option<String>,
}

/// Ends a [`CueLife::For`] cue when it runs out.
#[derive(Component)]
struct CueTimer(Timer);

/// Marks a cue to be finished on the next drive.
#[derive(Component)]
struct CueDone;

/// Trigger this on a [`CueLife::Until`] cue entity to finish it.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct FinishCue {
    /// The cue entity to finish.
    pub entity: Entity,
}

/// The presented line's sequence has played out.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct LineFinished {
    /// The runner whose line finished.
    pub entity: Entity,
}

/// Records the [`FinishCue`] intent for the driver.
pub(crate) fn on_finish_cue(finish: On<FinishCue>, mut commands: Commands) {
    commands.entity(finish.entity).insert(CueDone);
}

/// Builds the cue list for presenting an entry's line: its authored
/// sequence, or the default one when the entry has none or its evaluation
/// fails.
pub fn build_line_cues(
    world: &mut World,
    key: (ConversationId, EntryId),
    text: &str,
) -> Vec<CueRecord> {
    let line_end = line_end(world.resource::<SequencerSettings>(), text);
    world
        .resource::<CompiledScripts>()
        .sequence(key)
        .and_then(|ast| {
            eval_cues(world, &ast, line_end)
                .inspect_err(|error| {
                    warn!(
                        "sequence on entry {} of conversation {} failed: {error}; playing the default",
                        key.1.0, key.0.0
                    );
                })
                .ok()
        })
        .unwrap_or_else(|| default_cues(world, line_end))
}

/// The estimated reading time of `text`.
fn line_end(settings: &SequencerSettings, text: &str) -> f32 {
    (text.chars().count() as f32 / settings.chars_per_second.max(1.0)).max(settings.min_seconds)
}

/// The cue list of the default sequence. Empty when it's unset or broken.
fn default_cues(world: &mut World, line_end: f32) -> Vec<CueRecord> {
    let source = world
        .resource::<SequencerSettings>()
        .default_sequence
        .clone();
    if source.is_empty() {
        return Vec::new();
    }
    let engine = world.resource::<ScriptEngine>().0.clone();
    engine
        .compile(&source)
        .inspect_err(|error| warn!("the default sequence doesn't compile: {error}"))
        .ok()
        .and_then(|ast| {
            eval_cues(world, &ast, line_end)
                .inspect_err(|error| warn!("the default sequence failed: {error}"))
                .ok()
        })
        .unwrap_or_default()
}

/// Starts playing `cues` for `runner`'s presented line.
pub fn begin_sequence(world: &mut World, runner: Entity, cues: Vec<CueRecord>) {
    world.spawn((
        PlayingSequence {
            runner,
            pending: cues,
            active: Vec::new(),
            elapsed: 0.0,
            fired: HashSet::new(),
        },
        ChildOf(runner),
    ));
}

/// Advances every playing sequence by this frame's time.
pub fn drive_sequences(world: &mut World) {
    let delta = world.resource::<Time>().delta();
    let sequences: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayingSequence>>()
        .iter(world)
        .collect();
    for sequence in sequences {
        drive_sequence(world, sequence, delta);
    }
}

/// One frame of one sequence: finish ended cues, start due ones, and report
/// completion.
fn drive_sequence(world: &mut World, sequence: Entity, delta: Duration) {
    let Some(mut playing) = world.get_mut::<PlayingSequence>(sequence) else {
        return;
    };
    playing.elapsed += delta.as_secs_f32();
    let active = playing.active.clone();

    let ended: Vec<Entity> = active
        .into_iter()
        .filter(|&cue| cue_ended(world, cue, delta))
        .collect();
    for cue in ended {
        finish_cue(world, sequence, cue);
    }

    // Starting a cue can fire messages that make more cues due; repeat until
    // the frame settles.
    loop {
        let Some(mut playing) = world.get_mut::<PlayingSequence>(sequence) else {
            return;
        };
        let elapsed = playing.elapsed;
        let fired = playing.fired.clone();
        let (ready, waiting): (Vec<_>, Vec<_>) = playing.pending.drain(..).partition(|record| {
            record.at <= elapsed
                && record
                    .after
                    .as_ref()
                    .is_none_or(|message| fired.contains(message))
        });
        playing.pending = waiting;
        if ready.is_empty() {
            break;
        }
        ready
            .into_iter()
            .for_each(|record| start_cue(world, sequence, record));
    }

    let Some(playing) = world.get::<PlayingSequence>(sequence) else {
        return;
    };
    if playing.pending.is_empty() && playing.active.is_empty() {
        let runner = playing.runner;
        world.entity_mut(sequence).despawn();
        world.trigger(LineFinished { entity: runner });
    }
}

/// Ticks the cue's timer and reports whether its life has ended.
fn cue_ended(world: &mut World, cue: Entity, delta: Duration) -> bool {
    if world.get::<CueDone>(cue).is_some() {
        return true;
    }
    world
        .get_mut::<CueTimer>(cue)
        .is_some_and(|mut timer| timer.0.tick(delta).is_finished())
}

/// Starts one cue: built-ins directly, everything else through its command's
/// bridge.
fn start_cue(world: &mut World, sequence: Entity, record: CueRecord) {
    let cue = world
        .spawn((
            Cue {
                name: record.name.clone(),
                emits: record.emits.clone(),
            },
            ChildOf(sequence),
        ))
        .id();
    let life = match record.name.as_str() {
        "wait" => CueLife::For(Duration::from_secs_f32(
            record
                .args
                .first()
                .and_then(|secs| secs.as_float().ok())
                .unwrap_or(0.0) as f32,
        )),
        "emit" => {
            let message = record
                .args
                .first()
                .and_then(|message| message.clone().into_string().ok());
            fire(world, sequence, message);
            CueLife::Instant
        }
        name => world
            .resource::<SequencerCommands>()
            .bridge(name)
            .map_or_else(
                || {
                    warn!("no sequencer command `{name}` is registered");
                    CueLife::Instant
                },
                |bridge| {
                    bridge(world, cue, record.args.clone()).unwrap_or_else(|error| {
                        warn!("sequencer command `{name}` failed: {error}");
                        CueLife::Instant
                    })
                },
            ),
    };
    match life {
        CueLife::Instant => {
            fire(world, sequence, record.emits);
            world.entity_mut(cue).despawn();
        }
        CueLife::For(duration) => {
            world
                .entity_mut(cue)
                .insert(CueTimer(Timer::new(duration, TimerMode::Once)));
            push_active(world, sequence, cue);
        }
        CueLife::Until => push_active(world, sequence, cue),
    }
}

/// Finishes a started cue: fires its message and despawns it.
fn finish_cue(world: &mut World, sequence: Entity, cue: Entity) {
    let emits = world.get::<Cue>(cue).and_then(|cue| cue.emits.clone());
    world.entity_mut(cue).despawn();
    if let Some(mut playing) = world.get_mut::<PlayingSequence>(sequence) {
        playing.active.retain(|&alive| alive != cue);
    }
    fire(world, sequence, emits);
}

/// Fires a message into the sequence, unlocking `after` cues.
fn fire(world: &mut World, sequence: Entity, message: Option<String>) {
    if let (Some(message), Some(mut playing)) =
        (message, world.get_mut::<PlayingSequence>(sequence))
    {
        playing.fired.insert(message);
    }
}

/// Tracks a started cue as alive.
fn push_active(world: &mut World, sequence: Entity, cue: Entity) {
    if let Some(mut playing) = world.get_mut::<PlayingSequence>(sequence) {
        playing.active.push(cue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::DialogueDatabase;
    use crate::scripting::AddSequencerCommand;
    use rstest::{fixture, rstest};

    /// How many times `LineFinished` fired.
    #[derive(Resource, Default)]
    struct Finished(u32);

    /// An app with the driver wired up and a counting `LineFinished` observer.
    #[fixture]
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::TalksPlugin));
        app.init_resource::<Finished>();
        app.add_observer(|_: On<LineFinished>, mut finished: ResMut<Finished>| finished.0 += 1);
        app
    }

    /// Compiles `code`, begins it as a sequence on a fresh runner entity.
    fn begin(code: &str, world: &mut World) -> Entity {
        let engine = world.resource::<ScriptEngine>().0.clone();
        let ast = engine.compile(code).unwrap();
        let cues = eval_cues(world, &ast, 3.0).unwrap();
        let runner = world.spawn_empty().id();
        begin_sequence(world, runner, cues);
        runner
    }

    /// Advances time and drives every sequence once.
    fn drive(world: &mut World, secs: f32) {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(secs));
        drive_sequences(world);
    }

    fn finished(world: &World) -> u32 {
        world.resource::<Finished>().0
    }

    #[rstest]
    fn wait_holds_the_line_until_its_time_passes(mut app: App) {
        let world = app.world_mut();
        begin("wait(1.0)", world);
        drive(world, 0.1); // starts the wait
        drive(world, 0.6);
        assert_eq!(finished(world), 0);
        drive(world, 0.5); // the timer passes 1.0
        assert_eq!(finished(world), 1);
        drive(world, 1.0);
        assert_eq!(finished(world), 1, "a finished sequence is gone");
    }

    #[rstest]
    fn messages_unlock_after_cues_in_the_same_frame(mut app: App) {
        let world = app.world_mut();
        begin(r#"emit("go"); wait(0.2).after("go")"#, world);
        drive(world, 0.1); // emit fires and the wait starts right away
        assert_eq!(finished(world), 0);
        drive(world, 0.3);
        assert_eq!(finished(world), 1);
    }

    #[rstest]
    fn at_delays_a_cue(mut app: App) {
        let world = app.world_mut();
        begin(r#"emit("late").at(1.0)"#, world);
        drive(world, 0.1);
        assert_eq!(finished(world), 0, "the delayed cue is still pending");
        drive(world, 1.0);
        assert_eq!(finished(world), 1);
    }

    #[rstest]
    fn until_cues_end_on_finish_cue(mut app: App) {
        app.add_sequencer_command("hold", |In((_, ())): In<(Entity, ())>| CueLife::Until);
        app.update(); // rebuild the engine with the command
        let world = app.world_mut();
        begin("hold()", world);
        drive(world, 0.1);
        drive(world, 5.0);
        assert_eq!(finished(world), 0, "an Until cue outlives any clock");

        let cue = world
            .query_filtered::<Entity, With<Cue>>()
            .single(world)
            .unwrap();
        world.trigger(FinishCue { entity: cue });
        world.flush(); // the observer marks the cue through Commands
        drive(world, 0.1);
        assert_eq!(finished(world), 1);
    }

    #[rstest]
    fn lines_without_a_sequence_play_the_default(mut app: App) {
        let mut db = DialogueDatabase::default();
        db.conversations.push(crate::data::Conversation {
            id: ConversationId(1),
            entries: vec![
                crate::data::DialogueEntry {
                    id: EntryId(1),
                    sequence: r#"emit("authored")"#.to_owned(),
                    ..Default::default()
                },
                crate::data::DialogueEntry {
                    id: EntryId(2),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let _handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueDatabase>>()
            .add(db);
        app.update();
        app.update(); // asset events land a frame later

        let world = app.world_mut();
        let authored = build_line_cues(world, (ConversationId(1), EntryId(1)), "hi");
        assert_eq!(authored[0].name, "emit");

        // 60 chars at 30 cps, floor 1s: line_end is 2s.
        let default = build_line_cues(world, (ConversationId(1), EntryId(2)), &"x".repeat(60));
        assert_eq!(default[0].name, "wait");
        assert_eq!(default[0].args[0].as_float().unwrap(), 2.0);
    }
}
