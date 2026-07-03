//! Plays `assets/shop.dialogue.ron` in the terminal: a merchant whose
//! dialogue is driven by conditions and scripts.
//!
//! Run with `cargo run --example shop`. Press Enter to advance NPC lines and
//! type a number to pick a menu response.
//!
//! What to watch for:
//! - The greeting changes after the first visit (`Greeted` variable).
//! - "Buy the sword" only appears while you can afford it and don't own it
//!   (`gold()` is a game system called from the condition).
//! - Buying runs a script that spends gold and calls `give_item`, which
//!   pushes into a game resource.

use std::io::{BufRead, Write};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_talks::prelude::*;

/// Lines typed by the player, fed from the stdin thread.
#[derive(Resource)]
struct StdinInput(Mutex<mpsc::Receiver<String>>);

/// The player's gold. Game state, not a dialogue variable.
#[derive(Resource)]
struct Purse(f64);

/// What the player carries.
#[derive(Resource, Default)]
struct Inventory(Vec<String>);

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
    .insert_resource(Purse(12.0))
    .init_resource::<Inventory>()
    .add_systems(Startup, start)
    .add_systems(Update, handle_input)
    .add_observer(print_subtitle)
    .add_observer(print_menu)
    .add_observer(finish);

    // The systems dialogue logic can call.
    app.add_dialogue_system("gold", gold);
    app.add_dialogue_system("spend", spend);
    app.add_dialogue_system("give_item", give_item);

    app.run();
}

/// `gold()`: how much the player carries.
fn gold(_: In<()>, purse: Res<Purse>) -> f64 {
    purse.0
}

/// `spend(amount)`: takes gold from the purse.
fn spend(In(amount): In<f64>, mut purse: ResMut<Purse>) {
    purse.0 -= amount;
}

/// `give_item(name)`: puts an item in the player's inventory.
fn give_item(In(name): In<String>, mut inventory: ResMut<Inventory>) {
    println!("  * {name} added to your inventory");
    inventory.0.push(name);
}

/// Loads the database and spawns the conversation runner.
fn start(mut commands: Commands, assets: Res<AssetServer>) {
    println!("--- bevy_talks shop demo ---");
    let database: Handle<DialogueDatabase> = assets.load("shop.dialogue.ron");
    commands.spawn(DialogueRunner::new(
        database,
        ConversationRef::Title("Shop".to_owned()),
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

/// Prints the response menu, the purse, and prompts for a number.
fn print_menu(menu: On<ResponseMenuOpened>, purse: Res<Purse>, inventory: Res<Inventory>) {
    println!("\nYour reply ({} gold, carrying: {:?}):", purse.0, inventory.0);
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
