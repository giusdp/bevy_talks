use bevy::prelude::*;
use bevy_talks::prelude::*;

#[derive(Resource)]
struct ScreenplayHandle(Handle<Screenplay>);

#[derive(Resource)]
struct PrintEnabled(bool);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TalksPlugin)
        .insert_resource(PrintEnabled(true))
        .add_startup_system(setup)
        .add_system(interact)
        .add_system(print)
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<Screenplay> = server.load("talk_only.json");
    commands.insert_resource(ScreenplayHandle(handle));

    println!("Press space to advance the conversation.");
}

fn print(
    mut print_enabled: ResMut<PrintEnabled>,
    screenplays: Res<Assets<Screenplay>>,
    sp_handle: Res<ScreenplayHandle>,
) {
    if !print_enabled.0 {
        return;
    }
    if let Some(conversation) = screenplays.get(&sp_handle.0) {
        println!(
            "{}: {}",
            conversation
                .first_actor()
                .map(|a| a.name)
                .unwrap_or("Narrator".to_string()),
            conversation.text()
        );
        print_enabled.0 = false;
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    sp_handle: ResMut<ScreenplayHandle>,
    mut screenplays: ResMut<Assets<Screenplay>>,
    mut print_enabled: ResMut<PrintEnabled>,
) {
    if input.just_pressed(KeyCode::Space) {
        let script = screenplays.get_mut(&sp_handle.0).unwrap();
        match script.next_action() {
            Ok(_) => print_enabled.0 = true,
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}
