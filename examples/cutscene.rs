//! Plays `assets/cutscene.dialogue.ron` in the terminal: a stormy inn scene
//! that stages itself with sequences and cues.
//!
//! Run with `cargo run --example cutscene`. The scene plays on its own;
//! press Enter to skip a line's staging, type a number to pick a response.
//!
//! What to watch for:
//! - Lines advance by themselves: one observer turns `LineFinished` into
//!   `AdvanceConversation`, so the sequences pace the conversation.
//! - Sound effects land mid-line at authored times (`sfx(...).at(1.6)`).
//! - "Play a song" chains cues with messages: the coins only clatter
//!   `after("song")`, and that cue is `required()`, so skipping the song
//!   with Enter still pays the bard.

use std::io::{BufRead, Write};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_talks::prelude::*;

/// Lines typed by the player, fed from the stdin thread.
#[derive(Resource)]
struct StdinInput(Mutex<mpsc::Receiver<String>>);

fn main() {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))),
        AssetPlugin::default(),
        TalksPlugin,
    ))
    .insert_resource(StdinInput(Mutex::new(rx)))
    .add_systems(Startup, start)
    .add_systems(Update, handle_input)
    .add_observer(print_subtitle)
    .add_observer(print_menu)
    .add_observer(finish);

    // The sequences pace the dialogue: a finished line advances itself.
    app.add_observer(|line: On<LineFinished>, mut commands: Commands| {
        commands.trigger(AdvanceConversation {
            entity: line.entity,
        });
    });

    // The commands sequences can cue.
    app.add_sequencer_command("sfx", sfx);
    app.add_sequencer_command("strum", strum);

    app.run();
}

/// `sfx(text)`: an instant atmospheric effect.
fn sfx(In((_, text)): In<(Entity, String)>) -> CueLife {
    println!("      ({text})");
    CueLife::Instant
}

/// `strum()`: the bard plays for a while.
fn strum(In((_, ())): In<(Entity, ())>) -> CueLife {
    println!("      ~ you strum a slow, sad tune ~");
    CueLife::For(Duration::from_secs(2))
}

/// Loads the database and spawns the conversation runner.
fn start(mut commands: Commands, assets: Res<AssetServer>) {
    println!("--- bevy_talks cutscene demo ---");
    println!("(the scene plays itself; Enter skips a line)\n");
    let database: Handle<DialogueDatabase> = assets.load("cutscene.dialogue.ron");
    commands.spawn(DialogueRunner::new(
        database,
        ConversationRef::Title("Storm".to_owned()),
    ));
}

/// Prints a line; its sequence takes it from here.
fn print_subtitle(
    line: On<SubtitleStarted>,
    runners: Query<&DialogueRunner>,
    databases: Res<Assets<DialogueDatabase>>,
) {
    let speaker = runners
        .get(line.entity)
        .ok()
        .and_then(|r| databases.get(&r.database))
        .and_then(|db| db.actors.iter().find(|a| a.id == line.subtitle.actor))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| format!("actor {}", line.subtitle.actor.0));
    println!("\n{speaker}: {}", line.subtitle.text);
}

/// Prints the response menu and waits for a number.
fn print_menu(menu: On<ResponseMenuOpened>) {
    println!("\nYour move:");
    for (i, response) in menu.responses.iter().enumerate() {
        println!("  {}) {}", i + 1, response.text);
    }
    print!("> ");
    let _ = std::io::stdout().flush();
}

/// Says goodbye and quits.
fn finish(_: On<ConversationEnded>, mut exit: MessageWriter<AppExit>) {
    println!("\n--- scene over ---");
    exit.write(AppExit::Success);
}

/// Enter skips the presented line's staging; a number picks a response.
fn handle_input(
    input: Res<StdinInput>,
    runners: Query<(Entity, &DialogueRunner)>,
    mut commands: Commands,
) {
    let Ok(receiver) = input.0.lock() else {
        return;
    };
    while let Ok(line) = receiver.try_recv() {
        for (entity, runner) in &runners {
            match &runner.phase {
                Phase::Presenting { .. } => {
                    commands.trigger(SkipLine { entity });
                }
                Phase::AwaitingChoice { responses, .. } => match line.trim().parse::<usize>() {
                    Ok(n) if (1..=responses.len()).contains(&n) => {
                        commands.trigger(ChooseResponse {
                            entity,
                            index: n - 1,
                        });
                    }
                    _ => {
                        print!("pick 1..{} > ", responses.len());
                        let _ = std::io::stdout().flush();
                    }
                },
                _ => {}
            }
        }
    }
}
