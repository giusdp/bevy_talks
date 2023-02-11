use bevy::prelude::*;
use bevy_talks::prelude::*;

#[derive(Resource)]
struct ConvoHandle(Handle<Conversation>);

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
        .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<Conversation> = server.load("talk_only.json");
    commands.insert_resource(ConvoHandle(handle));
}

fn print(
    mut print_enabled: ResMut<PrintEnabled>,
    conversations: Res<Assets<Conversation>>,
    convo_handle: Res<ConvoHandle>,
) {
    if print_enabled.0 {
        let conversation = conversations.get(&convo_handle.0).unwrap();
        println!(
            "{}: {}",
            conversation
                .current_first_actor()
                .map(|a| a.name)
                .unwrap_or("Narrator".to_string()),
            conversation.current_text()
        );
        print_enabled.0 = false;
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    convo_handle: ResMut<ConvoHandle>,
    mut conversations: ResMut<Assets<Conversation>>,
    mut print_enabled: ResMut<PrintEnabled>,
) {
    let conversation = conversations.get_mut(&convo_handle.0).unwrap();

    if input.just_pressed(KeyCode::Space) {
        match conversation.next_action() {
            Ok(_) => print_enabled.0 = true,
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}
