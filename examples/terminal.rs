//! Plays `assets/test.dialogue.ron` in the terminal.
//!
//! Run with `cargo run --example terminal`. Press Enter to advance NPC lines
//! and type a number to pick a menu response.

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

    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))),
            AssetPlugin::default(),
            TalksPlugin,
        ))
        .insert_resource(StdinInput(Mutex::new(rx)))
        .add_systems(Startup, start)
        .add_systems(Update, handle_input)
        .add_observer(print_subtitle)
        .add_observer(print_menu)
        .add_observer(finish)
        .run();
}

/// Loads the database and spawns the conversation runner.
fn start(mut commands: Commands, assets: Res<AssetServer>) {
    println!("--- bevy_talks terminal demo ---");
    let database: Handle<DialogueDatabase> = assets.load("test.dialogue.ron");
    commands.spawn(DialogueRunner::new(
        database,
        ConversationRef::Title("Test".to_owned()),
    ));
}

/// Prints an NPC line and prompts for Enter.
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
    print!("  [Enter to continue] ");
    let _ = std::io::stdout().flush();
}

/// Prints the response menu and prompts for a number.
fn print_menu(menu: On<ResponseMenuOpened>) {
    println!("\nYour reply:");
    for (i, response) in menu.responses.iter().enumerate() {
        println!("  {}) {}", i + 1, response.text);
    }
    print!("> ");
    let _ = std::io::stdout().flush();
}

/// Says goodbye and quits.
fn finish(_: On<ConversationEnded>, mut exit: MessageWriter<AppExit>) {
    println!("\n--- conversation ended ---");
    exit.write(AppExit::Success);
}

/// Maps typed lines onto the runner: Enter advances, a number chooses.
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
                    commands.trigger(AdvanceConversation { entity });
                }
                Phase::AwaitingChoice { responses } => match line.trim().parse::<usize>() {
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
