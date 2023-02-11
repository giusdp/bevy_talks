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
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<Conversation> = server.load("choices.json");
    commands.insert_resource(ConvoHandle(handle));
}

fn print(
    mut print_enabled: ResMut<PrintEnabled>,
    conversations: Res<Assets<Conversation>>,
    convo_handle: Res<ConvoHandle>,
) {
    let convo = conversations.get(&convo_handle.0).unwrap();
    if print_enabled.0 {
        if convo.at_player_action() {
            println!("Choices:");
            for (i, choice) in convo.choices().unwrap().iter().enumerate() {
                println!("{}: {}", i + 1, choice.text);
            }
        } else {
            println!(
                "{}: {}",
                convo
                    .current_first_actor()
                    .map(|a| a.name)
                    .unwrap_or("".to_string()),
                convo.current_text()
            );
        }
        print_enabled.0 = false;
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    convo_handle: ResMut<ConvoHandle>,
    mut conversations: ResMut<Assets<Conversation>>,
    mut print_enabled: ResMut<PrintEnabled>,
) {
    let convo = conversations.get_mut(&convo_handle.0).unwrap();

    if convo.at_player_action() {
        if input.just_pressed(KeyCode::Key1) {
            choose(convo, convo.choices().unwrap()[0].next, &mut print_enabled);
        } else if input.just_pressed(KeyCode::Key2) {
            choose(convo, convo.choices().unwrap()[1].next, &mut print_enabled);
        }
    } else if input.just_pressed(KeyCode::Space) {
        match convo.next_action() {
            Ok(_) => print_enabled.0 = true,
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

fn choose(convo: &mut Conversation, id: ActionId, print: &mut PrintEnabled) {
    match convo.jump_to(id) {
        Ok(_) => print.0 = true,
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
